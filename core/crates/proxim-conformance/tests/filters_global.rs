//! Spec 7, global hard filters F01-F10 (first failure ends the call).
//!
//! For every filter: a case where exactly that filter rejects, the boundary on each side, and
//! (`ordering_*`) cases where two filters reject so the FIRST in spec order must win.
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

use proxim_conformance::*;
use serde_json::{Value, json};

/// 5 km north of the origin; far from every other fixture.
fn zone_point() -> P {
    offset(ORIGIN, 0, 5_000_000)
}

fn zone_policy(standoff_m: Option<i64>) -> Value {
    let mut z = json!({"id": "z-1", "center": pos(zone_point())});
    if let Some(s) = standoff_m {
        z["standoff_m"] = json!(s);
    }
    json!({"explain": true, "standoff_zones": [z]})
}

fn assert_global(step: &Step, code: &str) {
    assert!(step.is_ok(), "unexpected engine error: {}", step.text);
    assert!(
        !step.fix_ok(),
        "fix must be rejected with {code}: {}",
        step.text
    );
    assert_eq!(step.fix_reason(), code, "{}", step.text);
    assert!(step.fires().is_empty(), "a rejected call must not fire");
}

#[test]
fn baseline_is_accepted_and_ok() {
    let s = Call::base().run();
    assert!(s.is_ok() && s.fix_ok(), "{}", s.text);
    assert_eq!(s.fix_reason(), "OK");
    assert_eq!(s.output["v"], 1);
    assert_eq!(s.output["t_ms"], BASE_T);
    assert_eq!(s.trace_len(), 1);
}

// ---- F01 / F02 --------------------------------------------------------------------------------

#[test]
fn f01_consent_denied_wipes_mem_keeps_caps() {
    let (rig, last) = fired_state();
    let before = rig.state_json();
    assert!(before["caps"]["signposts"].as_array().unwrap().len() == 1);
    assert!(!before["mem"]["det"].as_array().unwrap().is_empty());
    for loc in [json!("denied"), json!("unknown")] {
        let mut c = Call::base();
        c.state = rig.state.clone();
        c.signposts = rig.signposts.clone();
        c.fix = fix_at(last + 1000, ORIGIN);
        c.clock = clock(last + 1000, 0);
        c.consent["location_matching"] = loc.clone();
        let s = c.run();
        assert_global(&s, "CONSENT_DENIED");
        assert_eq!(
            s.state["mem"],
            empty_mem(),
            "mem := empty (I4), consent {loc}"
        );
        assert_eq!(
            s.state["caps"], before["caps"],
            "caps kept byte for byte, consent {loc}"
        );
        assert!(s.decisions().is_empty());
    }
}

#[test]
fn f01_every_non_granted_value_fails_closed() {
    // Spec 3.2: any consent value other than exactly "granted" parses as `unknown` (I4).
    let strange: Vec<Value> = vec![
        json!("denied"),
        json!("unknown"),
        json!(""),
        json!("Granted"),
        json!("GRANTED"),
        json!("gRaNtEd"),
        json!(" granted"),
        json!("granted "),
        json!("granted\n"),
        json!("grant"),
        json!("granted1"),
        json!("true"),
        json!("yes"),
        json!("1"),
        json!("allow"),
        json!("granted\u{0}"),
        json!("gr\u{0430}nted"),  // Cyrillic a
        json!("\u{ff47}ranted"),  // full-width g
        json!("granted\u{200b}"), // zero-width space
        json!("\u{feff}granted"),
        json!(true),
        json!(1),
        json!(0),
        json!(-1),
        json!([]),
        json!(["granted"]),
        json!({}),
        json!({"v": "granted"}),
    ];
    for v in strange {
        let mut c = Call::base();
        c.consent["location_matching"] = v.clone();
        let s = c.run();
        // either a typed parse error (no decision) or a CONSENT_DENIED; never a processed fix
        if s.is_ok() {
            assert_global(&s, "CONSENT_DENIED");
            assert_eq!(s.state["mem"], empty_mem(), "value {v}");
        } else {
            assert_eq!(
                s.error.as_ref().unwrap()["code"],
                "E_PARSE",
                "value {v}: {}",
                s.text
            );
        }
    }
    // absent key, empty object
    let mut c = Call::base();
    unset(&mut c.consent, "location_matching");
    assert_global(&c.run(), "CONSENT_DENIED");
    let mut c = Call::base();
    c.consent = json!({});
    assert_global(&c.run(), "CONSENT_DENIED");
    // exactly "granted" is the only value that passes
    let s = Call::base().run();
    assert!(s.fix_ok());
    // `null` is not canonical JSON: it may be a parse error but must never mean "granted"
    let mut c = Call::base();
    let text = render(&c.consent).replace(
        "\"location_matching\":\"granted\"",
        "\"location_matching\":null",
    );
    let r = proxim_ffi::decide_json(
        &c.state,
        &render(&c.fix),
        &render(&Value::Array(c.signposts.clone())),
        &text,
        &render(&c.consent),
        &render(&c.clock),
        0,
    );
    c.consent = json!({});
    let p = parse_response(r);
    if p.is_ok() {
        assert_eq!(p.fix_reason(), "CONSENT_DENIED");
    }
}

#[test]
fn other_consent_scopes_do_not_unlock_location_matching() {
    let mut c = Call::base();
    c.consent = json!({
        "location_matching": "denied", "nearby_health_events": "granted",
        "aggregate_reporting": "granted", "local_notifications": "granted", "health_receipt": "r-1"
    });
    assert_global(&c.run(), "CONSENT_DENIED");
}

#[test]
fn f02_child_directed_disables_the_engine() {
    let (rig, last) = fired_state();
    let before = rig.state_json();
    let mut c = Call::base().with_policy("child_directed", json!(true));
    c.state = rig.state.clone();
    c.signposts = rig.signposts.clone();
    c.fix = fix_at(last + 1000, ORIGIN);
    c.clock = clock(last + 1000, 0);
    let s = c.run();
    assert_global(&s, "ENGINE_DISABLED");
    assert_eq!(s.state["mem"], empty_mem());
    assert_eq!(s.state["caps"], before["caps"]);
    // false does not disable
    let s = Call::base()
        .with_policy("child_directed", json!(false))
        .run();
    assert!(s.fix_ok());
}

// ---- F03 --------------------------------------------------------------------------------------

#[test]
fn f03_fix_invalid_ranges() {
    let bad = [
        ("lat_e7", json!(900_000_001)),
        ("lat_e7", json!(-900_000_001)),
        ("lon_e7", json!(1_800_000_001)),
        ("lon_e7", json!(-1_800_000_001)),
        ("course_ddeg", json!(3600)),
        ("course_ddeg", json!(65535)),
        ("t_ms", json!(-1)),
    ];
    for (k, v) in bad {
        let s = Call::base().with_fix(k, v.clone()).run();
        // a type-level rejection (u16 overflow etc.) is also a refusal; a range failure is F03
        if s.is_ok() {
            assert_global(&s, "FIX_INVALID");
            assert_eq!(s.trace_len(), 0, "an invalid fix is not retained ({k}={v})");
        } else {
            assert_eq!(s.error.as_ref().unwrap()["code"], "E_PARSE", "{k}={v}");
        }
    }
    // valid extremes are accepted
    for (k, v) in [
        ("lat_e7", json!(899_999_999)),
        ("lon_e7", json!(1_799_999_999)),
        ("lon_e7", json!(-1_799_999_999)),
        ("course_ddeg", json!(3599)),
    ] {
        let s = Call::base().with_fix(k, v.clone()).run();
        assert!(s.is_ok(), "{k}={v}: {}", s.text);
        assert!(s.fix_ok(), "{k}={v} is valid: {}", s.fix_reason());
    }
}

#[test]
fn f03_wrong_types_and_non_canonical_numbers_are_parse_errors_not_decisions() {
    for (k, tok) in [
        ("lat_e7", "\"450000000\""),
        ("lat_e7", "450000000.0"),
        ("lat_e7", "4.5e8"),
        ("lat_e7", "+450000000"),
        ("lat_e7", "0450000000"),
        ("lat_e7", "-0"),
        ("lat_e7", "null"),
        ("lat_e7", "NaN"),
        ("lat_e7", "Infinity"),
        ("lat_e7", "99999999999999999999999999"),
        ("lat_e7", "2147483648"),
        ("lat_e7", "-2147483649"),
        ("t_ms", "1.5"),
        ("t_ms", "9223372036854775808"),
        ("acc_mm", "-1"),
        ("acc_mm", "4294967296"),
        ("speed_mmps", "-1"),
        ("course_ddeg", "-1"),
        ("course_ddeg", "65536"),
        ("is_mock", "\"false\""),
        ("is_mock", "0"),
    ] {
        let s = Call::base().run_with_raw("fix", k, tok);
        assert!(!s.is_ok(), "{k}={tok} must be a typed error: {}", s.text);
        assert_eq!(s.error.as_ref().unwrap()["code"], "E_PARSE", "{k}={tok}");
        assert_eq!(s.error.as_ref().unwrap()["input"], "fix", "{k}={tok}");
    }
}

// ---- F04 --------------------------------------------------------------------------------------

#[test]
fn f04_mock_fix_unless_simulator_mode() {
    let s = Call::base().with_fix("is_mock", json!(true)).run();
    assert_global(&s, "FIX_MOCK");
    assert_eq!(s.trace_len(), 0);
    let s = Call::base()
        .with_fix("is_mock", json!(true))
        .with_policy("simulator_mode", json!(true))
        .run();
    assert!(
        s.fix_ok(),
        "simulator_mode accepts mock fixes: {}",
        s.fix_reason()
    );
    let s = Call::base().with_fix("is_mock", json!(false)).run();
    assert!(s.fix_ok());
    // simulator_mode must not exempt anything else
    let s = Call::base()
        .with_policy("simulator_mode", json!(true))
        .with_fix("lat_e7", json!(900_000_001))
        .run();
    assert!(s.is_ok() && !s.fix_ok());
}

// ---- F05 --------------------------------------------------------------------------------------

#[test]
fn f05_accuracy_too_coarse_default_and_narrowed() {
    assert!(
        Call::base()
            .with_fix("acc_mm", json!(100_000))
            .run()
            .fix_ok(),
        "100 m is the limit, inclusive"
    );
    assert_global(
        &Call::base().with_fix("acc_mm", json!(100_001)).run(),
        "FIX_ACCURACY_TOO_COARSE",
    );
    // partners may narrow
    let c = Call::base().with_policy("max_acc_mm", json!(50_000));
    assert!(c.clone().with_fix("acc_mm", json!(50_000)).run().fix_ok());
    assert_global(
        &c.with_fix("acc_mm", json!(50_001)).run(),
        "FIX_ACCURACY_TOO_COARSE",
    );
    // ... but never widen past the platform ceiling of 100 000 mm
    let c = Call::base().with_policy("max_acc_mm", json!(500_000));
    assert_global(
        &c.with_fix("acc_mm", json!(100_001)).run(),
        "FIX_ACCURACY_TOO_COARSE",
    );
    let c = Call::base().with_policy("max_acc_mm", json!(u32::MAX));
    assert_global(
        &c.with_fix("acc_mm", json!(100_001)).run(),
        "FIX_ACCURACY_TOO_COARSE",
    );
}

// ---- F06 / F07 --------------------------------------------------------------------------------

#[test]
fn f06_fix_from_future_boundary() {
    assert!(
        Call::base()
            .with_fix("t_ms", json!(BASE_T + 5000))
            .run()
            .fix_ok(),
        "t = now + 5000 is allowed"
    );
    assert_global(
        &Call::base().with_fix("t_ms", json!(BASE_T + 5001)).run(),
        "FIX_FROM_FUTURE",
    );
    assert_global(
        &Call::base()
            .with_fix("t_ms", json!(BASE_T + 3_600_000))
            .run(),
        "FIX_FROM_FUTURE",
    );
}

#[test]
fn f07_fix_stale_boundary() {
    assert!(
        Call::base()
            .with_fix("t_ms", json!(BASE_T - 30_000))
            .run()
            .fix_ok(),
        "age = 30000 is allowed"
    );
    assert_global(
        &Call::base().with_fix("t_ms", json!(BASE_T - 30_001)).run(),
        "FIX_STALE",
    );
    assert_global(
        &Call::base()
            .with_fix("t_ms", json!(BASE_T - 3_600_000))
            .run(),
        "FIX_STALE",
    );
}

// ---- F08 --------------------------------------------------------------------------------------

#[test]
fn f08_out_of_order_is_not_strictly_after_last() {
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 200));
    assert!(rig.step(&fix_at(BASE_T, ORIGIN)).fix_ok());
    let trace_len = rig.history.last().unwrap().trace_len();
    // equal time, earlier time: both rejected, nothing retained
    for t in [BASE_T, BASE_T - 1, BASE_T - 10_000] {
        let s = rig.call(&fix_at(t, ORIGIN), BASE_T);
        assert_global(&s, "FIX_OUT_OF_ORDER");
        assert_eq!(s.trace_len(), trace_len, "t={t}");
    }
    assert!(
        rig.call(&fix_at(BASE_T + 1, ORIGIN), BASE_T + 1).fix_ok(),
        "strictly later is accepted"
    );
}

// ---- F09 --------------------------------------------------------------------------------------

fn at(rig: &mut Rig, t: i64, north_m: i64, acc: i64) -> Step {
    let mut f = fix_at(t, offset(ORIGIN, 0, north_m * 1000));
    f["acc_mm"] = json!(acc);
    rig.step(&f)
}

#[test]
fn f09_jump_requires_speed_above_83_mps_and_distance_beyond_accuracies_plus_100_m() {
    // accuracies 5 m + 5 m, margin 100 m -> distance must exceed 110 m AND implied speed > 83 m/s
    let run = |dt_s: i64, north_m: i64| {
        let mut rig = Rig::new();
        assert!(at(&mut rig, BASE_T, 0, 5000).fix_ok());
        at(&mut rig, BASE_T + dt_s * 1000, north_m, 5000)
    };
    // 1 s, 1000 m: 1000 m/s and 1000 m > 110 m -> jump
    assert_global(&run(1, 1000), "FIX_JUMP");
    // 10 s, 900 m = 90 m/s -> jump
    assert_global(&run(10, 900), "FIX_JUMP");
    // 10 s, 800 m = 80 m/s (<= 83) -> accepted even though far
    assert!(run(10, 800).fix_ok());
    // 1 s, 100 m: fast (100 m/s) but within acc + 100 m = 110 m -> accepted
    assert!(run(1, 100).fix_ok());
    // 1 s, 150 m: beyond 110 m and 150 m/s -> jump
    assert_global(&run(1, 150), "FIX_JUMP");
    // gaps above 60 s are not compared (no trace fix within dt <= 60000)
    assert!(run(61, 20_000).fix_ok());
    // large accuracy widens the allowance: 40 m + 40 m + 100 m = 180 m
    let mut rig = Rig::new();
    at(&mut rig, BASE_T, 0, 40_000);
    assert!(
        at(&mut rig, BASE_T + 1000, 170, 40_000).fix_ok(),
        "170 m < 180 m"
    );
    let mut rig = Rig::new();
    at(&mut rig, BASE_T, 0, 40_000);
    assert_global(&at(&mut rig, BASE_T + 1000, 200, 40_000), "FIX_JUMP");
}

#[test]
fn f09_third_consecutive_reject_resets_trace_and_detectors_and_accepts() {
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 200));
    for k in 0..5 {
        rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
    }
    assert!(rig.history.last().unwrap().det("c-1").is_some());
    // 2 km away, one second apart: three consecutive rejects
    let far = offset(ORIGIN, 0, 2_000_000);
    let s1 = rig.step(&fix_at(BASE_T + 5000, far));
    assert_global(&s1, "FIX_JUMP");
    assert_eq!(s1.mem()["jump_rejects"], 1);
    let s2 = rig.step(&fix_at(BASE_T + 6000, far));
    assert_global(&s2, "FIX_JUMP");
    assert_eq!(s2.mem()["jump_rejects"], 2);
    assert!(
        s2.det("c-1").is_some(),
        "detectors survive the first two rejects"
    );
    let s3 = rig.step(&fix_at(BASE_T + 7000, far));
    assert!(
        s3.fix_ok(),
        "3rd reject accepts the fix: {}",
        s3.fix_reason()
    );
    assert_eq!(s3.trace_len(), 1, "trace reset to the accepted fix");
    assert!(s3.det("c-1").is_none(), "detectors reset");
    assert!(s3.mem()["jump_rejects"].is_null() || s3.mem()["jump_rejects"] == 0);
    // a good fix in between resets the consecutive counter
    let mut rig = Rig::new();
    at(&mut rig, BASE_T, 0, 5000);
    assert_global(&at(&mut rig, BASE_T + 1000, 2000, 5000), "FIX_JUMP");
    assert!(at(&mut rig, BASE_T + 2000, 10, 5000).fix_ok());
    assert_eq!(rig.history.last().unwrap().mem()["jump_rejects"], 0);
    assert_global(&at(&mut rig, BASE_T + 3000, 2000, 5000), "FIX_JUMP");
    assert_global(&at(&mut rig, BASE_T + 4000, 2000, 5000), "FIX_JUMP");
    assert!(at(&mut rig, BASE_T + 5000, 10, 5000).fix_ok());
}

// ---- F10 --------------------------------------------------------------------------------------

fn standoff_call(
    distance_m: i64,
    acc_mm: i64,
    explain: bool,
    standoff: Option<i64>,
) -> (Call, Step) {
    let mut c = Call::base();
    c.policy = zone_policy(standoff);
    c.policy["explain"] = json!(explain);
    let p = offset(zone_point(), 0, -distance_m * 1000);
    c.fix = fix_at(BASE_T, p);
    c.fix["acc_mm"] = json!(acc_mm);
    let s = c.run();
    (c, s)
}

#[test]
fn f10_standoff_boundary_is_standoff_plus_acc_eff() {
    // standoff 1000 m + acc_eff 10 m (acc 5 m is floored to 10 m): 1010 m
    assert_global(
        &standoff_call(980, 5000, true, None).1,
        "SENSITIVE_STANDOFF",
    );
    assert_global(
        &standoff_call(1005, 5000, true, None).1,
        "SENSITIVE_STANDOFF",
    );
    assert!(standoff_call(1020, 5000, true, None).1.fix_ok());
    assert!(standoff_call(1200, 5000, true, None).1.fix_ok());
    // accuracy widens the zone: acc 50 m -> 1050 m
    assert_global(
        &standoff_call(1040, 50_000, true, None).1,
        "SENSITIVE_STANDOFF",
    );
    assert!(standoff_call(1060, 50_000, true, None).1.fix_ok());
    // a zone may widen its own standoff
    assert_global(
        &standoff_call(1900, 5000, true, Some(2000)).1,
        "SENSITIVE_STANDOFF",
    );
    assert!(standoff_call(2100, 5000, true, Some(2000)).1.fix_ok());
    // dead centre
    assert_global(&standoff_call(0, 5000, true, None).1, "SENSITIVE_STANDOFF");
}

#[test]
fn f10_standoff_reports_only_generic_suppressed_without_explain() {
    let (_, s) = standoff_call(300, 5000, false, None);
    assert!(s.is_ok());
    assert!(!s.fix_ok());
    assert_eq!(
        s.fix_reason(),
        "SUPPRESSED",
        "I6: no zone-specific reason without explain"
    );
    assert!(s.decisions().is_empty());
    assert!(
        s.output.get("notes").is_none(),
        "no notes without explain: {}",
        s.text
    );
    assert!(
        !s.text.contains("STANDOFF"),
        "no standoff vocabulary may leak: {}",
        s.text
    );
    assert!(!s.text.contains("z-1"), "zone ids may not leak: {}", s.text);
    // and with explain the specific code is allowed
    let (_, s) = standoff_call(300, 5000, true, None);
    assert_eq!(s.fix_reason(), "SENSITIVE_STANDOFF");
}

#[test]
fn f10_standoff_suppresses_everything_wipes_mem_and_leaves_caps_untouched() {
    // Build caps + mem by firing near the origin, then walk into the zone with a different state.
    let (rig, last) = fired_state();
    let before = rig.state_json();
    let mut c = Call::base();
    c.state = rig.state.clone();
    c.signposts = rig.signposts.clone();
    c.policy = zone_policy(None);
    c.fix = fix_at(last + 1000, offset(zone_point(), 0, -500_000));
    c.clock = clock(last + 1000, 0);
    // the previous fix was 5 km away, so give the jump check a long enough gap
    c.fix["t_ms"] = json!(last + 70_000);
    c.clock = clock(last + 70_000, 0);
    let s = c.run();
    assert_global(&s, "SENSITIVE_STANDOFF");
    assert_eq!(s.state["mem"], empty_mem(), "no breadcrumb: mem is empty");
    assert_eq!(
        s.state["caps"], before["caps"],
        "caps byte-identical (hwm included)"
    );
    assert_eq!(render(&s.state["caps"]), render(&before["caps"]));
    // ALL classes are suppressed in M1: even a signpost sitting right at the fix gets nothing
    let mut c2 = c.clone();
    c2.signposts = vec![enter_circle_at(
        "c-here",
        offset(zone_point(), 0, -500_000),
        200,
    )];
    c2.state = empty_state();
    for k in 0..5 {
        c2.fix["t_ms"] = json!(BASE_T + k * 1000);
        c2.clock = clock(BASE_T + k * 1000, 0);
        let s = c2.run();
        assert_global(&s, "SENSITIVE_STANDOFF");
        assert_eq!(s.state["mem"], empty_mem());
        assert_eq!(
            s.state["caps"],
            json!({"hwm_ms": 0, "signposts": []}),
            "caps never written in a standoff"
        );
    }
}

#[test]
fn f10_leaving_the_standoff_does_not_remember_having_been_inside() {
    let mut rig = Rig::new().with_signpost(enter_circle_at(
        "c-z",
        offset(zone_point(), 0, -1500_000),
        200,
    ));
    rig.policy = zone_policy(None);
    // 4 s inside the signpost circle but 500 m from the zone (standoff): nothing accumulates
    for k in 0..6 {
        let s = rig.step(&fix_at(
            BASE_T + k * 1000,
            offset(zone_point(), 0, -500_000),
        ));
        assert_global(&s, "SENSITIVE_STANDOFF");
    }
    // outside the zone, at the signpost: the ENTER must be earned from scratch (4 fixes)
    let mut fires = vec![];
    for k in 0..6 {
        let s = rig.step(&fix_at(
            BASE_T + 7000 + k * 1000,
            offset(zone_point(), 0, -1500_000),
        ));
        assert!(s.fix_ok(), "{}", s.fix_reason());
        fires.push(s.fired("c-z"));
    }
    assert_eq!(fires, [false, false, false, true, false, false]);
}

// ---- ordering: when two filters reject, the first in spec order wins ---------------------------

/// A call that violates exactly the global filters in `which` (by number, 1..=10). A fix at
/// `BASE_T - 1000` (the accepted predecessor) is already in the state.
fn violating(which: &[u8]) -> Call {
    let mut c = Call::base();
    let mut rig = Rig::new();
    rig.step(&fix_at(BASE_T - 1000, ORIGIN));
    c.state = rig.state.clone();
    c.policy = zone_policy(None);
    let has = |n: u8| which.contains(&n);
    if has(1) {
        c.consent["location_matching"] = json!("denied");
    }
    if has(2) {
        c.policy["child_directed"] = json!(true);
    }
    if has(3) {
        c.fix["lon_e7"] = json!(1_800_000_001);
    }
    if has(4) {
        c.fix["is_mock"] = json!(true);
    }
    if has(5) {
        c.fix["acc_mm"] = json!(100_001);
    }
    // time: future (F06) / stale and before the predecessor (F07, also not after last_t, F08) / equal to last_t (F08)
    if has(6) {
        c.fix["t_ms"] = json!(BASE_T + 6000);
    } else if has(7) {
        c.fix["t_ms"] = json!(BASE_T - 31_000);
    } else if has(8) {
        c.fix["t_ms"] = json!(BASE_T - 1000);
    }
    // position: in the zone (F10) or a 2.2 km hop (F09)
    if has(10) {
        c.fix["lat_e7"] = json!(zone_point().0);
        c.fix["lon_e7"] = if has(3) {
            json!(1_800_000_001)
        } else {
            json!(zone_point().1)
        };
    } else if has(9) {
        c.fix["lat_e7"] = json!(ORIGIN.0 + 20_000_000);
    }
    c
}

fn code_of(n: u8) -> &'static str {
    [
        "CONSENT_DENIED",
        "ENGINE_DISABLED",
        "FIX_INVALID",
        "FIX_MOCK",
        "FIX_ACCURACY_TOO_COARSE",
        "FIX_FROM_FUTURE",
        "FIX_STALE",
        "FIX_OUT_OF_ORDER",
        "FIX_JUMP",
        "SENSITIVE_STANDOFF",
    ][usize::from(n) - 1]
}

#[test]
fn each_global_filter_alone_reports_its_own_code() {
    for f in 1..=10_u8 {
        let mut c = violating(&[f]);
        if f == 10 {
            // from a quiet history: a hop of 5 km in one second would be F09 first
            c.state = empty_state();
        }
        let s = c.run();
        assert_global(&s, code_of(f));
    }
}

#[test]
fn ordering_every_compatible_pair_of_global_filters_first_wins() {
    let mut checked = 0;
    for i in 1..=10_u8 {
        for j in (i + 1)..=10 {
            // impossible together: two different time faults (F06 vs F07/F08), a hop measured over a
            // non-positive dt (F07/F08 with F09), and an out-of-range longitude inside a zone
            if matches!((i, j), (6, 7) | (6, 8) | (7, 9) | (8, 9) | (3, 10)) {
                continue;
            }
            let s = violating(&[i, j]).run();
            assert!(s.is_ok(), "F{i:02}+F{j:02}: {}", s.text);
            assert_eq!(
                s.fix_reason(),
                code_of(i),
                "F{i:02}+F{j:02}: the first in spec order must win: {}",
                s.text
            );
            assert!(!s.fix_ok());
            checked += 1;
        }
    }
    assert_eq!(checked, 40, "pairs exercised");
}

#[test]
fn ordering_three_and_four_way_combinations() {
    // all violations at once among the compatible ones: consent first
    assert_eq!(
        violating(&[1, 2, 3, 4, 5]).run().fix_reason(),
        "CONSENT_DENIED"
    );
    assert_eq!(
        violating(&[2, 3, 4, 5]).run().fix_reason(),
        "ENGINE_DISABLED"
    );
    assert_eq!(violating(&[3, 4, 5]).run().fix_reason(), "FIX_INVALID");
    assert_eq!(violating(&[4, 5]).run().fix_reason(), "FIX_MOCK");
    assert_eq!(
        violating(&[5, 6]).run().fix_reason(),
        "FIX_ACCURACY_TOO_COARSE"
    );
    assert_eq!(
        violating(&[5, 7, 8]).run().fix_reason(),
        "FIX_ACCURACY_TOO_COARSE"
    );
    assert_eq!(violating(&[7, 8]).run().fix_reason(), "FIX_STALE");
    // a mock or coarse fix in a standoff reports the fix defect, not the zone
    assert_eq!(violating(&[4, 10]).run().fix_reason(), "FIX_MOCK");
    assert_eq!(
        violating(&[5, 10]).run().fix_reason(),
        "FIX_ACCURACY_TOO_COARSE"
    );
    assert_eq!(violating(&[6, 10]).run().fix_reason(), "FIX_FROM_FUTURE");
    // a hop into a zone is a jump before it is a standoff (spec order F09 < F10)
    assert_eq!(violating(&[9, 10]).run().fix_reason(), "FIX_JUMP");
    // consent denied inside a standoff: consent first
    assert_eq!(violating(&[1, 10]).run().fix_reason(), "CONSENT_DENIED");
    assert_eq!(
        violating(&[1, 2, 3, 4, 5, 6, 9, 10]).run().fix_reason(),
        "CONSENT_DENIED"
    );
    assert_eq!(
        violating(&[2, 3, 4, 5, 7, 8, 9, 10]).run().fix_reason(),
        "ENGINE_DISABLED"
    );
}

#[test]
fn rejected_fixes_f03_to_f08_are_never_added_to_the_trace() {
    for f in 3..=8_u8 {
        let c = violating(&[f]);
        let before = Rig {
            state: c.state.clone(),
            ..Rig::new()
        }
        .state_json();
        let s = c.run();
        assert_eq!(
            s.state["mem"]["trace"], before["mem"]["trace"],
            "F{f:02} must not retain the rejected fix"
        );
    }
}
