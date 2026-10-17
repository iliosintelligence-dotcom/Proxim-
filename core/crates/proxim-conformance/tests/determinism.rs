//! Spec 2 and 9: determinism. Same inputs give the same bytes; input order, seed, thread, process
//! history and key order never change the output.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unreachable,
    clippy::print_stderr,
    clippy::disallowed_types,
    clippy::inconsistent_digit_grouping,
    clippy::unreadable_literal
)]

use proxim_conformance::rng::Rng;
use proxim_conformance::*;
use serde_json::{Value, json};

fn world(seed: u64) -> (Vec<Value>, Vec<(Value, i64)>) {
    let mut rng = Rng::new(seed);
    let sps = mixed_signposts(&mut rng);
    let mut trace = random_trace(&mut rng, 60);
    trace.extend(
        scripted_trace(&mut rng, &sps, 6)
            .into_iter()
            .map(|(mut f, n)| {
                f["t_ms"] = json!(f["t_ms"].as_i64().unwrap() + 100_000_000);
                (f, n + 100_000_000)
            }),
    );
    (sps, trace)
}

fn run_all(sps: &[Value], trace: &[(Value, i64)], policy: &Value, seed: u64) -> Vec<String> {
    let mut rig = Rig::new();
    rig.signposts = sps.to_vec();
    rig.policy = policy.clone();
    rig.seed = seed;
    rig.consent["nearby_health_events"] = json!("granted");
    rig.consent["health_receipt"] = json!("receipt-1");
    rig.policy["profile"] = json!("sealed");
    rig.policy["health_tier"] = json!(true);
    trace.iter().map(|(f, n)| rig.call(f, *n).text).collect()
}

fn policy_with_zone() -> Value {
    json!({"explain": true, "standoff_zones": [{"id": "z-1", "center": pos(offset(ORIGIN, 1_500_000, -1_500_000))}]})
}

#[test]
fn same_trace_twice_gives_identical_bytes() {
    for seed in [1_u64, 2, 3, 4] {
        let (sps, trace) = world(seed);
        let a = run_all(&sps, &trace, &policy_with_zone(), 0);
        let b = run_all(&sps, &trace, &policy_with_zone(), 0);
        assert_eq!(a.len(), b.len());
        for (i, (x, y)) in a.iter().zip(&b).enumerate() {
            assert_eq!(x, y, "world {seed} step {i}");
        }
        assert!(
            a.iter().any(|t| t.contains("\"outcome\":\"fire\"")),
            "world {seed} must contain a fire"
        );
    }
}

#[test]
fn shuffled_signpost_order_gives_identical_bytes() {
    for seed in [11_u64, 12, 13] {
        let (sps, trace) = world(seed);
        let want = run_all(&sps, &trace, &policy_with_zone(), 0);
        let mut rng = Rng::new(seed ^ 0xABCD);
        for round in 0..4 {
            let mut shuffled = sps.clone();
            rng.shuffle(&mut shuffled);
            let got = run_all(&shuffled, &trace, &policy_with_zone(), 0);
            for (i, (x, y)) in want.iter().zip(&got).enumerate() {
                assert_eq!(
                    x, y,
                    "world {seed} round {round} step {i}: input order changed the output"
                );
            }
        }
        // reversed too
        let mut rev = sps.clone();
        rev.reverse();
        assert_eq!(run_all(&rev, &trace, &policy_with_zone(), 0), want);
    }
}

#[test]
fn ids_sort_by_utf8_bytes_not_by_locale() {
    // uppercase < underscore < lowercase in ASCII; prefix sorts before its extension; '-' < '.' < digits < ':' etc.
    let ids = [
        "b-1", "B-1", "a", "a-1", "a.1", "a_1", "a:1", "a1", "A", "Z9", "_x", "0", "z",
    ];
    let mut sorted = ids.to_vec();
    sorted.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    let mk = |order: &[&str]| {
        let mut rig = Rig::new();
        rig.signposts = order.iter().map(|id| enter_circle(id, 200)).collect();
        for k in 0..6 {
            rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
        }
        rig
    };
    let rig = mk(&ids);
    let st = rig.state_json();
    let det: Vec<&str> = st["mem"]["det"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert_eq!(det, sorted, "detector entries are ascending by id bytes");
    let rig2 = mk(&sorted.iter().rev().copied().collect::<Vec<_>>());
    assert_eq!(rig.state, rig2.state);
    // explain notes also come out in id order
    let s = rig.history.first().unwrap();
    let notes: Vec<&str> = s.output["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["signpost"].as_str().unwrap())
        .collect();
    assert_eq!(notes, sorted);
}

#[test]
fn the_seed_is_carried_and_has_no_effect() {
    let (sps, trace) = world(21);
    let want = run_all(&sps, &trace, &policy_with_zone(), 0);
    for seed in [1_u64, 42, u64::MAX, 1 << 63] {
        assert_eq!(
            run_all(&sps, &trace, &policy_with_zone(), seed),
            want,
            "seed {seed}"
        );
    }
}

#[test]
fn threads_and_history_do_not_matter() {
    let (sps, trace) = world(31);
    let want = run_all(&sps, &trace, &policy_with_zone(), 0);
    // WebAssembly (WASI) has no threads: there the four runs go one after another.
    #[cfg(not(target_family = "wasm"))]
    {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let (sps, trace) = (sps.clone(), trace.clone());
                std::thread::spawn(move || run_all(&sps, &trace, &policy_with_zone(), 0))
            })
            .collect();
        for h in handles {
            assert_eq!(h.join().unwrap(), want, "a concurrent run differs");
        }
    }
    #[cfg(target_family = "wasm")]
    for _ in 0..4 {
        assert_eq!(
            run_all(&sps, &trace, &policy_with_zone(), 0),
            want,
            "a repeated run differs"
        );
    }
    // after unrelated work in between, the same call still gives the same bytes
    let mut rig = Rig::new();
    rig.signposts = sps.clone();
    let first = rig.peek(&trace[0].0, trace[0].1).text;
    for (f, n) in trace.iter().take(30) {
        rig.call(f, *n);
    }
    let fresh = Rig {
        state: empty_state(),
        ..rig.clone()
    };
    assert_eq!(fresh.peek(&trace[0].0, trace[0].1).text, first);
}

#[test]
fn feeding_the_state_back_verbatim_equals_re_canonicalising_it() {
    // the state is canonical text; parse + canonical write must be the identity on it
    let (sps, trace) = world(41);
    let mut rig = Rig::new();
    rig.signposts = sps.clone();
    let mut rig2 = rig.clone();
    for (f, n) in trace.iter().take(200) {
        let a = rig.call(f, *n);
        let b = rig2.call(f, *n);
        assert_eq!(a.text, b.text);
        if a.is_ok() {
            let again = render(&serde_json::from_str::<Value>(&rig.state).unwrap());
            assert_eq!(again, rig.state, "state text is not canonical");
            // the other rig re-canonicalises through a different JSON tree order each time
            rig2.state = again;
        }
    }
}

#[test]
fn key_order_and_insignificant_whitespace_in_inputs_do_not_change_the_result() {
    // Canonical inputs are the contract (3.2); a port may refuse others. If the engine accepts a
    // variant it must return the same bytes as for the canonical text.
    let c = Call::base();
    let canonical = c.run().text;
    let pretty = |v: &Value| serde_json::to_string_pretty(v).unwrap();
    let rev_keys = |v: &Value| -> String {
        fn go(v: &Value, out: &mut String) {
            match v {
                Value::Object(m) => {
                    out.push('{');
                    let mut keys: Vec<&String> = m.keys().collect();
                    keys.sort_by(|a, b| b.cmp(a));
                    for (i, k) in keys.iter().enumerate() {
                        if i > 0 {
                            out.push(',');
                        }
                        out.push_str(&serde_json::to_string(k).unwrap());
                        out.push(':');
                        go(&m[*k], out);
                    }
                    out.push('}');
                }
                Value::Array(a) => {
                    out.push('[');
                    for (i, x) in a.iter().enumerate() {
                        if i > 0 {
                            out.push(',');
                        }
                        go(x, out);
                    }
                    out.push(']');
                }
                other => out.push_str(&serde_json::to_string(other).unwrap()),
            }
        }
        let mut s = String::new();
        go(v, &mut s);
        s
    };
    let state: Value = serde_json::from_str(&c.state).unwrap();
    for f in [pretty, rev_keys] {
        let t = proxim_ffi::decide_json(
            &f(&state),
            &f(&c.fix),
            &f(&Value::Array(c.signposts.clone())),
            &f(&c.policy),
            &f(&c.consent),
            &f(&c.clock),
            0,
        );
        if !t.starts_with("{\"error\"") {
            assert_eq!(
                t, canonical,
                "an accepted non-canonical spelling must give canonical, identical output"
            );
        }
    }
}

#[test]
fn engine_spec_version_is_exposed_and_is_two() {
    assert_eq!(proxim_ffi::engine_spec(), 2);
    assert_eq!(proxim_types::ENGINE_SPEC, 2);
}

#[test]
fn duplicate_and_superseded_revisions_are_order_independent() {
    // same id twice with different revisions, and an exact duplicate: whatever the engine decides,
    // it must decide identically for every input order.
    let mut a = enter_circle("dup", 200);
    a["rev"] = json!(1);
    let mut b = enter_circle("dup", 300);
    b["rev"] = json!(2);
    let c = b.clone();
    let orders: [[&Value; 3]; 6] = [
        [&a, &b, &c],
        [&a, &c, &b],
        [&b, &a, &c],
        [&b, &c, &a],
        [&c, &a, &b],
        [&c, &b, &a],
    ];
    let mut results = std::collections::BTreeSet::new();
    for o in orders {
        let mut rig = Rig::new();
        rig.signposts = o.iter().map(|v| (*v).clone()).collect();
        let texts: Vec<String> = (0..6)
            .map(|k| rig.step(&fix_at(BASE_T + k * 1000, ORIGIN)).text)
            .collect();
        results.insert(texts.join("\n"));
    }
    assert_eq!(
        results.len(),
        1,
        "input order changed the output for duplicate revisions"
    );
}
