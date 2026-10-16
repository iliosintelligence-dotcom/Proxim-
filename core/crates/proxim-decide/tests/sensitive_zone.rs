//! I6 and the M1 minimal standoff: F10 (the user's own fix) and F19 (health geometry).
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_decide::decide;
use proxim_types::canon::to_canonical_json;
use proxim_types::{
    Class, Id, Mem, Outcome, Policy, Position, ReasonCode, Shape, StandoffZone, State,
};

fn zone(id: &str, e_mm: i32, n_mm: i32, standoff_m: Option<u32>) -> StandoffZone {
    StandoffZone {
        id: Id::new(id).unwrap(),
        center: pos(e_mm, n_mm),
        standoff_m,
    }
}

fn with_zones(zones: Vec<StandoffZone>, explain_on: bool) -> Policy {
    Policy {
        explain: Some(explain_on),
        standoff_zones: zones,
        ..Policy::default()
    }
}

/// A warm state: fired once (caps non-empty), detectors and trace populated.
fn warm(signposts: Vec<proxim_types::Signpost>, policy: &Policy) -> Sim {
    let mut sim = Sim::new(signposts);
    sim.policy = policy.clone();
    let fixes = visit(T0, 2_000_000, 0, 5, 0);
    let outs = sim.run(&fixes);
    assert!(!fire_indices(&outs).is_empty(), "setup must have fired");
    assert!(!sim.state.caps.signposts.is_empty() && !sim.state.mem.trace.is_empty());
    sim
}

#[test]
fn nothing_logged_or_queued_inside_standoff() {
    for explain_on in [false, true] {
        let policy = with_zones(vec![zone("z-1", 0, 0, None)], explain_on);
        let mut sim = warm(
            vec![circle_at("c-far", 2_000_000, 0, 200), circle("c-1", 200)],
            &policy,
        );
        let caps_before = to_canonical_json(&sim.state.caps).unwrap();
        // Standing inside both the zone's standoff and the circle: nothing may fire or be kept.
        // More than 60 s later, so that the 2 km move is not a FIX_JUMP (F09 runs before F10).
        let t = sim.state.mem.last_t_ms.unwrap() + 70_000;
        for k in 0..8 {
            let f = still(t + 4000 * k, 0, 0);
            let o = sim.step(&f);
            assert!(!o.fix.ok);
            assert_eq!(
                o.fix.reason,
                if explain_on {
                    ReasonCode::SensitiveStandoff
                } else {
                    ReasonCode::Suppressed
                }
            );
            assert!(o.decisions.is_empty() && o.notes.is_none());
            assert_eq!(
                (
                    o.path.tier,
                    o.path.conf_milli,
                    o.path.speed_mmps,
                    o.path.heading_valid
                ),
                (0, 0, None, false)
            );
            // Mem is empty: no trace point, no detector, no filter, no jump counter.
            assert_eq!(sim.state.mem, Mem::default());
            // Caps are untouched byte for byte (not even the high-water mark moves).
            assert_eq!(to_canonical_json(&sim.state.caps).unwrap(), caps_before);
            // The output carries no coordinate of the fix.
            let text = to_canonical_json(&o).unwrap();
            assert!(!text.contains(&f.lat_e7.to_string()) && !text.contains(&f.lon_e7.to_string()));
        }
    }
}

#[test]
fn leaving_the_standoff_starts_from_nothing() {
    let policy = with_zones(vec![zone("z-1", 0, 0, None)], false);
    let mut sim = Sim::new(vec![circle_at("c-out", 3_000_000, 0, 200)]);
    sim.policy = policy;
    sim.step(&still(T0, 0, 0));
    sim.step(&still(T0 + 4000, 0, 0));
    // First fix outside: the trace holds exactly that fix, nothing from inside.
    sim.step(&still(T0 + 8000, 3_000_000, 0));
    assert_eq!(sim.state.mem.trace.len(), 1);
    assert_eq!(sim.state.mem.trace[0].t_ms, T0 + 8000);
}

#[test]
fn standoff_boundary_is_standoff_plus_effective_accuracy() {
    // 1000 m standoff + acc_eff 10 m (acc 5 m is floored to 10 m): 1010 m. Margins of 50 mm
    // absorb the 11 mm e7 grid.
    let policy = with_zones(vec![zone("z-1", 0, 0, None)], true);
    let at = |n_mm: i32, acc: u32| {
        let mut sim = Sim::new(vec![]);
        sim.policy = policy.clone();
        let mut f = still(T0, 0, n_mm);
        f.acc_mm = acc;
        sim.step(&f)
    };
    assert_eq!(
        at(1_009_950, 5000).fix.reason,
        ReasonCode::SensitiveStandoff
    );
    assert_eq!(at(1_010_050, 5000).fix.reason, ReasonCode::Ok);
    // Worse accuracy widens it: acc 60 m -> 1060 m.
    assert_eq!(
        at(1_059_950, 60_000).fix.reason,
        ReasonCode::SensitiveStandoff
    );
    assert_eq!(at(1_060_050, 60_000).fix.reason, ReasonCode::Ok);
    // Accuracy below the 10 m floor does not shrink it.
    assert_eq!(
        at(1_009_950, 1000).fix.reason,
        ReasonCode::SensitiveStandoff
    );
    // Direction does not matter.
    for (e, n) in [
        (1_009_950, 0),
        (-1_009_950, 0),
        (0, -1_009_950),
        (713_000, 713_000),
    ] {
        let mut sim = Sim::new(vec![]);
        sim.policy = policy.clone();
        let o = sim.step(&still(T0, e, n));
        // (713 m, 713 m) is 1008.3 m from the centre: inside as well.
        assert_eq!(o.fix.reason, ReasonCode::SensitiveStandoff, "({e}, {n})");
    }
}

#[test]
fn the_standoff_comparison_is_inclusive_to_the_millimetre() {
    // Choose the accuracy so that the projected distance equals standoff + acc_eff exactly.
    let zone_pos = pos(0, 0);
    let frame = proxim_geo::frame_for(zone_pos).unwrap();
    let policy = with_zones(vec![zone("z-1", 0, 0, None)], true);
    let mut checked = 0;
    for dlat in [91_000, 91_500, 92_000, 95_000, 97_000, 98_000] {
        let p = Position {
            lat_e7: zone_pos.lat_e7 + dlat,
            lon_e7: zone_pos.lon_e7,
        };
        let q = frame.project(p).unwrap();
        let d = i64::from(q.e_mm).abs().max(i64::from(q.n_mm).abs());
        let acc = d - 1_000_000; // standoff 1000 m
        assert!((10_000..=100_000).contains(&acc), "setup: acc {acc}");
        let at = |acc_mm: i64| {
            let mut sim = Sim::new(vec![]);
            sim.policy = policy.clone();
            let mut f = still(T0, 0, 0);
            f.lat_e7 = p.lat_e7;
            f.lon_e7 = p.lon_e7;
            f.acc_mm = u32::try_from(acc_mm).unwrap();
            sim.step(&f).fix.reason
        };
        assert_eq!(
            at(acc),
            ReasonCode::SensitiveStandoff,
            "dist == reach is inside"
        );
        assert_eq!(
            at(acc - 1),
            ReasonCode::Ok,
            "one mm less accuracy is outside"
        );
        checked += 1;
    }
    assert_eq!(checked, 6);
}

#[test]
fn a_zone_may_widen_its_standoff_but_never_shrink_it() {
    let at = |standoff: Option<u32>, n_mm: i32| {
        let mut sim = Sim::new(vec![]);
        sim.policy = with_zones(vec![zone("z-1", 0, 0, standoff)], true);
        sim.step(&still(T0, 0, n_mm)).fix.reason
    };
    // Some(0) and Some(100) are floored to the 1000 m default.
    assert_eq!(at(Some(0), 500_000), ReasonCode::SensitiveStandoff);
    assert_eq!(at(Some(100), 900_000), ReasonCode::SensitiveStandoff);
    assert_eq!(at(None, 900_000), ReasonCode::SensitiveStandoff);
    // Widened to 3 km.
    assert_eq!(at(Some(3000), 2_900_000), ReasonCode::SensitiveStandoff);
    assert_eq!(at(Some(3000), 3_100_000), ReasonCode::Ok);
    // Capped at 100 km (inside the 150 km frame).
    assert_eq!(
        at(Some(u32::MAX), 99_000_000),
        ReasonCode::SensitiveStandoff
    );
    assert_eq!(at(Some(u32::MAX), 101_000_000), ReasonCode::Ok);
}

#[test]
fn every_class_is_suppressed_and_any_zone_counts() {
    let mut health = circle_at("h-1", 5_000_000, 0, 200);
    health.class = Class::Health;
    let policy = with_zones(
        vec![
            zone("z-1", -9_000_000, 0, None),
            zone("z-2", 5_000_000, 0, None),
        ],
        true,
    );
    let mut sim = Sim::new(vec![health, circle_at("g-1", 5_000_000, 0, 200)]);
    sim.policy = policy;
    sim.consent = granted_health();
    let outs = sim.run(&visit(T0, 5_000_000, 0, 6, 0));
    assert!(
        outs.iter()
            .all(|o| o.fix.reason == ReasonCode::SensitiveStandoff)
    );
    assert!(fire_indices(&outs).is_empty(), "general and health alike");
    // Far from both zones the same signposts work (control).
    let mut sim = Sim::new(vec![circle_at("g-1", 5_000_000, 0, 200)]);
    sim.policy = with_zones(vec![zone("z-1", -9_000_000, 0, None)], true);
    let outs = sim.run(&visit(T0, 5_000_000, 0, 6, 0));
    assert!(!fire_indices(&outs).is_empty());
}

#[test]
fn a_fix_far_outside_a_zones_frame_is_not_in_it() {
    // 400 km away: beyond the 150 km zone frame. Not a standoff, and no panic or error.
    let policy = with_zones(vec![zone("z-1", 0, 0, None)], true);
    let mut sim = Sim::new(vec![]);
    sim.policy = policy;
    let p = Position {
        lat_e7: 480_000_000,
        lon_e7: 70_000_000,
    };
    let mut f = still(T0, 0, 0);
    f.lat_e7 = p.lat_e7;
    f.lon_e7 = p.lon_e7;
    assert_eq!(sim.step(&f).fix.reason, ReasonCode::Ok);
}

#[test]
fn zone_defects_are_whole_call_errors_never_a_silent_skip() {
    let fix = still(T0, 0, 0);
    let call = |policy: &Policy| {
        decide(
            &State::empty(),
            &fix,
            &[],
            policy,
            &granted(),
            &clock(T0),
            0,
        )
    };
    // Above 80 degrees the frame is unsupported: the zone cannot be evaluated, so no decision.
    let mut z = zone("z-1", 0, 0, None);
    z.center = Position {
        lat_e7: 850_000_000,
        lon_e7: 0,
    };
    assert_eq!(
        call(&with_zones(vec![z], false)),
        Err(proxim_types::EngineError::UnsupportedLatitude)
    );
    let mut z = zone("z-1", 0, 0, None);
    z.center = Position {
        lat_e7: 950_000_000,
        lon_e7: 0,
    };
    assert_eq!(
        call(&with_zones(vec![z], false)),
        Err(proxim_types::EngineError::Parse)
    );
    let many: Vec<_> = (0..257)
        .map(|i| zone(&format!("z-{i}"), 0, 0, None))
        .collect();
    assert_eq!(
        call(&with_zones(many, false)),
        Err(proxim_types::EngineError::Limit)
    );
    let ok: Vec<_> = (0..256)
        .map(|i| zone(&format!("z-{i}"), 0, 0, None))
        .collect();
    assert!(call(&with_zones(ok, false)).is_ok());
}

#[test]
fn trigger_in_standoff_drops_health_geometry_within_a_zone() {
    // F19: independent of where the user is. Zone at the origin, standoff 1000 m.
    let policy = with_zones(vec![zone("z-1", 0, 0, None)], true);
    let health = |mut sp: proxim_types::Signpost| {
        sp.class = Class::Health;
        sp
    };
    let cases = [
        // circle centre 1100 m away, radius 200: nearest edge at 900 m -> inside the standoff
        (health(circle_at("h-1", 1_100_000, 0, 200)), true),
        // centre 1300 m, radius 200: edge at 1100 m -> clear
        (health(circle_at("h-2", 1_300_000, 0, 200)), false),
        // square containing the zone centre
        (health(square_at("h-3", 0, 0, 300)), true),
        // square whose nearest edge is 950 m away
        (health(square_at("h-4", 1_250_000, 0, 300)), true),
        // square whose nearest edge is 1100 m away
        (health(square_at("h-5", 1_400_000, 0, 300)), false),
        // route passing 500 m east of the zone, north-south
        (
            {
                let mut r = health(route("h-6", 3000, 30));
                r.trigger.origin = pos(500_000, -1_000_000);
                if let Shape::Route { vertices, .. } = &mut r.trigger.shape {
                    *vertices = vec![pos(500_000, -1_000_000), pos(500_000, 2_000_000)];
                }
                r
            },
            true,
        ),
        // route 1200 m away
        (
            {
                let mut r = health(route("h-7", 3000, 30));
                r.trigger.origin = pos(1_200_000, -1_000_000);
                if let Shape::Route { vertices, .. } = &mut r.trigger.shape {
                    *vertices = vec![pos(1_200_000, -1_000_000), pos(1_200_000, 2_000_000)];
                }
                r
            },
            false,
        ),
    ];
    for (sp, blocked) in cases {
        let id = sp.id.to_string();
        let mut sim = Sim::new(vec![sp]);
        sim.policy = policy.clone();
        sim.consent = granted_health();
        // The user is 20 km away, so only F19 can act.
        let o = sim.step(&still(T0, 20_000_000, 0));
        assert_eq!(
            has_drop(&o, &id, ReasonCode::TriggerInStandoff),
            blocked,
            "{id}"
        );
    }
    // A general-class signpost with the same geometry is not F19-dropped.
    let mut sim = Sim::new(vec![circle_at("g-1", 1_100_000, 0, 200)]);
    sim.policy = policy;
    let o = sim.step(&still(T0, 20_000_000, 0));
    assert!(!has_drop(&o, "g-1", ReasonCode::TriggerInStandoff));
}

#[test]
fn trigger_in_standoff_is_inclusive_to_the_millimetre() {
    // A health circle whose nearest edge is exactly `standoff` from the zone centre is inside
    // (`<=`); one millimetre-grid step (1 m of radius) further out is clear. Find a latitude
    // offset whose projected distance is a whole number of metres so the radius can hit it.
    let zone_pos = pos(0, 0);
    let frame = proxim_geo::frame_for(zone_pos).unwrap();
    let policy = with_zones(vec![zone("z-1", 0, 0, None)], true);
    let mut found = 0;
    for dlat in 104_000..110_000 {
        let c = Position {
            lat_e7: zone_pos.lat_e7 + dlat,
            lon_e7: zone_pos.lon_e7,
        };
        let n = i64::from(frame.project(c).unwrap().n_mm);
        if n % 1000 != 0 {
            continue;
        }
        let radius_m = u32::try_from((n - 1_000_000) / 1000).unwrap();
        assert!(radius_m >= 150);
        let mk = |r: u32| {
            let mut sp = circle_at("h-1", 0, 0, 200);
            sp.class = Class::Health;
            sp.trigger.origin = c;
            sp.trigger.shape = Shape::Circle {
                center: c,
                radius_m: r,
            };
            sp
        };
        let blocked = |r: u32| {
            let mut sim = Sim::new(vec![mk(r)]);
            sim.policy = policy.clone();
            sim.consent = granted_health();
            has_drop(
                &sim.step(&still(T0, 20_000_000, 0)),
                "h-1",
                ReasonCode::TriggerInStandoff,
            )
        };
        assert!(blocked(radius_m), "edge exactly at the standoff is inside");
        assert!(!blocked(radius_m - 1), "1 m further out is clear");
        found += 1;
        if found == 3 {
            break;
        }
    }
    assert_eq!(found, 3);
}

#[test]
fn nearby_zones_never_create_decisions_for_the_standoff_itself() {
    // Defence in depth for I6: with explain, the only trace of a standoff is the fix reason.
    let policy = with_zones(vec![zone("z-1", 0, 0, None)], true);
    let mut sim = Sim::new(vec![circle("c-1", 200)]);
    sim.policy = policy;
    let o = sim.step(&still(T0, 0, 0));
    assert!(o.decisions.iter().all(|d| d.outcome != Outcome::Fire));
    assert!(o.decisions.is_empty());
}
