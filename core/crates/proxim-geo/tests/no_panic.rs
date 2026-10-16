//! Totality: every public function returns a value or a typed error for the whole input domain,
//! including poles, the antimeridian, `i32`/`i64` limits and hostile geometry. A panic or an
//! arithmetic overflow here (the test profile has overflow checks on) fails the test.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::enu;
use proptest::prelude::*;
use proxim_geo::*;
use proxim_types::{EngineError, Enu, Position};

fn any_enu() -> impl Strategy<Value = Enu> {
    prop_oneof![
        3 => (any::<i32>(), any::<i32>()).prop_map(|(e, n)| enu(e, n)),
        1 => (prop::sample::select(vec![i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX]),
              prop::sample::select(vec![i32::MIN, -1, 0, 1, i32::MAX]))
            .prop_map(|(e, n)| enu(e, n)),
        2 => (-200_000_000_i32..200_000_000, -200_000_000_i32..200_000_000).prop_map(|(e, n)| enu(e, n)),
    ]
}

fn ok_or_typed<T>(r: Result<T, EngineError>) -> bool {
    matches!(
        r,
        Ok(_)
            | Err(EngineError::Overflow
                | EngineError::OutOfFrame
                | EngineError::UnsupportedLatitude
                | EngineError::DegenerateGeometry
                | EngineError::Limit
                | EngineError::Parse)
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1500))]

    #[test]
    fn primitives_never_panic(a in any::<i64>(), b in any::<i64>(), c in any::<i64>(), d in any::<i64>(),
                              n in any::<u128>(), s in any::<u32>(), x in any::<i128>()) {
        prop_assert!(ok_or_typed(div_round(a, b)));
        prop_assert!(ok_or_typed(rsr(x, s)));
        prop_assert!(ok_or_typed(orient2d([a, b], [c, d], [b, a])));
        prop_assert!(ok_or_typed(orient2d([i64::MIN, a], [i64::MAX, b], [c, i64::MAX])));
        let r = isqrt(n);
        let r2 = u128::from(r) * u128::from(r);
        prop_assert!(r2 <= n);
        if let Some(next) = (u128::from(r) + 1).checked_mul(u128::from(r) + 1) {
            prop_assert!(next > n);
        }
        let _ = normalize_q30(a, b);
        let _ = rem_along_mm(a, b);
    }

    #[test]
    fn dot_and_vectors_never_panic(a in any::<[i32; 2]>(), b in any::<[i32; 2]>(), course in any::<u16>(), tol in any::<u16>()) {
        let _ = dot_q30(a, b);
        prop_assert!(ok_or_typed(unit_from_course(course)));
        prop_assert!(ok_or_typed(cos_tol_q30(tol)));
        if course < 3600 {
            prop_assert!(unit_from_course(course).is_ok());
        } else {
            prop_assert_eq!(unit_from_course(course), Err(EngineError::Parse));
        }
    }

    #[test]
    fn shapes_never_panic_on_extreme_enu(
        poly in prop::collection::vec(any_enu(), 0..9), p in any_enu(), radius in any::<i64>(),
        acc in any::<i64>(), depth in any::<i64>(), k in any::<u32>(), a32 in any::<u32>(),
    ) {
        prop_assert!(ok_or_typed(point_in_polygon(&poly, p)));
        prop_assert!(ok_or_typed(signed_depth_polygon_mm(&poly, p)));
        prop_assert!(ok_or_typed(signed_depth_circle_mm(p, radius, enu(0, 0))));
        prop_assert!(ok_or_typed(dist_mm(p, enu(i32::MIN, i32::MAX))));
        prop_assert!(ok_or_typed(classify(depth, acc, k)));
        prop_assert!(ok_or_typed(acc_eff_mm(a32, k)));
        let q = poly.first().copied().unwrap_or(p);
        prop_assert!(ok_or_typed(dist_point_segment_mm(p, q, poly.last().copied().unwrap_or(q))));
    }

    #[test]
    fn polylines_never_panic_on_extreme_enu(
        v in prop::collection::vec(any_enu(), 0..9), p in any_enu(),
        w0 in any::<i64>(), w1 in any::<i64>(), use_window in any::<bool>(),
    ) {
        match Polyline::new(&v) {
            Ok(pl) => {
                let w = if use_window { Some((w0, w1)) } else { None };
                match project_to_polyline(&pl, p, w) {
                    Ok(r) => {
                        prop_assert!(r.along_mm >= 0 && r.along_mm <= pl.length_mm());
                        prop_assert_eq!(r.xt_mm.abs(), r.dist_mm);
                    }
                    Err(e) => prop_assert!(ok_or_typed::<()>(Err(e))),
                }
            }
            Err(e) => prop_assert!(ok_or_typed::<()>(Err(e))),
        }
    }

    #[test]
    fn frames_never_panic(olat in any::<i32>(), olon in any::<i32>(), plat in any::<i32>(), plon in any::<i32>(),
                          e in any::<i32>(), n in any::<i32>()) {
        let o = Position { lat_e7: olat, lon_e7: olon };
        let p = Position { lat_e7: plat, lon_e7: plon };
        prop_assert!(ok_or_typed(frame_for(o)));
        if let Ok(f) = frame_for(o) {
            prop_assert!(ok_or_typed(f.project(p)));
            prop_assert!(ok_or_typed(f.unproject(enu(e, n))));
            prop_assert!(ok_or_typed(f.project_all(&[p, o])));
        }
    }
}

#[test]
fn named_extremes() {
    // The corner cases a fuzzer finds first, spelled out.
    let big = [
        enu(i32::MIN, i32::MIN),
        enu(i32::MAX, i32::MIN),
        enu(i32::MAX, i32::MAX),
        enu(i32::MIN, i32::MAX),
    ];
    for p in [
        enu(0, 0),
        enu(i32::MIN, i32::MIN),
        enu(i32::MAX, i32::MAX),
        enu(i32::MAX, i32::MIN),
    ] {
        assert!(ok_or_typed(point_in_polygon(&big, p)));
        assert!(ok_or_typed(signed_depth_polygon_mm(&big, p)));
    }
    assert_eq!(point_in_polygon(&big, enu(0, 0)), Ok(PipResult::Inside));
    assert_eq!(
        point_in_polygon(&big, enu(i32::MIN, 0)),
        Ok(PipResult::Boundary)
    );
    // Perpendicular distance squared of the extreme span exceeds u128: typed error, not wrap.
    let far = dist_point_segment_mm(
        enu(0, i32::MAX),
        enu(i32::MIN, i32::MIN),
        enu(i32::MAX, i32::MIN),
    );
    assert!(ok_or_typed(far));
    let pl = Polyline::new(&[enu(i32::MIN, 0), enu(i32::MAX, 0)]).unwrap();
    assert!(ok_or_typed(project_to_polyline(
        &pl,
        enu(0, i32::MAX),
        None
    )));
    assert_eq!(rem_along_mm(i64::MIN, i64::MAX), Err(EngineError::Overflow));
    assert_eq!(div_round(i64::MIN, 1), Ok(i64::MIN));
    assert_eq!(
        normalize_q30(i64::MIN, i64::MIN).map(|u| u[0] == u[1]),
        Some(true)
    );
}
