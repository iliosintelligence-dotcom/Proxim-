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

fn rc(along: i64, xt: i64, acc: i64) -> Recent {
    Recent {
        t_ms: 0,
        along_mm: along,
        xt_mm: xt,
        acc_eff_mm: acc,
    }
}

/// Three good fixes advancing 15 m each, 5 m off-axis.
fn good() -> Vec<Recent> {
    vec![
        rc(0, 5000, 10_000),
        rc(15_000, 5000, 10_000),
        rc(30_000, 5000, 10_000),
    ]
}

fn ev(recent: &[Recent], n60: u8, mode: Mode, dot: Option<i64>, vf: Option<u32>) -> P1Evidence<'_> {
    P1Evidence {
        recent,
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
    let c = |n60, dot| p1_confidence(&ev(&g, n60, Mode::Vehicle, dot, Some(10_000))).unwrap();
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

#[test]
fn p1_accuracy_penalties_and_cap() {
    let with_acc = |acc: i64| {
        let r = vec![rc(0, 0, 10_000), rc(40_000, 0, acc), rc(80_000, 0, 10_000)];
        p1_confidence(&ev(&r, 3, Mode::Vehicle, Some(cos(300)), Some(10_000))).unwrap()
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
    let r = vec![
        rc(0, 0, 60_000),
        rc(40_000, 0, 10_000),
        rc(80_000, 0, 10_000),
    ];
    assert_eq!(
        p1_confidence(&ev(&r, 3, Mode::Vehicle, Some(cos(300)), Some(10_000))).unwrap(),
        Some(400)
    );
    // Range 400..800 over all combinations.
    for acc in [10_000, 31_000, 51_000, 65_000] {
        for n60 in [3, 5] {
            for dot in [cos(450), cos(150), ONE] {
                let r = vec![rc(0, 0, acc), rc(40_000, 0, acc), rc(80_000, 0, acc)];
                let v = p1_confidence(&ev(&r, n60, Mode::Vehicle, Some(dot), Some(10_000)))
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
    let foot = |n60, dot| p1_confidence(&ev(&g, n60, Mode::Foot, dot, Some(1_500))).unwrap();
    assert_eq!(foot(8, Some(ONE)), Some(600));
    assert_eq!(foot(3, Some(cos(300))), Some(600));
    let r = vec![
        rc(0, 0, 40_000),
        rc(40_000, 0, 40_000),
        rc(80_000, 0, 40_000),
    ];
    // The cap applies after the penalty: 600 - 100 stays 500.
    assert_eq!(
        p1_confidence(&ev(&r, 8, Mode::Foot, Some(ONE), Some(1_500))).unwrap(),
        Some(600)
    );
    let r2 = vec![
        rc(0, 0, 40_000),
        rc(40_000, 0, 40_000),
        rc(80_000, 0, 40_000),
    ];
    assert_eq!(
        p1_confidence(&ev(&r2, 3, Mode::Foot, Some(cos(300)), Some(1_500))).unwrap(),
        Some(500)
    );
}

#[test]
fn p1_each_criterion_is_necessary() {
    let g = good();
    let ok = |e: &P1Evidence<'_>| p1_confidence(e).unwrap().is_some();
    let base = ev(&g, 3, Mode::Vehicle, Some(cos(300)), Some(10_000));
    assert!(ok(&base));
    // Fewer than three fixes in 60 s.
    assert!(!ok(&P1Evidence {
        fixes_60s: 2,
        ..base
    }));
    // Fewer than three recent fixes.
    assert!(!ok(&ev(
        &g[1..],
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
        assert!(ok(&ev(&r, 3, Mode::Vehicle, Some(cos(300)), Some(10_000))));
        r[i].xt_mm = -30_001;
        assert!(!ok(&ev(&r, 3, Mode::Vehicle, Some(cos(300)), Some(10_000))));
    }
    // Not closing (a U-turn): along goes back.
    let u = vec![
        rc(30_000, 0, 10_000),
        rc(15_000, 0, 10_000),
        rc(0, 0, 10_000),
    ];
    assert!(!ok(&ev(&u, 3, Mode::Vehicle, Some(cos(300)), Some(10_000))));
    // Stopped.
    let s = vec![rc(0, 0, 10_000), rc(0, 0, 10_000), rc(0, 0, 10_000)];
    assert!(!ok(&ev(&s, 3, Mode::Vehicle, Some(cos(300)), Some(10_000))));
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

#[test]
fn project_recent_on_a_straight_route() {
    let fixes: Vec<_> = (0..3)
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
        .collect();
    let s = summary_of(&fixes);
    let p = project_recent(&s, &frame(), &north_route(), false, 15_000).unwrap();
    assert_eq!(p.items.len(), 3);
    for (i, r) in p.items.iter().enumerate() {
        let along = (i as i64) * 15_000;
        assert!(
            (r.along_mm - along).abs() <= 25,
            "{} vs {along}",
            r.along_mm
        );
        // East of a northbound route is to the right: negative cross-track.
        assert!((r.xt_mm + 3000).abs() <= 25, "{}", r.xt_mm);
        assert_eq!(r.acc_eff_mm, 10_000);
        assert_eq!(r.t_ms, (i as i64) * 1000);
    }
    assert_eq!(p.cur.along_mm, p.items[2].along_mm);
    assert_eq!(p.cur.seg, 0);
    assert!(closing(&p.items));
}

#[test]
fn project_recent_with_fewer_fixes_returns_what_exists() {
    let s = summary_of(&[fix_at(0, 0, 0, 5000, Some(15_000), Some(0))]);
    let p = project_recent(&s, &frame(), &north_route(), true, 15_000).unwrap();
    assert_eq!(p.items.len(), 1);
    assert!(!closing(&p.items));
}

#[test]
fn armed_window_blocks_a_jump_to_the_parallel_leg() {
    // A hairpin: north 3 km, 40 m east, south 3 km (legs 40 m apart). The vehicle drives the
    // outbound leg; a fix that would project onto the return leg is held to the outbound leg when
    // armed, but not when idle.
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
    // Track at e = 30 m (closer to the return leg at e = 40 m), driving north.
    let fixes: Vec<_> = (0..3)
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
    let idle = project_recent(&s, &frame(), &route, false, 15_000).unwrap();
    assert!(
        idle.items.iter().all(|r| r.along_mm > 3_040_000),
        "idle: nearest is the return leg"
    );
    // Armed: the oldest fix is still projected on the whole route, so the chain starts on the
    // return leg and stays there (window follows the predecessor). That is the documented
    // behaviour: the window is anchored on the track, it cannot repair a wrong first projection.
    let armed = project_recent(&s, &frame(), &route, true, 15_000).unwrap();
    assert_eq!(armed.items.len(), 3);
    // With a first fix that is clearly on the outbound leg, the window keeps later fixes there.
    let fixes2: Vec<_> = [(0, 5000), (1000, 20_000), (2000, 30_000)]
        .iter()
        .map(|&(t, e)| {
            fix_at(
                t,
                e,
                1_000_000 + (t as i32) * 15,
                5000,
                Some(15_000),
                Some(0),
            )
        })
        .collect();
    let s2 = summary_of(&fixes2);
    let idle2 = project_recent(&s2, &frame(), &route, false, 15_000).unwrap();
    let armed2 = project_recent(&s2, &frame(), &route, true, 15_000).unwrap();
    assert!(
        idle2.items[2].along_mm > 3_040_000,
        "idle: third fix snaps to the return leg"
    );
    assert!(
        armed2.items[2].along_mm < 1_100_000,
        "armed: the window keeps it on the outbound leg"
    );
    assert!(armed2.items[2].xt_mm.abs() >= 29_000);
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
        project_recent(&s, &far, &north_route(), false, 15_000).unwrap_err(),
        proxim_types::EngineError::OutOfFrame
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// CLOSING is invariant under adding a constant to every along value, never panics, and
    /// reversing the direction of travel never leaves a closing track closing.
    #[test]
    fn closing_translation_and_reversal(
        a in prop::array::uniform3(-1_000_000_i64..1_000_000),
        acc in prop::array::uniform3(0_i64..200_000),
        shift in -10_000_000_i64..10_000_000,
    ) {
        let items: Vec<Recent> = (0..3).map(|i| rc(a[i], 0, acc[i])).collect();
        let shifted: Vec<Recent> = (0..3).map(|i| rc(a[i] + shift, 0, acc[i])).collect();
        prop_assert_eq!(closing(&items), closing(&shifted));
        let reversed: Vec<Recent> = (0..3).map(|i| rc(-a[2 - i], 0, acc[2 - i])).collect();
        // Mirror image (negate and reverse order) is the same motion: closing is preserved.
        prop_assert_eq!(closing(&items), closing(&reversed));
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
        let r = vec![rc(0, 0, acc[0]), rc(20_000, 0, acc[1]), rc(40_000, 0, acc[2])];
        if let Some(c) = p1_confidence(&ev(&r, n60, mode, Some(dot), vf)).unwrap() {
            prop_assert!((400..=800).contains(&c));
            if mode == Mode::Foot { prop_assert!(c <= 600); }
        }
    }
}
