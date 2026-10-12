//! ENTER with exit hysteresis: scenario tests (engine-spec 6.1, 11).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{Driver, circle, fix_at, signpost, square};
use proxim_detect::{Detected, Prepared, detect};
use proxim_path::path_update;
use proxim_types::{DetState, EventKind, Fix, Mem, ReasonCode};

/// Fixes moving due north through the origin at `v` mm/s, 1 Hz, accuracy `acc`, offset `e` east;
/// `n_i = n0 + v i`.
fn pass(n0: i32, v: i32, e: i32, count: i32, acc: u32) -> Vec<Fix> {
    (0..count)
        .map(|i| {
            fix_at(
                i64::from(i) * 1000,
                e,
                n0 + v * i,
                acc,
                Some(v as u32),
                Some(0),
            )
        })
        .collect()
}

fn states(d: &Driver) -> Vec<Option<DetState>> {
    d.steps.iter().map(|s| s.out.state).collect()
}

#[test]
fn drive_in_at_15_ms_enters_on_the_fourth_inside_fix() {
    // Radius 150 m, acc_eff 10 m: Inside needs distance <= 140 m. From -300 m at 15 m/s the
    // first Inside fix is i=11 (-135 m); ENTER needs 3 fixes spanning 3 s: i=14 (-90 m).
    let mut d = Driver::new(&circle(150));
    d.feed_all(&pass(-300_000, 15_000, 0, 45, 5000));
    let ev = d.events();
    assert_eq!(ev.len(), 1, "exactly one ENTER");
    let (i, e) = ev[0];
    assert_eq!(i, 14);
    assert_eq!(e.kind, EventKind::Enter);
    assert_eq!(e.t_ms, 14_000);
    assert_eq!((e.path_tier, e.path_conf_milli), (0, 500));
    assert_eq!(
        (e.d_rem_mm, e.t_hat_ms, e.dist_to_shape_mm),
        (None, None, 0)
    );
    // Candidate counting before the event.
    let st = states(&d);
    assert_eq!(
        st[10], None,
        "-150 m is exactly on the boundary: a straddle, idle"
    );
    assert_eq!(
        st[11],
        Some(DetState::Cand {
            n: 1,
            first_t: 11_000,
            last_t: 11_000
        })
    );
    assert_eq!(
        st[12],
        Some(DetState::Cand {
            n: 2,
            first_t: 11_000,
            last_t: 12_000
        })
    );
    assert_eq!(
        st[13],
        Some(DetState::Cand {
            n: 3,
            first_t: 11_000,
            last_t: 13_000
        }),
        "n = 3 but only 2 s"
    );
    assert_eq!(
        st[14],
        Some(DetState::In {
            xn: 0,
            x_first_t: None,
            last_t: 14_000
        })
    );
    // Leaving the far side: Outside needs >= 160 m (i = 31 at +165 m); the fifth Outside fix
    // (i = 35) confirms the exit; EXIT delivers nothing.
    assert!(
        matches!(st[34], Some(DetState::In { xn: 4, .. })),
        "{:?}",
        st[34]
    );
    assert_eq!(
        st[35],
        Some(DetState::Cool {
            until_ms: 35_000 + 1_800_000
        })
    );
    assert!(d.steps[35].out.event.is_none());
}

#[test]
fn drive_past_with_30_m_clearance_never_enters() {
    // Passing 30 m beyond the boundary (180 m from the centre): never Inside.
    let mut d = Driver::new(&circle(150));
    d.feed_all(&pass(-500_000, 15_000, 180_000, 70, 5000));
    assert!(d.events().is_empty());
    assert!(
        states(&d).iter().all(Option::is_none),
        "the detector never even starts a candidate"
    );
    // Passing 5 m inside the boundary: depth 5 m < acc_eff 10 m, so every fix straddles.
    let mut d = Driver::new(&circle(150));
    d.feed_all(&pass(-500_000, 15_000, 145_000, 70, 5000));
    assert!(d.events().is_empty());
    assert!(states(&d).iter().all(Option::is_none));
    // A deep chord 100 m from the centre does enter.
    let mut d = Driver::new(&circle(150));
    d.feed_all(&pass(-500_000, 15_000, 100_000, 70, 5000));
    assert_eq!(d.events().len(), 1);
}

#[test]
fn boundary_depth_is_exact_around_acc_eff() {
    // Stationary fixes east of the centre: 139.5 m is Inside (depth 10.5 m), 140.5 m is not.
    for (e, enters) in [
        (139_500, true),
        (140_500, false),
        (100_000, true),
        (150_000, false),
    ] {
        let mut d = Driver::new(&circle(150));
        for i in 0..6 {
            d.feed(fix_at(i * 1000, e, 0, 5000, Some(2000), None), 0);
        }
        assert_eq!(d.events().len(), usize::from(enters), "e = {e}");
    }
}

#[test]
fn u_turn_before_the_third_inside_fix_never_enters() {
    // Inside at -135 and -120 m, then back out: three Inside fixes would be needed.
    let mut d = Driver::new(&circle(150));
    let ns = [
        -165_000, -150_000, -135_000, -120_000, -135_000, -150_000, -165_000, -180_000,
    ];
    for (i, n) in ns.iter().enumerate() {
        d.feed(
            fix_at(i as i64 * 1000, 0, *n, 5000, Some(15_000), Some(0)),
            0,
        );
    }
    // -135, -120, -135 are three Inside fixes but they span 2 s < 3 s; -150 straddles.
    assert!(d.events().is_empty());
    assert_eq!(d.st, None);
}

#[test]
fn u_turn_after_enter_stays_in_and_does_not_re_enter() {
    let mut d = Driver::new(&circle(150));
    let mut fixes = pass(-300_000, 15_000, 0, 16, 5000); // enter at i = 14, then at -75 m
    // U-turn at 0 m... drive back south inside, then out and in again, all inside the dwell.
    for i in 16..30 {
        let n = -75_000 + 15_000 - (i64::from(i) - 16) * 15_000;
        fixes.push(fix_at(
            i64::from(i) * 1000,
            0,
            n as i32,
            5000,
            Some(15_000),
            Some(1800),
        ));
    }
    d.feed_all(&fixes);
    assert_eq!(d.events().len(), 1, "one ENTER only");
}

#[test]
fn gps_gap_restarts_the_candidate() {
    let mut d = Driver::new(&circle(150));
    d.feed(fix_at(0, 0, 0, 5000, Some(5000), None), 0);
    d.feed(fix_at(1000, 0, 0, 5000, Some(5000), None), 0);
    // 14 s later: gap > 10 s restarts at n = 1.
    let o = d.feed(fix_at(15_000, 0, 0, 5000, Some(5000), None), 0);
    assert_eq!(
        o.state,
        Some(DetState::Cand {
            n: 1,
            first_t: 15_000,
            last_t: 15_000
        })
    );
    for t in [16_000, 17_000] {
        assert!(
            d.feed(fix_at(t, 0, 0, 5000, Some(5000), None), 0)
                .event
                .is_none()
        );
    }
    assert!(
        d.feed(fix_at(18_000, 0, 0, 5000, Some(5000), None), 0)
            .event
            .is_some()
    );
    // The gap limit itself: exactly 10 s keeps the candidate.
    let mut d = Driver::new(&circle(150));
    d.feed(fix_at(0, 0, 0, 5000, Some(5000), None), 0);
    let o = d.feed(fix_at(10_000, 0, 0, 5000, Some(5000), None), 0);
    assert_eq!(
        o.state,
        Some(DetState::Cand {
            n: 2,
            first_t: 0,
            last_t: 10_000
        })
    );
}

#[test]
fn in_survives_gaps_up_to_900_seconds_and_cools_beyond() {
    let mut d = Driver::new(&circle(150));
    for i in 0..5 {
        d.feed(fix_at(i * 1000, 0, 0, 5000, Some(5000), None), 0);
    }
    assert_eq!(d.events().len(), 1);
    let t_in = 4000;
    // 900 s of silence, then Inside: still `in` (no gap > 900 s), no new event.
    let o = d.feed(fix_at(t_in + 900_000, 0, 0, 5000, Some(5000), None), 0);
    assert_eq!(
        o.state,
        Some(DetState::In {
            xn: 0,
            x_first_t: None,
            last_t: t_in + 900_000
        })
    );
    assert!(o.event.is_none());
    // 900.001 s of silence: the user is assumed gone; cool-down from the last seen time.
    let mut d2 = Driver::new(&circle(150));
    for i in 0..5 {
        d2.feed(fix_at(i * 1000, 0, 0, 5000, Some(5000), None), 0);
    }
    let o = d2.feed(fix_at(t_in + 900_001, 0, 0, 5000, Some(5000), None), 0);
    assert_eq!(
        o.state,
        Some(DetState::Cool {
            until_ms: t_in + 1_800_000
        })
    );
    assert!(
        o.event.is_none(),
        "an Inside fix after the long gap does not re-ENTER"
    );
}

#[test]
fn outside_run_does_not_survive_a_gap_above_120_seconds() {
    let mut d = Driver::new(&circle(150));
    for i in 0..5 {
        d.feed(fix_at(i * 1000, 0, 0, 5000, Some(5000), None), 0);
    }
    // Four Outside fixes (xn = 4), then 130 s of nothing, then one more Outside.
    for i in 0..4 {
        d.feed(
            fix_at(10_000 + i * 1000, 400_000, 0, 5000, Some(5000), None),
            0,
        );
    }
    assert!(matches!(d.st, Some(DetState::In { xn: 4, .. })));
    let o = d.feed(
        fix_at(13_000 + 130_000, 400_000, 0, 5000, Some(5000), None),
        0,
    );
    assert_eq!(
        o.state,
        Some(DetState::In {
            xn: 1,
            x_first_t: Some(143_000),
            last_t: 143_000
        }),
        "the run restarts; without the reset this would be a confirmed exit"
    );
    // A gap of exactly 120 s keeps the run.
    let mut d = Driver::new(&circle(150));
    for i in 0..5 {
        d.feed(fix_at(i * 1000, 0, 0, 5000, Some(5000), None), 0);
    }
    for i in 0..4 {
        d.feed(
            fix_at(10_000 + i * 1000, 400_000, 0, 5000, Some(5000), None),
            0,
        );
    }
    let o = d.feed(
        fix_at(13_000 + 120_000, 400_000, 0, 5000, Some(5000), None),
        0,
    );
    assert!(
        matches!(o.state, Some(DetState::Cool { .. })),
        "{:?}",
        o.state
    );
}

#[test]
fn exit_needs_five_outside_fixes_or_two_ten_seconds_apart() {
    let enter = |d: &mut Driver| {
        for i in 0..5 {
            d.feed(fix_at(i * 1000, 0, 0, 5000, Some(5000), None), 0);
        }
    };
    let out = |t| fix_at(t, 400_000, 0, 5000, Some(5000), None);
    let inside = |t| fix_at(t, 0, 0, 5000, Some(5000), None);
    // Four Outside fixes: not yet.
    let mut d = Driver::new(&circle(150));
    enter(&mut d);
    for t in [5000, 6000, 7000, 8000] {
        d.feed(out(t), 0);
    }
    assert_eq!(
        d.st,
        Some(DetState::In {
            xn: 4,
            x_first_t: Some(5000),
            last_t: 8000
        })
    );
    // A fifth confirms.
    let o = d.feed(out(9000), 0);
    assert_eq!(
        o.state,
        Some(DetState::Cool {
            until_ms: 9000 + 1_800_000
        })
    );
    assert!(o.event.is_none());
    // Any non-Outside fix zeroes the run.
    let mut d = Driver::new(&circle(150));
    enter(&mut d);
    for t in [5000, 6000, 7000, 8000] {
        d.feed(out(t), 0);
    }
    let o = d.feed(inside(9000), 0);
    assert_eq!(
        o.state,
        Some(DetState::In {
            xn: 0,
            x_first_t: None,
            last_t: 9000
        })
    );
    // A straddling fix (150 m from the centre = on the boundary) also zeroes it.
    let mut d = Driver::new(&circle(150));
    enter(&mut d);
    for t in [5000, 6000, 7000, 8000] {
        d.feed(out(t), 0);
    }
    let o = d.feed(fix_at(9000, 150_000, 0, 5000, Some(5000), None), 0);
    assert_eq!(
        o.state,
        Some(DetState::In {
            xn: 0,
            x_first_t: None,
            last_t: 9000
        })
    );
    // Two Outside fixes 10 s apart confirm; 9.999 s apart do not.
    let mut d = Driver::new(&circle(150));
    enter(&mut d);
    d.feed(out(5000), 0);
    let o = d.feed(out(15_000), 0);
    assert_eq!(
        o.state,
        Some(DetState::Cool {
            until_ms: 15_000 + 1_800_000
        })
    );
    let mut d = Driver::new(&circle(150));
    enter(&mut d);
    d.feed(out(5000), 0);
    let o = d.feed(out(14_999), 0);
    assert_eq!(
        o.state,
        Some(DetState::In {
            xn: 2,
            x_first_t: Some(5000),
            last_t: 14_999
        })
    );
    // Outside means depth <= -acc_eff: 159.5 m is not Outside, 160.5 m is.
    let mut d = Driver::new(&circle(150));
    enter(&mut d);
    for i in 0..5 {
        d.feed(
            fix_at(5000 + i * 1000, 159_500, 0, 5000, Some(5000), None),
            0,
        );
    }
    assert!(
        matches!(d.st, Some(DetState::In { xn: 0, .. })),
        "{:?}",
        d.st
    );
    for i in 0..5 {
        d.feed(
            fix_at(10_000 + i * 1000, 160_500, 0, 5000, Some(5000), None),
            0,
        );
    }
    assert!(matches!(d.st, Some(DetState::Cool { .. })));
}

#[test]
fn cool_down_blocks_re_entry_for_thirty_minutes_then_releases() {
    let mut d = Driver::new(&circle(150));
    for i in 0..5 {
        d.feed(fix_at(i * 1000, 0, 0, 5000, Some(5000), None), 0);
    }
    for i in 0..5 {
        d.feed(
            fix_at(5000 + i * 1000, 400_000, 0, 5000, Some(5000), None),
            0,
        );
    }
    let until = 9000 + 1_800_000;
    assert_eq!(d.st, Some(DetState::Cool { until_ms: until }));
    // Back inside during the cool-down: nothing.
    for i in 0..20 {
        let o = d.feed(fix_at(10_000 + i * 1000, 0, 0, 5000, Some(5000), None), 0);
        assert!(o.event.is_none());
        assert_eq!(o.state, Some(DetState::Cool { until_ms: until }));
    }
    // The fix at `until` releases to idle and is not counted.
    let o = d.feed(fix_at(until, 0, 0, 5000, Some(5000), None), 0);
    assert_eq!(o.state, None);
    let mut n_ev = 0;
    for i in 1..6 {
        n_ev += usize::from(
            d.feed(fix_at(until + i * 1000, 0, 0, 5000, Some(5000), None), 0)
                .event
                .is_some(),
        );
    }
    assert_eq!(n_ev, 1, "a fresh ENTER after the cool-down");
    // One millisecond early is still cool.
    let mut d = Driver::new(&circle(150));
    for i in 0..5 {
        d.feed(fix_at(i * 1000, 0, 0, 5000, Some(5000), None), 0);
    }
    for i in 0..5 {
        d.feed(
            fix_at(5000 + i * 1000, 400_000, 0, 5000, Some(5000), None),
            0,
        );
    }
    let o = d.feed(fix_at(until - 1, 0, 0, 5000, Some(5000), None), 0);
    assert_eq!(o.state, Some(DetState::Cool { until_ms: until }));
}

#[test]
fn slow_or_unknown_speed_needs_ten_seconds() {
    let run = |speed: Option<u32>| {
        let mut d = Driver::new(&circle(150));
        let mut first = None;
        for i in 0..14 {
            let o = d.feed(fix_at(i * 1000, 0, 0, 5000, speed, None), 0);
            if o.event.is_some() && first.is_none() {
                first = Some(i);
            }
        }
        first
    };
    assert_eq!(run(Some(1500)), Some(3), "1.5 m/s: 3 s");
    assert_eq!(run(Some(1499)), Some(10), "1.499 m/s: 10 s");
    assert_eq!(run(Some(0)), Some(10));
    // No Doppler and no movement: displacement says 0 mm/s from the fourth fix on, and the
    // first three fixes have no speed at all: both are slow.
    assert_eq!(run(None), Some(10));
}

#[test]
fn accuracy_gate_never_transitions() {
    // Circle: acc 50 m is allowed (Inside then needs depth >= 50 m), 50.001 m is ACC_TOO_COARSE.
    let sp = circle(150);
    let mut d = Driver::new(&sp);
    let o = d.feed(fix_at(0, 0, 0, 50_001, Some(5000), None), 0);
    assert_eq!(o.reason, Some(ReasonCode::AccTooCoarse));
    assert_eq!((o.state, o.event), (None, None));
    let o = d.feed(fix_at(1000, 0, 0, 50_000, Some(5000), None), 0);
    assert!(matches!(o.state, Some(DetState::Cand { n: 1, .. })));
    // A coarse fix in the middle of a candidate leaves it exactly as it was.
    let before = d.st;
    let o = d.feed(fix_at(2000, 0, 0, 50_001, Some(5000), None), 0);
    assert_eq!(o.state, before);
    assert_eq!(o.reason, Some(ReasonCode::AccTooCoarse));
    // ...and inside `in` it neither exits nor counts.
    let mut d = Driver::new(&sp);
    for i in 0..5 {
        d.feed(fix_at(i * 1000, 0, 0, 5000, Some(5000), None), 0);
    }
    let before = d.st;
    for i in 0..10 {
        let o = d.feed(
            fix_at(5000 + i * 1000, 400_000, 0, 60_000, Some(5000), None),
            0,
        );
        assert_eq!(o.state, before);
    }
}

#[test]
fn polygon_inradius_gate_and_edges() {
    // Square with half side 200 m and a declared inradius of 40 m: acc_eff 40 m is the limit.
    let sp = square(200, Some(40));
    let mut d = Driver::new(&sp);
    let o = d.feed(fix_at(0, 0, 0, 40_000, Some(5000), None), 0);
    assert!(matches!(o.state, Some(DetState::Cand { .. })), "{o:?}");
    let o = d.feed(fix_at(1000, 0, 0, 40_001, Some(5000), None), 0);
    assert_eq!(o.reason, Some(ReasonCode::AccTooCoarse));
    // Without a declared inradius there is no gate; Inside needs depth >= acc_eff.
    let sp = square(200, None);
    let mut d = Driver::new(&sp);
    for i in 0..5 {
        // 195 m east of the centre: 5 m inside the east edge: a straddle for acc_eff 10 m.
        d.feed(fix_at(i * 1000, 195_000, 0, 5000, Some(5000), None), 0);
    }
    assert!(d.events().is_empty());
    for i in 5..10 {
        // 180 m: 20 m inside.
        d.feed(fix_at(i * 1000, 180_000, 0, 5000, Some(5000), None), 0);
    }
    assert_eq!(d.events().len(), 1);
    // Leaving through a corner: Outside needs 10 m clear of every edge.
    for i in 10..14 {
        d.feed(
            fix_at(i * 1000, 205_000, 205_000, 5000, Some(5000), None),
            0,
        );
    }
    assert!(
        matches!(d.st, Some(DetState::In { xn: 0, .. })),
        "{:?}",
        d.st
    );
    for i in 14..19 {
        d.feed(
            fix_at(i * 1000, 215_000, 215_000, 5000, Some(5000), None),
            0,
        );
    }
    assert!(matches!(d.st, Some(DetState::Cool { .. })), "{:?}", d.st);
}

#[test]
fn reserved_and_internal_events_fail_closed() {
    for ev in [
        EventKind::Cross,
        EventKind::Dwell,
        EventKind::Pass,
        EventKind::Sequence,
        EventKind::Exit,
    ] {
        let mut sp = circle(150);
        sp.event = ev;
        assert_eq!(
            Prepared::new(&sp).unwrap_err(),
            ReasonCode::UnsupportedEvent,
            "{ev:?}"
        );
        let (mem, summary) =
            path_update(&Mem::default(), &fix_at(0, 0, 0, 5000, Some(5000), None)).unwrap();
        let _ = mem;
        for explain in [false, true] {
            // Even with an existing state the detector is dropped.
            for st in [
                None,
                Some(DetState::In {
                    xn: 0,
                    x_first_t: None,
                    last_t: 0,
                }),
            ] {
                let o = detect(
                    st,
                    &sp,
                    &summary,
                    &fix_at(0, 0, 0, 5000, Some(5000), None),
                    0,
                    explain,
                )
                .unwrap();
                assert_eq!(
                    o,
                    Detected {
                        state: None,
                        event: None,
                        reason: Some(ReasonCode::UnsupportedEvent)
                    },
                    "terminal and independent of explain"
                );
            }
        }
    }
}

#[test]
fn shape_event_mismatch_and_bad_triggers_are_trigger_invalid() {
    use common::{ORIGIN, route, trigger};
    use proxim_types::Shape;
    // enter with a route shape, approach with a circle.
    let mut sp = route(30);
    sp.event = EventKind::Enter;
    assert_eq!(Prepared::new(&sp).unwrap_err(), ReasonCode::TriggerInvalid);
    let mut sp = circle(150);
    sp.event = EventKind::Approach;
    assert_eq!(Prepared::new(&sp).unwrap_err(), ReasonCode::TriggerInvalid);
    // Radius below 150 m.
    assert_eq!(
        Prepared::new(&circle(149)).unwrap_err(),
        ReasonCode::TriggerInvalid
    );
    assert!(Prepared::new(&circle(150)).is_ok());
    // Polygon with 2 vertices, with 257.
    let two = signpost(
        EventKind::Enter,
        trigger(Shape::Polygon {
            vertices: vec![ORIGIN, ORIGIN],
            inradius_m: None,
        }),
    );
    assert_eq!(Prepared::new(&two).unwrap_err(), ReasonCode::TriggerInvalid);
    let many = signpost(
        EventKind::Enter,
        trigger(Shape::Polygon {
            vertices: vec![ORIGIN; 257],
            inradius_m: None,
        }),
    );
    assert_eq!(
        Prepared::new(&many).unwrap_err(),
        ReasonCode::TriggerInvalid
    );
    // Latitude beyond 80 degrees and a centre beyond the 150 km frame.
    let mut sp = circle(150);
    sp.trigger.origin.lat_e7 = 850_000_000;
    assert_eq!(Prepared::new(&sp).unwrap_err(), ReasonCode::TriggerInvalid);
    let mut sp = circle(150);
    if let Shape::Circle { center, .. } = &mut sp.trigger.shape {
        center.lat_e7 += 20_000_000; // two degrees north: 222 km
    }
    assert_eq!(Prepared::new(&sp).unwrap_err(), ReasonCode::TriggerInvalid);
    // Invalid trigger through `detect` is a terminal drop and independent of explain.
    let (_, summary) =
        path_update(&Mem::default(), &fix_at(0, 0, 0, 5000, Some(5000), None)).unwrap();
    let o = detect(
        None,
        &circle(10),
        &summary,
        &fix_at(0, 0, 0, 5000, Some(5000), None),
        0,
        false,
    )
    .unwrap();
    assert_eq!(o.reason, Some(ReasonCode::TriggerInvalid));
    assert_eq!((o.state, o.event), (None, None));
}

#[test]
fn path_floors_make_enter_unreachable_for_higher_tiers() {
    // ENTER is tier P0 (confidence 500): a signpost demanding more never fires and never moves.
    for (tier, conf, why) in [
        (Some(1), None, ReasonCode::PathTierLow),
        (None, Some(501), ReasonCode::PathConfidenceLow),
        (None, Some(700), ReasonCode::PathConfidenceLow),
    ] {
        let mut sp = circle(150);
        sp.min_path_tier = tier;
        sp.min_path_conf_milli = conf;
        let mut d = Driver::new(&sp);
        for i in 0..8 {
            let o = d.feed(fix_at(i * 1000, 0, 0, 5000, Some(5000), None), 0);
            assert_eq!((o.state, o.event, o.reason), (None, None, Some(why)));
        }
    }
    let mut sp = circle(150);
    sp.min_path_conf_milli = Some(500);
    let mut d = Driver::new(&sp);
    for i in 0..8 {
        d.feed(fix_at(i * 1000, 0, 0, 5000, Some(5000), None), 0);
    }
    assert_eq!(d.events().len(), 1);
}

#[test]
fn out_of_order_fix_leaves_state_untouched() {
    let mut d = Driver::new(&circle(150));
    d.feed(fix_at(5000, 0, 0, 5000, Some(5000), None), 0);
    d.feed(fix_at(6000, 0, 0, 5000, Some(5000), None), 0);
    let before = d.st;
    let (_, summary) =
        path_update(&Mem::default(), &fix_at(0, 0, 0, 5000, Some(5000), None)).unwrap();
    for t in [6000, 5999, 0, i64::MIN] {
        let f = fix_at(t, 0, 0, 5000, Some(5000), None);
        let o = d.prep.step(before, &summary, &f, t, true).unwrap();
        assert_eq!(o.state, before, "t = {t}");
        assert_eq!(o.event, None);
        assert_eq!(o.reason, Some(ReasonCode::FixOutOfOrder));
    }
}

#[test]
fn state_and_event_do_not_depend_on_explain() {
    let fixes = pass(-300_000, 15_000, 0, 45, 5000);
    let run = |explain: bool| {
        let mut d = Driver::new(&circle(150));
        d.explain = explain;
        d.feed_all(&fixes)
            .into_iter()
            .map(|o| (o.state, o.event, o.reason.is_some()))
            .collect::<Vec<_>>()
    };
    let (a, b) = (run(true), run(false));
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!((x.0, x.1), (y.0, y.1));
        assert!(
            !y.2,
            "without explain a non-terminal reason is never reported"
        );
    }
}

#[test]
fn foreign_state_variants_restart_from_idle() {
    // An Armed/Latched state handed to an ENTER signpost (corrupt or tampered) is dropped.
    let sp = circle(150);
    let (_, summary) =
        path_update(&Mem::default(), &fix_at(0, 0, 0, 5000, Some(5000), None)).unwrap();
    let f = fix_at(0, 0, 0, 5000, Some(5000), None);
    for st in [
        DetState::Armed {
            confirm_t: Some(1),
            last_t: 1,
        },
        DetState::Latched {
            why: proxim_types::LatchWhy::Fired,
            last_t: 1,
        },
    ] {
        let o = detect(Some(st), &sp, &summary, &f, 0, true).unwrap();
        assert_eq!(
            o.state,
            Some(DetState::Cand {
                n: 1,
                first_t: 0,
                last_t: 0
            })
        );
    }
}

#[test]
fn unknown_speed_at_the_deciding_fix_needs_ten_seconds() {
    // A summary with no visible speed (vf None) is treated as slow: 3 s is not enough.
    let prep = Prepared::new(&circle(150)).unwrap();
    let f = fix_at(3000, 0, 0, 5000, None, None);
    let (_, mut summary) = path_update(&Mem::default(), &f).unwrap();
    assert_eq!(summary.vf_mmps, None);
    let st = DetState::Cand {
        n: 2,
        first_t: 0,
        last_t: 2000,
    };
    let o = prep.step(Some(st), &summary, &f, 3000, true).unwrap();
    assert!(o.event.is_none());
    assert_eq!(
        o.state,
        Some(DetState::Cand {
            n: 3,
            first_t: 0,
            last_t: 3000
        })
    );
    // With a visible 1.5 m/s the same step fires.
    summary.vf_mmps = Some(1500);
    let o = prep.step(Some(st), &summary, &f, 3000, true).unwrap();
    assert!(o.event.is_some());
    // And with None at 10 s it fires too.
    summary.vf_mmps = None;
    let f10 = fix_at(10_000, 0, 0, 5000, None, None);
    let st = DetState::Cand {
        n: 3,
        first_t: 0,
        last_t: 9000,
    };
    assert!(
        prep.step(Some(st), &summary, &f10, 10_000, true)
            .unwrap()
            .event
            .is_some()
    );
}

#[test]
fn out_of_order_fix_in_the_in_state_does_not_count() {
    let prep = Prepared::new(&circle(150)).unwrap();
    let f = fix_at(9000, 400_000, 0, 5000, Some(5000), None); // Outside, but before last_t
    let (_, summary) = path_update(&Mem::default(), &f).unwrap();
    let st = DetState::In {
        xn: 4,
        x_first_t: Some(1000),
        last_t: 10_000,
    };
    for t in [10_000, 9000, i64::MIN] {
        let f = fix_at(t, 400_000, 0, 5000, Some(5000), None);
        let o = prep.step(Some(st), &summary, &f, t, true).unwrap();
        assert_eq!(
            o.state,
            Some(st),
            "t = {t}: would otherwise confirm the exit"
        );
        assert_eq!(o.reason, Some(ReasonCode::FixOutOfOrder));
    }
    // One millisecond later it does confirm.
    let f = fix_at(10_001, 400_000, 0, 5000, Some(5000), None);
    let o = prep.step(Some(st), &summary, &f, f.t_ms, true).unwrap();
    assert!(matches!(o.state, Some(DetState::Cool { .. })));
}
