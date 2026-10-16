//! Spec 7, per-signpost static filters F11-F19 (a failure is a drop record and the detector entry is
//! removed) plus the cross-product ordering test: for every subset of defects the FIRST in spec
//! order is the one reported.
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

fn call_for(sp: Value) -> Call {
    let mut c = Call::base();
    c.signposts = vec![sp];
    c
}

fn drops_of(c: &Call) -> Vec<String> {
    let s = c.run();
    assert!(s.is_ok(), "{}", s.text);
    assert!(s.fix_ok(), "fix should be accepted: {}", s.fix_reason());
    assert!(s.fires().is_empty());
    assert!(
        s.det("c-1").is_none(),
        "a statically rejected signpost keeps no detector entry"
    );
    s.drops("c-1")
}

fn assert_drop(c: &Call, code: &str) {
    assert_eq!(
        drops_of(c),
        vec![code.to_string()],
        "expected exactly one drop {code}"
    );
}

fn assert_not_dropped(c: &Call) {
    let s = c.run();
    assert!(s.is_ok(), "{}", s.text);
    assert!(
        s.drops("c-1").is_empty(),
        "unexpected drop: {:?}",
        s.drops("c-1")
    );
    assert!(s.det("c-1").is_some(), "a live signpost is tracked");
}

fn health_ok_consent(c: &mut Call) {
    c.consent["nearby_health_events"] = json!("granted");
    c.consent["health_receipt"] = json!("receipt-1");
}

// ---- F11 --------------------------------------------------------------------------------------

#[test]
fn f11_reserved_events_are_unsupported_and_ignored() {
    for ev in ["dwell", "cross", "pass", "sequence", "exit"] {
        let mut sp = enter_circle("c-1", 200);
        sp["event"] = json!(ev);
        let c = call_for(sp);
        assert_drop(&c, "UNSUPPORTED_EVENT");
        // not even after many fixes inside the shape
        let mut rig = Rig::new();
        let mut sp = enter_circle("c-1", 200);
        sp["event"] = json!(ev);
        rig.signposts.push(sp);
        for k in 0..15 {
            let s = rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
            assert!(s.fires().is_empty(), "{ev} must never fire in M1");
        }
        assert!(rig.history.last().unwrap().det("c-1").is_none());
        assert_eq!(rig.history.last().unwrap().mem()["det"], json!([]));
    }
    // route-shaped reserved events too
    let mut sp = approach_route("c-1", 2000);
    sp["event"] = json!("sequence");
    assert_drop(&call_for(sp), "UNSUPPORTED_EVENT");
    // without explain a drop is not visible at all
    let mut c = call_for({
        let mut sp = enter_circle("c-1", 200);
        sp["event"] = json!("dwell");
        sp
    });
    c.policy = json!({});
    let s = c.run();
    assert!(
        s.decisions().is_empty() && s.output.get("notes").is_none(),
        "{}",
        s.text
    );
}

#[test]
fn unknown_event_names_are_parse_errors() {
    for ev in ["Enter", "ENTER", "exit ", "enters", "", "arrive"] {
        let mut sp = enter_circle("c-1", 200);
        sp["event"] = json!(ev);
        let s = call_for(sp).run();
        assert!(!s.is_ok(), "event {ev:?} must be rejected: {}", s.text);
        assert_eq!(s.error.as_ref().unwrap()["code"], "E_PARSE");
    }
}

#[test]
fn reserved_signpost_and_policy_fields_are_accepted_and_ignored() {
    // Spec 10: accepted, ignored. The result must be byte-identical to the same call without them.
    let plain = Call::base();
    let mut with = Call::base();
    for (k, v) in [
        ("intent", json!("promote")),
        ("content_mode", json!("card")),
        ("precision_class", json!("coarse")),
        ("min_mode", json!("foot")),
        ("max_acc_m", json!(50)),
        ("arm_scope", json!("route")),
        ("relation", json!("near")),
        ("selector", json!("x")),
        ("live_at_ms", json!(5)),
        ("fresh_required_s", json!(30)),
        ("cooldown_s", json!(60)),
        ("creatives", json!(["a"])),
        ("rot_days", json!(3)),
    ] {
        with.signposts[0][k] = v;
    }
    with.policy["rot_lane"] = json!(1);
    with.policy["rot_lanes"] = json!(2);
    let (a, b) = (plain.run(), with.run());
    if b.is_ok() {
        assert_eq!(
            a.text, b.text,
            "reserved fields must not change the decision"
        );
    } else {
        // the value types of reserved fields are not specified; report which one the engine refuses
        panic!("reserved fields rejected: {}", b.text);
    }
}

#[test]
fn unknown_fields_are_parse_errors_everywhere_strict() {
    let mut c = Call::base();
    c.signposts[0]["bogus"] = json!(1);
    assert_eq!(c.run().error.unwrap()["code"], "E_PARSE");
    let c = Call::base().with_fix("bogus", json!(1));
    assert_eq!(c.run().error.unwrap()["input"], "fix");
    let c = Call::base().with_policy("bogus", json!(1));
    assert_eq!(c.run().error.unwrap()["input"], "policy");
    let c = Call::base().with_consent("bogus", json!("granted"));
    assert_eq!(c.run().error.unwrap()["input"], "consent");
    let mut c = Call::base();
    c.clock["bogus"] = json!(1);
    assert_eq!(c.run().error.unwrap()["input"], "clock");
    let mut c = Call::base();
    c.signposts[0]["trigger"]["bogus"] = json!(1);
    assert_eq!(c.run().error.unwrap()["code"], "E_PARSE");
    let mut c = Call::base();
    c.signposts[0]["trigger"]["shape"]["bogus"] = json!(1);
    assert_eq!(c.run().error.unwrap()["code"], "E_PARSE");
    let mut c = Call::base();
    let mut st: Value = serde_json::from_str(&c.state).unwrap();
    st["mem"]["bogus"] = json!(1);
    c.state = render(&st);
    assert_eq!(c.run().error.unwrap()["input"], "state");
}

// ---- F12 --------------------------------------------------------------------------------------

#[test]
fn f12_trigger_invalid_cases() {
    let mut bad: Vec<(&str, Value)> = vec![];
    let mut sp = enter_circle("c-1", 149);
    bad.push(("radius 149 m (< 150)", sp.clone()));
    sp = enter_circle("c-1", 0);
    bad.push(("radius 0", sp.clone()));
    // event/shape mismatch
    sp = enter_circle("c-1", 200);
    sp["event"] = json!("approach");
    bad.push(("approach on a circle", sp));
    sp = approach_route("c-1", 2000);
    sp["event"] = json!("enter");
    bad.push(("enter on a route", sp));
    // route parameters
    for (k, v, name) in [
        ("trigger.shape.buffer_m", json!(29), "buffer 29"),
        ("trigger.shape.buffer_m", json!(501), "buffer 501"),
        ("trigger.lead_s", json!(14), "lead 14"),
        ("trigger.lead_s", json!(121), "lead 121"),
        ("trigger.d_hi_m", json!(5001), "d_hi 5001"),
    ] {
        let mut sp = approach_route("c-1", 2000);
        set(&mut sp, k, v);
        bad.push((name, sp));
    }
    // bad validity window
    sp = enter_circle("c-1", 200);
    sp["valid_from_ms"] = json!(2000);
    sp["valid_until_ms"] = json!(1000);
    bad.push(("valid_until < valid_from", sp));
    sp = enter_circle("c-1", 200);
    sp["valid_from_ms"] = json!(1000);
    sp["valid_until_ms"] = json!(1000);
    bad.push(("empty validity window", sp));
    // geometry outside the frame / degenerate
    sp = enter_circle("c-1", 200);
    set(
        &mut sp,
        "trigger.shape.center",
        pos(offset(ORIGIN, 160_000_000, 0)),
    );
    bad.push(("centre 160 km from the origin", sp));
    sp = approach_route("c-1", 2000);
    set(
        &mut sp,
        "trigger.shape.vertices",
        json!([pos(ORIGIN), pos(ORIGIN)]),
    );
    bad.push(("zero-length route", sp));
    sp = approach_route("c-1", 2000);
    set(&mut sp, "trigger.origin", pos((850_000_000, 70_000_000)));
    set(
        &mut sp,
        "trigger.shape.vertices",
        json!([
            pos((849_990_000, 70_000_000)),
            pos((850_000_000, 70_000_000))
        ]),
    );
    bad.push(("latitude 85 degrees", sp));
    // polygon with fewer than 3 vertices
    sp = enter_circle("c-1", 200);
    set(
        &mut sp,
        "trigger.shape",
        json!({"kind": "polygon", "vertices": [pos(ORIGIN), pos(offset(ORIGIN, 1000, 0))]}),
    );
    bad.push(("polygon with 2 vertices", sp));
    // anchor beyond the route
    sp = approach_route("c-1", 2000);
    set(&mut sp, "trigger.shape.anchor_along_m", json!(5000));
    bad.push(("anchor beyond route length", sp));
    // d_lo above d_hi
    sp = approach_route("c-1", 2000);
    set(&mut sp, "trigger.d_lo_m", json!(4000));
    set(&mut sp, "trigger.d_hi_m", json!(1000));
    bad.push(("d_lo > d_hi", sp));
    // bearing tolerance outside 150..450
    for tol in [149, 451] {
        sp = approach_route("c-1", 2000);
        set(&mut sp, "trigger.bearing_tol_ddeg", json!(tol));
        bad.push(("bearing tol out of 150..450", sp));
    }

    for (name, sp) in bad {
        let s = call_for(sp).run();
        // A parse-level refusal of the whole call is not what the spec says for a per-signpost defect,
        // so a rejection must be the drop record.
        assert!(
            s.is_ok(),
            "{name}: per-signpost defects are drops, not engine errors: {}",
            s.text
        );
        assert_eq!(
            s.drops("c-1"),
            vec!["TRIGGER_INVALID".to_string()],
            "{name}: {}",
            s.text
        );
        assert!(s.det("c-1").is_none(), "{name}");
        assert!(s.fires().is_empty(), "{name}");
    }
}

#[test]
fn f12_boundaries_that_are_valid() {
    let mut cases: Vec<(&str, Value)> = vec![("radius 150", enter_circle("c-1", 150))];
    for (k, v) in [
        ("trigger.shape.buffer_m", json!(30)),
        ("trigger.shape.buffer_m", json!(500)),
        ("trigger.lead_s", json!(15)),
        ("trigger.lead_s", json!(120)),
        ("trigger.d_hi_m", json!(5000)),
        ("trigger.bearing_tol_ddeg", json!(150)),
        ("trigger.bearing_tol_ddeg", json!(450)),
    ] {
        let mut sp = approach_route("c-1", 2000);
        set(&mut sp, k, v);
        cases.push((k, sp));
    }
    for (name, sp) in cases {
        let s = call_for(sp).run();
        assert!(
            s.is_ok() && s.drops("c-1").is_empty(),
            "{name}: {:?} {}",
            s.drops("c-1"),
            s.text
        );
    }
}

#[test]
fn f12_limits_on_vertex_counts_are_enforced() {
    // 4096 signposts, 1024 route vertices, 256 polygon vertices: excess is TRIGGER_INVALID or E_LIMIT
    let too_many_route: Vec<Value> = (0..1025)
        .map(|i| pos(offset(ORIGIN, 0, i64::from(i) * 1000 - 1_025_000)))
        .collect();
    let mut sp = approach_route("c-1", 2000);
    set(
        &mut sp,
        "trigger.shape.vertices",
        Value::Array(too_many_route),
    );
    let s = call_for(sp).run();
    assert!(
        s.error.as_ref().is_some_and(|e| e["code"] == "E_LIMIT")
            || s.drops("c-1") == ["TRIGGER_INVALID"],
        "1025 route vertices: {}",
        &s.text[..s.text.len().min(300)]
    );
    assert!(s.fires().is_empty());

    let ring = |n: i64| -> Vec<Value> {
        (0..n)
            .map(|i| {
                // a regular-ish 100 m "circle" of n points using integer stepping around a square ring
                let a = i * 4 / n;
                let t = (i * 4 % n) * 400_000 / n;
                let (e, nn) = match a {
                    0 => (-200_000 + t, -200_000),
                    1 => (200_000, -200_000 + t),
                    2 => (200_000 - t, 200_000),
                    _ => (-200_000, 200_000 - t),
                };
                pos(offset(ORIGIN, e, nn))
            })
            .collect()
    };
    for (n, ok) in [(256, true), (257, false)] {
        let mut sp = enter_circle("c-1", 200);
        set(
            &mut sp,
            "trigger.shape",
            json!({"kind": "polygon", "vertices": ring(n)}),
        );
        let s = call_for(sp).run();
        let dropped = s.drops("c-1") == ["TRIGGER_INVALID"]
            || s.error.as_ref().is_some_and(|e| e["code"] == "E_LIMIT");
        if ok {
            assert!(
                s.is_ok() && s.drops("c-1").is_empty(),
                "256 vertices is allowed: {}",
                &s.text[..s.text.len().min(300)]
            );
        } else {
            assert!(
                dropped,
                "257 vertices must be refused: {}",
                &s.text[..s.text.len().min(300)]
            );
        }
    }
    // 4097 signposts
    let mut c = Call::base();
    c.signposts = (0..4097)
        .map(|i| enter_circle(&format!("s-{i:05}"), 200))
        .collect();
    let s = c.run();
    assert!(
        !s.is_ok() && s.error.unwrap()["code"] == "E_LIMIT",
        "more than 4096 signposts is E_LIMIT"
    );
    // exactly 4096 is fine
    let mut c = Call::base();
    c.signposts = (0..4096)
        .map(|i| enter_circle(&format!("s-{i:05}"), 200))
        .collect();
    assert!(c.run().is_ok());
}

// ---- F13 / F14 --------------------------------------------------------------------------------

#[test]
fn f13_not_yet_live_and_f14_expired() {
    let mut sp = enter_circle("c-1", 200);
    sp["valid_from_ms"] = json!(BASE_T + 1);
    assert_drop(&call_for(sp), "NOT_YET_LIVE");
    let mut sp = enter_circle("c-1", 200);
    sp["valid_from_ms"] = json!(BASE_T + 86_400_000);
    assert_drop(&call_for(sp), "NOT_YET_LIVE");
    let mut sp = enter_circle("c-1", 200);
    sp["valid_until_ms"] = json!(BASE_T - 1);
    assert_drop(&call_for(sp), "EXPIRED");
    let mut sp = enter_circle("c-1", 200);
    sp["valid_from_ms"] = json!(0);
    sp["valid_until_ms"] = json!(1);
    assert_drop(&call_for(sp), "EXPIRED");
    // live windows
    let mut sp = enter_circle("c-1", 200);
    sp["valid_from_ms"] = json!(BASE_T - 1);
    sp["valid_until_ms"] = json!(BASE_T + 1);
    assert_not_dropped(&call_for(sp));
    // the window is judged on the engine clock `now`, not on the fix time
    let mut sp = enter_circle("c-1", 200);
    sp["valid_until_ms"] = json!(BASE_T + 1000);
    let mut c = call_for(sp);
    c.fix["t_ms"] = json!(BASE_T - 20_000);
    c.clock = clock(BASE_T + 2000, 0);
    assert_eq!(c.run().drops("c-1"), vec!["EXPIRED".to_string()]);
}

// ---- F15 --------------------------------------------------------------------------------------

#[test]
fn f15_health_needs_nearby_health_events_granted_and_a_receipt() {
    let mut sp = enter_circle("c-1", 200);
    sp["class"] = json!("health");
    let mut c = call_for(sp);
    assert_drop(&c, "HEALTH_CONSENT_MISSING");
    // granted without receipt
    c.consent["nearby_health_events"] = json!("granted");
    assert_drop(&c, "HEALTH_CONSENT_MISSING");
    // receipt without grant
    let mut c2 = c.clone();
    c2.consent["nearby_health_events"] = json!("unknown");
    c2.consent["health_receipt"] = json!("receipt-1");
    assert_drop(&c2, "HEALTH_CONSENT_MISSING");
    c2.consent["nearby_health_events"] = json!("denied");
    assert_drop(&c2, "HEALTH_CONSENT_MISSING");
    // strange grant spellings fail closed
    for v in ["Granted", "GRANTED", " granted", "granted ", "yes", "true"] {
        let mut c3 = c.clone();
        c3.consent["nearby_health_events"] = json!(v);
        c3.consent["health_receipt"] = json!("receipt-1");
        assert_drop(&c3, "HEALTH_CONSENT_MISSING");
    }
    // location consent alone does not unlock health
    c.consent["aggregate_reporting"] = json!("granted");
    c.consent["local_notifications"] = json!("granted");
    assert_drop(&c, "HEALTH_CONSENT_MISSING");
    // both: not dropped by F15
    health_ok_consent(&mut c);
    let s = c.run();
    assert!(
        !s.drops("c-1")
            .contains(&"HEALTH_CONSENT_MISSING".to_string()),
        "{}",
        s.text
    );
    // a general signpost never needs it
    assert_not_dropped(&Call::base());
}

#[test]
fn health_enter_never_fires_in_m1_because_p0_confidence_is_below_the_floor() {
    // Spec 5.3: "Health's floor 700 is reachable by P1 only ..., never by P0". ENTER is a P0 event.
    let mut sp = enter_circle("c-1", 200);
    sp["class"] = json!("health");
    let mut rig = Rig::new().with_signpost(sp);
    rig.consent["nearby_health_events"] = json!("granted");
    rig.consent["health_receipt"] = json!("receipt-1");
    for k in 0..20 {
        let s = rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
        assert!(s.fires().is_empty(), "health ENTER fired at step {k}");
        assert!(
            !s.drops("c-1")
                .contains(&"HEALTH_CONSENT_MISSING".to_string())
        );
    }
}

// ---- F16 / F17 / F18 --------------------------------------------------------------------------

#[test]
fn f16_f17_partner_blocks() {
    let c = Call::base().with_policy("categories_blocked", json!(["cafe"]));
    assert_drop(&c, "PARTNER_CATEGORY_BLOCKED");
    let c = Call::base().with_policy("categories_blocked", json!(["bar", "cafe", "zoo"]));
    assert_drop(&c, "PARTNER_CATEGORY_BLOCKED");
    assert_not_dropped(&Call::base().with_policy("categories_blocked", json!(["bar", "zoo"])));
    assert_not_dropped(&Call::base().with_policy("categories_blocked", json!([])));
    let c = Call::base().with_policy("senders_blocked", json!(["sender-1"]));
    assert_drop(&c, "SENDER_BLOCKED");
    assert_not_dropped(&Call::base().with_policy("senders_blocked", json!(["sender-2"])));
    // a blocked category is not a blocked sender and vice versa
    assert_not_dropped(&Call::base().with_policy("senders_blocked", json!(["cafe"])));
    assert_not_dropped(&Call::base().with_policy("categories_blocked", json!(["sender-1"])));
}

#[test]
fn f18_sensitivity_ceiling_default_two_and_partners_only_narrow() {
    for (sens, ceiling, dropped) in [
        (0, None, false),
        (2, None, false),
        (3, None, true),
        (255, None, true),
        (2, Some(1), true),
        (1, Some(1), false),
        (0, Some(0), false),
        (1, Some(0), true),
        (3, Some(5), true), // a partner cannot raise the platform ceiling of 2
        (3, Some(255), true),
    ] {
        let mut sp = enter_circle("c-1", 200);
        sp["sensitivity"] = json!(sens);
        let mut c = call_for(sp);
        if let Some(cl) = ceiling {
            c = c.with_policy("sensitivity_ceiling", json!(cl));
        }
        if dropped {
            assert_drop(&c, "SENSITIVITY_ABOVE_CEILING");
        } else {
            assert_not_dropped(&c);
        }
    }
}

// ---- F19 --------------------------------------------------------------------------------------

fn zone_east(m: i64) -> Value {
    json!({"explain": true, "standoff_zones": [{"id": "z-1", "center": pos(offset(ORIGIN, m * 1000, 0))}]})
}

#[test]
fn f19_health_geometry_within_a_standoff_is_dropped_but_general_is_not() {
    // The fix is at the ORIGIN, 1100 m from the zone: outside the 1010 m fix standoff.
    // The circle (200 m) reaches to 900 m from the zone centre: within the 1000 m standoff.
    let mut sp = enter_circle("c-1", 200);
    sp["class"] = json!("health");
    let mut c = call_for(sp);
    c.policy = zone_east(1100);
    health_ok_consent(&mut c);
    assert_drop(&c, "TRIGGER_IN_STANDOFF");
    // the same geometry as a general signpost is untouched (F19 tests health only)
    let mut g = Call::base();
    g.policy = zone_east(1100);
    let s = g.run();
    assert!(s.fix_ok() && s.drops("c-1").is_empty(), "{}", s.text);
    // health, but the shape stays clear of the zone: kept (circle edge at 1300 - 200 = 1100 m > 1000 m)
    let mut sp = enter_circle("c-1", 200);
    sp["class"] = json!("health");
    let mut c = call_for(sp);
    c.policy = zone_east(1500);
    health_ok_consent(&mut c);
    let s = c.run();
    assert!(
        !s.drops("c-1").contains(&"TRIGGER_IN_STANDOFF".to_string()),
        "{}",
        s.text
    );
}

#[test]
fn f19_boundary_is_edge_distance_at_most_standoff() {
    // radius 150 m, zone centre `d` m east: the circle edge is d - 150 m from the centre, standoff 1000 m.
    // The fix (at the origin) stays outside the 1010 m fix standoff in every row.
    for (d, dropped) in [
        (1135, true),
        (1145, true),
        (1165, false),
        (1200, false),
        (1500, false),
    ] {
        let mut sp = enter_circle("c-1", 150);
        sp["class"] = json!("health");
        let mut c = call_for(sp);
        c.policy = zone_east(d);
        health_ok_consent(&mut c);
        let s = c.run();
        assert!(s.is_ok() && s.fix_ok(), "{}", s.text);
        let got = s.drops("c-1").contains(&"TRIGGER_IN_STANDOFF".to_string());
        assert_eq!(
            got,
            dropped,
            "zone {d} m east, circle radius 150 m (edge at {} m)",
            d - 150
        );
    }
}

#[test]
fn f19_polygon_and_route_shapes() {
    // polygon around the origin (+-200 m); zone 1100 m east: nearest polygon point is 900 m away
    let mut sp = enter_circle("c-1", 200);
    sp["class"] = json!("health");
    let sq = [(-200, -200), (200, -200), (200, 200), (-200, 200)]
        .map(|(e, n)| pos(offset(ORIGIN, e * 1000, n * 1000)));
    set(
        &mut sp,
        "trigger.shape",
        json!({"kind": "polygon", "vertices": sq}),
    );
    let mut c = call_for(sp);
    c.policy = zone_east(1100);
    health_ok_consent(&mut c);
    assert_drop(&c, "TRIGGER_IN_STANDOFF");
    // zone centre INSIDE the polygon: distance 0
    c.policy = zone_east(100);
    // the fix at the origin is now inside the zone standoff; move the fix away to the east edge
    // outside 1010 m instead, so only F19 can fire: put the fix 1500 m south
    c.fix = fix_at(BASE_T, offset(ORIGIN, 0, -1_500_000));
    assert_drop(&c, "TRIGGER_IN_STANDOFF");

    // route passing within the standoff of a zone: route north-south, zone 800 m east of its middle
    let mut sp = approach_route("c-1", 2000);
    sp["class"] = json!("health");
    let mut c = call_for(sp);
    c.policy = json!({"explain": true, "standoff_zones": [{"id": "z-1", "center": pos(offset(ORIGIN, 800_000, -1_000_000))}]});
    c.fix = fix_at(BASE_T, offset(ORIGIN, 0, -2_500_000)); // 2.5 km south of the anchor, far from the zone
    health_ok_consent(&mut c);
    assert_drop(&c, "TRIGGER_IN_STANDOFF");
    // zone 1300 m east of the route: clear
    c.policy = json!({"explain": true, "standoff_zones": [{"id": "z-1", "center": pos(offset(ORIGIN, 1_300_000, -1_000_000))}]});
    let s = c.run();
    assert!(
        !s.drops("c-1").contains(&"TRIGGER_IN_STANDOFF".to_string()),
        "{}",
        s.text
    );
}

// ---- ordering: every subset of defects reports the first in spec order -------------------------

/// Defects F11..F19 as mutations of (call). Each, applied alone, yields only its own code (checked).
fn apply(c: &mut Call, f: u8) {
    match f {
        11 => c.signposts[0]["event"] = json!("dwell"),
        12 => set(&mut c.signposts[0], "trigger.shape.radius_m", json!(100)),
        13 => c.signposts[0]["valid_from_ms"] = json!(BASE_T + 60_000),
        14 => c.signposts[0]["valid_until_ms"] = json!(BASE_T - 60_000),
        15 => {
            c.signposts[0]["class"] = json!("health");
            // baseline consent has no health grant at all
        }
        16 => c.policy["categories_blocked"] = json!(["cafe"]),
        17 => c.policy["senders_blocked"] = json!(["sender-1"]),
        18 => c.signposts[0]["sensitivity"] = json!(3),
        19 => {
            c.signposts[0]["class"] = json!("health");
            c.policy["standoff_zones"] =
                json!([{"id": "z-1", "center": pos(offset(ORIGIN, 1_100_000, 0))}]);
        }
        _ => unreachable!(),
    }
}

fn code_of(f: u8) -> &'static str {
    match f {
        11 => "UNSUPPORTED_EVENT",
        12 => "TRIGGER_INVALID",
        13 => "NOT_YET_LIVE",
        14 => "EXPIRED",
        15 => "HEALTH_CONSENT_MISSING",
        16 => "PARTNER_CATEGORY_BLOCKED",
        17 => "SENDER_BLOCKED",
        18 => "SENSITIVITY_ABOVE_CEILING",
        19 => "TRIGGER_IN_STANDOFF",
        _ => unreachable!(),
    }
}

fn defect_call(set_of: &[u8]) -> Call {
    let mut c = Call::base();
    // a health grant is present unless F15 is among the defects (so F19 alone passes F15)
    if !set_of.contains(&15) {
        health_ok_consent(&mut c);
    }
    for &f in set_of {
        apply(&mut c, f);
    }
    c
}

#[test]
fn each_static_filter_alone_reports_exactly_its_code() {
    for f in 11..=19_u8 {
        let c = defect_call(&[f]);
        assert_drop(&c, code_of(f));
    }
}

#[test]
fn ordering_every_subset_of_static_defects_reports_the_first() {
    let mut checked = 0;
    for mask in 1_u32..(1 << 9) {
        let set_of: Vec<u8> = (11..=19_u8)
            .filter(|f| mask & (1 << (f - 11)) != 0)
            .collect();
        // F13 (from > now) and F14 (until < now) together make the window itself invalid (F12 territory)
        if set_of.contains(&13) && set_of.contains(&14) {
            continue;
        }
        let c = defect_call(&set_of);
        let s = c.run();
        assert!(s.is_ok() && s.fix_ok(), "{set_of:?}: {}", s.text);
        let want = code_of(set_of[0]);
        assert_eq!(
            s.drops("c-1"),
            vec![want.to_string()],
            "defects {set_of:?}: first in order must win"
        );
        assert!(s.det("c-1").is_none() && s.fires().is_empty(), "{set_of:?}");
        checked += 1;
    }
    assert_eq!(checked, 511 - 128, "subsets exercised");
}

#[test]
fn a_static_drop_does_not_affect_other_signposts() {
    let mut c = Call::base();
    c.signposts = vec![enter_circle("a-bad", 100), enter_circle("b-good", 200)];
    let s = c.run();
    assert_eq!(s.drops("a-bad"), vec!["TRIGGER_INVALID".to_string()]);
    assert!(s.drops("b-good").is_empty());
    assert!(s.det("b-good").is_some() && s.det("a-bad").is_none());
}
