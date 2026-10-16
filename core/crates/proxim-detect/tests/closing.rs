//! CLOSING as a time-span rule (spec 5.3, errata E-P12): walkers reach P1, a stationary device
//! that jitters never closes or fires, U-turns and GPS gaps still fail closed.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

mod common;

use common::{
    Driver, ORIGIN, ROUTE_M, along_of, armed_fields, drive_north, fix_at, pos, route, signpost,
    trigger, with_mode,
};
use proxim_types::{DetState, EventKind, Fix, Mode, ReasonCode, Shape};

const ANCHOR_MM: i64 = ROUTE_M * 1000;

/// A walker on the route axis, 1 Hz, Doppler speed and course 0, from `n0_mm`.
fn walk(n0_mm: i64, v_mmps: i64, count: usize, acc_mm: u32) -> Vec<Fix> {
    (0..count as i64)
        .map(|i| {
            fix_at(
                i * 1000,
                0,
                (n0_mm + v_mmps * i) as i32,
                acc_mm,
                Some(v_mmps as u32),
                Some(0),
            )
        })
        .collect()
}

fn first_armed(d: &Driver) -> Option<usize> {
    d.steps
        .iter()
        .position(|s| matches!(s.out.state, Some(DetState::Armed { .. })))
}

#[test]
fn a_walker_at_1_4_mps_and_1_hz_reaches_p1_within_8_seconds() {
    let mut d = Driver::new(&with_mode(route(30), Mode::Foot));
    // Start 1500 m before the anchor: inside d_hi, far outside d_lo.
    d.feed_all(&walk(ANCHOR_MM - 1_500_000, 1400, 12, 5000));
    let i = first_armed(&d).expect("a walker must reach P1");
    // Baseline 4 s old: 5.6 m >= max(5 m, acc_eff / 2 = 5 m). Armed means every P1 gate passed.
    assert_eq!(
        i,
        4,
        "reasons: {:?}",
        d.steps.iter().map(|s| s.out.reason).collect::<Vec<_>>()
    );
    assert!(d.steps[i].fix.t_ms <= 8000);
    // The four fixes before it (ages 0..3 s) fail CLOSING for lack of a 4 s old baseline.
    for s in &d.steps[..4] {
        assert_eq!(s.out.reason, Some(ReasonCode::NotClosing));
        assert_eq!(s.out.state, None);
    }
    // And it stays armed from then on.
    assert!(
        d.steps[i..]
            .iter()
            .all(|s| matches!(s.out.state, Some(DetState::Armed { .. })))
    );
}

#[test]
fn walker_p1_time_follows_the_span_model() {
    // First P1 index = smallest i >= 4 with v x i >= acc_eff / 2 (trace holds 15 s of history).
    // (acc_mm, speed mm/s, expected first index)
    for (acc, v, expect) in [
        (5000_u32, 1400_i64, 4_usize), // 5.6 m >= 5 m
        (11_000, 1400, 4),             // 5.6 m >= 5.5 m
        (11_300, 1400, 5),             // 5.6 m <  5.65 m -> 7.0 m at 5 s
        (12_000, 1400, 5),
        (20_000, 1400, 8), // 9.8 m < 10 m at 7 s, 11.2 m at 8 s
        (5000, 1000, 5),   // 4.0 m, 5.0 m at 5 s
        (5000, 800, 7),    // 4.8 m at 6 s (< 5 m), 5.6 m at 7 s
        (5000, 2500, 4),
        (30_000, 1400, 11), // 15 m needs 10.7 s
    ] {
        let mut d = Driver::new(&with_mode(route(30), Mode::Foot));
        d.feed_all(&walk(ANCHOR_MM - 1_500_000, v, 16, acc));
        assert_eq!(first_armed(&d), Some(expect), "acc {acc} v {v}");
    }
    // 50 m accuracy: tol 25 m would need 17.9 s, but only 15 s of fixes are retained: never P1.
    let mut d = Driver::new(&with_mode(route(30), Mode::Foot));
    d.feed_all(&walk(ANCHOR_MM - 1_500_000, 1400, 60, 50_000));
    assert_eq!(first_armed(&d), None);
    assert!(
        d.steps
            .iter()
            .skip(5)
            .all(|s| s.out.reason == Some(ReasonCode::NotClosing))
    );
}

#[test]
fn a_walker_fires_at_1_hz_and_a_stopped_walker_does_not() {
    let sp = with_mode(route(30), Mode::Foot);
    let mut d = Driver::new(&sp);
    d.feed_all(&walk(ANCHOR_MM - 400_000, 1400, 400, 5000));
    let ev = d.events();
    assert_eq!(ev.len(), 1);
    let e = ev[0].1;
    assert_eq!((e.path_tier, e.path_conf_milli), (1, 600));
    assert!((90_000..=100_000).contains(&e.d_rem_mm.unwrap()), "{e:?}");
    // The same walker standing still 300 m out (Doppler says 0): never armed.
    let mut d = Driver::new(&sp);
    let fixes: Vec<Fix> = (0..200)
        .map(|i| {
            fix_at(
                i * 1000,
                0,
                (ANCHOR_MM - 300_000) as i32,
                5000,
                Some(0),
                None,
            )
        })
        .collect();
    d.feed_all(&fixes);
    assert!(d.events().is_empty());
    assert_eq!(first_armed(&d), None);
}

/// Deterministic scatter: the standing point plus a pseudo-random offset in `[-j, j]` mm on the
/// route axis and `[-j_e, j_e]` across it (xorshift, no `rand`).
fn scatter(seed: u64, j: i64, j_e: i64) -> impl FnMut() -> (i32, i32) {
    let mut s = seed | 1;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    move || {
        let a = (next() % (2 * j as u64 + 1)) as i64 - j;
        let b = (next() % (2 * j_e as u64 + 1)) as i64 - j_e;
        (b as i32, a as i32)
    }
}

#[test]
fn a_stationary_device_with_jitter_below_acc_over_4_never_closes_or_fires() {
    // The OS claims 10 m/s and course 0 (a stuck device with a stale Doppler), so every gate
    // except CLOSING passes. Standing 300 m before the anchor, inside the fire zone
    // (D_fire = 430 m): without CLOSING it would arm and fire within three fixes.
    for acc in [5000_u32, 10_000, 20_000, 40_000, 65_000] {
        let acc_eff = i64::from(acc.max(10_000));
        let j = (acc_eff / 2 - 1000) / 2; // strictly below acc_eff / 4
        let mut d = Driver::new(&route(30));
        let mut sc = scatter(0x9E37_79B9_7F4A_7C15 ^ u64::from(acc), j, 25_000);
        for i in 0..300_i64 {
            let (e, n) = sc();
            d.feed(
                fix_at(
                    i * 1000,
                    e,
                    (ANCHOR_MM - 300_000) as i32 + n,
                    acc,
                    Some(10_000),
                    Some(0),
                ),
                0,
            );
        }
        assert!(d.events().is_empty(), "acc {acc}");
        assert_eq!(first_armed(&d), None, "acc {acc}");
        let nc = d
            .steps
            .iter()
            .filter(|s| s.out.reason == Some(ReasonCode::NotClosing))
            .count();
        assert!(
            nc >= 250,
            "acc {acc}: CLOSING must be the gate that stops it ({nc})"
        );
    }
}

#[test]
fn vehicle_u_turn_idles_on_the_first_reversed_fix_even_with_a_stale_course() {
    // Armed at 15 m/s, 1500 m out (D_fire = 645 m). The next fix is 15 m back along the route:
    // the baseline is still behind it (net advance positive) but the last step retreats.
    for course in [0_u16, 1800] {
        let mut d = Driver::new(&route(30));
        let mut fixes = drive_north(0, ANCHOR_MM - 1_500_000 - 15_000 * 59, 15_000, 60, 0);
        let last_n = ANCHOR_MM - 1_500_000;
        fixes.push(fix_at(
            60_000,
            0,
            (last_n - 15_000) as i32,
            5000,
            Some(15_000),
            Some(course),
        ));
        d.feed_all(&fixes);
        let n = d.steps.len();
        assert!(matches!(
            d.steps[n - 2].out.state,
            Some(DetState::Armed { .. })
        ));
        assert_eq!(d.steps[n - 1].out.state, None, "course {course}");
        assert_eq!(d.steps[n - 1].out.reason, Some(ReasonCode::NotClosing));
        assert!(d.events().is_empty());
    }
}

#[test]
fn walker_u_turn_idles_within_8_seconds_and_never_fires_after_the_turn() {
    // A walker reverses at a point `turn_rem` before the anchor. A position-based rule cannot see
    // a 1.4 m/s reversal faster than the 15 s baseline catches up (about 7 s); until then it is
    // armed, but the walker is moving away so the fire condition (d_rem <= d_lo) is not met.
    for turn_rem in [
        400_000_i64,
        250_000,
        150_000,
        130_000,
        110_000,
        104_000,
        101_000,
        99_000,
    ] {
        for flip_course in [false, true] {
            let mut d = Driver::new(&with_mode(route(30), Mode::Foot));
            let start = ANCHOR_MM - turn_rem - 1400 * 30;
            d.feed_all(&walk(start, 1400, 30, 5000)); // the last fix is `turn_rem` out
            let fired_before = d.events().len();
            let armed_at_turn = armed_fields(d.st).is_some();
            let mut n = ANCHOR_MM - turn_rem;
            let mut idle_at = None;
            for k in 1..=30_i64 {
                n -= 1400;
                let c = if flip_course { 1800 } else { 0 };
                let o = d.feed(
                    fix_at((29 + k) * 1000, 0, n as i32, 5000, Some(1400), Some(c)),
                    0,
                );
                if o.state.is_none() && idle_at.is_none() {
                    idle_at = Some(k);
                }
            }
            assert_eq!(
                d.events().len(),
                fired_before,
                "turn at {turn_rem} flip {flip_course}: fired after the turn"
            );
            if armed_at_turn {
                let k = idle_at.expect("must go idle");
                assert!(
                    k <= 8,
                    "turn at {turn_rem} flip {flip_course}: idle after {k} s"
                );
            }
        }
    }
}

#[test]
fn a_gap_over_60_s_makes_old_fixes_no_baseline_and_a_shorter_gap_keeps_them() {
    // Vehicle 20 m/s, 1 Hz: 40 fixes, a gap, then 8 more fixes on the same line, all inside d_hi.
    let n0 = 2_000_000;
    let build = |gap_s: i64| {
        let mut fixes = drive_north(0, n0, 20_000, 40, 0);
        let n_last = n0 + 20_000 * 39;
        let t_last = 39_000;
        for k in 1..=8_i64 {
            let t = t_last + gap_s * 1000 + (k - 1) * 1000;
            let n = n_last + 20_000 * (gap_s + k - 1);
            fixes.push(fix_at(t, 0, n as i32, 5000, Some(20_000), Some(0)));
        }
        fixes
    };
    let reasons_after =
        |d: &Driver| -> Vec<_> { d.steps[40..].iter().map(|s| s.out.reason).collect() };
    // 59 s gap: the pre-gap fix 60 s back is still an eligible baseline (60 s is inclusive) and
    // three fixes lie within 60 s, so the detector arms on the first post-gap fix.
    let mut d = Driver::new(&route(30));
    d.feed_all(&build(59));
    assert_eq!(first_armed(&d), Some(40), "{:?}", reasons_after(&d));
    // 61 s gap: every pre-gap fix is older than 60 s, so CLOSING waits for a post-gap fix that is
    // 4 s old; before that it is NOT_CLOSING.
    let mut d = Driver::new(&route(30));
    d.feed_all(&build(61));
    let r = reasons_after(&d);
    assert!(
        r[..4].iter().all(|x| *x == Some(ReasonCode::NotClosing)),
        "{r:?}"
    );
    assert_eq!(first_armed(&d), Some(44), "{r:?}");
}

#[test]
fn every_armed_step_stores_the_along_of_its_own_fix() {
    // a_prev_mm is the centre of the next fix's window (errata E-P13): it must be written on the
    // arming fix as well as on every later armed step.
    let mut d = Driver::new(&route(30));
    d.feed_all(&drive_north(0, 0, 20_000, 160, 0));
    let armed: Vec<_> = d
        .steps
        .iter()
        .filter(|s| matches!(s.out.state, Some(DetState::Armed { .. })))
        .collect();
    assert!(armed.len() >= 2, "{}", armed.len());
    for s in armed {
        let (_, a, last_t) = armed_fields(s.out.state).unwrap();
        assert_eq!(last_t, s.fix.t_ms);
        let a = a.expect("an armed state written by the detector carries a_prev_mm");
        assert!(
            (a - along_of(&s.fix)).abs() <= 25,
            "{a} vs {}",
            along_of(&s.fix)
        );
    }
}

#[test]
fn an_armed_track_that_drifts_toward_a_parallel_leg_keeps_its_own_leg() {
    // Hairpin: 6 km north, 40 m east, 6 km south (legs 40 m apart), buffer 50 m, anchor at 3000 m
    // on the outbound leg (the return leg lies beyond the along window all the way to the fire point).
    // The vehicle drives the outbound leg at e = 10 m, then drifts to e = 22 m at 1500 m, which
    // is nearer to the return leg (18 m) than to the outbound leg (22 m).
    let v = vec![
        ORIGIN,
        pos(0, 6_000_000),
        pos(40_000, 6_000_000),
        pos(40_000, 0),
    ];
    let sp = signpost(
        EventKind::Approach,
        trigger(Shape::Route {
            vertices: v,
            buffer_m: 50,
            anchor_along_m: Some(3000),
        }),
    );
    let drive = |from_n: i64, e_before: i32, drift_at: i64| -> Driver {
        let mut d = Driver::new(&sp);
        let fixes: Vec<Fix> = (0..)
            .map(|i| from_n + 20_000 * i)
            .take_while(|n| *n < 2_600_000)
            .enumerate()
            .map(|(i, n)| {
                let e = if n < drift_at { e_before } else { 22_000 };
                fix_at(i as i64 * 1000, e, n as i32, 5000, Some(20_000), Some(0))
            })
            .collect();
        d.feed_all(&fixes);
        d
    };
    // Armed on the clean part, the drift does not capture it: one fire at D_fire, on its own leg.
    let d = drive(0, 10_000, 1_500_000);
    let ev = d.events();
    assert_eq!(
        ev.len(),
        1,
        "{:?}",
        d.steps
            .iter()
            .rev()
            .take(3)
            .map(|s| s.out.reason)
            .collect::<Vec<_>>()
    );
    let rem = ev[0].1.d_rem_mm.unwrap();
    assert!((820_000..=880_000).contains(&rem), "{rem}");
    assert!(
        ev[0].1.dist_to_shape_mm >= 21_000,
        "measured against the outbound leg"
    );
    // Control: the same drifted track started cold (idle projects it onto the return leg).
    let d = drive(1_500_000, 22_000, 0);
    assert!(d.events().is_empty());
    assert!(d.steps.iter().all(|s| s.out.state.is_none()));
    assert!(
        d.steps
            .iter()
            .skip(4)
            .all(|s| s.out.reason == Some(ReasonCode::BearingMismatch))
    );
}
