//! Speed filter, heading filter, trace and visibility rules (engine-spec 5.1, 5.2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{fix_at, pos};
use proptest::prelude::*;
use proxim_geo::unit_from_course;
use proxim_path::{PathSummary, floor_mmps, min_speed_mmps, path_update, v_eff_for};
use proxim_types::{EngineError, Fix, Mem, Mode};

const ONE: i32 = 1 << 30;

/// Feed fixes in order, returning the final memory and the last summary.
fn run(fixes: &[Fix]) -> (Mem, PathSummary) {
    let mut mem = Mem::default();
    let mut last = None;
    for f in fixes {
        let (m, s) = path_update(&mem, f).unwrap();
        mem = m;
        last = Some(s);
    }
    (mem, last.unwrap())
}

#[test]
fn first_doppler_measurement_seeds_the_filter() {
    let (mem, s) = run(&[fix_at(1000, 0, 0, 5000, Some(12_345), None)]);
    assert_eq!(mem.vf_mmps, Some(12_345));
    assert_eq!(s.vf_mmps, Some(12_345));
    assert_eq!(s.info.speed_mmps, Some(12_345));
    assert_eq!(mem.last_t_ms, Some(1000));
    assert_eq!(mem.trace.len(), 1);
    assert_eq!(mem.trace[0].v_mmps, Some(12_345));
    assert_eq!(s.fixes_60s, 1);
    assert_eq!(s.recent[0], None);
    assert_eq!(s.recent[1], None);
    assert_eq!(s.recent[2], Some(mem.trace[0]));
    assert_eq!(s.info.tier, 0);
    assert!(!s.info.heading_valid);
    assert_eq!(s.heading_q30, None);
}

#[test]
fn ewma_uses_the_rational_weight_exactly() {
    // dt 1000: alpha_q16 = floor(1000 x 65536 / 9000) = 7281; (10000 x 7281 + 2^15) >> 16 = 1111.
    let (mem, _) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), None),
        fix_at(1000, 0, 0, 5000, Some(20_000), None),
    ]);
    assert_eq!(mem.vf_mmps, Some(11_111));
    // Falling: (-10000 x 7281 + 2^15) >> 16 = -1111 (arithmetic shift rounds the half up).
    let (mem, _) = run(&[
        fix_at(0, 0, 0, 5000, Some(20_000), None),
        fix_at(1000, 0, 0, 5000, Some(10_000), None),
    ]);
    assert_eq!(mem.vf_mmps, Some(18_889));
    // dt 5000 (the largest dt that still blends): alpha = floor(5000 x 65536 / 13000) = 25206.
    let (mem, _) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), None),
        fix_at(5000, 0, 0, 5000, Some(20_000), None),
    ]);
    assert_eq!(mem.vf_mmps, Some(13_846));
}

#[test]
fn constant_speed_is_a_fixed_point() {
    let fixes: Vec<Fix> = (0..40)
        .map(|i| fix_at(i * 1000, 0, 0, 5000, Some(29_000), None))
        .collect();
    let (mem, s) = run(&fixes);
    assert_eq!(mem.vf_mmps, Some(29_000));
    assert_eq!(s.vf_mmps, Some(29_000));
}

#[test]
fn gap_reseeds_from_the_median_of_recent_measurements() {
    // Three measurements of 10 m/s, then a 6 s gap and 20 m/s: median of [20, 10, 10] = 10 m/s.
    let (mem, _) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), None),
        fix_at(1000, 0, 0, 5000, Some(10_000), None),
        fix_at(2000, 0, 0, 5000, Some(10_000), None),
        fix_at(8000, 0, 0, 5000, Some(20_000), None),
    ]);
    assert_eq!(mem.vf_mmps, Some(10_000));
    // After a gap above 10 s no older measurement is recent enough: seed with the measurement.
    let (mem, _) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), None),
        fix_at(1000, 0, 0, 5000, Some(10_000), None),
        fix_at(2000, 0, 0, 5000, Some(10_000), None),
        fix_at(14_000, 0, 0, 5000, Some(20_000), None),
    ]);
    assert_eq!(mem.vf_mmps, Some(20_000));
    // A gap of exactly 5000 ms still blends (dt > 5000 reseeds); 5001 reseeds.
    let (blend, _) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), None),
        fix_at(5000, 0, 0, 5000, Some(20_000), None),
    ]);
    let (seed, _) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), None),
        fix_at(5001, 0, 0, 5000, Some(20_000), None),
    ]);
    assert_eq!(blend.vf_mmps, Some(13_846));
    assert_eq!(
        seed.vf_mmps,
        Some(10_000),
        "median of [20000, 10000] is the lower value"
    );
}

#[test]
fn a_hidden_speed_does_not_weigh_into_a_new_measurement() {
    // The speed of t=0 is hidden at t=12001 (nothing 3..10 s old); at t=13001 a Doppler value
    // arrives. It is seeded (20000), not blended with the stale 10 m/s (which would give 11111).
    let (mem, _) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), None),
        fix_at(2000, 0, 0, 5000, None, None),
        fix_at(12_001, 0, 0, 5000, None, None),
        fix_at(13_001, 0, 0, 5000, Some(20_000), None),
    ]);
    assert_eq!(mem.vf_mmps, Some(20_000));
}

#[test]
fn speed_is_hidden_ten_seconds_after_the_last_measurement() {
    // Fixes placed so that no trace entry is 3..10 s old (no displacement measurement either).
    let mut mem = Mem::default();
    let mut vis = Vec::new();
    for (t, v) in [(0, Some(10_000)), (2000, None), (10_000, None)] {
        let (m, s) = path_update(&mem, &fix_at(t, 0, 0, 5000, v, None)).unwrap();
        mem = m;
        vis.push((s.vf_mmps, mem.trace.last().unwrap().v_mmps));
    }
    // t = 2000: visible. t = 10000: the entry at t=0 is exactly 10 s old, so a displacement
    // measurement (0 mm/s) is taken; that is a measurement, hence visible and blended.
    assert_eq!(vis[0], (Some(10_000), Some(10_000)));
    assert_eq!(vis[1], (Some(10_000), None));
    assert_eq!(vis[2].1, Some(0));
    // Now a real hiding: nothing 3..10 s old at t = 12001 (entries at 0 and 2000).
    let mut mem = Mem::default();
    let mut out = Vec::new();
    for t in [0_i64, 2000, 12_001] {
        let v = if t == 0 { Some(10_000) } else { None };
        let (m, s) = path_update(&mem, &fix_at(t, 0, 0, 5000, v, None)).unwrap();
        mem = m;
        out.push(s.vf_mmps);
    }
    assert_eq!(out, vec![Some(10_000), Some(10_000), None]);
    assert_eq!(
        mem.vf_mmps,
        Some(10_000),
        "the stored value is unchanged, only hidden"
    );
    // Exactly 10 s after the last measurement it is still visible; one ms later it is hidden.
    let (m0, _) = path_update(&Mem::default(), &fix_at(0, 0, 0, 5000, Some(10_000), None)).unwrap();
    let (m1, s1) = path_update(&m0, &fix_at(2000, 0, 0, 5000, None, None)).unwrap();
    assert_eq!(s1.vf_mmps, Some(10_000));
    let far = |t| {
        path_update(&m1, &fix_at(t, 0, 0, 5000, None, None))
            .unwrap()
            .1
            .vf_mmps
    };
    // t = 10000: entry t=0 is 10000 old -> displacement measurement; use 12_000/12_001 instead.
    assert!(
        far(12_000).is_some(),
        "entry t=2000 is exactly 10 s old: measured"
    );
    assert_eq!(far(12_001), None);
}

#[test]
fn speed_from_displacement_when_doppler_is_absent_or_untrusted() {
    // Moving north at exactly 10 m/s, no Doppler: no measurement until an entry is 3 s old.
    let fixes: Vec<Fix> = (0..6)
        .map(|i| fix_at(i * 1000, 0, (i as i32) * 10_000, 5000, None, None))
        .collect();
    let mut mem = Mem::default();
    let mut vfs = Vec::new();
    for f in &fixes {
        let (m, s) = path_update(&mem, f).unwrap();
        mem = m;
        vfs.push(s.vf_mmps);
    }
    assert_eq!(&vfs[..3], &[None, None, None]);
    // Entry aged 3 s is the one at t=0..: 30 m in 3 s. Frame rounding is within a few mm/s.
    let v = vfs[3].unwrap();
    assert!((9_990..=10_010).contains(&v), "v = {v}");
    // An untrusted Doppler (accuracy > 5 m/s) falls back to the displacement.
    let mut bad = fix_at(6000, 0, 60_000, 5000, Some(99_000), None);
    bad.speed_acc_mmps = Some(5001);
    let (m, s) = path_update(&mem, &bad).unwrap();
    let v = s.vf_mmps.unwrap();
    assert!(
        v < 20_000,
        "untrusted Doppler 99 m/s must not be used, got {v}"
    );
    assert_eq!(
        m.trace.last().unwrap().v_mmps.map(|x| x < 20_000),
        Some(true)
    );
    // A trusted Doppler accuracy of exactly 5000 is used.
    let mut ok = fix_at(6000, 0, 60_000, 5000, Some(99_000), None);
    ok.speed_acc_mmps = Some(5000);
    let (m2, _) = path_update(&mem, &ok).unwrap();
    assert_eq!(m2.trace.last().unwrap().v_mmps, Some(99_000));
}

#[test]
fn displacement_speed_ignores_entries_that_are_too_young_or_too_old() {
    // Only an entry 2 s old: no measurement.
    let (_, s) = run(&[
        fix_at(0, 0, 0, 5000, None, None),
        fix_at(2000, 0, 20_000, 5000, None, None),
    ]);
    assert_eq!(s.vf_mmps, None);
    // Only an entry 11 s old: none.
    let (_, s) = run(&[
        fix_at(0, 0, 0, 5000, None, None),
        fix_at(11_000, 0, 110_000, 5000, None, None),
    ]);
    assert_eq!(s.vf_mmps, None);
    // 10 s old is in range: 100 m in 10 s.
    let (_, s) = run(&[
        fix_at(0, 0, 0, 5000, None, None),
        fix_at(10_000, 0, 100_000, 5000, None, None),
    ]);
    let v = s.vf_mmps.unwrap();
    assert!((9_990..=10_010).contains(&v), "v = {v}");
}

#[test]
fn speed_zero_and_extreme_speeds_are_handled() {
    let (mem, s) = run(&[
        fix_at(0, 0, 0, 5000, Some(0), None),
        fix_at(1000, 0, 0, 5000, Some(0), None),
    ]);
    assert_eq!(mem.vf_mmps, Some(0));
    assert_eq!(s.vf_mmps, Some(0));
    assert_eq!(
        s.v_eff_mmps, 4000,
        "the vehicle floor applies at a standstill"
    );
    let (mem, _) = run(&[
        fix_at(0, 0, 0, 5000, Some(u32::MAX), None),
        fix_at(1000, 0, 0, 5000, Some(0), None),
        fix_at(2000, 0, 0, 5000, Some(u32::MAX), None),
    ]);
    assert!(mem.vf_mmps.is_some());
}

#[test]
fn v_eff_floors_per_mode() {
    assert_eq!(v_eff_for(Mode::Vehicle, Some(3000)), 4000);
    assert_eq!(v_eff_for(Mode::Vehicle, Some(5000)), 5000);
    assert_eq!(v_eff_for(Mode::Cycle, Some(1000)), 2000);
    assert_eq!(v_eff_for(Mode::Foot, Some(100)), 700);
    assert_eq!(v_eff_for(Mode::Foot, None), 700);
    assert_eq!(floor_mmps(Mode::Vehicle), 4000);
    assert_eq!(min_speed_mmps(Mode::Vehicle), 2500);
    assert_eq!(min_speed_mmps(Mode::Cycle), 1500);
    assert_eq!(min_speed_mmps(Mode::Foot), 500);
}

#[test]
fn os_course_seeds_the_heading_and_needs_speed() {
    // Heading east (900 ddeg) at 10 m/s.
    let (mem, s) = run(&[fix_at(0, 0, 0, 5000, Some(10_000), Some(900))]);
    assert_eq!(mem.h, Some([ONE, 0]));
    assert_eq!(mem.h_t_ms, Some(0));
    assert_eq!(s.heading_q30, Some([ONE, 0]));
    assert!(s.info.heading_valid);
    // vf = 799 mm/s: the OS course is not trusted.
    let (mem, s) = run(&[fix_at(0, 0, 0, 5000, Some(799), Some(900))]);
    assert_eq!(mem.h, None);
    assert!(!s.info.heading_valid);
    // vf = 800 is enough.
    let (mem, _) = run(&[fix_at(0, 0, 0, 5000, Some(800), Some(900))]);
    assert_eq!(mem.h, Some([ONE, 0]));
    // A course accuracy of 45.0 degrees (450) is accepted, 45.1 degrees is not.
    let mut f = fix_at(0, 0, 0, 5000, Some(10_000), Some(900));
    f.course_acc_ddeg = Some(450);
    assert!(path_update(&Mem::default(), &f).unwrap().0.h.is_some());
    f.course_acc_ddeg = Some(451);
    assert!(path_update(&Mem::default(), &f).unwrap().0.h.is_none());
    // A course outside 0..3599 is treated as absent, not an error.
    let f = fix_at(0, 0, 0, 5000, Some(10_000), Some(3600));
    assert!(path_update(&Mem::default(), &f).unwrap().0.h.is_none());
    let f = fix_at(0, 0, 0, 5000, Some(10_000), Some(u16::MAX));
    assert!(path_update(&Mem::default(), &f).is_ok());
}

#[test]
fn heading_blend_is_the_exact_rational_weight() {
    // North, then east one second later: a = floor(1000 x 65536 / 5000) = 13107;
    // blend = (214745088, 858996736), normalised to Q30.
    let (mem, s) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), Some(0)),
        fix_at(1000, 0, 10_000, 5000, Some(10_000), Some(900)),
    ]);
    assert_eq!(mem.h, Some([260_415_970, 1_041_683_747]));
    assert_eq!(s.heading_q30, mem.h);
    assert_eq!(mem.h_t_ms, Some(1000));
}

#[test]
fn heading_reseeds_after_a_gap_and_on_an_exact_cancellation() {
    // Gap above 5 s: reseed with the measurement, no blend.
    let (mem, _) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), Some(0)),
        fix_at(5001, 0, 50_000, 5000, Some(10_000), Some(900)),
    ]);
    assert_eq!(mem.h, Some([ONE, 0]));
    // Gap of exactly 5000 still blends.
    let (mem, _) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), Some(0)),
        fix_at(5000, 0, 50_000, 5000, Some(10_000), Some(900)),
    ]);
    assert_ne!(mem.h, Some([ONE, 0]));
    // dt = tau = 4000 gives a = 32768 = 1/2: north + south cancel exactly to a zero vector, which
    // is never stored; the measurement (south) reseeds.
    let (m0, _) = path_update(
        &Mem::default(),
        &fix_at(0, 0, 0, 5000, Some(10_000), Some(0)),
    )
    .unwrap();
    let (m1, _) = path_update(&m0, &fix_at(4000, 0, 0, 5000, Some(10_000), Some(1800))).unwrap();
    assert_eq!(m1.h, Some([0, -ONE]));
    // Opposite headings at other dt stay unit vectors.
    for dt in [1, 500, 1000, 2000, 3999, 4001, 5000] {
        let (m1, _) = path_update(&m0, &fix_at(dt, 0, 0, 5000, Some(10_000), Some(1800))).unwrap();
        let h = m1.h.unwrap();
        let len2 = i128::from(h[0]).pow(2) + i128::from(h[1]).pow(2);
        let one2 = i128::from(ONE).pow(2);
        assert!((len2 - one2).abs() < one2 / 1_000_000, "dt {dt}: {h:?}");
    }
    // A small dt barely moves the heading.
    let (m1, _) = path_update(&m0, &fix_at(1, 0, 0, 5000, Some(10_000), Some(900))).unwrap();
    assert!(m1.h.unwrap()[1] > ONE - 1_000_000);
}

#[test]
fn heading_validity_expires_after_ten_seconds() {
    let mut mem = Mem::default();
    let mut valid = Vec::new();
    for (t, c) in [(0, Some(0)), (5000, None), (10_000, None), (10_001, None)] {
        let (m, s) = path_update(&mem, &fix_at(t, 0, 0, 5000, Some(10_000), c)).unwrap();
        mem = m;
        valid.push((s.info.heading_valid, s.heading_q30.is_some()));
    }
    assert_eq!(
        valid,
        vec![(true, true), (true, true), (true, true), (false, false)]
    );
    // The stored heading is untouched; only visibility lapses.
    assert_eq!(mem.h, Some([0, ONE]));
}

#[test]
fn heading_from_displacement_needs_distance_and_age() {
    // Driving east at 11 m/s, no course. With acc_eff 10 m the bar is max(15 m, 30 m) = 30 m
    // over the newest entry that is at least 3 s old: 33 m in 3 s clears it.
    let drive = |acc: u32| {
        let mut mem = Mem::default();
        let mut last = None;
        for i in 0..6_i64 {
            let (m, s) = path_update(
                &mem,
                &fix_at(i * 1000, (i as i32) * 11_000, 0, acc, Some(11_000), None),
            )
            .unwrap();
            mem = m;
            last = Some(s);
        }
        last.unwrap()
    };
    let s = drive(5000);
    let h = s.heading_q30.unwrap();
    assert!(h[0] > ONE - 100_000 && h[1].abs() < 100_000, "east: {h:?}");
    assert!(s.info.heading_valid);
    // Poor accuracy raises the bar to 3 x 20 m = 60 m: 33 m is not enough.
    assert_eq!(drive(20_000).heading_q30, None);
    // A walker (1.4 m/s) never reaches 30 m in 3 s: no displacement heading (documented false
    // negative; the OS course covers that case).
    let mut mem = Mem::default();
    let mut last = None;
    for i in 0..12_i64 {
        let (m, s) = path_update(
            &mem,
            &fix_at(i * 1000, (i as i32) * 1400, 0, 5000, Some(1_400), None),
        )
        .unwrap();
        mem = m;
        last = Some(s);
    }
    assert_eq!(last.unwrap().heading_q30, None);
    // The threshold itself: 30.0 m in 3 s works, 29.9 m does not.
    let (_, s) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), None),
        fix_at(3000, 30_010, 0, 5000, Some(10_000), None),
    ]);
    assert!(s.heading_q30.is_some());
    let (_, s) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), None),
        fix_at(3000, 29_900, 0, 5000, Some(10_000), None),
    ]);
    assert_eq!(s.heading_q30, None);
    // Entries younger than 3 s are not used even when far: 100 m in 2 s.
    let (_, s) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), None),
        fix_at(2000, 100_000, 0, 5000, Some(10_000), None),
    ]);
    assert_eq!(s.heading_q30, None);
}

#[test]
fn trace_is_capped_at_sixteen_oldest_first() {
    let fixes: Vec<Fix> = (0..40)
        .map(|i| fix_at(i * 1000, 0, 0, 5000, Some(10_000), Some(0)))
        .collect();
    let (mem, s) = run(&fixes);
    assert_eq!(mem.trace.len(), 16);
    assert_eq!(mem.trace[0].t_ms, 24_000);
    assert_eq!(mem.trace[15].t_ms, 39_000);
    assert!(mem.trace.windows(2).all(|w| w[0].t_ms < w[1].t_ms));
    assert_eq!(s.fixes_60s, 16);
    let times: Vec<i64> = s.recent_fixes().map(|f| f.t_ms).collect();
    assert_eq!(times, vec![37_000, 38_000, 39_000]);
}

#[test]
fn fixes_in_the_last_sixty_seconds_are_counted() {
    // An entry exactly 60 s old still counts; 60.001 s does not.
    let (_, s) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), None),
        fix_at(30_000, 0, 0, 5000, Some(10_000), None),
        fix_at(60_000, 0, 0, 5000, Some(10_000), None),
    ]);
    assert_eq!(s.fixes_60s, 3, "the fix at t=0 is exactly 60000 ms old");
    let (_, s) = run(&[
        fix_at(0, 0, 0, 5000, Some(10_000), None),
        fix_at(30_000, 0, 0, 5000, Some(10_000), None),
        fix_at(60_001, 0, 0, 5000, Some(10_000), None),
    ]);
    assert_eq!(s.fixes_60s, 2, "the fix at t=0 is 60001 ms old");
}

#[test]
fn out_of_order_and_duplicate_times_are_rejected_without_state_change() {
    let (mem, _) = run(&[fix_at(5000, 0, 0, 5000, Some(10_000), None)]);
    assert_eq!(
        path_update(&mem, &fix_at(5000, 0, 0, 5000, Some(10_000), None)).unwrap_err(),
        EngineError::Parse
    );
    assert_eq!(
        path_update(&mem, &fix_at(4999, 0, 0, 5000, Some(10_000), None)).unwrap_err(),
        EngineError::Parse
    );
    assert_eq!(
        path_update(&mem, &fix_at(i64::MIN, 0, 0, 5000, None, None)).unwrap_err(),
        EngineError::Parse
    );
    assert!(path_update(&mem, &fix_at(i64::MAX, 0, 0, 5000, None, None)).is_ok());
}

#[test]
fn input_memory_is_never_mutated() {
    let (mem, _) = run(&[fix_at(0, 0, 0, 5000, Some(10_000), Some(0))]);
    let before = mem.clone();
    let _ = path_update(&mem, &fix_at(1000, 0, 10_000, 5000, Some(11_000), Some(10)));
    assert_eq!(mem, before);
}

#[test]
fn unit_vectors_agree_with_the_geo_table() {
    for c in [0_u16, 450, 900, 1799, 2700, 3599] {
        let (m, _) = path_update(
            &Mem::default(),
            &fix_at(0, 0, 0, 5000, Some(10_000), Some(c)),
        )
        .unwrap();
        assert_eq!(m.h, Some(unit_from_course(c).unwrap()), "course {c}");
    }
}

fn arb_fix() -> impl Strategy<Value = Fix> {
    (
        any::<i64>(),
        prop_oneof![any::<i32>(), -900_000_000..900_000_000_i32],
        prop_oneof![any::<i32>(), -1_800_000_000..1_800_000_000_i32],
        any::<u32>(),
        prop::option::of(any::<u32>()),
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Hostile fixes (any times, coordinates, accuracies, speeds, courses) never panic and never
    /// corrupt the memory: the trace stays capped and ordered and `last_t` is the accepted time.
    #[test]
    fn path_update_is_total_on_hostile_input(fixes in prop::collection::vec(arb_fix(), 1..40)) {
        let mut mem = Mem::default();
        for f in &fixes {
            match path_update(&mem, f) {
                Ok((m, s)) => {
                    prop_assert_eq!(m.last_t_ms, Some(f.t_ms));
                    prop_assert!(m.trace.len() <= 16);
                    prop_assert!(m.trace.windows(2).all(|w| w[0].t_ms < w[1].t_ms));
                    prop_assert_eq!(s.recent[2].map(|r| r.t_ms), Some(f.t_ms));
                    prop_assert!(s.v_eff_mmps >= 4000);
                    if let Some(h) = m.h {
                        let len2 = i128::from(h[0]).pow(2) + i128::from(h[1]).pow(2);
                        let one2 = i128::from(ONE).pow(2);
                        prop_assert!((len2 - one2).abs() < one2 / 1000, "not a unit vector: {:?}", h);
                    }
                    mem = m;
                }
                Err(e) => {
                    prop_assert!(matches!(e, EngineError::Parse | EngineError::Overflow));
                }
            }
        }
    }

    /// Same memory and fix give the same result (pure function), and the input is untouched.
    #[test]
    fn path_update_is_a_pure_function(fixes in prop::collection::vec(arb_fix(), 1..12)) {
        let mut mem = Mem::default();
        for f in &fixes {
            let before = mem.clone();
            let a = path_update(&mem, f);
            let b = path_update(&mem, f);
            prop_assert_eq!(&a, &b);
            prop_assert_eq!(&mem, &before);
            if let Ok((m, _)) = a { mem = m; }
        }
    }

    /// The filtered speed is a convex combination of measurements: it never leaves their range.
    #[test]
    fn filtered_speed_stays_within_the_measured_range(
        speeds in prop::collection::vec(0_u32..80_000, 2..60),
        dts in prop::collection::vec(1_i64..9000, 2..60),
    ) {
        let mut mem = Mem::default();
        let mut t = 0_i64;
        let (mut lo, mut hi) = (u32::MAX, 0_u32);
        for (v, dt) in speeds.iter().zip(dts.iter()) {
            t += dt;
            lo = lo.min(*v);
            hi = hi.max(*v);
            let (m, _) = path_update(&mem, &fix_at(t, 0, 0, 5000, Some(*v), None)).unwrap();
            let vf = m.vf_mmps.unwrap();
            prop_assert!(vf >= lo && vf <= hi, "vf {} outside [{}, {}]", vf, lo, hi);
            mem = m;
        }
    }

    /// Position is irrelevant to the speed filter when Doppler is present (determinism of a
    /// numeric trace under translation).
    #[test]
    fn doppler_filter_ignores_position(
        speeds in prop::collection::vec(0_u32..60_000, 2..30),
        shift in -50_000_i32..50_000,
    ) {
        let mut a = Mem::default();
        let mut b = Mem::default();
        for (i, v) in speeds.iter().enumerate() {
            let t = (i as i64 + 1) * 1000;
            a = path_update(&a, &fix_at(t, 0, 0, 5000, Some(*v), None)).unwrap().0;
            b = path_update(&b, &fix_at(t, shift, shift, 5000, Some(*v), None)).unwrap().0;
            prop_assert_eq!(a.vf_mmps, b.vf_mmps);
        }
    }
}

#[test]
fn pos_helper_round_trips() {
    let p = pos(1000, -2000);
    assert!(p.lat_e7 < 450_000_000);
}
