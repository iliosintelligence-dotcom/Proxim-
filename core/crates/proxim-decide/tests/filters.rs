//! The ordered hard filters F03-F09 (global) and F11-F21 (per signpost): one reason each, in the
//! spec's order, with boundary values, and no state change where the spec says none.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_decide::decide;
use proxim_types::canon::to_canonical_json;
use proxim_types::{Clock, Fix, Mem, Outcome, Policy, ReasonCode, Shape, Signpost, State};

fn call(
    state: &State,
    fix: &Fix,
    signposts: &[Signpost],
    policy: &Policy,
    now_ms: i64,
) -> (proxim_types::Output, State) {
    decide(
        state,
        fix,
        signposts,
        policy,
        &granted_health(),
        &Clock {
            now_ms,
            tz_offset_min: 0,
        },
        0,
    )
    .unwrap()
}

fn text(s: &State) -> String {
    to_canonical_json(s).unwrap()
}

/// A rejected fix leaves the state byte-identical (F03-F08).
fn assert_rejected(reason: ReasonCode, fix: &Fix, policy: &Policy, now_ms: i64, state: &State) {
    let (o, next) = call(state, fix, &[circle("c-1", 200)], policy, now_ms);
    assert!(!o.fix.ok, "{reason:?}");
    assert_eq!(o.fix.reason, reason);
    assert!(o.decisions.is_empty() && o.notes.is_none());
    assert_eq!(
        text(&next),
        text(state),
        "{reason:?}: state must not change"
    );
}

fn warm() -> State {
    let mut sim = Sim::new(vec![circle("c-1", 200)]);
    sim.run(&visit(T0, 0, 0, 2, 0));
    assert!(!sim.state.mem.trace.is_empty());
    sim.state
}

#[test]
fn f03_fix_invalid_ranges() {
    let st = warm();
    let base = still(T0 + 20_000, 0, 0);
    let bad: Vec<Fix> = vec![
        Fix {
            lat_e7: 900_000_001,
            ..base
        },
        Fix {
            lat_e7: -900_000_001,
            ..base
        },
        Fix {
            lon_e7: 1_800_000_001,
            ..base
        },
        Fix {
            lon_e7: -1_800_000_001,
            ..base
        },
        Fix {
            course_ddeg: Some(3600),
            ..base
        },
        Fix {
            course_ddeg: Some(u16::MAX),
            ..base
        },
        Fix { t_ms: -1, ..base },
        Fix {
            lat_e7: i32::MIN,
            ..base
        },
        Fix {
            lon_e7: i32::MAX,
            ..base
        },
    ];
    for f in bad {
        assert_rejected(ReasonCode::FixInvalid, &f, &explain(), T0 + 20_000, &st);
    }
    // The extremes themselves are valid coordinates: no error, no panic, no fire.
    for (lat, lon) in [
        (900_000_000, 1_800_000_000),
        (-900_000_000, -1_800_000_000),
        (0, 0),
    ] {
        let f = Fix {
            lat_e7: lat,
            lon_e7: lon,
            course_ddeg: Some(3599),
            ..base
        };
        let (o, _) = call(&State::empty(), &f, &world(), &explain(), T0 + 20_000);
        assert!(o.fix.ok, "({lat}, {lon})");
        assert!(fires(&o).is_empty());
    }
}

#[test]
fn f04_mock_fix_needs_simulator_mode() {
    let st = warm();
    let f = Fix {
        is_mock: true,
        ..still(T0 + 20_000, 0, 0)
    };
    assert_rejected(ReasonCode::FixMock, &f, &explain(), T0 + 20_000, &st);
    let sim_policy = Policy {
        simulator_mode: Some(true),
        ..explain()
    };
    let (o, _) = call(&st, &f, &[circle("c-1", 200)], &sim_policy, T0 + 20_000);
    assert!(o.fix.ok);
    let off = Policy {
        simulator_mode: Some(false),
        ..explain()
    };
    assert_rejected(ReasonCode::FixMock, &f, &off, T0 + 20_000, &st);
}

#[test]
fn f05_accuracy_boundary_and_ceiling() {
    let st = warm();
    let at = |acc: u32, policy: &Policy| {
        let f = Fix {
            acc_mm: acc,
            ..still(T0 + 20_000, 0, 0)
        };
        call(&st, &f, &[], policy, T0 + 20_000).0.fix.reason
    };
    assert_eq!(
        at(100_000, &explain()),
        ReasonCode::Ok,
        "the ceiling itself passes"
    );
    assert_eq!(at(100_001, &explain()), ReasonCode::FixAccuracyTooCoarse);
    let narrow = Policy {
        max_acc_mm: Some(30_000),
        ..explain()
    };
    assert_eq!(at(30_000, &narrow), ReasonCode::Ok);
    assert_eq!(at(30_001, &narrow), ReasonCode::FixAccuracyTooCoarse);
    let wide = Policy {
        max_acc_mm: Some(u32::MAX),
        ..explain()
    };
    assert_eq!(
        at(100_001, &wide),
        ReasonCode::FixAccuracyTooCoarse,
        "partners cannot widen"
    );
    assert_rejected(
        ReasonCode::FixAccuracyTooCoarse,
        &Fix {
            acc_mm: u32::MAX,
            ..still(T0 + 20_000, 0, 0)
        },
        &explain(),
        T0 + 20_000,
        &st,
    );
}

#[test]
fn f06_future_and_f07_stale_boundaries() {
    let st = warm();
    let now = T0 + 100_000;
    let reason = |t: i64| {
        call(&st, &still(t, 0, 0), &[], &explain(), now)
            .0
            .fix
            .reason
    };
    assert_eq!(reason(now + 5000), ReasonCode::Ok);
    assert_eq!(reason(now + 5001), ReasonCode::FixFromFuture);
    assert_eq!(reason(now - 30_000), ReasonCode::Ok);
    assert_eq!(reason(now - 30_001), ReasonCode::FixStale);
    assert_rejected(
        ReasonCode::FixFromFuture,
        &still(now + 60_000, 0, 0),
        &explain(),
        now,
        &st,
    );
    assert_rejected(
        ReasonCode::FixStale,
        &still(now - 60_000, 0, 0),
        &explain(),
        now,
        &st,
    );
    // Extreme times saturate instead of overflowing.
    let (o, _) = call(
        &State::empty(),
        &still(i64::MAX, 0, 0),
        &[],
        &explain(),
        i64::MAX,
    );
    assert_eq!(o.fix.reason, ReasonCode::Ok);
    let (o, _) = call(&State::empty(), &still(0, 0, 0), &[], &explain(), i64::MAX);
    assert_eq!(o.fix.reason, ReasonCode::FixStale);
}

#[test]
fn f08_out_of_order_equal_or_older() {
    let st = warm();
    let last = st.mem.last_t_ms.unwrap();
    assert_rejected(
        ReasonCode::FixOutOfOrder,
        &still(last, 0, 0),
        &explain(),
        last + 1000,
        &st,
    );
    assert_rejected(
        ReasonCode::FixOutOfOrder,
        &still(last - 1, 0, 0),
        &explain(),
        last + 1000,
        &st,
    );
    let (o, _) = call(&st, &still(last + 1, 0, 0), &[], &explain(), last + 1000);
    assert!(o.fix.ok);
}

#[test]
fn f09_jump_rejects_count_and_the_third_resets_and_accepts() {
    let mut sim = Sim::new(vec![circle("c-1", 200)]);
    sim.run(&visit(T0, 0, 0, 3, 0)); // candidate n = 3, trace of 3
    assert_eq!(sim.state.mem.det.len(), 1);
    let t = T0 + 12_000;
    // 2 km in 10 s = 200 m/s and far beyond both accuracies plus 100 m: a jump.
    let o1 = sim.step(&still(t + 10_000, 2_000_000, 0));
    assert_eq!(o1.fix.reason, ReasonCode::FixJump);
    assert_eq!(sim.state.mem.jump_rejects, 1);
    assert_eq!(
        sim.state.mem.last_t_ms,
        Some(T0 + 8000),
        "a rejected fix is not recorded"
    );
    assert_eq!(sim.state.mem.trace.len(), 3);
    let o2 = sim.step(&still(t + 14_000, 2_000_000, 0));
    assert_eq!(o2.fix.reason, ReasonCode::FixJump);
    assert_eq!(sim.state.mem.jump_rejects, 2);
    // The third consecutive reject resets trace and detectors and accepts the fix.
    let o3 = sim.step(&still(t + 18_000, 2_000_000, 0));
    assert!(o3.fix.ok, "{:?}", o3.fix);
    assert_eq!(sim.state.mem.jump_rejects, 0);
    assert_eq!(sim.state.mem.trace.len(), 1);
    assert_eq!(sim.state.mem.trace[0].t_ms, t + 18_000);
    assert!(
        sim.state.mem.det.is_empty(),
        "detectors were reset (and none applies at 2 km)"
    );
    // Caps are never touched by a reset.
    assert_eq!(sim.state.caps.hwm_ms, t + 18_000);
}

#[test]
fn f09_an_accepted_fix_clears_the_reject_counter() {
    let mut sim = Sim::new(vec![]);
    sim.step(&still(T0, 0, 0));
    sim.step(&still(T0 + 10_000, 2_000_000, 0));
    sim.step(&still(T0 + 14_000, 2_000_000, 0));
    assert_eq!(sim.state.mem.jump_rejects, 2);
    // A sane fix comes back near the last accepted one: counter resets, no reset of the trace.
    let o = sim.step(&still(T0 + 18_000, 5_000, 0));
    assert!(o.fix.ok);
    assert_eq!(sim.state.mem.jump_rejects, 0);
    assert_eq!(sim.state.mem.trace.len(), 2);
    // The next jump starts counting from zero again.
    sim.step(&still(T0 + 28_000, 2_000_000, 0));
    assert_eq!(sim.state.mem.jump_rejects, 1);
}

#[test]
fn f09_needs_both_speed_and_distance_and_a_recent_fix() {
    let judge = |dt_ms: i64, d_mm: i32, acc: u32| {
        let mut sim = Sim::new(vec![]);
        let mut a = still(T0, 0, 0);
        a.acc_mm = acc;
        sim.step(&a);
        let mut b = still(T0 + dt_ms, 0, d_mm);
        b.acc_mm = acc;
        sim.step(&b).fix.reason
    };
    // Accuracies 5 + 5 m + 100 m = 110 m margin; speed limit 83 m/s.
    assert_eq!(
        judge(1000, 109_900, 5000),
        ReasonCode::Ok,
        "fast but within the margin"
    );
    assert_eq!(judge(1000, 110_100, 5000), ReasonCode::FixJump);
    assert_eq!(
        judge(10_000, 829_900, 5000),
        ReasonCode::Ok,
        "far but not faster than 83 m/s"
    );
    assert_eq!(judge(10_000, 830_100, 5000), ReasonCode::FixJump);
    // Worse accuracies widen the margin: 60 + 60 + 100 = 220 m.
    assert_eq!(judge(1000, 219_900, 60_000), ReasonCode::Ok);
    assert_eq!(judge(1000, 220_100, 60_000), ReasonCode::FixJump);
    // A previous fix older than 60 s is not compared at all.
    assert_eq!(judge(60_000, 90_000_000, 5000), ReasonCode::FixJump);
    assert_eq!(judge(60_001, 90_000_000, 5000), ReasonCode::Ok);
}

#[test]
fn global_filters_apply_in_spec_order() {
    // A fix violating several filters reports the first in F03..F09 order.
    let st = warm();
    let last = st.mem.last_t_ms.unwrap();
    let now = last + 200_000;
    let worst = Fix {
        lat_e7: 900_000_001,
        is_mock: true,
        acc_mm: 200_000,
        t_ms: last - 5,
        ..still(0, 0, 0)
    };
    let steps: [(ReasonCode, Fix); 6] = [
        (ReasonCode::FixInvalid, worst),
        (ReasonCode::FixMock, Fix { lat_e7: 0, ..worst }),
        (
            ReasonCode::FixAccuracyTooCoarse,
            Fix {
                lat_e7: 0,
                is_mock: false,
                ..worst
            },
        ),
        (
            ReasonCode::FixFromFuture,
            Fix {
                lat_e7: 0,
                is_mock: false,
                acc_mm: 5000,
                t_ms: now + 9000,
                ..worst
            },
        ),
        (
            ReasonCode::FixStale,
            Fix {
                lat_e7: 0,
                is_mock: false,
                acc_mm: 5000,
                ..worst
            },
        ),
        (
            ReasonCode::FixOutOfOrder,
            Fix {
                lat_e7: 0,
                is_mock: false,
                acc_mm: 5000,
                t_ms: last,
                ..worst
            },
        ),
    ];
    // FIX_STALE vs FIX_OUT_OF_ORDER: make the out-of-order fix recent enough to pass F07.
    for (i, (reason, f)) in steps.iter().enumerate() {
        let now_i = if i == 5 { last + 10_000 } else { now };
        let (o, _) = call(&st, f, &[], &explain(), now_i);
        assert_eq!(o.fix.reason, *reason, "case {i}");
    }
    // Consent (F01) and child_directed (F02) come before all of them.
    let (o, _) = decide(
        &st,
        &worst,
        &[],
        &explain(),
        &proxim_types::Consent::default(),
        &clock(now),
        0,
    )
    .unwrap();
    assert_eq!(o.fix.reason, ReasonCode::ConsentDenied);
}

#[test]
fn f10_follows_f09() {
    // A jump into a standoff is reported as a jump (F09 precedes F10), without leaking the zone.
    use proxim_types::{Id, StandoffZone};
    let mut sim = Sim::new(vec![]);
    sim.step(&still(T0, 5_000_000, 0));
    sim.policy.standoff_zones = vec![StandoffZone {
        id: Id::new("z").unwrap(),
        center: pos(0, 0),
        standoff_m: None,
    }];
    let o = sim.step(&still(T0 + 5000, 0, 0));
    assert_eq!(o.fix.reason, ReasonCode::FixJump);
}

// ---- per-signpost static filters ----

fn reasons_for(sp: Signpost, policy: &Policy, consent_health: bool, now: i64) -> Vec<ReasonCode> {
    let mut sim = Sim::new(vec![sp]);
    sim.policy = policy.clone();
    if !consent_health {
        sim.consent = granted();
    } else {
        sim.consent = granted_health();
    }
    let o = sim.step_at(&still(now, 0, 0), now);
    o.decisions
        .iter()
        .filter(|d| d.outcome == Outcome::Drop)
        .map(|d| d.reason)
        .collect()
}

fn drops(sp: Signpost) -> Vec<ReasonCode> {
    reasons_for(sp, &explain(), true, T0)
}

#[test]
fn f12_trigger_defects_are_per_signpost_drops() {
    let invalid = ReasonCode::TriggerInvalid;
    // radius floor
    assert_eq!(drops(circle("c", 149)), vec![invalid]);
    assert!(drops(circle("c", 150)).is_empty());
    // buffer range
    assert_eq!(drops(route("r", 2500, 29)), vec![invalid]);
    assert_eq!(drops(route("r", 2500, 501)), vec![invalid]);
    assert!(drops(route("r", 2500, 30)).is_empty());
    assert!(drops(route("r", 2500, 500)).is_empty());
    // lead range (authored value), also when the policy would clamp it into range
    let lead = |l: u32| {
        let mut r = route("r", 2500, 30);
        r.trigger.lead_s = Some(l);
        r
    };
    assert_eq!(drops(lead(14)), vec![invalid]);
    assert_eq!(drops(lead(121)), vec![invalid]);
    assert!(drops(lead(15)).is_empty() && drops(lead(120)).is_empty());
    // d_hi range
    let dhi = |d: u32| {
        let mut r = route("r", 2500, 30);
        r.trigger.d_hi_m = Some(d);
        r
    };
    assert_eq!(drops(dhi(0)), vec![invalid]);
    assert_eq!(drops(dhi(5001)), vec![invalid]);
    assert!(drops(dhi(5000)).is_empty());
    // shape / event mismatch
    let mut sp = circle("c", 200);
    sp.event = proxim_types::EventKind::Approach;
    assert_eq!(drops(sp), vec![invalid]);
    let mut sp = route("r", 2500, 30);
    sp.event = proxim_types::EventKind::Enter;
    assert_eq!(drops(sp), vec![invalid]);
    // validity window
    let mut sp = circle("c", 200);
    sp.valid_from_ms = 10;
    sp.valid_until_ms = 10;
    assert_eq!(drops(sp), vec![invalid]);
    let mut sp = circle("c", 200);
    sp.valid_from_ms = 11;
    sp.valid_until_ms = 10;
    assert_eq!(drops(sp), vec![invalid]);
    // too few polygon vertices, too many route vertices
    let mut sp = square_at("p", 0, 0, 300);
    if let Shape::Polygon { vertices, .. } = &mut sp.trigger.shape {
        vertices.truncate(2);
    }
    assert_eq!(drops(sp), vec![invalid]);
    let mut sp = route("r", 2500, 30);
    if let Shape::Route { vertices, .. } = &mut sp.trigger.shape {
        let first = vertices[0];
        let last = *vertices.last().unwrap();
        *vertices = (0..1025)
            .map(|i| if i % 2 == 0 { first } else { last })
            .collect();
    }
    assert_eq!(drops(sp), vec![invalid]);
    // geometry outside the 150 km frame of its own origin
    let mut sp = circle("c", 200);
    if let Shape::Circle { center, .. } = &mut sp.trigger.shape {
        center.lat_e7 += 30_000_000; // 3 degrees north
    }
    assert_eq!(drops(sp), vec![invalid]);
    // origin above 80 degrees
    let mut sp = circle("c", 200);
    sp.trigger.origin.lat_e7 = 850_000_000;
    assert_eq!(drops(sp), vec![invalid]);
    // anchor beyond the route
    let mut sp = route("r", 2500, 30);
    if let Shape::Route { anchor_along_m, .. } = &mut sp.trigger.shape {
        *anchor_along_m = Some(2501);
    }
    assert_eq!(drops(sp), vec![invalid]);
}

#[test]
fn a_defective_signpost_never_blocks_its_neighbours() {
    let mut sim = Sim::new(vec![
        circle("a-bad", 10),
        circle("b-ok", 200),
        circle("c-bad", 149),
    ]);
    let outs = sim.run(&visit(T0, 0, 0, 5, 0));
    assert!(
        outs.iter()
            .all(|o| has_drop(o, "a-bad", ReasonCode::TriggerInvalid))
    );
    assert!(
        outs.iter()
            .all(|o| has_drop(o, "c-bad", ReasonCode::TriggerInvalid))
    );
    assert_eq!(fire_indices(&outs), vec![3]);
    assert!(sim.state.mem.det.iter().all(|e| e.id.as_str() == "b-ok"));
}

#[test]
fn f13_f14_live_window_is_half_open() {
    let mut sp = circle("c", 200);
    sp.valid_from_ms = T0;
    sp.valid_until_ms = T0 + 1000;
    let at = |now: i64| reasons_for(sp.clone(), &explain(), true, now);
    assert_eq!(at(T0 - 1), vec![ReasonCode::NotYetLive]);
    assert!(at(T0).is_empty(), "valid_from is inclusive");
    assert!(at(T0 + 999).is_empty());
    assert_eq!(
        at(T0 + 1000),
        vec![ReasonCode::Expired],
        "valid_until is exclusive"
    );
    assert_eq!(at(T0 + 5000), vec![ReasonCode::Expired]);
}

#[test]
fn f15_to_f18_block_lists_and_ceiling() {
    let mut sp = circle("c", 200);
    sp.category = "gambling".into();
    sp.sender = "acme".into();
    sp.sensitivity = 2;
    let p = |f: &dyn Fn(&mut Policy)| {
        let mut p = explain();
        f(&mut p);
        p
    };
    assert!(reasons_for(sp.clone(), &explain(), true, T0).is_empty());
    assert_eq!(
        reasons_for(
            sp.clone(),
            &p(&|p| p.categories_blocked = vec!["gambling".into()]),
            true,
            T0
        ),
        vec![ReasonCode::PartnerCategoryBlocked]
    );
    assert!(
        reasons_for(
            sp.clone(),
            &p(&|p| p.categories_blocked = vec!["Gambling".into(), "gambling ".into()]),
            true,
            T0
        )
        .is_empty(),
        "exact string match only"
    );
    assert_eq!(
        reasons_for(
            sp.clone(),
            &p(&|p| p.senders_blocked = vec!["acme".into()]),
            true,
            T0
        ),
        vec![ReasonCode::SenderBlocked]
    );
    // Category is checked before sender.
    assert_eq!(
        reasons_for(
            sp.clone(),
            &p(&|p| {
                p.categories_blocked = vec!["gambling".into()];
                p.senders_blocked = vec!["acme".into()];
            }),
            true,
            T0
        ),
        vec![ReasonCode::PartnerCategoryBlocked]
    );
    // Sensitivity ceiling: default 2, a partner may lower it but never raise it.
    assert_eq!(
        reasons_for(
            sp.clone(),
            &p(&|p| p.sensitivity_ceiling = Some(1)),
            true,
            T0
        ),
        vec![ReasonCode::SensitivityAboveCeiling]
    );
    let mut hi = sp.clone();
    hi.sensitivity = 3;
    assert_eq!(
        reasons_for(
            hi.clone(),
            &p(&|p| p.sensitivity_ceiling = Some(200)),
            true,
            T0
        ),
        vec![ReasonCode::SensitivityAboveCeiling],
        "a partner cannot raise the platform ceiling"
    );
    assert_eq!(
        reasons_for(hi, &explain(), true, T0),
        vec![ReasonCode::SensitivityAboveCeiling]
    );
    // F15 precedes F16-F18.
    let mut h = sp;
    h.class = proxim_types::Class::Health;
    assert_eq!(
        reasons_for(
            h,
            &p(&|p| p.categories_blocked = vec!["gambling".into()]),
            false,
            T0
        ),
        vec![ReasonCode::HealthConsentMissing]
    );
}

#[test]
fn static_filters_apply_in_spec_order() {
    // Everything wrong at once: reserved event first (F11), then trigger defects (F12), then the
    // live window (F13), expiry (F14), health consent (F15), category (F16), sender (F17).
    let mut sp = circle("c", 200);
    sp.event = proxim_types::EventKind::Pass;
    sp.class = proxim_types::Class::Health;
    sp.category = "x".into();
    sp.sender = "y".into();
    sp.sensitivity = 9;
    let policy = Policy {
        categories_blocked: vec!["x".into()],
        senders_blocked: vec!["y".into()],
        ..explain()
    };
    type Mutation = Box<dyn Fn(&mut Signpost)>;
    let walk: Vec<(Mutation, ReasonCode)> = vec![
        (Box::new(|_| {}), ReasonCode::UnsupportedEvent),
        (
            Box::new(|s| {
                s.event = proxim_types::EventKind::Enter;
                s.valid_until_ms = s.valid_from_ms;
            }),
            ReasonCode::TriggerInvalid,
        ),
        (
            Box::new(|s| {
                s.valid_from_ms = T0 + 10;
                s.valid_until_ms = T0 + 20;
            }),
            ReasonCode::NotYetLive,
        ),
        (
            Box::new(|s| {
                s.valid_from_ms = 0;
                s.valid_until_ms = T0;
            }),
            ReasonCode::Expired,
        ),
        (
            Box::new(|s| s.valid_until_ms = i64::MAX),
            ReasonCode::HealthConsentMissing,
        ),
    ];
    let mut s = sp;
    for (mutate, want) in walk {
        mutate(&mut s);
        assert_eq!(reasons_for(s.clone(), &policy, false, T0), vec![want]);
    }
    s.class = proxim_types::Class::General;
    assert_eq!(
        reasons_for(s.clone(), &policy, false, T0),
        vec![ReasonCode::PartnerCategoryBlocked]
    );
    s.category = "ok".into();
    assert_eq!(
        reasons_for(s.clone(), &policy, false, T0),
        vec![ReasonCode::SenderBlocked]
    );
    s.sender = "ok".into();
    assert_eq!(
        reasons_for(s.clone(), &policy, false, T0),
        vec![ReasonCode::SensitivityAboveCeiling]
    );
    s.sensitivity = 2;
    assert!(reasons_for(s, &policy, false, T0).is_empty());
}

#[test]
fn revisions_and_duplicates() {
    // The highest revision is processed; lower ones are superseded (TRIGGER_INVALID drop).
    let mut v1 = circle("a", 200);
    v1.rev = 1;
    let mut v2 = circle("a", 200);
    v2.rev = 2;
    let mut sim = Sim::new(vec![v1.clone(), v2.clone()]);
    let outs = sim.run(&visit(T0, 0, 0, 5, 0));
    assert_eq!(fire_indices(&outs), vec![3]);
    assert_eq!(fires(&outs[3])[0].rev, 2);
    assert!(
        outs[0]
            .decisions
            .iter()
            .any(|d| d.rev == 1 && d.reason == ReasonCode::TriggerInvalid)
    );
    // Two copies of the same id and revision are ambiguous: neither is processed.
    let mut sim = Sim::new(vec![v2.clone(), v2.clone()]);
    let outs = sim.run(&visit(T0, 0, 0, 5, 0));
    assert!(fire_indices(&outs).is_empty());
    assert_eq!(
        outs[0]
            .decisions
            .iter()
            .filter(|d| d.reason == ReasonCode::TriggerInvalid)
            .count(),
        2
    );
    assert!(sim.state.mem.det.is_empty());
    // A new revision replaces the detector entry: progress of the old one is not inherited.
    let mut sim = Sim::new(vec![v1]);
    sim.run(&visit(T0, 0, 0, 3, 0)); // candidate n = 3 on rev 1
    sim.signposts = vec![v2.clone()];
    let o = sim.step(&still(T0 + 12_000, 0, 0));
    assert!(fires(&o).is_empty(), "rev 2 starts from idle");
    assert_eq!(sim.state.mem.det[0].rev, 2);
    // E-D7: an id that is ambiguous for one call loses its detector entry, so a half-confirmed
    // candidate cannot fire on its old confirmation when a single copy returns.
    let mut sim = Sim::new(vec![v2.clone()]);
    // Fixes at 0, 4 and 8 s: candidate n = 3, but a still walker needs 10 s, so not yet confirmed.
    sim.run(&visit(T0, 0, 0, 3, 0));
    assert_eq!(sim.state.mem.det.len(), 1);
    sim.signposts = vec![v2.clone(), v2.clone()];
    let o = sim.step(&still(T0 + 9_000, 0, 0));
    assert!(fires(&o).is_empty());
    assert!(
        sim.state.mem.det.is_empty(),
        "ambiguous id: detector removed"
    );
    sim.signposts = vec![v2];
    let o = sim.step(&still(T0 + 11_000, 0, 0));
    assert!(
        fires(&o).is_empty(),
        "restarts from idle, no inherited confirmation"
    );
}

// ---- event filters F20 / F21 with an aged clock ----

#[test]
fn f20_event_too_old_with_an_aged_clock() {
    // ENTER event at the 4th still fix; the clock is read 6 s later. Default 30 s: fine.
    let run = |max_age: Option<u32>, lag_ms: i64| {
        let mut sp = circle("c", 200);
        sp.trigger.max_event_age_s = max_age;
        let mut sim = Sim::new(vec![sp]);
        let fixes = visit(T0, 0, 0, 4, 0);
        for f in &fixes[..3] {
            sim.step(f);
        }
        sim.step_at(&fixes[3], fixes[3].t_ms + lag_ms)
    };
    assert_eq!(fire_indices(&[run(None, 6000)]), vec![0]);
    assert_eq!(
        fire_indices(&[run(Some(5), 5000)]),
        vec![0],
        "age == limit passes"
    );
    let o = run(Some(5), 5001);
    assert!(has_drop(&o, "c", ReasonCode::EventTooOld));
    assert!(fires(&o).is_empty());
    // Default limits: ENTER 30 s (F07 allows at most 30 s of lag, so the default never trips).
    assert_eq!(fire_indices(&[run(None, 30_000)]), vec![0]);
}

#[test]
fn a_dropped_event_is_consumed_not_retried() {
    let mut sp = circle("c", 200);
    sp.trigger.max_event_age_s = Some(1);
    let mut sim = Sim::new(vec![sp]);
    let fixes = visit(T0, 0, 0, 8, 0);
    let mut outs = Vec::new();
    for (i, f) in fixes.iter().enumerate() {
        outs.push(sim.step_at(f, f.t_ms + if i == 3 { 5000 } else { 0 }));
    }
    assert!(has_drop(&outs[3], "c", ReasonCode::EventTooOld));
    assert!(
        fire_indices(&outs).is_empty(),
        "the detector is `in`: no second event while inside"
    );
    assert!(matches!(
        sim.state.mem.det[0].st,
        proxim_types::DetState::In { .. }
    ));
    assert!(
        sim.state.caps.signposts.is_empty(),
        "a dropped event spends no cap"
    );
    assert_eq!(sim.state.caps.last_fire_ms, None);
}

#[test]
fn f21_event_too_far_uses_cross_track_for_a_route() {
    // Drive 12 m east of the route axis (inside the 30 m buffer). With max_event_dist_m = 5 the
    // event is 12 m from the route: dropped. On the axis (0 m) it passes even with limit 0.
    let run = |max_dist: Option<u32>, e_mm: i32| {
        let mut sp = route("r", 2500, 30);
        sp.trigger.max_event_dist_m = max_dist;
        let mut sim = Sim::new(vec![sp]);
        sim.run(&drive_north(T0, 1_400_000, 15_000, 60, e_mm))
    };
    assert_eq!(fire_indices(&run(None, 12_000)), vec![32]);
    let outs = run(Some(5), 12_000);
    assert!(fire_indices(&outs).is_empty());
    assert!(
        outs.iter()
            .any(|o| has_drop(o, "r", ReasonCode::EventTooFar))
    );
    assert_eq!(
        fire_indices(&run(Some(0), 0)),
        vec![32],
        "exactly 0 m is not too far"
    );
    assert_eq!(
        fire_indices(&run(Some(12), 12_000)),
        vec![32],
        "limit == distance passes"
    );
}

#[test]
fn only_the_fix_that_made_the_event_decides_no_other_state_leaks() {
    // State after a fire differs from the state before only in caps, mem and nothing else.
    let mut sim = Sim::new(vec![circle("c", 200)]);
    let fixes = visit(T0, 0, 0, 4, 0);
    for f in &fixes[..3] {
        sim.step(f);
    }
    let before: State = sim.state.clone();
    sim.step(&fixes[3]);
    let after = &sim.state;
    assert_eq!(after.caps.signposts.len(), 1);
    assert_eq!(after.caps.signposts[0].day_n, 1);
    assert_eq!(after.caps.signposts[0].week_n, 1);
    assert_eq!(after.caps.last_fire_ms, Some(fixes[3].t_ms));
    assert_eq!(after.caps.hwm_ms, fixes[3].t_ms);
    assert_ne!(before.mem, Mem::default());
    assert_eq!(after.v, 1);
}

// ---- detector entry pruning ----

#[test]
fn absent_detector_entries_are_pruned_after_900_seconds_but_cool_downs_are_kept() {
    use proxim_types::{DetEntry, DetState, Id};
    let now = T0 + 10_000_000;
    let entry = |id: &str, st: DetState| DetEntry {
        id: Id::new(id).unwrap(),
        rev: 1,
        st,
    };
    let cand = |last_t: i64| DetState::Cand {
        n: 1,
        first_t: last_t,
        last_t,
    };
    let mut st = State::empty();
    st.mem.det = vec![
        entry("a-fresh", cand(now - 899_999)),
        entry("b-exactly", cand(now - 900_000)),
        entry("c-old", cand(now - 900_001)),
        entry("d-cool-live", DetState::Cool { until_ms: now + 1 }),
        entry("e-cool-over", DetState::Cool { until_ms: now }),
        entry(
            "f-in-old",
            DetState::In {
                xn: 0,
                x_first_t: None,
                last_t: now - 1_000_000,
            },
        ),
    ];
    st.caps.hwm_ms = now - 1000;
    // No signposts at all: every entry is absent.
    let (_, next) = call(&st, &still(now, 0, 0), &[], &explain(), now);
    let kept: Vec<_> = next.mem.det.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(kept, ["a-fresh", "b-exactly", "d-cool-live"]);
    // The same entries for ids that ARE in the input are never pruned by age: they are stepped.
    let present: Vec<Signpost> = ["a-fresh", "c-old", "d-cool-live", "f-in-old"]
        .iter()
        .map(|id| circle(id, 200))
        .collect();
    let mut st2 = st.clone();
    st2.mem.det.retain(|e| present.iter().any(|p| p.id == e.id));
    let (_, next) = call(&st2, &still(now, 6_000_000, 0), &present, &explain(), now);
    let kept: Vec<_> = next.mem.det.iter().map(|e| e.id.as_str()).collect();
    // d-cool-live stays in cool-down; f-in-old became a cool-down after the long gap; the two
    // candidates went idle on a fix far outside their circles.
    assert_eq!(kept, ["d-cool-live", "f-in-old"]);
    assert!(
        next.mem
            .det
            .iter()
            .all(|e| matches!(e.st, DetState::Cool { .. }))
    );
}
