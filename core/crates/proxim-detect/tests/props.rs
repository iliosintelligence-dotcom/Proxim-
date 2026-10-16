//! Property tests for the detectors: purity, determinism, accuracy gating, hysteresis, hostile
//! input (spec 6, 11; ADR-001 verification list).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

mod common;

use common::{Driver, ORIGIN, circle, drive_north, fix_at, route, square, with_mode};
use proptest::prelude::*;
use proxim_detect::{Detected, Prepared, detect};
use proxim_geo::isqrt;
use proxim_path::{PathSummary, fire_distance_mm, path_update};
use proxim_types::{
    DetState, EventKind, Fix, LatchWhy, Mem, Mode, PathInfo, ReasonCode, Signpost, TraceFix,
};

fn arb_state() -> impl Strategy<Value = Option<DetState>> {
    let why = prop::sample::select(vec![LatchWhy::Fired, LatchWhy::Late]);
    prop_oneof![
        1 => Just(None),
        1 => (any::<u32>(), any::<i64>(), any::<i64>())
            .prop_map(|(n, first_t, last_t)| Some(DetState::Cand { n, first_t, last_t })),
        1 => (any::<u32>(), prop::option::of(any::<i64>()), any::<i64>())
            .prop_map(|(xn, x_first_t, last_t)| Some(DetState::In { xn, x_first_t, last_t })),
        1 => any::<i64>().prop_map(|until_ms| Some(DetState::Cool { until_ms })),
        1 => (prop::option::of(any::<i64>()), prop::option::of(any::<i64>()), any::<i64>())
            .prop_map(|(confirm_t, a_prev_mm, last_t)| Some(DetState::Armed { confirm_t, a_prev_mm, last_t })),
        1 => (why, any::<i64>()).prop_map(|(why, last_t)| Some(DetState::Latched { why, last_t })),
    ]
}

fn arb_fix() -> impl Strategy<Value = Fix> {
    (
        any::<i64>(),
        prop_oneof![any::<i32>(), 440_000_000..460_000_000_i32],
        prop_oneof![any::<i32>(), 60_000_000..80_000_000_i32],
        prop_oneof![any::<u32>(), 0..200_000_u32],
        prop::option::of(prop_oneof![any::<u32>(), 0..40_000_u32]),
        prop::option::of(any::<u32>()),
        prop::option::of(any::<u16>()),
        prop::option::of(any::<u16>()),
    )
        .prop_map(|(t, lat, lon, acc, sp, spa, c, ca)| Fix {
            t_ms: t,
            lat_e7: lat,
            lon_e7: lon,
            acc_mm: acc,
            speed_mmps: sp,
            speed_acc_mmps: spa,
            course_ddeg: c,
            course_acc_ddeg: ca,
            is_mock: false,
        })
}

fn arb_trace_fix() -> impl Strategy<Value = TraceFix> {
    (
        any::<i64>(),
        prop_oneof![any::<i32>(), 449_000_000..451_000_000_i32],
        prop_oneof![any::<i32>(), 69_000_000..71_000_000_i32],
        any::<u32>(),
        prop::option::of(any::<u32>()),
    )
        .prop_map(|(t_ms, lat_e7, lon_e7, acc_mm, v_mmps)| TraceFix {
            t_ms,
            lat_e7,
            lon_e7,
            acc_mm,
            v_mmps,
        })
}

fn arb_summary() -> impl Strategy<Value = PathSummary> {
    (
        prop::option::of(any::<u32>()),
        prop::option::of(prop::array::uniform2(any::<i32>())),
        any::<u8>(),
        prop::array::uniform3(prop::option::of(arb_trace_fix())),
        prop::option::of(arb_trace_fix()),
        any::<i64>(),
        any::<u32>(),
    )
        .prop_map(
            |(vf, h, n60, recent, closing_base, t_ms, v_eff)| PathSummary {
                info: PathInfo {
                    tier: 0,
                    conf_milli: 0,
                    speed_mmps: vf,
                    heading_valid: h.is_some(),
                },
                v_eff_mmps: v_eff,
                heading_q30: h,
                vf_mmps: vf,
                fixes_60s: n60,
                recent,
                closing_base,
                t_ms,
            },
        )
}

fn signposts() -> Vec<Signpost> {
    let mut with_reach = route(30);
    with_reach.trigger.reach_m = Some(1);
    vec![
        circle(150),
        square(200, Some(40)),
        square(300, None),
        route(30),
        with_mode(route(500), Mode::Foot),
        with_mode(route(100), Mode::Cycle),
        with_reach,
    ]
}

fn arb_signpost() -> impl Strategy<Value = Signpost> {
    prop::sample::select(signposts())
}

fn ok_terminal(r: ReasonCode) -> bool {
    matches!(
        r,
        ReasonCode::TooLateForLead | ReasonCode::UnsupportedEvent | ReasonCode::TriggerInvalid
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// Hostile everything (state, summary, fix, clock, signpost, explain): never panics, always
    /// `Ok`, deterministic, and the result obeys the structural invariants.
    #[test]
    fn detect_is_total_pure_and_well_formed(
        st in arb_state(),
        sp in arb_signpost(),
        summary in arb_summary(),
        fix in arb_fix(),
        now in prop_oneof![any::<i64>(), -1000_i64..1_000_000],
        explain in any::<bool>(),
    ) {
        let a = detect(st, &sp, &summary, &fix, now, explain);
        let b = detect(st, &sp, &summary, &fix, now, explain);
        prop_assert_eq!(&a, &b, "pure");
        let o = a.expect("detect fails closed with a value, not an error");
        // `explain` never changes state or event.
        let c = detect(st, &sp, &summary, &fix, now, !explain).unwrap();
        prop_assert_eq!((o.state, o.event), (c.state, c.event));
        // Terminal reasons are always reported; others only with explain.
        if !explain {
            let fine = o.reason.is_none_or(ok_terminal);
            prop_assert!(fine, "unexpected reason without explain: {:?}", o.reason);
        }
        if let Some(e) = o.event {
            // An event means a fire transition: ENTER -> in, APPROACH -> latched{fired}.
            match e.kind {
                EventKind::Enter => {
                    let ok = matches!(o.state, Some(DetState::In { xn: 0, .. }));
                    prop_assert!(ok);
                }
                EventKind::Approach => {
                    let ok = matches!(o.state, Some(DetState::Latched { why: LatchWhy::Fired, .. }));
                    prop_assert!(ok);
                }
                other => prop_assert!(false, "unexpected kind {:?}", other),
            }
            prop_assert_eq!(e.t_ms, fix.t_ms);
            prop_assert!(e.path_tier <= 1 && e.path_conf_milli >= 400 && e.path_conf_milli <= 800);
            prop_assert!(e.dist_to_shape_mm >= 0);
            if e.kind == EventKind::Approach {
                prop_assert!(e.d_rem_mm.is_some() && e.t_hat_ms.is_some());
            }
        }
        if o.reason == Some(ReasonCode::TooLateForLead) {
            let ok = matches!(o.state, Some(DetState::Latched { why: LatchWhy::Late, .. }));
            prop_assert!(ok);
            prop_assert!(o.event.is_none());
        }
    }

    /// Fed the same trace twice, a detector gives identical output; `explain` changes nothing but
    /// the reasons; running the engine in two halves with the state threaded gives the same result.
    #[test]
    fn same_trace_same_result(
        sp in arb_signpost(),
        steps in prop::collection::vec((1_i64..40_000, -300_000_i32..300_000, -300_000_i32..6_300_000, 0_u32..70_000, prop::option::of(0_u32..40_000), prop::option::of(0_u16..3600)), 1..60),
    ) {
        let mut t = 0_i64;
        let fixes: Vec<Fix> = steps.iter().map(|(dt, e, n, acc, v, c)| {
            t += dt;
            fix_at(t, *e, *n, *acc, *v, *c)
        }).collect();
        let run = |explain: bool| {
            let mut d = Driver::new(&sp);
            d.explain = explain;
            let outs = d.feed_all(&fixes);
            (outs, d.mem, d.st)
        };
        let (o1, m1, s1) = run(true);
        let (o2, m2, s2) = run(true);
        prop_assert_eq!(&o1, &o2);
        prop_assert_eq!((&m1, s1), (&m2, s2));
        let (o3, _, s3) = run(false);
        prop_assert_eq!(s1, s3);
        for (a, b) in o1.iter().zip(o3.iter()) {
            prop_assert_eq!((a.state, a.event), (b.state, b.event));
        }
        // Split the trace: state threaded explicitly gives the same final state.
        let k = fixes.len() / 2;
        let mut d = Driver::new(&sp);
        d.feed_all(&fixes[..k]);
        let (mem, st) = (d.mem.clone(), d.st);
        let mut d2 = Driver::new(&sp);
        d2.mem = mem;
        d2.st = st;
        d2.feed_all(&fixes[k..]);
        prop_assert_eq!(d2.st, s1);
    }

    /// A fix whose accuracy is worse than the circle gate (50 m) never causes a transition, in any
    /// reachable state.
    #[test]
    fn coarse_fix_never_causes_a_transition_circle(
        steps in prop::collection::vec((1_i64..5_000, -250_000_i32..250_000, -250_000_i32..250_000, 0_u32..50_000), 0..40),
        coarse in 50_001_u32..2_000_000,
        pos in (-400_000_i32..400_000, -400_000_i32..400_000),
        gap in 1_i64..2_000_000,
    ) {
        let mut d = Driver::new(&circle(150));
        let mut t = 0;
        for (dt, e, n, acc) in &steps {
            t += dt;
            d.feed(fix_at(t, *e, *n, *acc, Some(5000), None), 0);
        }
        let before = d.st;
        let f = fix_at(t + gap, pos.0, pos.1, coarse, Some(5000), None);
        let (_, summary) = path_update(&d.mem, &f).unwrap();
        let o = d.prep.step(before, &summary, &f, f.t_ms, true).unwrap();
        prop_assert_eq!(o.state, before);
        prop_assert!(o.event.is_none());
        prop_assert_eq!(o.reason, Some(ReasonCode::AccTooCoarse));
    }

    /// Same for a polygon with a declared inradius: acc_eff above the inradius never transitions.
    #[test]
    fn coarse_fix_never_causes_a_transition_polygon(
        steps in prop::collection::vec((1_i64..5_000, -250_000_i32..250_000, -250_000_i32..250_000), 0..30),
        coarse in 40_001_u32..500_000,
        pos in (-400_000_i32..400_000, -400_000_i32..400_000),
    ) {
        let mut d = Driver::new(&square(200, Some(40)));
        let mut t = 0;
        for (dt, e, n) in &steps {
            t += dt;
            d.feed(fix_at(t, *e, *n, 5000, Some(5000), None), 0);
        }
        let before = d.st;
        let f = fix_at(t + 1000, pos.0, pos.1, coarse, Some(5000), None);
        let (_, summary) = path_update(&d.mem, &f).unwrap();
        let o = d.prep.step(before, &summary, &f, f.t_ms, false).unwrap();
        prop_assert_eq!(o.state, before);
        prop_assert!(o.event.is_none());
    }

    /// Jitter inside the accuracy band cannot ENTER from idle: every fix straddles.
    #[test]
    fn boundary_jitter_never_enters(
        jit in prop::collection::vec((1_i64..4_000, -9_950_i64..=9_950, 0_usize..8), 1..80),
        acc in 0_u32..=10_000,
    ) {
        // acc <= 10 m -> acc_eff 10 m; distances within +-9.95 m of the 150 m edge straddle.
        let mut d = Driver::new(&circle(150));
        let mut t = 0;
        for (dt, off, dir) in &jit {
            t += dt;
            let (e, n) = polar(150_000 + off, *dir);
            let o = d.feed(fix_at(t, e, n, acc, Some(8000), None), 0);
            prop_assert!(o.event.is_none());
            prop_assert_eq!(o.state, None);
        }
    }

    /// Jitter inside the accuracy band cannot EXIT either: from `in` the detector stays `in` with
    /// no outside run, however long it lasts. That is the anti-flapping guarantee.
    #[test]
    fn boundary_jitter_never_exits_and_never_flaps(
        jit in prop::collection::vec((1_i64..4_000, -9_950_i64..=9_950, 0_usize..8), 1..120),
        acc in 0_u32..=10_000,
    ) {
        let mut d = Driver::new(&circle(150));
        for i in 0..5 {
            d.feed(fix_at(i * 1000, 0, 0, 5000, Some(8000), None), 0);
        }
        prop_assert_eq!(d.events().len(), 1);
        let mut t = 5000;
        for (dt, off, dir) in &jit {
            t += dt;
            let (e, n) = polar(150_000 + off, *dir);
            let o = d.feed(fix_at(t, e, n, acc, Some(8000), None), 0);
            prop_assert!(o.event.is_none(), "no second ENTER");
            let ok = matches!(o.state, Some(DetState::In { xn: 0, x_first_t: None, .. }));
            prop_assert!(ok, "{:?}", o.state);
        }
        prop_assert_eq!(d.events().len(), 1);
    }

    /// Hostile traces through the whole pipeline (huge gaps, out-of-order times, extreme
    /// coordinates, zero and enormous speeds, all accuracies): no panic; `path_update` refuses
    /// time going backwards; `detect` never errors.
    #[test]
    fn pipeline_survives_hostile_traces(
        sp in arb_signpost(),
        fixes in prop::collection::vec(arb_fix(), 1..40),
    ) {
        let prep = Prepared::new(&sp).unwrap();
        let mut mem = Mem::default();
        let mut st: Option<DetState> = None;
        for f in &fixes {
            match path_update(&mem, f) {
                Ok((m, s)) => {
                    let o: Detected = prep.step(st, &s, f, f.t_ms.saturating_add(500), true).unwrap();
                    mem = m;
                    st = o.state;
                }
                Err(e) => {
                    prop_assert!(f.t_ms <= mem.last_t_ms.unwrap_or(i64::MIN) || matches!(e, proxim_types::EngineError::Overflow));
                }
            }
        }
    }

    /// Steady approach at any speed 3..35 m/s, 1 Hz, within the corridor: exactly one fire, on the
    /// second confirming fix, one to two steps inside D_fire of the anchor.
    #[test]
    fn steady_approach_fires_once_near_d_fire(
        v in 3_000_u32..35_000,
        offset in -25_000_i32..=25_000,
        lead in 15_u32..=120,
        acc in 3_000_u32..=30_000,
    ) {
        let mut sp = route(30);
        sp.trigger.lead_s = Some(lead);
        let mut d = Driver::new(&sp);
        let n = (6_000_000 / v) as usize + 3;
        let fixes: Vec<Fix> = drive_north(0, 0, v, n, offset).into_iter().map(|mut f| { f.acc_mm = acc; f }).collect();
        d.feed_all(&fixes);
        let ev = d.events();
        // CLOSING (errata E-P12) compares with the oldest of the 16 retained fixes, 15 s back at
        // 1 Hz: at 3 m/s that is 45 m against at most acc_eff / 2 = 15 m, so every track in this
        // domain closes (before the 1 Hz walker fix, slow + inaccurate tracks never fired).
        prop_assert_eq!(ev.len(), 1, "v {} lead {} acc {}", v, lead, acc);
        let v_eff = v.max(4000);
        let d_fire = fire_distance_mm(v_eff, lead, 3000, 150_000, 3_000_000).unwrap();
        let rem = ev[0].1.d_rem_mm.unwrap();
        let step = i64::from(v);
        // Arming is on the first fix within d_hi and is not itself a confirming fix, so near
        // d_hi the fire lands two to three steps inside it.
        let upper = (d_fire - step).min(3_000_000 - 2 * step) + 100;
        let lower = (d_fire - 2 * step).min(3_000_000 - 3 * step) - 100;
        prop_assert!(rem <= upper, "rem {} d_fire {} v {}", rem, d_fire, v);
        prop_assert!(rem > lower, "rem {} d_fire {} v {}", rem, d_fire, v);
        // Fired by the time rule, or by the crawl rule (d_rem <= d_lo = 150 m).
        prop_assert!(ev[0].1.t_hat_ms.unwrap() <= i64::from(lead) * 1000 + 3000 || rem <= 150_000);
    }

    /// A stationary device whose fixes scatter by less than `acc_eff / 4` around one point never
    /// gets CLOSING, never arms and never fires (errata E-P12), even when the OS claims a speed and
    /// a course along the route so that every other gate passes and the standing point lies in
    /// the fire zone. Irregular sampling (0.5..3 s) and any accuracy 3..65 m.
    #[test]
    fn stationary_jitter_never_arms_or_fires(
        acc in 3_000_u32..=65_000,
        v in 3_000_u32..30_000,
        pts in prop::collection::vec((-1000_i64..=1000, -1000_i64..=1000, 500_i64..3000), 40..120),
    ) {
        let acc_eff = i64::from(acc.max(10_000));
        let j = (acc_eff / 2 - 1000) / 2; // strictly below acc_eff / 4
        let d_fire = fire_distance_mm(v.max(4000), 40, 3000, 150_000, 3_000_000).unwrap();
        let n0 = 6_000_000 - d_fire * 8 / 10;
        let mut d = Driver::new(&route(30));
        let mut t = 0_i64;
        for (fe, fn_, dt) in pts {
            let f = fix_at(t, (fe * 25_000 / 1000) as i32, (n0 + fn_ * j / 1000) as i32, acc, Some(v), Some(0));
            d.feed(f, 0);
            t += dt;
        }
        prop_assert!(d.events().is_empty());
        let armed = d.steps.iter().any(|s| matches!(s.out.state, Some(DetState::Armed { .. })));
        prop_assert!(!armed);
    }

    /// Never fire when the track stays beyond the corridor buffer, whatever the speed or heading.
    #[test]
    fn off_corridor_never_fires(
        v in 3_000_u32..35_000,
        offset in 31_000_i32..500_000,
        side in any::<bool>(),
    ) {
        let off = if side { offset } else { -offset };
        let mut d = Driver::new(&route(30));
        let n = (6_000_000 / v) as usize + 3;
        d.feed_all(&drive_north(0, 0, v, n, off));
        prop_assert!(d.events().is_empty());
        prop_assert!(d.steps.iter().all(|s| s.out.state.is_none()));
    }

    /// A vehicle moving away from the anchor never fires, at any speed.
    #[test]
    fn moving_away_never_fires(v in 3_000_u32..35_000, start in 0_i64..6_000_000) {
        let fixes: Vec<Fix> = (0..150_i64).map(|i| {
            let n = (start - i64::from(v) * i).max(-1_000_000);
            fix_at(i * 1000, 0, n as i32, 5000, Some(v), Some(1800))
        }).collect();
        let mut d = Driver::new(&route(30));
        d.feed_all(&fixes);
        prop_assert!(d.events().is_empty());
    }

    /// Speed zero, missing and invalid values at every position never panic and never fire an
    /// approach (a stopped vehicle is not closing).
    #[test]
    fn stopped_vehicle_never_fires(n in 0_i32..6_000_000, count in 3_usize..60, speed in prop::option::of(Just(0_u32))) {
        let fixes: Vec<Fix> = (0..count as i64).map(|i| fix_at(i * 1000, 0, n, 5000, speed, Some(0))).collect();
        let mut d = Driver::new(&route(30));
        d.feed_all(&fixes);
        prop_assert!(d.events().is_empty());
    }
}

/// A point at integer distance `d_mm` from the origin in one of 8 directions (45 degree
/// directions use 0.707107, error < 1 mm).
fn polar(d_mm: i64, dir: usize) -> (i32, i32) {
    let diag = d_mm * 707_107 / 1_000_000;
    let (e, n) = match dir {
        0 => (d_mm, 0),
        1 => (diag, diag),
        2 => (0, d_mm),
        3 => (-diag, diag),
        4 => (-d_mm, 0),
        5 => (-diag, -diag),
        6 => (0, -d_mm),
        _ => (diag, -diag),
    };
    (e as i32, n as i32)
}

#[test]
fn polar_helper_is_accurate() {
    for dir in 0..8 {
        let (e, n) = polar(150_000, dir);
        let d =
            isqrt((i128::from(e) * i128::from(e) + i128::from(n) * i128::from(n)).unsigned_abs());
        assert!((d as i64 - 150_000).abs() <= 2, "dir {dir}: {d}");
    }
    assert_eq!(ORIGIN.lat_e7, 450_000_000);
}
