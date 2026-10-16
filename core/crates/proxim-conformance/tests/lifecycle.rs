//! Spec 3.3 state, 6.1 gap rules, 6.2 reach, 7 pruning/limits/policy narrowing, polygon ENTER and
//! accuracy gates, health tiers, id validation and strictness of the wire format.
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

// ---- ENTER shapes and accuracy gates --------------------------------------------------------------

fn square_polygon(id: &str, half_m: i64) -> Value {
    let mut sp = enter_circle(id, 200);
    let v = [
        (-half_m, -half_m),
        (half_m, -half_m),
        (half_m, half_m),
        (-half_m, half_m),
    ]
    .map(|(e, n)| pos(offset(ORIGIN, e * 1000, n * 1000)));
    set(
        &mut sp,
        "trigger.shape",
        json!({"kind": "polygon", "vertices": v}),
    );
    sp
}

#[test]
fn polygon_enter_uses_depth_against_the_nearest_edge() {
    // 400 m square. A fix at the centre (depth 200 m) enters; 5 m inside an edge straddles.
    let mut rig = Rig::new().with_signpost(square_polygon("p-1", 200));
    let fired: Vec<bool> = (0..6)
        .map(|k| rig.step(&fix_at(BASE_T + k * 1000, ORIGIN)).fired("p-1"))
        .collect();
    assert_eq!(fired, [false, false, false, true, false, false]);
    let mut rig = Rig::new().with_signpost(square_polygon("p-1", 200));
    for k in 0..30 {
        let s = rig.step(&fix_at(BASE_T + k * 1000, offset(ORIGIN, 195_000, 0)));
        assert!(
            !s.fired("p-1"),
            "5 m inside an edge with acc_eff 10 m is a straddle"
        );
    }
    let mut rig = Rig::new().with_signpost(square_polygon("p-1", 200));
    for k in 0..30 {
        let s = rig.step(&fix_at(BASE_T + k * 1000, offset(ORIGIN, 185_000, 0)));
        if k == 3 {
            assert!(s.fired("p-1"), "15 m inside is Inside");
        }
    }
    // outside never enters
    let mut rig = Rig::new().with_signpost(square_polygon("p-1", 200));
    for k in 0..30 {
        assert!(
            !rig.step(&fix_at(BASE_T + k * 1000, offset(ORIGIN, 400_000, 0)))
                .fired("p-1")
        );
    }
}

#[test]
fn polygon_with_a_small_inradius_is_acc_too_coarse() {
    // inradius 5 m < k_in x acc_eff (10 m): the shape is too small for this accuracy
    let mut sp = square_polygon("p-1", 200);
    set(&mut sp, "trigger.shape.inradius_m", json!(5));
    let mut rig = Rig::new().with_signpost(sp);
    let mut seen = false;
    for k in 0..10 {
        let s = rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
        assert!(!s.fired("p-1"));
        seen |= s.reasons("p-1").contains(&"ACC_TOO_COARSE".to_string());
    }
    assert!(seen, "ACC_TOO_COARSE expected");
    // inradius exactly acc_eff (10 m) is enough
    let mut sp = square_polygon("p-1", 200);
    set(&mut sp, "trigger.shape.inradius_m", json!(10));
    let mut rig = Rig::new().with_signpost(sp);
    let fired: Vec<bool> = (0..6)
        .map(|k| rig.step(&fix_at(BASE_T + k * 1000, ORIGIN)).fired("p-1"))
        .collect();
    assert!(fired.iter().any(|f| *f));
}

#[test]
fn circle_needs_acc_at_most_50_m_and_radius_at_least_acc_eff() {
    // acc 60 m (> 50 m): never enters, even at the centre of a 5 km circle
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 5000));
    let mut seen = false;
    for k in 0..10 {
        let mut f = fix_at(BASE_T + k * 1000, ORIGIN);
        f["acc_mm"] = json!(60_000);
        let s = rig.step(&f);
        assert!(!s.fired("c-1"));
        seen |= s.reasons("c-1").contains(&"ACC_TOO_COARSE".to_string());
    }
    assert!(seen);
    // acc 50 m exactly is allowed: radius 5 km, centre fix
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 5000));
    let mut fired = false;
    for k in 0..6 {
        let mut f = fix_at(BASE_T + k * 1000, ORIGIN);
        f["acc_mm"] = json!(50_000);
        fired |= rig.step(&f).fired("c-1");
    }
    assert!(fired);
    // a coarse fix leaves a candidate untouched (it neither counts nor resets)
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 5000));
    for k in 0..2 {
        rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
    }
    let before = rig.history.last().unwrap().det("c-1").unwrap();
    let mut f = fix_at(BASE_T + 2000, ORIGIN);
    f["acc_mm"] = json!(60_000);
    let s = rig.step(&f);
    assert_eq!(s.det("c-1").unwrap()["st"]["n"], before["st"]["n"]);
}

// ---- gaps and pruning -------------------------------------------------------------------------------

#[test]
fn in_state_survives_a_100_s_gap_and_becomes_cool_after_900_s() {
    let mk = || {
        let mut rig = Rig::new().with_signpost(enter_circle("c-1", 150));
        for k in 0..6 {
            rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
        }
        assert_eq!(
            rig.history.last().unwrap().det("c-1").unwrap()["st"]["s"],
            "in"
        );
        rig
    };
    let last_t = BASE_T + 5000;
    // 100 s later, still inside: remains `in`
    let mut rig = mk();
    let s = rig.step(&fix_at(last_t + 100_000, ORIGIN));
    assert_eq!(s.det("c-1").unwrap()["st"]["s"], "in");
    // 901 s later: cool-down from the previous fix time, and the new fix does not re-enter
    let mut rig = mk();
    let s = rig.step(&fix_at(last_t + 901_000, ORIGIN));
    let st = s.det("c-1").unwrap()["st"].clone();
    assert_eq!(st["s"], "cool");
    assert_eq!(
        st["until_ms"],
        last_t + 1_800_000,
        "cool until prev_last_t + 30 min"
    );
    assert!(s.fires().is_empty());
}

#[test]
fn detectors_not_in_the_input_are_pruned_after_900_s() {
    let mut rig = Rig::new()
        .with_signpost(enter_circle("c-1", 200))
        .with_signpost(enter_circle("c-2", 200));
    for k in 0..2 {
        rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
    }
    assert!(rig.history.last().unwrap().det("c-2").is_some());
    let last = BASE_T + 1000;
    // c-2 leaves the regional set; keep feeding fixes (candidate for c-1 restarts: 10 s gaps cap)
    rig.signposts.truncate(1);
    let s = rig.step(&fix_at(last + 500_000, ORIGIN));
    assert!(s.det("c-2").is_some(), "unseen for 499 s: kept");
    let s = rig.step(&fix_at(last + 899_000, ORIGIN));
    assert!(s.det("c-2").is_some(), "unseen for 899 s: kept");
    let s = rig.step(&fix_at(last + 902_000, ORIGIN));
    assert!(s.det("c-2").is_none(), "unseen for more than 900 s: pruned");
    assert!(s.det("c-1").is_some());
}

#[test]
fn a_changed_revision_replaces_the_detector_entry() {
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 200));
    for k in 0..2 {
        rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
    }
    assert_eq!(
        rig.history.last().unwrap().det("c-1").unwrap()["st"]["n"],
        2
    );
    rig.signposts[0]["rev"] = json!(2);
    let s = rig.step(&fix_at(BASE_T + 2000, ORIGIN));
    let d = s.det("c-1").unwrap();
    assert_eq!(d["rev"], 2);
    assert_eq!(d["st"]["n"], 1, "a new revision starts from idle: {d}");
    assert_eq!(
        s.decisions()
            .iter()
            .filter(|x| x["outcome"] == "fire")
            .count(),
        0
    );
}

#[test]
fn armed_detector_returns_to_idle_when_the_fix_leaves_reach() {
    let d = Drive::new(2500, 15_000);
    let mut sp = approach_route("r-1", 2500);
    set(&mut sp, "trigger.reach_m", json!(3000));
    let mut rig = Rig::new().with_signpost(sp);
    let mut armed_seen = false;
    for k in 0..40 {
        let s = rig.step(&d.fix(k));
        armed_seen |= s.det("r-1").is_some_and(|e| e["st"]["s"] == "armed");
    }
    assert!(armed_seen, "must arm before the fire distance");
    // a fix 20 km away (long gap, so no jump check) is beyond reach
    let t = d.t_ms(40) + 100_000;
    let s = rig.call(&fix_at(t, offset(ORIGIN, 0, 20_000_000)), t);
    assert!(s.fix_ok());
    let det = s.det("r-1");
    assert!(
        det.is_none() || det.unwrap()["st"]["s"] != "armed",
        "armed -> idle beyond reach_m"
    );
}

// ---- policy narrowing for lead time ---------------------------------------------------------------

fn fire_d_rem(sp: Value, policy: Option<(&str, i64)>, speed: i64) -> i64 {
    let mut rig = Rig::new().with_signpost(sp);
    if let Some((k, v)) = policy {
        rig.policy[k] = json!(v);
    }
    let d = Drive::new(3400, speed);
    let (_, f) = drive_until_fire(&mut rig, &d, "r-1", 300).expect("fires");
    f["d_rem_mm"].as_i64().unwrap()
}

#[test]
fn policy_clamps_the_authored_lead() {
    let sp = || {
        let mut sp = approach_route("r-1", 3400);
        set(&mut sp, "trigger.lead_s", json!(100));
        sp
    };
    // authored 100 s at 15 m/s: D = 15 x 103 = 1545 m
    let base = fire_d_rem(sp(), None, 15_000);
    assert!(
        (1545_000 - 50_000..=1545_000 + 3000).contains(&base),
        "{base}"
    );
    // lead_s_max 40 narrows to 40 s: D = 15 x 43 = 645 m
    let narrowed = fire_d_rem(sp(), Some(("lead_s_max", 40)), 15_000);
    assert!(
        (645_000 - 50_000..=645_000 + 2000).contains(&narrowed),
        "{narrowed}"
    );
    // lead_s_max cannot exceed the platform 120: authored 100 is kept
    let wide = fire_d_rem(sp(), Some(("lead_s_max", 500)), 15_000);
    assert_eq!(
        wide / 1000 / 20,
        base / 1000 / 20,
        "lead_s_max 500 clamps to 120, authored 100 still applies"
    );
    // lead_s_min raises a short authored lead
    let mut short = approach_route("r-1", 3400);
    set(&mut short, "trigger.lead_s", json!(20));
    let raised = fire_d_rem(short, Some(("lead_s_min", 60)), 15_000);
    assert!(
        (15_000 * 63 - 50_000..=15_000 * 63 + 2000).contains(&raised),
        "lead 60 s: D = 945 m, got {raised}"
    );
}

#[test]
fn authored_d_hi_and_d_lo_clamp_the_fire_distance() {
    // d_hi 500 m: 29 m/s would fire at 1247 m, but never beyond 500 m
    let mut sp = approach_route("r-1", 2500);
    set(&mut sp, "trigger.d_hi_m", json!(500));
    let d = Drive::new(2500, 29_000);
    let mut rig = Rig::new().with_signpost(sp);
    let (_, f) = drive_until_fire(&mut rig, &d, "r-1", 200).expect("fires inside d_hi");
    assert!(f["d_rem_mm"].as_i64().unwrap() <= 500_000, "{f}");
    // d_lo 400 m: a 3 m/s crawl (D = 172 m) fires at 400 m
    let mut sp = approach_route("r-1", 800);
    set(&mut sp, "trigger.d_lo_m", json!(400));
    let d = Drive::new(800, 3_000);
    let mut rig = Rig::new().with_signpost(sp);
    let (_, f) = drive_until_fire(&mut rig, &d, "r-1", 400).expect("fires");
    let r = f["d_rem_mm"].as_i64().unwrap();
    assert!(
        (400_000 - 10_000..=400_000).contains(&r),
        "crawl rule at d_lo=400 m: {r}"
    );
}

// ---- health approach tier floor (D-HEALTH-FLOOR) ---------------------------------------------------

fn health_run(acc_mm: u32, course: u16) -> bool {
    let mut sp = approach_route("r-1", 2500);
    sp["class"] = json!("health");
    let mut rig = Rig::new().with_signpost(sp);
    rig.consent["nearby_health_events"] = json!("granted");
    rig.consent["health_receipt"] = json!("receipt-1");
    let mut d = Drive::new(2500, 15_000);
    d.acc_mm = acc_mm;
    d.course = Some(course);
    drive_until_fire(&mut rig, &d, "r-1", 200).is_some()
}

#[test]
fn health_approach_fires_with_confidence_800_and_700() {
    assert!(health_run(5_000, 0), "800 >= 700");
    assert!(health_run(5_000, 200), "600 + 100 + 0 = 700 >= 700");
    assert!(health_run(40_000, 0), "600 + 100 + 100 - 100 = 700 >= 700");
}

#[test]
fn health_approach_below_700_confidence_does_not_fire() {
    // acc_eff 55 m: 600 + 100 + 100 - 200 = 600 < 700 (spec 5.3: Health's floor 700)
    assert!(
        !health_run(55_000, 0),
        "health at confidence 600 must not fire"
    );
}

// ---- ids and strict wire format -------------------------------------------------------------------

#[test]
fn ids_must_be_ascii_and_at_most_64_chars() {
    let long = "a".repeat(64);
    let toolong = "a".repeat(65);
    for (id, ok) in [
        ("a", true),
        (long.as_str(), true),
        ("A.b_c:d-9", true),
        (toolong.as_str(), false),
        ("", false),
        ("a b", false),
        ("a/b", false),
        ("a\u{e9}", false),
        ("a\u{0}", false),
        ("a\n", false),
        ("a#", false),
        ("\u{1f600}", false),
    ] {
        let s = Call {
            signposts: vec![enter_circle(id, 200)],
            ..Call::base()
        }
        .run();
        let accepted = s.is_ok();
        assert_eq!(
            accepted,
            ok,
            "id {id:?}: {}",
            &s.text[..s.text.len().min(200)]
        );
        if !ok {
            assert_eq!(s.error.unwrap()["code"], "E_PARSE");
        }
    }
    // zone ids follow the same grammar
    let mut c = Call::base();
    c.policy =
        json!({"explain": true, "standoff_zones": [{"id": "bad id", "center": pos(ORIGIN)}]});
    assert!(!c.run().is_ok());
}

#[test]
fn duplicate_keys_are_rejected_in_every_argument() {
    let c = Call::base();
    let dup = |t: &str| t.replacen('{', "{\"zz\":1,\"zz\":2,", 1);
    let fix_t = render(&c.fix).replacen("\"acc_mm\":5000", "\"acc_mm\":5000,\"acc_mm\":5000", 1);
    let r = proxim_ffi::decide_json(
        &c.state,
        &fix_t,
        &render(&Value::Array(c.signposts.clone())),
        &render(&c.policy),
        &render(&c.consent),
        &render(&c.clock),
        0,
    );
    assert!(
        r.starts_with("{\"error\""),
        "duplicate key in fix must be refused: {r}"
    );
    let _ = dup;
    let pol = "{\"explain\":true,\"explain\":false}";
    let r = proxim_ffi::decide_json(
        &c.state,
        &render(&c.fix),
        &render(&Value::Array(c.signposts.clone())),
        pol,
        &render(&c.consent),
        &render(&c.clock),
        0,
    );
    assert!(r.contains("\"input\":\"policy\""), "{r}");
    let cons = "{\"location_matching\":\"denied\",\"location_matching\":\"granted\"}";
    let r = proxim_ffi::decide_json(
        &c.state,
        &render(&c.fix),
        &render(&Value::Array(c.signposts.clone())),
        &render(&c.policy),
        cons,
        &render(&c.clock),
        0,
    );
    assert!(
        r.contains("\"input\":\"consent\""),
        "last-wins would turn denied into granted: {r}"
    );
}

#[test]
fn nested_structs_are_not_accepted_as_positional_arrays() {
    // canonical Position is an object; an array form is a different spelling no port would accept
    let mut c = Call::base();
    c.signposts[0]["trigger"]["origin"] = json!([ORIGIN.0, ORIGIN.1]);
    assert!(!c.run().is_ok(), "positional array accepted for Position");
    let c = Call::base();
    let r = proxim_ffi::decide_json(
        &c.state,
        "[1,2,3,4,5]",
        &render(&Value::Array(c.signposts.clone())),
        &render(&c.policy),
        &render(&c.consent),
        &render(&c.clock),
        0,
    );
    assert!(
        r.starts_with("{\"error\""),
        "top-level fix array accepted: {r}"
    );
}

#[test]
fn state_version_and_shape_are_checked() {
    let c = Call::base();
    for st in [
        r#"{"caps":{"hwm_ms":0,"signposts":[]},"mem":{"det":[],"jump_rejects":0,"trace":[]},"v":2}"#,
        r#"{"caps":{"hwm_ms":0,"signposts":[]},"mem":{"det":[],"jump_rejects":0,"trace":[]}}"#,
        r#"{"v":1}"#,
        r#"[]"#,
        r#""state""#,
        "",
        "{",
        r#"{"caps":{"hwm_ms":-1,"signposts":[]},"mem":{"det":[],"jump_rejects":0,"trace":[]},"v":1}"#,
    ] {
        let r = proxim_ffi::decide_json(
            st,
            &render(&c.fix),
            &render(&Value::Array(c.signposts.clone())),
            &render(&c.policy),
            &render(&c.consent),
            &render(&c.clock),
            0,
        );
        let s = parse_response(r);
        assert!(!s.is_ok(), "state {st:?} must be refused");
        let e = s.error.as_ref().unwrap();
        assert_eq!(e["code"], "E_PARSE", "{st:?}");
        assert!(e.get("input").is_none_or(|i| i == "state"), "{st:?}: {e}");
    }
}

#[test]
fn an_error_response_never_carries_state_or_output() {
    let r = Call::base().with_fix("bogus", json!(1)).run();
    assert!(r.output.is_null() && r.state.is_null());
    assert!(
        r.text
            .starts_with("{\"error\":{\"code\":\"E_PARSE\",\"input\":\"fix\"}}"),
        "{}",
        r.text
    );
}

#[test]
fn clock_and_timezone_ranges() {
    for (now, tz, ok) in [
        (BASE_T, 840_i64, true),
        (BASE_T, -840, true),
        (BASE_T, 841, false),
        (BASE_T, -841, false),
        (-1, 0, false),
        (0, 0, true),
    ] {
        let mut c = Call::base();
        c.clock = json!({"now_ms": now, "tz_offset_min": tz});
        if now <= 0 {
            c.fix["t_ms"] = json!(now.max(0));
        }
        let s = c.run();
        if !ok {
            assert!(
                !s.is_ok() || !s.fix_ok(),
                "now {now} tz {tz} must be refused: {}",
                s.text
            );
        } else {
            assert!(s.is_ok(), "now {now} tz {tz}: {}", s.text);
        }
    }
    // tz values outside i16 are refused
    let c = Call::base();
    let r = c.run_with_raw("clock", "tz_offset_min", "40000");
    assert!(!r.is_ok());
}
