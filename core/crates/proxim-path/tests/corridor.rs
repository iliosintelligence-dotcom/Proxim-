//! Corridor evidence, CLOSING, the P1 confidence ladder and window projection (spec 5.3, 6.2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{fix_at, frame};
use proptest::prelude::*;
use proxim_geo::{Polyline, cos_tol_q30};
use proxim_path::{
    P1Evidence, Recent, closing, copy_for_tier, heading_dot, p1_confidence, path_update,
    project_recent,
};
use proxim_types::{Copy as AdCopy, Enu, Mem, Mode};

const ONE: i64 = 1 << 30;

fn rc(t: i64, along: i64, xt: i64, acc: i64) -> Recent {
    Recent {
        t_ms: t,
        along_mm: along,
        xt_mm: xt,
        acc_eff_mm: acc,
    }
}

/// Three good fixes advancing 15 m per second, 5 m off-axis.
fn good() -> Vec<Recent> {
    vec![
        rc(0, 0, 5000, 10_000),
        rc(1000, 15_000, 5000, 10_000),
        rc(2000, 30_000, 5000, 10_000),
    ]
}

/// A baseline 4 s before the last fix on the same 15 m/s track (60 m behind it).
fn base_of(recent: &[Recent]) -> Recent {
    let last = recent.last().unwrap();
    rc(last.t_ms - 4000, last.along_mm - 60_000, 0, 10_000)
}

fn ev<'a>(
    recent: &'a [Recent],
    base: &'a Recent,
    n60: u8,
    mode: Mode,
    dot: Option<i64>,
    vf: Option<u32>,
) -> P1Evidence<'a> {
    P1Evidence {
        recent,
        base: Some(base),
        fixes_60s: n60,
        vf_mmps: vf,
        mode,
        heading_dot_q30: dot,
        buffer_mm: 30_000,
    }
}

fn cos(ddeg: u16) -> i64 {
    i64::from(cos_tol_q30(ddeg).unwrap())
}

#[test]
fn p1_ladder_values() {
    let g = good();
    let b = base_of(&g);
    let c = |n60, dot| p1_confidence(&ev(&g, &b, n60, Mode::Vehicle, dot, Some(10_000))).unwrap();
    // Base 600: three fixes, heading between 15 and 45 degrees.
    assert_eq!(c(3, Some(cos(300))), Some(600));
    // +100 for five fixes in 60 s.
    assert_eq!(c(5, Some(cos(300))), Some(700));
    // +100 for a heading within 15 degrees (inclusive).
    assert_eq!(c(3, Some(cos(150))), Some(700));
    assert_eq!(c(3, Some(cos(150) - 1)), Some(600));
    // Both: 800, the top of the range.
    assert_eq!(c(8, Some(ONE)), Some(800));
}

/// Three fixes at 40 m intervals with the given accuracies (the baseline is always 10 m).
fn spaced(acc: [i64; 3]) -> Vec<Recent> {
    vec![
        rc(0, 0, 0, acc[0]),
        rc(1000, 40_000, 0, acc[1]),
        rc(2000, 80_000, 0, acc[2]),
    ]
}

#[test]
fn p1_accuracy_penalties_and_cap() {
    let with_acc = |acc: i64| {
        let r = spaced([10_000, acc, 10_000]);
        let b = base_of(&r);
        p1_confidence(&ev(&r, &b, 3, Mode::Vehicle, Some(cos(300)), Some(10_000))).unwrap()
    };
    assert_eq!(
        with_acc(30_000),
        Some(600),
        "30 m is not above the first threshold"
    );
    assert_eq!(with_acc(30_001), Some(500), "-100 above 30 m");
    assert_eq!(with_acc(50_000), Some(500));
    assert_eq!(
        with_acc(50_001),
        Some(400),
        "-200 above 50 m (replaces, not adds)"
    );
    assert_eq!(with_acc(65_000), Some(400));
    assert_eq!(with_acc(65_001), None, "above 65 m is not P1");
    // The worst of the last three counts, wherever it sits.
    let r = spaced([60_000, 10_000, 10_000]);
    let b = base_of(&r);
    assert_eq!(
        p1_confidence(&ev(&r, &b, 3, Mode::Vehicle, Some(cos(300)), Some(10_000))).unwrap(),
        Some(400)
    );
    // Range 400..800 over all combinations.
    for acc in [10_000, 31_000, 51_000, 65_000] {
        for n60 in [3, 5] {
            for dot in [cos(450), cos(150), ONE] {
                let r = spaced([acc; 3]);
                let b = base_of(&r);
                let v = p1_confidence(&ev(&r, &b, n60, Mode::Vehicle, Some(dot), Some(10_000)))
                    .unwrap()
                    .unwrap();
                assert!((400..=800).contains(&v), "{v}");
            }
        }
    }
}

#[test]
fn p1_foot_is_capped_at_600() {
    let g = good();
    let gb = base_of(&g);
    let foot = |n60, dot| p1_confidence(&ev(&g, &gb, n60, Mode::Foot, dot, Some(1_500))).unwrap();
    assert_eq!(foot(8, Some(ONE)), Some(600));
    assert_eq!(foot(3, Some(cos(300))), Some(600));
    let r = spaced([40_000; 3]);
    let b = base_of(&r);
    // The cap applies after the penalty: 600 - 100 stays 500.
    assert_eq!(
        p1_confidence(&ev(&r, &b, 8, Mode::Foot, Some(ONE), Some(1_500))).unwrap(),
        Some(600)
    );
    assert_eq!(
        p1_confidence(&ev(&r, &b, 3, Mode::Foot, Some(cos(300)), Some(1_500))).unwrap(),
        Some(500)
    );
}

#[test]
fn p1_each_criterion_is_necessary() {
    let g = good();
    let b = base_of(&g);
    let ok = |e: &P1Evidence<'_>| p1_confidence(e).unwrap().is_some();
    let base = ev(&g, &b, 3, Mode::Vehicle, Some(cos(300)), Some(10_000));
    assert!(ok(&base));
    // Fewer than three fixes in 60 s.
    assert!(!ok(&P1Evidence {
        fixes_60s: 2,
        ..base
    }));
    // Fewer than three recent fixes.
    assert!(!ok(&ev(
        &g[1..],
        &b,
        3,
        Mode::Vehicle,
        Some(cos(300)),
        Some(10_000)
    )));
    // Speed: hidden, below the mode minimum, at the minimum.
    assert!(!ok(&P1Evidence {
        vf_mmps: None,
        ..base
    }));
    assert!(!ok(&P1Evidence {
        vf_mmps: Some(2499),
        ..base
    }));
    assert!(ok(&P1Evidence {
        vf_mmps: Some(2500),
        ..base
    }));
    assert!(!ok(&P1Evidence {
        mode: Mode::Cycle,
        vf_mmps: Some(1499),
        ..base
    }));
    assert!(ok(&P1Evidence {
        mode: Mode::Cycle,
        vf_mmps: Some(1500),
        ..base
    }));
    assert!(!ok(&P1Evidence {
        mode: Mode::Foot,
        vf_mmps: Some(499),
        ..base
    }));
    assert!(ok(&P1Evidence {
        mode: Mode::Foot,
        vf_mmps: Some(500),
        ..base
    }));
    // Heading: invalid, just outside 45 degrees, exactly 45 degrees.
    assert!(!ok(&P1Evidence {
        heading_dot_q30: None,
        ..base
    }));
    assert!(!ok(&P1Evidence {
        heading_dot_q30: Some(cos(450) - 1),
        ..base
    }));
    assert!(ok(&P1Evidence {
        heading_dot_q30: Some(cos(450)),
        ..base
    }));
    // Cross-track: exactly the buffer passes, one mm beyond fails, on any of the three.
    for i in 0..3 {
        let mut r = g.clone();
        r[i].xt_mm = 30_000;
        assert!(ok(&ev(
            &r,
            &b,
            3,
            Mode::Vehicle,
            Some(cos(300)),
            Some(10_000)
        )));
        r[i].xt_mm = -30_001;
        assert!(!ok(&ev(
            &r,
            &b,
            3,
            Mode::Vehicle,
            Some(cos(300)),
            Some(10_000)
        )));
    }
    // CLOSING is necessary: no baseline, a baseline that is too young, no net advance.
    assert!(!ok(&P1Evidence { base: None, ..base }));
    let young = rc(g[2].t_ms - 3999, g[2].along_mm - 60_000, 0, 10_000);
    assert!(!ok(&P1Evidence {
        base: Some(&young),
        ..base
    }));
    let level = rc(g[2].t_ms - 4000, g[2].along_mm, 0, 10_000);
    assert!(!ok(&P1Evidence {
        base: Some(&level),
        ..base
    }));
    // Not closing (a U-turn): along goes back.
    let u = vec![
        rc(0, 30_000, 0, 10_000),
        rc(1000, 15_000, 0, 10_000),
        rc(2000, 0, 0, 10_000),
    ];
    let ub = rc(-2000, 0, 0, 10_000);
    assert!(!ok(&ev(
        &u,
        &ub,
        3,
        Mode::Vehicle,
        Some(cos(300)),
        Some(10_000)
    )));
    // A U-turn on the last step only: far ahead of the baseline, but retreating now.
    let turn = vec![
        rc(0, 60_000, 0, 10_000),
        rc(1000, 75_000, 0, 10_000),
        rc(2000, 60_000, 0, 10_000),
    ];
    assert!(!ok(&ev(
        &turn,
        &b,
        3,
        Mode::Vehicle,
        Some(cos(300)),
        Some(10_000)
    )));
    // Stopped.
    let s = vec![
        rc(0, 0, 0, 10_000),
        rc(1000, 0, 0, 10_000),
        rc(2000, 0, 0, 10_000),
    ];
    let sb = rc(-2000, 0, 0, 10_000);
    assert!(!ok(&ev(
        &s,
        &sb,
        3,
        Mode::Vehicle,
        Some(cos(300)),
        Some(10_000)
    )));
}

#[test]
fn heading_dot_and_copy() {
    assert_eq!(heading_dot(None, [0, 1 << 30]), None);
    assert_eq!(heading_dot(Some([0, 1 << 30]), [0, 1 << 30]), Some(ONE));
    assert_eq!(heading_dot(Some([1 << 30, 0]), [0, 1 << 30]), Some(0));
    assert_eq!(heading_dot(Some([0, -(1 << 30)]), [0, 1 << 30]), Some(-ONE));
    assert_eq!(copy_for_tier(0), AdCopy::NearYou);
    assert_eq!(copy_for_tier(1), AdCopy::Ahead);
}

/// Feed fixes, return the summary at the end.
fn summary_of(fixes: &[proxim_types::Fix]) -> proxim_path::PathSummary {
    let mut mem = Mem::default();
    let mut last = None;
    for f in fixes {
        let (m, s) = path_update(&mem, f).unwrap();
        mem = m;
        last = Some(s);
    }
    last.unwrap()
}

fn north_route() -> Polyline {
    Polyline::new(&[
        Enu { e_mm: 0, n_mm: 0 },
        Enu {
            e_mm: 0,
            n_mm: 6_000_000,
        },
    ])
    .unwrap()
}

/// A 6 km northbound route made of six 1 km segments (so that along windows can select segments).
fn stepped_route() -> Polyline {
    let v: Vec<Enu> = (0..=6)
        .map(|i| Enu {
            e_mm: 0,
            n_mm: i * 1_000_000,
        })
        .collect();
    Polyline::new(&v).unwrap()
}

/// 15 m/s northbound on the axis, 1 Hz, `count` fixes from t = 0.
fn steady(count: i64) -> Vec<proxim_types::Fix> {
    (0..count)
        .map(|i| {
            fix_at(
                i * 1000,
                3000,
                (i as i32) * 15_000,
                5000,
                Some(15_000),
                Some(0),
            )
        })
        .collect()
}

#[test]
fn project_recent_on_a_straight_route() {
    let s = summary_of(&steady(5));
    let p = project_recent(&s, &frame(), &north_route(), None, 15_000).unwrap();
    assert_eq!(p.items.len(), 3);
    for (i, r) in p.items.iter().enumerate() {
        let along = (i as i64 + 2) * 15_000;
        assert!(
            (r.along_mm - along).abs() <= 25,
            "{} vs {along}",
            r.along_mm
        );
        // East of a northbound route is to the right: negative cross-track.
        assert!((r.xt_mm + 3000).abs() <= 25, "{}", r.xt_mm);
        assert_eq!(r.acc_eff_mm, 10_000);
        assert_eq!(r.t_ms, (i as i64 + 2) * 1000);
    }
    assert_eq!(p.cur.along_mm, p.items[2].along_mm);
    assert_eq!(p.cur.seg, 0);
    // The baseline is the oldest retained fix, at t = 0 (4 s older than the newest).
    let b = p.base.unwrap();
    assert_eq!(b.t_ms, 0);
    assert!(b.along_mm.abs() <= 25 && (b.xt_mm + 3000).abs() <= 25);
    assert!(closing(&p.items, p.base.as_ref()));
}

#[test]
fn project_recent_without_a_four_second_old_fix_has_no_baseline_and_is_not_closing() {
    for n in 1..=4 {
        let s = summary_of(&steady(n));
        assert!(s.closing_base.is_none(), "{n} fixes span {} s", n - 1);
        let p = project_recent(&s, &frame(), &north_route(), None, 15_000).unwrap();
        assert!(p.base.is_none());
        assert!(!closing(&p.items, p.base.as_ref()), "{n} fixes");
    }
    let s = summary_of(&steady(5));
    let p = project_recent(&s, &frame(), &north_route(), None, 15_000).unwrap();
    assert!(closing(&p.items, p.base.as_ref()));
}

#[test]
fn project_recent_with_one_fix_returns_what_exists() {
    let s = summary_of(&[fix_at(0, 0, 0, 5000, Some(15_000), Some(0))]);
    let p = project_recent(&s, &frame(), &north_route(), Some(0), 15_000).unwrap();
    assert_eq!(p.items.len(), 1);
    assert!(p.base.is_none());
    assert!(!closing(&p.items, p.base.as_ref()));
}

#[test]
fn a_prev_window_holds_the_current_fix_to_the_outbound_leg() {
    // A hairpin: north 3 km, 40 m east, south 3 km (legs 40 m apart). The vehicle drives the
    // outbound leg at e = 30 m, nearer to the return leg at e = 40 m.
    let route = Polyline::new(&[
        Enu { e_mm: 0, n_mm: 0 },
        Enu {
            e_mm: 0,
            n_mm: 3_000_000,
        },
        Enu {
            e_mm: 40_000,
            n_mm: 3_000_000,
        },
        Enu {
            e_mm: 40_000,
            n_mm: 0,
        },
    ])
    .unwrap();
    let fixes: Vec<_> = (0..5)
        .map(|i| {
            fix_at(
                i * 1000,
                30_000,
                1_000_000 + (i as i32) * 15_000,
                5000,
                Some(15_000),
                Some(0),
            )
        })
        .collect();
    let s = summary_of(&fixes);
    let idle = project_recent(&s, &frame(), &route, None, 15_000).unwrap();
    assert!(
        idle.items.iter().all(|r| r.along_mm > 3_040_000),
        "idle: every fix snaps to the nearer return leg"
    );
    // The previous fix was on the outbound leg (along 1_045 m): with the exact a_prev the window
    // holds all three recent fixes and the baseline there, which the old three-fix chain could not
    // do when its first projection was wrong.
    let armed = project_recent(&s, &frame(), &route, Some(1_045_000), 15_000).unwrap();
    assert_eq!(armed.items.len(), 3);
    for r in &armed.items {
        assert!(r.along_mm < 1_100_000, "{r:?}");
        assert!(r.xt_mm.abs() >= 29_000);
    }
    assert!(armed.base.unwrap().along_mm < 1_100_000);
    assert!(armed.cur.along_mm < 1_100_000);
    assert!(closing(&armed.items, armed.base.as_ref()));
}

/// A summary whose newest fix is on a 1 km stepped route at northing `n_mm`, preceded by one
/// older fix `gap_ms` earlier at `n_old_mm`.
fn two_fixes(gap_ms: i64, n_old_mm: i32, n_mm: i32) -> proxim_path::PathSummary {
    summary_of(&[
        fix_at(0, 0, n_old_mm, 5000, Some(15_000), Some(0)),
        fix_at(gap_ms, 0, n_mm, 5000, Some(15_000), Some(0)),
    ])
}

#[test]
fn a_prev_window_edges_are_exact_for_the_current_fix() {
    // v_eff 15 m/s: window = [a - 100 m, a + 500 m + 450 m] = [a - 100_000, a + 950_000].
    let route = stepped_route();
    // Current fix at 2.5 km (segment 2, cum 2_000_000..3_000_000).
    let s = summary_of(&[fix_at(0, 0, 2_500_000, 5000, Some(15_000), Some(0))]);
    let along = |a: i64| {
        project_recent(&s, &frame(), &route, Some(a), 15_000)
            .unwrap()
            .cur
            .along_mm
    };
    // hi = a + 950_000 reaches cum[2] = 2_000_000 exactly at a = 1_050_000: segment 2 is a candidate.
    assert!((along(1_050_000) - 2_500_000).abs() <= 25);
    // One mm less and segment 2 is out: the fix is pinned to the end of segment 1.
    assert!((along(1_049_999) - 2_000_000).abs() <= 25);
    // Lower edge: lo = a - 100_000. Segment 2 ends at cum[3] = 3_000_000; it is a candidate while
    // cum[3] >= lo, i.e. a <= 3_100_000. Beyond that the fix is pinned to segment 3's start.
    assert!((along(3_100_000) - 2_500_000).abs() <= 25);
    assert!((along(3_100_001) - 3_000_000).abs() <= 25);
    // No a_prev: the whole route, whatever the history.
    let whole = project_recent(&s, &frame(), &route, None, 15_000).unwrap();
    assert!((whole.cur.along_mm - 2_500_000).abs() <= 25);
}

#[test]
fn older_fixes_get_a_back_margin_of_v_eff_times_their_age() {
    let route = stepped_route();
    // Older fix 10 s before the current one at 1.5 km (segment 1); current fix at 2.6 km.
    // v_eff 15 m/s: the margin is 100 m + 150 m behind a_prev for the older fix, 100 m for the
    // current one. Segment 1 ends at 2_000_000, so it is a candidate while a - 250_000 <= 2_000_000.
    let s = two_fixes(10_000, 1_500_000, 2_600_000);
    let old_along = |a: i64| {
        project_recent(&s, &frame(), &route, Some(a), 15_000)
            .unwrap()
            .items[0]
            .along_mm
    };
    assert!(
        (old_along(2_250_000) - 1_500_000).abs() <= 25,
        "at the edge"
    );
    assert!(
        (old_along(2_250_001) - 2_000_000).abs() <= 25,
        "one mm beyond: pinned to the start of segment 2"
    );
    // The same a_prev with no age (the current fix) is already out at 2_100_001.
    let s1 = summary_of(&[fix_at(0, 0, 1_500_000, 5000, Some(15_000), Some(0))]);
    let cur_along = |a: i64| {
        project_recent(&s1, &frame(), &route, Some(a), 15_000)
            .unwrap()
            .cur
            .along_mm
    };
    assert!((cur_along(2_100_000) - 1_500_000).abs() <= 25);
    assert!((cur_along(2_100_001) - 2_000_000).abs() <= 25);
}

#[test]
fn the_baseline_is_projected_with_the_window_of_its_age() {
    let route = stepped_route();
    // 20 fixes at 1 Hz, 15 m/s from n = 1.5 km; the baseline is the oldest retained one (age 15 s).
    let fixes: Vec<_> = (0..20)
        .map(|i| {
            fix_at(
                i * 1000,
                0,
                1_500_000 + (i as i32) * 15_000,
                5000,
                Some(15_000),
                Some(0),
            )
        })
        .collect();
    let s = summary_of(&fixes);
    assert_eq!(s.closing_base.unwrap().t_ms, 4000, "trace holds 16 fixes");
    let base_along = |a: i64| {
        project_recent(&s, &frame(), &route, Some(a), 15_000)
            .unwrap()
            .base
            .unwrap()
            .along_mm
    };
    // Baseline at n = 1_560_000. a = 1_785_000 (cur is at 1_785_000): margin 100 m + 15 s x 15 m/s
    // = 325 m behind: lo = 1_460_000 -> segment 1 is in.
    assert!((base_along(1_785_000) - 1_560_000).abs() <= 25);
    // a far ahead: segment 1 (ends at 2_000_000) is out once lo > 2_000_000, i.e. a > 2_325_000.
    assert!((base_along(2_325_000) - 1_560_000).abs() <= 25);
    assert!((base_along(2_325_001) - 2_000_000).abs() <= 25);
}

#[test]
fn a_window_that_overlaps_no_segment_falls_back_to_the_whole_route() {
    // a_prev far beyond the route end: the window overlaps nothing, so the whole route is searched
    // rather than failing the call (E-G5).
    let route = stepped_route();
    let s = summary_of(&[fix_at(0, 0, 2_500_000, 5000, Some(15_000), Some(0))]);
    let p = project_recent(&s, &frame(), &route, Some(50_000_000), 15_000).unwrap();
    assert!((p.cur.along_mm - 2_500_000).abs() <= 25);
    let p = project_recent(&s, &frame(), &route, Some(i64::MIN), 15_000).unwrap();
    assert!((p.cur.along_mm - 2_500_000).abs() <= 25);
    let p = project_recent(&s, &frame(), &route, Some(i64::MAX), u32::MAX).unwrap();
    assert!((p.cur.along_mm - 2_500_000).abs() <= 25);
}

#[test]
fn project_recent_reports_out_of_frame() {
    let s = summary_of(&[fix_at(0, 0, 0, 5000, Some(15_000), Some(0))]);
    // A route whose frame is 200 km away from the fix: the fix is outside the 150 km frame.
    let far = proxim_geo::frame_for(proxim_types::Position {
        lat_e7: 470_000_000,
        lon_e7: 70_000_000,
    })
    .unwrap();
    assert_eq!(
        project_recent(&s, &far, &north_route(), None, 15_000).unwrap_err(),
        proxim_types::EngineError::OutOfFrame
    );
}

#[test]
fn the_summary_baseline_is_the_oldest_retained_fix_aged_4_to_60_s() {
    let base_t = |fixes: &[proxim_types::Fix]| summary_of(fixes).closing_base.map(|f| f.t_ms);
    let at = |t: i64| {
        fix_at(
            t,
            0,
            (t / 1000) as i32 * 15_000,
            5000,
            Some(15_000),
            Some(0),
        )
    };
    let seq = |ts: &[i64]| ts.iter().map(|&t| at(t)).collect::<Vec<_>>();
    assert_eq!(base_t(&seq(&[0, 1000, 2000, 3000])), None, "3 s");
    assert_eq!(base_t(&seq(&[0, 1000, 2000, 3999])), None, "3.999 s");
    assert_eq!(base_t(&seq(&[0, 1000, 2000, 4000])), Some(0), "exactly 4 s");
    // The oldest, not the nearest, qualifying fix.
    assert_eq!(base_t(&seq(&[0, 1000, 2000, 3000, 4000, 5000])), Some(0));
    // A fix aged 3.999 s is not eligible but an older one is.
    assert_eq!(base_t(&seq(&[0, 1001, 5000])), Some(0));
    // Exactly 60 s is the oldest accepted age; 60.001 s is dropped and the next one is used.
    assert_eq!(base_t(&seq(&[0, 1000, 61_000])), Some(1000));
    assert_eq!(base_t(&seq(&[0, 1001, 61_000])), Some(1001));
    assert_eq!(base_t(&seq(&[0, 60_000])), Some(0));
    assert_eq!(base_t(&seq(&[0, 60_001])), None, "a lone 60.001 s old fix");
    // The trace keeps 16 fixes, so at 1 Hz the baseline is 15 s old.
    let long: Vec<_> = (0..40).map(|i| at(i * 1000)).collect();
    assert_eq!(base_t(&long), Some(24_000));
    // At 5 Hz the 16 retained fixes span 3 s: no baseline, CLOSING is never true.
    let fast: Vec<_> = (0..40).map(|i| at(i * 200)).collect();
    assert_eq!(base_t(&fast), None);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// CLOSING is invariant under adding a constant to every along value, monotone in the newest
    /// along (moving the newest fix forward never turns a closing track into a non-closing one),
    /// and a track and its mirror image are never both closing.
    #[test]
    fn closing_translation_monotonicity_and_mirror(
        a in prop::array::uniform4(-1_000_000_i64..1_000_000),
        acc in prop::array::uniform4(0_i64..200_000),
        span in 3000_i64..70_000,
        shift in -10_000_000_i64..10_000_000,
        more in 0_i64..1_000_000,
    ) {
        let mk = |a: [i64; 4]| -> (Vec<Recent>, Recent) {
            let items = vec![rc(span - 2000, a[1], 0, acc[1]), rc(span - 1000, a[2], 0, acc[2]), rc(span, a[3], 0, acc[3])];
            (items, rc(0, a[0], 0, acc[0]))
        };
        let (items, base) = mk(a);
        let c = closing(&items, Some(&base));
        let (items2, base2) = mk([a[0] + shift, a[1] + shift, a[2] + shift, a[3] + shift]);
        prop_assert_eq!(c, closing(&items2, Some(&base2)));
        let (items3, base3) = mk([-a[0], -a[1], -a[2], -a[3]]);
        prop_assert!(!(c && closing(&items3, Some(&base3))));
        let (items4, base4) = mk([a[0], a[1], a[2], a[3] + more]);
        if c { prop_assert!(closing(&items4, Some(&base4))); }
    }

    /// A stationary device whose fixes scatter within `acc_eff / 4` (strictly below the
    /// threshold / 2) of one point never closes, whatever the sampling times.
    #[test]
    fn stationary_scatter_never_closes(
        acc in 10_000_i64..65_000,
        rel in prop::collection::vec(-1000_i64..=1000, 4),
        span in 4000_i64..60_000,
        centre in -5_000_000_i64..5_000_000,
    ) {
        // Scatter strictly below acc/4: |j| <= (acc/2 - 1) / 2.
        let jmax = (acc / 2 - 1) / 2;
        let j = |k: usize| rel[k] * jmax / 1000;
        let items = vec![
            rc(span - 2000, centre + j(1), 0, acc),
            rc(span - 1000, centre + j(2), 0, acc),
            rc(span, centre + j(3), 0, acc),
        ];
        let base = rc(0, centre + j(0), 0, acc);
        prop_assert!(!closing(&items, Some(&base)));
    }

    /// The P1 confidence is always within 400..=800 when present.
    #[test]
    fn p1_confidence_range(
        acc in prop::array::uniform3(0_i64..70_000),
        n60 in 0_u8..20,
        dot in -ONE..=ONE,
        vf in prop::option::of(0_u32..40_000),
        mode in prop::sample::select(vec![Mode::Vehicle, Mode::Cycle, Mode::Foot]),
    ) {
        let r = spaced(acc);
        let b = base_of(&r);
        if let Some(c) = p1_confidence(&ev(&r, &b, n60, mode, Some(dot), vf)).unwrap() {
            prop_assert!((400..=800).contains(&c));
            if mode == Mode::Foot { prop_assert!(c <= 600); }
        }
    }
}
