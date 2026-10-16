//! Polylines: construction, nearest-segment projection, cross-track sign, along-route distance,
//! windows, ties, corridor membership, vector helpers (spec 4, 5.3, 6.2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::enu;
use proptest::prelude::*;
use proxim_geo::{
    Polyline, cos_tol_q30, dist_mm, dist_point_segment_mm, dot_q30, isqrt, normalize_q30,
    project_to_polyline, rem_along_mm, unit_from_course,
};
use proxim_types::{EngineError, Enu};

const ONE: i32 = 1 << 30;

/// East 1000 m, then north 1000 m (units are mm).
fn l_route() -> Polyline {
    Polyline::new(&[enu(0, 0), enu(1_000_000, 0), enu(1_000_000, 1_000_000)]).unwrap()
}

/// A hairpin: east 1 km, north 40 m, back west 1 km. The legs are 40 m apart.
fn hairpin() -> Polyline {
    Polyline::new(&[
        enu(0, 0),
        enu(1_000_000, 0),
        enu(1_000_000, 40_000),
        enu(0, 40_000),
    ])
    .unwrap()
}

#[test]
fn construction_lengths_and_tangents() {
    let pl = l_route();
    assert_eq!(pl.cum_mm, vec![0, 1_000_000, 2_000_000]);
    assert_eq!(pl.length_mm(), 2_000_000);
    assert_eq!(pl.segments(), 2);
    assert_eq!(pl.tangents_q30, vec![[ONE, 0], [0, ONE]]);
    // A 3-4-5 segment: length 5, tangent (0.6, 0.8) in Q30.
    let t = Polyline::new(&[enu(0, 0), enu(3000, 4000)]).unwrap();
    assert_eq!(t.cum_mm, vec![0, 5000]);
    assert_eq!(t.tangents_q30, vec![[644_245_094, 858_993_459]]);
    // Reverse direction flips the tangent.
    let r = Polyline::new(&[enu(3000, 4000), enu(0, 0)]).unwrap();
    assert_eq!(r.tangents_q30, vec![[-644_245_094, -858_993_459]]);
    // Segment lengths are floor(sqrt(len2)); the route length is exactly their sum.
    let d = Polyline::new(&[enu(0, 0), enu(1, 1), enu(3, 3)]).unwrap();
    assert_eq!(d.cum_mm, vec![0, 1, 3]); // floor(1.41) = 1, floor(2.83) = 2
}

#[test]
fn construction_rejects_bad_input() {
    assert_eq!(Polyline::new(&[]), Err(EngineError::DegenerateGeometry));
    assert_eq!(
        Polyline::new(&[enu(0, 0)]),
        Err(EngineError::DegenerateGeometry)
    );
    assert_eq!(
        Polyline::new(&[enu(0, 0), enu(0, 0)]),
        Err(EngineError::DegenerateGeometry)
    );
    assert_eq!(
        Polyline::new(&[enu(0, 0), enu(5, 5), enu(5, 5), enu(9, 9)]),
        Err(EngineError::DegenerateGeometry),
        "zero-length middle segment"
    );
    // A closed loop (last == first) is fine: only consecutive duplicates are rejected.
    assert!(Polyline::new(&[enu(0, 0), enu(10, 0), enu(10, 10), enu(0, 0)]).is_ok());
    let many: Vec<Enu> = (0..1025).map(|i| enu(i * 10, 0)).collect();
    assert_eq!(Polyline::new(&many), Err(EngineError::Limit));
    assert!(Polyline::new(&many[..1024]).is_ok());
    // Extreme but legal i32 coordinates do not overflow the length computation.
    let far = Polyline::new(&[enu(i32::MIN, i32::MIN), enu(i32::MAX, i32::MAX)]).unwrap();
    assert_eq!(far.length_mm(), 6_074_000_998);
}

#[test]
fn projection_signs_and_along() {
    let pl = l_route();
    // 30 m along the first leg, 5 m north of it = left of eastward travel: positive.
    let q = project_to_polyline(&pl, enu(30_000, 5_000), None).unwrap();
    assert_eq!(
        (q.seg, q.xt_mm, q.along_mm, q.dist_mm),
        (0, 5_000, 30_000, 5_000)
    );
    assert_eq!(q.tangent_q30, [ONE, 0]);
    // 5 m south = right: negative.
    let q = project_to_polyline(&pl, enu(30_000, -5_000), None).unwrap();
    assert_eq!((q.seg, q.xt_mm, q.along_mm), (0, -5_000, 30_000));
    // On the line: zero.
    let q = project_to_polyline(&pl, enu(30_000, 0), None).unwrap();
    assert_eq!((q.xt_mm, q.dist_mm), (0, 0));
    // Second leg runs north: left is west (negative east).
    let q = project_to_polyline(&pl, enu(1_000_000 - 7_000, 400_000), None).unwrap();
    assert_eq!((q.seg, q.xt_mm, q.along_mm), (1, 7_000, 1_400_000));
    assert_eq!(q.tangent_q30, [0, ONE]);
    let q = project_to_polyline(&pl, enu(1_000_000 + 7_000, 400_000), None).unwrap();
    assert_eq!((q.seg, q.xt_mm, q.along_mm), (1, -7_000, 1_400_000));
}

#[test]
fn projection_beyond_the_ends_is_clamped_and_distance_is_to_the_end() {
    let pl = l_route();
    // 1 km before the start, level with the route: along clamps to 0, distance 1 km (not 0).
    let q = project_to_polyline(&pl, enu(-1_000_000, 0), None).unwrap();
    assert_eq!((q.seg, q.along_mm, q.dist_mm), (0, 0, 1_000_000));
    assert_eq!(q.xt_mm.abs(), q.dist_mm);
    // 1 km past the end.
    let q = project_to_polyline(&pl, enu(1_000_000, 2_000_000), None).unwrap();
    assert_eq!((q.seg, q.along_mm, q.dist_mm), (1, 2_000_000, 1_000_000));
    // Corridor test as in 5.3: abs(xt) <= buffer. Past the end is NOT in a 30 m corridor.
    assert!(q.xt_mm.abs() > 30_000);
}

#[test]
fn nearest_segment_ties_go_to_the_lowest_index() {
    let pl = l_route();
    // Outside the corner, equidistant (to the vertex) from both segments' clamped ends.
    let q = project_to_polyline(&pl, enu(1_100_000, -100_000), None).unwrap();
    assert_eq!(q.seg, 0);
    assert_eq!(q.dist_mm, 141_421);
    assert_eq!(q.along_mm, 1_000_000);
    // A point exactly on the vertex belongs to segment 0 (distance 0 on both).
    let q = project_to_polyline(&pl, enu(1_000_000, 0), None).unwrap();
    assert_eq!((q.seg, q.along_mm, q.dist_mm), (0, 1_000_000, 0));
    // Inside the corner, on the bisector: perpendicular to both legs at equal distance 50 m.
    let q = project_to_polyline(&pl, enu(950_000, 50_000), None).unwrap();
    assert_eq!((q.seg, q.dist_mm), (0, 50_000));
}

#[test]
fn window_selects_the_leg_on_a_hairpin() {
    let pl = hairpin();
    // The point is 10 m from the return leg (along 1.04 km..2.04 km) and 30 m from the outbound leg.
    let p = enu(500_000, 30_000);
    let free = project_to_polyline(&pl, p, None).unwrap();
    assert_eq!((free.seg, free.dist_mm), (2, 10_000));
    assert_eq!(free.along_mm, 1_040_000 + 500_000);
    // With the window on the outbound leg the parallel leg is invisible (blocks confusion, 6.2).
    let w = project_to_polyline(&pl, p, Some((400_000, 700_000))).unwrap();
    assert_eq!((w.seg, w.dist_mm, w.along_mm), (0, 30_000, 500_000));
    assert_eq!(w.tangent_q30, [ONE, 0]);
    // Window ends are inclusive: ending at 1_039_999 hides the return leg (starts at 1_040_000),
    // ending at 1_040_000 admits it and it wins again.
    let w = project_to_polyline(&pl, p, Some((900_000, 1_039_999))).unwrap();
    assert_eq!((w.seg, w.dist_mm), (0, 30_000));
    let w = project_to_polyline(&pl, p, Some((900_000, 1_040_000))).unwrap();
    assert_eq!((w.seg, w.dist_mm), (2, 10_000));
}

#[test]
fn window_edges_and_errors() {
    let pl = l_route();
    // Inclusive at both ends: a window ending exactly at cum[1] still admits segment 1, and the
    // point lying on it (distance 0) beats segment 0 (distance 500 m).
    let q = project_to_polyline(&pl, enu(1_000_000, 500_000), Some((0, 1_000_000))).unwrap();
    assert_eq!(q.seg, 1);
    let q = project_to_polyline(&pl, enu(1_000_000, 500_000), Some((0, 999_999))).unwrap();
    assert_eq!(q.seg, 0, "window ends before segment 1 starts");
    let q =
        project_to_polyline(&pl, enu(1_000_000, 500_000), Some((1_000_001, 1_000_001))).unwrap();
    assert_eq!(q.seg, 1, "window start after cum[1] hides segment 0");
    // Inverted window and windows beyond the route are typed errors, not a wrong answer.
    assert_eq!(
        project_to_polyline(&pl, enu(0, 0), Some((10, 5))),
        Err(EngineError::DegenerateGeometry)
    );
    assert_eq!(
        project_to_polyline(&pl, enu(0, 0), Some((2_000_001, 3_000_000))),
        Err(EngineError::DegenerateGeometry)
    );
    assert_eq!(
        project_to_polyline(&pl, enu(0, 0), Some((-5_000_000, -1))),
        Err(EngineError::DegenerateGeometry)
    );
    // A window that merely touches the very end still works.
    assert!(project_to_polyline(&pl, enu(0, 0), Some((2_000_000, 9_000_000))).is_ok());
}

#[test]
fn self_crossing_loop_is_resolved_by_the_window() {
    // Figure-eight style crossing: east, north, west past start, south through the first leg.
    let pl = Polyline::new(&[
        enu(0, 0),
        enu(200_000, 0),
        enu(200_000, 100_000),
        enu(100_000, 100_000),
        enu(100_000, -100_000),
    ])
    .unwrap();
    let crossing = enu(100_000, 0); // lies on leg 0 and on the last leg
    let any = project_to_polyline(&pl, crossing, None).unwrap();
    assert_eq!((any.seg, any.dist_mm, any.along_mm), (0, 0, 100_000));
    // Later in the trip (window around along ~ 500 km) the same point maps to the last leg.
    let late = project_to_polyline(&pl, crossing, Some((500_000, 700_000))).unwrap();
    assert_eq!(late.seg, 3);
    assert_eq!(late.dist_mm, 0);
    assert_eq!(late.along_mm, 200_000 + 100_000 + 100_000 + 100_000);
}

#[test]
fn parallel_roads_inside_the_buffer_look_the_same() {
    // Documented false positive (spec 11): a parallel road D < buffer is indistinguishable by xt.
    let pl = Polyline::new(&[enu(0, 0), enu(5_000_000, 0)]).unwrap();
    let on_route = project_to_polyline(&pl, enu(1_000_000, 4_000), None).unwrap();
    let parallel = project_to_polyline(&pl, enu(1_000_000, 25_000), None).unwrap();
    let buffer = 30_000;
    assert!(on_route.xt_mm.abs() <= buffer && parallel.xt_mm.abs() <= buffer);
    // A parallel road beyond the buffer is excluded.
    let far = project_to_polyline(&pl, enu(1_000_000, 31_000), None).unwrap();
    assert!(far.xt_mm.abs() > buffer);
    // Opposite carriageway: same xt magnitude but opposite sign, tangent says travel direction.
    let opposite = project_to_polyline(&pl, enu(1_000_000, -8_000), None).unwrap();
    assert_eq!(opposite.xt_mm, -8_000);
}

#[test]
fn rem_along_signs() {
    assert_eq!(rem_along_mm(1_200_000, 200_000), Ok(1_000_000));
    assert_eq!(rem_along_mm(1_200_000, 1_200_000), Ok(0));
    assert_eq!(
        rem_along_mm(1_200_000, 1_500_000),
        Ok(-300_000),
        "negative once passed"
    );
    assert_eq!(rem_along_mm(i64::MIN, 1), Err(EngineError::Overflow));
    assert_eq!(rem_along_mm(i64::MAX, -1), Err(EngineError::Overflow));
}

#[test]
fn normalize_known_values_and_zero() {
    assert_eq!(normalize_q30(3, 4), Some([644_245_094, 858_993_459]));
    assert_eq!(normalize_q30(-3, 4), Some([-644_245_094, 858_993_459]));
    assert_eq!(normalize_q30(5, 0), Some([ONE, 0]));
    assert_eq!(normalize_q30(0, -7), Some([0, -ONE]));
    assert_eq!(normalize_q30(0, 0), None);
    // Full-range inputs: the squared sum is 2^127, still exact.
    let u = normalize_q30(i64::MAX, i64::MAX).unwrap();
    assert!((i64::from(u[0]) - 759_250_125).abs() <= 2 && u[0] == u[1]);
    let u = normalize_q30(i64::MIN, i64::MIN).unwrap();
    assert!((i64::from(u[0]) + 759_250_125).abs() <= 2 && u[0] == u[1]);
    assert_eq!(normalize_q30(i64::MIN, 0), Some([-ONE, 0]));
    // Floor-sqrt precision: a 1 mm diagonal is not a unit vector (documented, harmless at >= 15 m).
    assert_eq!(normalize_q30(1, 1), Some([ONE, ONE]));
}

#[test]
fn dot_q30_known_values() {
    let e = [ONE, 0];
    let n = [0, ONE];
    assert_eq!(dot_q30(e, e), i64::from(ONE));
    assert_eq!(dot_q30(e, n), 0);
    assert_eq!(dot_q30(e, [-ONE, 0]), -i64::from(ONE));
    // Heading 40 degrees off the tangent passes a 45 degree gate and fails a 30 degree gate.
    let t = unit_from_course(900).unwrap(); // due east
    let h40 = unit_from_course(900 - 400).unwrap();
    let d = dot_q30(h40, t);
    assert!(d >= i64::from(cos_tol_q30(450).unwrap()));
    assert!(d < i64::from(cos_tol_q30(300).unwrap()));
    // Exactly at the gate: 45 degrees off gives cos 45 within rounding of the table value.
    let h45 = unit_from_course(450).unwrap();
    assert!((dot_q30(h45, t) - i64::from(cos_tol_q30(450).unwrap())).abs() <= 1);
    // Rounding: half rounds up (rsr), it is not a floor. 2^29 / 2^30 = 0.5 -> 1; just below -> 0;
    // -0.5 -> 0 (half up); just below -0.5 -> -1.
    assert_eq!(dot_q30([1, 0], [1 << 29, 0]), 1);
    assert_eq!(dot_q30([1, 0], [(1 << 29) - 1, 0]), 0);
    assert_eq!(dot_q30([1, 0], [-(1 << 29), 0]), 0);
    assert_eq!(dot_q30([1, 0], [-(1 << 29) - 1, 0]), -1);
    assert_eq!(dot_q30([3, 3], [1 << 29, 1 << 29]), 3);
    // Extreme i32 inputs saturate to a defined value rather than panic.
    let m = dot_q30([i32::MIN, i32::MIN], [i32::MIN, i32::MIN]);
    assert_eq!(m, 1_i64 << 33);
}

#[test]
fn dot_matches_cos_of_course_difference() {
    // Table self-consistency: dot(unit(a), unit(b)) ~ cos_tol(|a-b|) to a few LSB, for all pairs on
    // a grid including wrap-around differences.
    for a in (0..3600_u16).step_by(7) {
        for delta in (0..=900_u16).step_by(11) {
            let b = (a + delta) % 3600;
            let got = dot_q30(unit_from_course(a).unwrap(), unit_from_course(b).unwrap());
            let want = i64::from(cos_tol_q30(delta).unwrap());
            assert!(
                (got - want).abs() <= 3,
                "a {a} delta {delta}: {got} vs {want}"
            );
        }
    }
}

// ---- properties ---------------------------------------------------------------------------

fn coord() -> impl Strategy<Value = i32> {
    -100_000_i32..100_000
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(800))]

    /// Exactness: the reported floor distance r satisfies r^2 <= d^2 < (r+1)^2 for the true
    /// (rational) squared distance, for random segments including degenerate-short ones.
    #[test]
    fn point_segment_distance_is_exact_floor(
        px in coord(), py in coord(), ax in coord(), ay in coord(), bx in coord(), by in coord(),
    ) {
        prop_assume!((ax, ay) != (bx, by));
        let (p, a, b) = (enu(px, py), enu(ax, ay), enu(bx, by));
        let r = i128::from(dist_point_segment_mm(p, a, b).unwrap());
        let (abx, aby) = (i128::from(bx - ax), i128::from(by - ay));
        let (apx, apy) = (i128::from(px - ax), i128::from(py - ay));
        let len2 = abx * abx + aby * aby;
        let dot = abx * apx + aby * apy;
        // d2 as a fraction num/len2.
        let (num, den) = if dot <= 0 {
            (apx * apx + apy * apy, 1)
        } else if dot >= len2 {
            let (bpx, bpy) = (i128::from(px - bx), i128::from(py - by));
            (bpx * bpx + bpy * bpy, 1)
        } else {
            let cross = abx * apy - aby * apx;
            (cross * cross, len2)
        };
        prop_assert!(r * r * den <= num, "r^2 too big: {r}");
        prop_assert!((r + 1) * (r + 1) * den > num, "r^2 too small: {r}");
    }

    /// Cross-track distance is symmetric w.r.t. segment direction (exactly), and so is the
    /// projection of a whole polyline when it is reversed: same distance, flipped sign, mirrored
    /// along (within 1 mm of floor rounding per segment).
    #[test]
    fn distance_is_symmetric_under_segment_reversal(
        px in coord(), py in coord(), ax in coord(), ay in coord(), bx in coord(), by in coord(),
    ) {
        let (p, a, b) = (enu(px, py), enu(ax, ay), enu(bx, by));
        prop_assert_eq!(
            dist_point_segment_mm(p, a, b).unwrap(),
            dist_point_segment_mm(p, b, a).unwrap()
        );
    }

    #[test]
    fn polyline_reversal_flips_sign_and_mirrors_along(
        pts in prop::collection::vec((coord(), coord()), 2..8),
        q in (coord(), coord()),
    ) {
        let mut v: Vec<Enu> = pts.iter().map(|&(e, n)| enu(e, n)).collect();
        v.dedup();
        prop_assume!(v.len() >= 2);
        let fwd = Polyline::new(&v).unwrap();
        let mut rv = v.clone();
        rv.reverse();
        let bwd = Polyline::new(&rv).unwrap();
        let p = enu(q.0, q.1);
        let f = project_to_polyline(&fwd, p, None).unwrap();
        let b = project_to_polyline(&bwd, p, None).unwrap();
        prop_assert_eq!(f.dist_mm, b.dist_mm, "distance is direction independent");
        prop_assert_eq!(f.xt_mm.abs(), b.xt_mm.abs());
        // Segment-wise floor lengths differ by < 1 mm per segment between directions? No: lengths
        // are identical (same len2), so along mirrors up to the floor in the interpolation.
        let n_segs = i64::try_from(v.len() - 1).unwrap();
        let mirrored = fwd.length_mm() - f.along_mm;
        prop_assert!((mirrored - b.along_mm).abs() <= n_segs.max(2), "{mirrored} vs {}", b.along_mm);
        // Sign flips whenever the point is strictly off the line of the chosen segment and both
        // searches picked the same physical segment.
        if usize::try_from(f.seg).unwrap() + usize::try_from(b.seg).unwrap() == v.len() - 2
            && f.xt_mm != 0 && b.xt_mm != 0
        {
            prop_assert_eq!(f.xt_mm, -b.xt_mm);
        }
    }

    /// along is within [0, length], dist matches the explicit nearest point, and the result does not
    /// depend on the window when the window contains the whole route.
    #[test]
    fn projection_invariants(
        pts in prop::collection::vec((coord(), coord()), 2..10),
        q in (coord(), coord()),
    ) {
        let mut v: Vec<Enu> = pts.iter().map(|&(e, n)| enu(e, n)).collect();
        v.dedup();
        prop_assume!(v.len() >= 2);
        let pl = Polyline::new(&v).unwrap();
        let p = enu(q.0, q.1);
        let r = project_to_polyline(&pl, p, None).unwrap();
        prop_assert!(r.along_mm >= 0 && r.along_mm <= pl.length_mm());
        prop_assert_eq!(r.xt_mm.abs(), r.dist_mm);
        // Brute force minimum over segments equals the reported distance.
        let best = v.windows(2).map(|w| dist_point_segment_mm(p, w[0], w[1]).unwrap()).min().unwrap();
        prop_assert_eq!(r.dist_mm, best);
        // Nearest vertex distance is an upper bound.
        for vtx in &v {
            prop_assert!(r.dist_mm <= dist_mm(*vtx, p).unwrap());
        }
        let wide = project_to_polyline(&pl, p, Some((i64::MIN, i64::MAX))).unwrap();
        prop_assert_eq!(wide, r);
        // The along position lies inside the chosen segment's span.
        let s = usize::try_from(r.seg).unwrap();
        prop_assert!(r.along_mm >= pl.cum_mm[s] && r.along_mm <= pl.cum_mm[s + 1]);
    }

    /// Windowed projection only ever returns a segment overlapping the window.
    #[test]
    fn window_is_respected(
        pts in prop::collection::vec((coord(), coord()), 3..10),
        q in (coord(), coord()),
        w0 in 0_i64..400_000, wlen in 0_i64..300_000,
    ) {
        let mut v: Vec<Enu> = pts.iter().map(|&(e, n)| enu(e, n)).collect();
        v.dedup();
        prop_assume!(v.len() >= 3);
        let pl = Polyline::new(&v).unwrap();
        let (lo, hi) = (w0, w0 + wlen);
        match project_to_polyline(&pl, enu(q.0, q.1), Some((lo, hi))) {
            Ok(r) => {
                let s = usize::try_from(r.seg).unwrap();
                prop_assert!(pl.cum_mm[s] <= hi && pl.cum_mm[s + 1] >= lo);
            }
            Err(e) => {
                prop_assert_eq!(e, EngineError::DegenerateGeometry);
                // Then really no segment overlaps.
                prop_assert!(pl.cum_mm.windows(2).all(|c| c[0] > hi || c[1] < lo));
            }
        }
    }

    /// normalize_q30 is a unit vector to within 2/n relative error (floor sqrt) and keeps direction.
    #[test]
    fn normalize_is_unit_within_floor_error(x in -50_000_000_i64..50_000_000, y in -50_000_000_i64..50_000_000) {
        prop_assume!(x.abs().max(y.abs()) >= 1000);
        let u = normalize_q30(x, y).unwrap();
        let len2 = i128::from(u[0]) * i128::from(u[0]) + i128::from(u[1]) * i128::from(u[1]);
        let one2 = 1_i128 << 60;
        let n = i128::from(isqrt(u128::try_from(i128::from(x) * i128::from(x) + i128::from(y) * i128::from(y)).unwrap()));
        // relative error of the length is < 1/n  =>  |len2 - 2^60| <= 2^60 * 2.2 / n + rounding.
        prop_assert!((len2 - one2).abs() <= one2 * 22 / (10 * n) + (1 << 32), "x {x} y {y}");
        prop_assert_eq!(u[0].signum(), i32::try_from(x.signum()).unwrap());
        prop_assert_eq!(u[1].signum(), i32::try_from(y.signum()).unwrap());
    }
}
