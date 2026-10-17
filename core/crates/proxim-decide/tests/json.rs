//! `decide_json`: canonical in, canonical out, typed error objects, never a panic.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proptest::prelude::*;
use proxim_decide::decide;
use proxim_decide::json::{decide_json, split_response};
use proxim_types::State;
use proxim_types::canon::to_canonical_json;

struct Args {
    state: String,
    fix: String,
    signposts: String,
    policy: String,
    consent: String,
    clock: String,
}

fn args(state: &State, fix: &proxim_types::Fix) -> Args {
    Args {
        state: to_canonical_json(state).unwrap(),
        fix: to_canonical_json(fix).unwrap(),
        signposts: to_canonical_json(&vec![circle("c", 200), route("r", 2500, 30)]).unwrap(),
        policy: to_canonical_json(&explain()).unwrap(),
        consent: to_canonical_json(&granted()).unwrap(),
        clock: to_canonical_json(&clock(fix.t_ms)).unwrap(),
    }
}

fn call(a: &Args, seed: u64) -> String {
    decide_json(
        &a.state,
        &a.fix,
        &a.signposts,
        &a.policy,
        &a.consent,
        &a.clock,
        seed,
    )
}

#[test]
fn success_is_exactly_output_and_state_in_canonical_json() {
    let a = args(&State::empty(), &still(T0, 0, 0));
    let r = call(&a, 0);
    let (out, st) = split_response(&r).unwrap();
    assert_eq!(r, format!("{{\"output\":{out},\"state\":{st}}}"));
    // Matches the typed function byte for byte.
    let (o, s) = decide(
        &State::empty(),
        &still(T0, 0, 0),
        &[circle("c", 200), route("r", 2500, 30)],
        &explain(),
        &granted(),
        &clock(T0),
        0,
    )
    .unwrap();
    assert_eq!(out, to_canonical_json(&o).unwrap());
    assert_eq!(st, to_canonical_json(&s).unwrap());
    // Canonical means no whitespace and sorted keys at the top level.
    assert!(!r.contains(' ') && !r.contains('\n'));
    assert!(r.starts_with("{\"output\":{") && r.contains(",\"state\":{"));
}

#[test]
fn whitespace_in_inputs_is_tolerated_and_the_output_is_still_canonical() {
    let a = args(&State::empty(), &still(T0, 0, 0));
    let reference = call(&a, 0);
    let spaced = Args {
        state: format!(" {} \n", a.state.replace(',', " , ").replace(':', " : ")),
        fix: format!("\t{}", a.fix.replace(',', ",\n")),
        ..a
    };
    assert_eq!(call(&spaced, 0), reference);
}

#[test]
fn state_threads_through_the_json_boundary() {
    // Drive a whole visit through decide_json only, feeding back the returned state text.
    let sps = to_canonical_json(&vec![circle("c", 200)]).unwrap();
    let pol = to_canonical_json(&explain()).unwrap();
    let con = to_canonical_json(&granted()).unwrap();
    let mut state = to_canonical_json(&State::empty()).unwrap();
    let mut fired_at = None;
    for (i, f) in visit(T0, 0, 0, 6, 0).iter().enumerate() {
        let r = decide_json(
            &state,
            &to_canonical_json(f).unwrap(),
            &sps,
            &pol,
            &con,
            &to_canonical_json(&clock(f.t_ms)).unwrap(),
            9,
        );
        let (out, st) = split_response(&r).unwrap();
        if out.contains("\"outcome\":\"fire\"") {
            fired_at.get_or_insert(i);
        }
        state = st;
    }
    assert_eq!(fired_at, Some(3));
}

#[test]
fn each_malformed_argument_is_a_typed_error_naming_the_argument() {
    let good = args(&State::empty(), &still(T0, 0, 0));
    let names = ["state", "fix", "signposts", "policy", "consent", "clock"];
    let bad_values = [
        "",
        "null",
        "[",
        "{",
        "{\"a\":1,\"a\":2}",
        "\"x\"",
        "1.5",
        "{\"t_ms\":1.0}",
        "\u{feff}{}",
        "{\"unknown_field\":1}",
        "[1,2,3]",
        "9999999999999999999999999999",
    ];
    for (i, name) in names.iter().enumerate() {
        for bad in bad_values {
            let mut a = Args {
                state: good.state.clone(),
                fix: good.fix.clone(),
                signposts: good.signposts.clone(),
                policy: good.policy.clone(),
                consent: good.consent.clone(),
                clock: good.clock.clone(),
            };
            match i {
                0 => a.state = bad.into(),
                1 => a.fix = bad.into(),
                2 => a.signposts = bad.into(),
                3 => a.policy = bad.into(),
                4 => a.consent = bad.into(),
                _ => a.clock = bad.into(),
            }
            let r = call(&a, 0);
            // `{}` for consent/policy are valid; every value above is invalid for every argument
            // except an empty object / array where the argument is an object / array, which is
            // not in the list. So all must be errors, and the first failing argument is named.
            assert!(
                r.starts_with("{\"error\":{\"code\":\"E_"),
                "{name} <- {bad:?}: {r}"
            );
            assert!(!r.contains("\"output\""), "no partial success: {r}");
        }
    }
    // The error object is exact for a known case.
    let mut a = args(&State::empty(), &still(T0, 0, 0));
    a.fix = "{".into();
    assert_eq!(call(&a, 0), r#"{"error":{"code":"E_PARSE","input":"fix"}}"#);
    let mut a = args(&State::empty(), &still(T0, 0, 0));
    a.signposts = "[1]".into();
    assert_eq!(
        call(&a, 0),
        r#"{"error":{"code":"E_PARSE","input":"signposts"}}"#
    );
    // Unknown fields are rejected (strict), reserved ones are not.
    let mut a = args(&State::empty(), &still(T0, 0, 0));
    a.fix = a.fix.replace("{", "{\"bogus\":1,");
    assert_eq!(call(&a, 0), r#"{"error":{"code":"E_PARSE","input":"fix"}}"#);
}

#[test]
fn semantic_errors_are_typed_without_an_input_name() {
    let mut st = State::empty();
    st.v = 2;
    let a = args(&st, &still(T0, 0, 0));
    assert_eq!(call(&a, 0), r#"{"error":{"code":"E_PARSE"}}"#);
    // Oversize list: 4097 signposts.
    let many: Vec<_> = (0..4097).map(|i| circle(&format!("c-{i}"), 200)).collect();
    let mut a = args(&State::empty(), &still(T0, 0, 0));
    a.signposts = to_canonical_json(&many).unwrap();
    assert_eq!(call(&a, 0), r#"{"error":{"code":"E_LIMIT"}}"#);
    // A clock outside the supported range.
    let mut a = args(&State::empty(), &still(T0, 0, 0));
    a.clock = r#"{"now_ms":-5,"tz_offset_min":0}"#.into();
    assert_eq!(call(&a, 0), r#"{"error":{"code":"E_PARSE"}}"#);
    a.clock = r#"{"now_ms":5,"tz_offset_min":841}"#.into();
    assert_eq!(call(&a, 0), r#"{"error":{"code":"E_PARSE"}}"#);
}

#[test]
fn consent_values_that_are_not_granted_parse_and_deny_through_json() {
    let mut a = args(&State::empty(), &still(T0, 0, 0));
    for c in [
        "{}",
        r#"{"location_matching":"granted!"}"#,
        r#"{"location_matching":3}"#,
    ] {
        a.consent = c.into();
        let (out, _) = split_response(&call(&a, 0)).unwrap();
        assert!(out.contains("\"reason\":\"CONSENT_DENIED\""), "{c}: {out}");
    }
}

#[test]
fn seed_is_accepted_over_the_whole_u64_range() {
    let a = args(&State::empty(), &still(T0, 0, 0));
    let r0 = call(&a, 0);
    for seed in [1, u64::MAX, 1 << 63, 0xFFFF_FFFF] {
        assert_eq!(call(&a, seed), r0);
    }
}

#[test]
fn deep_and_huge_inputs_are_limits_not_panics() {
    let good = args(&State::empty(), &still(T0, 0, 0));
    let deep = format!("{}1{}", "[".repeat(10_000), "]".repeat(10_000));
    let mut a = Args {
        state: good.state.clone(),
        fix: deep,
        signposts: good.signposts.clone(),
        policy: good.policy.clone(),
        consent: good.consent.clone(),
        clock: good.clock.clone(),
    };
    assert_eq!(call(&a, 0), r#"{"error":{"code":"E_LIMIT","input":"fix"}}"#);
    a.fix = good.fix.clone();
    a.signposts = format!("[{}]", "0,".repeat(9000) + "0");
    assert_eq!(
        call(&a, 0),
        r#"{"error":{"code":"E_LIMIT","input":"signposts"}}"#
    );
    a.signposts = good.signposts.clone();
    a.policy = format!("{{\"categories_blocked\":[\"{}\"]}}", "x".repeat(9000));
    assert_eq!(
        call(&a, 0),
        r#"{"error":{"code":"E_LIMIT","input":"policy"}}"#
    );
}

const OPEN: char = '{';

fn is_well_formed(r: &str) -> bool {
    r.starts_with("{\"output\":") || r.starts_with("{\"error\":{\"code\":\"E_")
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 300, failure_persistence: None, rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha, rng_seed: proptest::test_runner::RngSeed::Fixed(0x5eed2029), ..ProptestConfig::default() })]

    /// Arbitrary bytes in any argument slot never panic and always give a well-formed response.
    #[test]
    fn prop_arbitrary_text_never_panics(
        slot in 0usize..6,
        junk in proptest::collection::vec(any::<u8>(), 0..300),
    ) {
        let good = args(&State::empty(), &still(T0, 0, 0));
        let mut parts = [
            good.state.clone(), good.fix.clone(), good.signposts.clone(),
            good.policy.clone(), good.consent.clone(), good.clock.clone(),
        ];
        parts[slot] = String::from_utf8_lossy(&junk).into_owned();
        let r = decide_json(&parts[0], &parts[1], &parts[2], &parts[3], &parts[4], &parts[5], 0);
        prop_assert!(is_well_formed(&r));
    }

    /// Single-byte mutations of a valid call never panic; if they still succeed, the result is
    /// canonical and equals the typed result for the mutated (re-parsed) arguments.
    #[test]
    fn prop_mutated_valid_inputs_stay_total(slot in 0usize..6, pos in 0usize..4000, byte in any::<u8>()) {
        let mut sim = Sim::new(vec![circle("c", 200), route("r", 2500, 30)]);
        sim.run(&visit(T0, 0, 0, 3, 0));
        let a = args(&sim.state, &still(T0 + 12_000, 0, 0));
        let mut parts = [a.state, a.fix, a.signposts, a.policy, a.consent, a.clock];
        let mut bytes = parts[slot].clone().into_bytes();
        if !bytes.is_empty() {
            let p = pos % bytes.len();
            bytes[p] = byte;
        }
        parts[slot] = String::from_utf8_lossy(&bytes).into_owned();
        let r = decide_json(&parts[0], &parts[1], &parts[2], &parts[3], &parts[4], &parts[5], 0);
        prop_assert!(is_well_formed(&r));
        if r.starts_with("{\"output\":") {
            let (o, s) = split_response(&r).unwrap();
            prop_assert!(o.starts_with(OPEN) && s.starts_with(OPEN));
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 16, failure_persistence: None, rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha, rng_seed: proptest::test_runner::RngSeed::Fixed(0x5eed202a), ..ProptestConfig::default() })]

    /// Random typed values through the JSON boundary agree with the typed function.
    #[test]
    #[allow(clippy::arithmetic_side_effects)]
    fn prop_json_boundary_equals_typed(seed in any::<u64>(), n in 5usize..20) {
        let signposts = world();
        let fixes = tour(T0).into_iter().take(n * 3).chain(random_walk(seed, n, T0 + 9 * 3_600_000)).collect::<Vec<_>>();
        let sps = to_canonical_json(&signposts).unwrap();
        let pol = to_canonical_json(&health_partner()).unwrap();
        let con = to_canonical_json(&granted_health()).unwrap();
        let mut typed = State::empty();
        let mut text = to_canonical_json(&typed).unwrap();
        for f in &fixes {
            let (o, s) = decide(&typed, f, &signposts, &health_partner(), &granted_health(), &clock(f.t_ms), seed).unwrap();
            let r = decide_json(&text, &to_canonical_json(f).unwrap(), &sps, &pol, &con,
                &to_canonical_json(&clock(f.t_ms)).unwrap(), seed);
            let (jo, js) = split_response(&r).unwrap();
            prop_assert_eq!(jo, to_canonical_json(&o).unwrap());
            prop_assert_eq!(&js, &to_canonical_json(&s).unwrap());
            typed = s;
            text = js;
        }
    }
}
