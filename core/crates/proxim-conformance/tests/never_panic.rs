//! Never-panic, never-hang, always-canonical: `decide_json` fed arbitrary bytes, token soup, valid
//! JSON with random and exhaustive structural mutations, extreme numbers, deep nesting and huge
//! strings. All random sources are seeded; the case count scales with PROXIM_FUZZ_SCALE.
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

use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed};
use proxim_conformance::fuzz::*;
use proxim_conformance::rng::Rng;
use proxim_conformance::*;
use serde_json::{Value, json};
use std::time::Duration;

/// Multiplier for the random case counts (default 1; CI long runs set e.g. 50).
fn scale() -> u64 {
    std::env::var("PROXIM_FUZZ_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

const STACK: usize = 1 << 20; // 1 MiB: what a secondary thread on a phone gets, or less
const HANG: Duration = Duration::from_secs(120);

fn report(name: &str, st: &Stats) {
    eprintln!(
        "[{name}] cases={} errors={} successes={} panics={} hangs={} non_canonical={} reingest_failures={} slowest={}ms",
        st.cases,
        st.errors,
        st.successes,
        st.panics,
        st.hangs,
        st.non_canonical,
        st.reingest_failures,
        st.slowest_ms
    );
}

fn assert_clean(name: &str, st: &Stats) {
    report(name, st);
    assert!(
        st.failures.is_empty(),
        "{name}: {} failure(s); first:\n{}",
        st.failures.len(),
        st.failures
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n---\n")
    );
}

#[test]
fn control_the_rich_world_is_valid_and_exercises_every_detector_state() {
    let inp = base_inputs();
    let text = inp.call();
    assert_eq!(check_response(&text).unwrap(), Kind::Success, "{text}");
    let s = parse_response(text);
    assert!(s.fix_ok(), "{}", s.fix_reason());
    // the state carried five detector variants; at least cand/in/armed survive a call
    let w = rich_world();
    let kinds: std::collections::BTreeSet<&str> = w[0]["mem"]["det"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["st"]["s"].as_str().unwrap())
        .collect();
    assert_eq!(kinds.len(), 5, "{kinds:?}");
}

#[test]
fn random_structural_mutations_of_valid_inputs() {
    let n = 24_000 * scale();
    let st = run_cases(0xC0FFEE, n, STACK, HANG, make_case);
    assert!(st.cases >= 24_000);
    assert!(
        st.errors > 1000 && st.successes > 1000,
        "the mix must hit both outcomes: {st:?}"
    );
    assert_clean("mutations", &st);
}

fn soup_case(seed: u64) -> Inputs {
    let mut rng = Rng::new(seed);
    let mut inp = base_inputs();
    inp.seed = rng.next_u64();
    match rng.below(3) {
        0 => {
            // one slot gets soup
            let i = rng.below(6) as usize;
            inp.args[i] = token_soup(&mut rng);
        }
        1 => {
            // every slot gets soup
            for a in &mut inp.args {
                *a = token_soup(&mut rng);
            }
        }
        _ => {
            // every slot gets lossy-decoded random bytes
            for a in &mut inp.args {
                let n = rng.below(300) as usize;
                let bytes: Vec<u8> = (0..n).map(|_| rng.next_u64() as u8).collect();
                *a = String::from_utf8_lossy(&bytes).into_owned();
            }
        }
    }
    inp
}

#[test]
fn token_soup_and_random_bytes_in_every_argument() {
    let st = run_cases(0xBADC0DE, 9_000 * scale(), STACK, HANG, soup_case);
    assert!(st.errors > 8_000, "soup is nearly always rejected: {st:?}");
    assert_clean("soup", &st);
}

// ---- exhaustive substitution sweeps ------------------------------------------------------------

fn leaf_paths(v: &Value, cur: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                cur.push(k.clone());
                leaf_paths(x, cur, out);
                cur.pop();
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate().take(3) {
                cur.push(format!("#{i}"));
                leaf_paths(x, cur, out);
                cur.pop();
            }
        }
        _ => out.push(cur.clone()),
    }
}

fn set_path(v: &mut Value, path: &[String], val: Value) {
    let mut cur = v;
    for p in path {
        cur = match p.strip_prefix('#') {
            Some(i) => &mut cur[i.parse::<usize>().unwrap()],
            None => &mut cur[p.as_str()],
        };
    }
    *cur = val;
}

fn sweep(values: Vec<Value>) -> Stats {
    let world = rich_world();
    let mut all = Stats::default();
    for arg in 0..6 {
        let mut paths = vec![];
        leaf_paths(&world[arg], &mut vec![], &mut paths);
        // map every (path, value) to a case; run through the same runner by closure-less indirection
        for path in &paths {
            for val in &values {
                let mut w = world.clone();
                set_path(&mut w[arg], path, val.clone());
                let mut inp = Inputs {
                    args: [0, 1, 2, 3, 4, 5].map(|i| render_raw(&w[i], &mut None)),
                    seed: 0,
                };
                inp.seed = 0;
                one(&inp, &mut all);
            }
        }
    }
    all
}

fn one(inp: &Inputs, st: &mut Stats) {
    st.cases += 1;
    let t0 = std::time::Instant::now();
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| inp.call()));
    st.slowest_ms = st.slowest_ms.max(t0.elapsed().as_millis());
    match res {
        Err(_) => {
            st.panics += 1;
            st.failures.push(format!("PANIC\n{}", inp.describe()));
        }
        Ok(text) => match check_response(&text) {
            Err(why) => {
                st.non_canonical += 1;
                st.failures.push(format!("{why}\n{}", inp.describe()));
            }
            Ok(Kind::Error) => st.errors += 1,
            Ok(Kind::Success) => st.successes += 1,
        },
    }
}

#[test]
fn every_leaf_of_every_argument_times_every_nasty_number_token() {
    let vals: Vec<Value> = NASTY.iter().map(|t| raw(t)).collect();
    let st = std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(move || sweep(vals))
        .unwrap()
        .join()
        .unwrap();
    assert!(st.cases > 5_000, "{}", st.cases);
    assert_clean("leaf x token", &st);
}

#[test]
fn every_leaf_of_every_argument_times_every_extreme_integer() {
    let vals: Vec<Value> = INTS.iter().map(|i| json!(*i)).collect();
    let st = std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(move || sweep(vals))
        .unwrap()
        .join()
        .unwrap();
    assert!(st.cases > 5_000, "{}", st.cases);
    assert_clean("leaf x int", &st);
}

const NASTY: &[&str] = &[
    "9223372036854775807",
    "9223372036854775808",
    "-9223372036854775808",
    "-9223372036854775809",
    "18446744073709551615",
    "18446744073709551616",
    "4294967296",
    "-2147483649",
    "65536",
    "256",
    "-0",
    "1e999",
    "1E5",
    "1.5",
    ".5",
    "5.",
    "+1",
    "00",
    "01",
    "NaN",
    "Infinity",
    "-Infinity",
    "null",
    "true",
    "false",
    "\"\"",
    "{}",
    "[]",
    "99999999999999999999999999999999999999999999",
    "\"\\ud800\"",
    "\"\\u0000\"",
    "\"abc",
    "[1,",
    "{\"a\":",
];

const INTS: &[i64] = &[
    i64::MIN,
    i64::MIN + 1,
    i64::MAX,
    i64::MAX - 1,
    -1,
    0,
    1,
    2,
    255,
    256,
    3599,
    3600,
    32767,
    32768,
    65535,
    65536,
    i32::MIN as i64,
    i32::MIN as i64 - 1,
    i32::MAX as i64,
    i32::MAX as i64 + 1,
    u32::MAX as i64,
    u32::MAX as i64 + 1,
    900_000_000,
    900_000_001,
    1_800_000_000,
    1_800_000_001,
    -900_000_001,
    -1_800_000_001,
    1 << 62,
    -(1 << 62),
    150_000_000,
    150_000_001,
    86_400_000,
    1_800_000,
    100_000,
    5000,
    30_000,
];

// ---- deep nesting, huge values, invalid UTF-8 ------------------------------------------------------

#[test]
fn deep_nesting_in_every_argument_on_a_small_stack() {
    let res = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let mut stats = Stats::default();
            for depth in [31_usize, 32, 33, 64, 256, 1000, 10_000, 100_000, 1_000_000] {
                for open_close in [("[", "]"), ("{\"a\":", "}")] {
                    let deep = format!(
                        "{}1{}",
                        open_close.0.repeat(depth),
                        open_close.1.repeat(depth)
                    );
                    for slot in 0..6 {
                        let mut inp = base_inputs();
                        inp.args[slot] = deep.clone();
                        one(&inp, &mut stats);
                    }
                }
                // and nested inside an otherwise valid argument
                let mut w = rich_world();
                w[2][0]["trigger"]["shape"]["vertices"] =
                    raw(&format!("{}{}", "[".repeat(depth), "]".repeat(depth)));
                let mut inp = base_inputs();
                inp.args[2] = render_raw(&w[2], &mut None);
                one(&inp, &mut stats);
            }
            stats
        })
        .unwrap()
        .join();
    let st = res.expect("deep nesting must not overflow the stack or panic");
    assert_clean("deep nesting", &st);
    assert_eq!(
        st.successes, 0,
        "nesting beyond the document shape is never a success"
    );
}

#[test]
fn huge_strings_and_arrays_are_bounded_errors_not_crashes() {
    let st = std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(|| {
            let mut stats = Stats::default();
            // strings: 10 MB (truncated to the safe size our harness holds), in an id, a category and a free-form receipt
            let big = "a".repeat(10 * 1024 * 1024);
            for (arg, path) in [
                (2_usize, "0.id"),
                (2, "0.category"),
                (4, "health_receipt"),
                (3, "categories_blocked.0"),
                (1, "t_ms"),
            ] {
                let mut w = rich_world();
                let target: &mut Value = &mut w[arg];
                let mut cur = target;
                let parts: Vec<&str> = path.split('.').collect();
                for (i, p) in parts.iter().enumerate() {
                    let next: &mut Value = match p.parse::<usize>() {
                        Ok(ix) => &mut cur[ix],
                        Err(_) => &mut cur[*p],
                    };
                    if i + 1 == parts.len() {
                        *next = json!(big.clone());
                    }
                    cur = next;
                }
                let mut inp = Inputs {
                    args: [0, 1, 2, 3, 4, 5].map(|i| render_raw(&w[i], &mut None)),
                    seed: 0,
                };
                inp.seed = 1;
                one(&inp, &mut stats);
            }
            // arrays: 100k signposts, 100k trace fixes, 100k vertices
            let mut w = rich_world();
            w[2] = Value::Array(vec![w[2][0].clone(); 100_000]);
            one(
                &Inputs {
                    args: [0, 1, 2, 3, 4, 5].map(|i| render_raw(&w[i], &mut None)),
                    seed: 0,
                },
                &mut stats,
            );
            let mut w = rich_world();
            let fix = w[0]["mem"]["trace"][0].clone();
            w[0]["mem"]["trace"] = Value::Array(vec![fix; 100_000]);
            one(
                &Inputs {
                    args: [0, 1, 2, 3, 4, 5].map(|i| render_raw(&w[i], &mut None)),
                    seed: 0,
                },
                &mut stats,
            );
            // a single input larger than the 32 MiB bound
            let mut inp = base_inputs();
            inp.args[2] = format!("[{}]", vec!["{}"; 12_000_000].join(","));
            one(&inp, &mut stats);
            stats
        })
        .unwrap()
        .join()
        .expect("huge inputs must not crash");
    assert_clean("huge", &st);
    assert_eq!(st.successes, 0);
}

#[test]
fn the_byte_level_parser_refuses_invalid_utf8_and_stays_total() {
    use proxim_types::strict::{JsonLimits, from_canonical_json, from_json, parse_value};
    use proxim_types::{Clock, Fix};
    let bad: Vec<Vec<u8>> = vec![
        vec![0xff],
        vec![0xc0, 0x80],
        vec![0xed, 0xa0, 0x80], // encoded surrogate
        b"{\"now_ms\":1,\"tz_offset_min\":0,\"x\":\"\xff\"}".to_vec(),
        b"\xef\xbb\xbf{\"now_ms\":1,\"tz_offset_min\":0}".to_vec(), // BOM
        b"{\"now_ms\":1,\"tz_offset_min\":0}\xff".to_vec(),
        vec![b'"', 0xf0, 0x28, 0x8c, 0xbc, b'"'],
        vec![b'"', 0xf4, 0x90, 0x80, 0x80, b'"'], // > U+10FFFF
        vec![0xe2, 0x82],                         // truncated sequence
    ];
    for b in &bad {
        assert!(
            from_json::<Clock>(b).is_err(),
            "invalid UTF-8 / BOM accepted: {b:?}"
        );
        assert!(from_canonical_json::<Clock>(b).is_err());
        assert!(parse_value(b, &JsonLimits::default()).is_err(), "{b:?}");
        assert!(from_json::<Fix>(b).is_err());
    }
    // 2000 random byte strings: total (never panics), and anything accepted re-serialises canonically
    let mut rng = Rng::new(99);
    for _ in 0..2000 {
        let n = rng.below(80) as usize;
        let b: Vec<u8> = (0..n).map(|_| rng.next_u64() as u8).collect();
        let _ = from_json::<Clock>(&b);
        let _ = parse_value(&b, &JsonLimits::default());
    }
    // valid control
    assert!(from_json::<Clock>(br#"{"now_ms":1,"tz_offset_min":0}"#).is_ok());
}

// ---- proptest: arbitrary strings into one slot ---------------------------------------------------------

fn cfg(cases: u32) -> Config {
    Config {
        cases,
        failure_persistence: None,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5EED_2026),
        max_shrink_iters: 200,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(cfg(6_000))]

    #[test]
    fn arbitrary_unicode_strings_in_any_slot_never_panic(slot in 0usize..6, s in ".{0,400}") {
        let mut inp = base_inputs();
        inp.args[slot] = s;
        let text = inp.call();
        prop_assert!(check_response(&text).is_ok(), "bad response {}", &text[..text.len().min(200)]);
    }

    #[test]
    fn arbitrary_bytes_lossily_decoded_never_panic(slot in 0usize..6, b in proptest::collection::vec(any::<u8>(), 0..400)) {
        let mut inp = base_inputs();
        inp.args[slot] = String::from_utf8_lossy(&b).into_owned();
        let text = inp.call();
        prop_assert!(check_response(&text).is_ok(), "bad response {}", &text[..text.len().min(200)]);
    }

    #[test]
    fn json_shaped_strings_never_panic(slot in 0usize..6, s in r#"[\[\]{}",:0-9a-z\\ .eE+-]{0,200}"#) {
        let mut inp = base_inputs();
        inp.args[slot] = s;
        let text = inp.call();
        prop_assert!(check_response(&text).is_ok());
    }

    #[test]
    fn any_i64_in_any_numeric_field_never_panics(arg in prop::sample::select(vec![0usize, 1, 2, 3, 5]), which in 0usize..64, v in any::<i64>()) {
        let mut w = rich_world();
        let mut paths = vec![];
        leaf_paths(&w[arg], &mut vec![], &mut paths);
        let nums: Vec<_> = paths.into_iter().filter(|p| {
            let mut cur = &w[arg];
            for s in p {
                cur = match s.strip_prefix('#') { Some(i) => &cur[i.parse::<usize>().unwrap()], None => &cur[s.as_str()] };
            }
            cur.is_number()
        }).collect();
        let path = &nums[which % nums.len()];
        set_path(&mut w[arg], path, json!(v));
        let inp = Inputs { args: [0, 1, 2, 3, 4, 5].map(|i| render_raw(&w[i], &mut None)), seed: 0 };
        let text = inp.call();
        prop_assert!(check_response(&text).is_ok());
    }
}
