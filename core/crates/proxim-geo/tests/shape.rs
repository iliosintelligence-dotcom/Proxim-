//! Point-in-polygon, signed depth, circle depth, accuracy classification (spec 1.4, 4, 11).
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
    Depth, PipResult, acc_eff_mm, classify, dist_mm, isqrt, orient2d, point_in_polygon,
    signed_depth_circle_mm, signed_depth_polygon_mm,
};
use proxim_types::{EngineError, Enu};

const SQ: [(i32, i32); 4] = [(0, 0), (100_000, 0), (100_000, 100_000), (0, 100_000)];

fn square() -> Vec<Enu> {
    SQ.iter().map(|&(e, n)| enu(e, n)).collect()
}

#[test]
fn square_inside_outside_boundary() {
    let sq = square();
    let pip = |e, n| point_in_polygon(&sq, enu(e, n)).unwrap();
    assert_eq!(pip(50_000, 50_000), PipResult::Inside);
    assert_eq!(pip(1, 1), PipResult::Inside);
    assert_eq!(pip(99_999, 99_999), PipResult::Inside);
    assert_eq!(pip(-1, 50_000), PipResult::Outside);
    assert_eq!(pip(100_001, 50_000), PipResult::Outside);
    assert_eq!(pip(50_000, -1), PipResult::Outside);
    assert_eq!(pip(50_000, 100_001), PipResult::Outside);
    // Every vertex and edge midpoint is Boundary.
    for &(e, n) in &SQ {
        assert_eq!(pip(e, n), PipResult::Boundary, "vertex {e},{n}");
    }
    for (e, n) in [
        (50_000, 0),
        (100_000, 50_000),
        (50_000, 100_000),
        (0, 50_000),
    ] {
        assert_eq!(pip(e, n), PipResult::Boundary, "edge {e},{n}");
    }
}

#[test]
fn winding_direction_does_not_matter() {
    let mut cw = square();
    cw.reverse();
    for (e, n, want) in [
        (50_000, 50_000, PipResult::Inside),
        (-5, 5, PipResult::Outside),
        (0, 0, PipResult::Boundary),
        (100_000, 70_000, PipResult::Boundary),
    ] {
        assert_eq!(point_in_polygon(&cw, enu(e, n)).unwrap(), want);
    }
}

#[test]
fn ray_through_a_vertex_is_counted_once() {
    // Diamond: the horizontal ray from a point level with the left/right vertices passes exactly
    // through vertices; the half-open rule must still give the right answer.
    let d = [enu(0, -10), enu(10, 0), enu(0, 10), enu(-10, 0)];
    assert_eq!(point_in_polygon(&d, enu(0, 0)).unwrap(), PipResult::Inside);
    assert_eq!(point_in_polygon(&d, enu(-5, 0)).unwrap(), PipResult::Inside);
    assert_eq!(
        point_in_polygon(&d, enu(-11, 0)).unwrap(),
        PipResult::Outside
    );
    assert_eq!(
        point_in_polygon(&d, enu(11, 0)).unwrap(),
        PipResult::Outside
    );
    assert_eq!(
        point_in_polygon(&d, enu(-10, 0)).unwrap(),
        PipResult::Boundary
    );
    // Level with the top and bottom vertices too.
    assert_eq!(
        point_in_polygon(&d, enu(-1, 10)).unwrap(),
        PipResult::Outside
    );
    assert_eq!(
        point_in_polygon(&d, enu(1, -10)).unwrap(),
        PipResult::Outside
    );
    assert_eq!(
        point_in_polygon(&d, enu(0, 10)).unwrap(),
        PipResult::Boundary
    );
}

#[test]
fn horizontal_edges_and_collinear_vertices() {
    // A square with an extra collinear vertex on the bottom edge and a repeated vertex.
    let poly = [
        enu(0, 0),
        enu(50_000, 0),
        enu(100_000, 0),
        enu(100_000, 100_000),
        enu(100_000, 100_000),
        enu(0, 100_000),
    ];
    assert_eq!(
        point_in_polygon(&poly, enu(50_000, 0)).unwrap(),
        PipResult::Boundary
    );
    assert_eq!(
        point_in_polygon(&poly, enu(70_000, 0)).unwrap(),
        PipResult::Boundary
    );
    assert_eq!(
        point_in_polygon(&poly, enu(50_000, 1)).unwrap(),
        PipResult::Inside
    );
    assert_eq!(
        point_in_polygon(&poly, enu(150_000, 0)).unwrap(),
        PipResult::Outside
    );
    assert_eq!(
        point_in_polygon(&poly, enu(-1, 0)).unwrap(),
        PipResult::Outside
    );
}

#[test]
fn concave_u_shape() {
    // U opening upward: notch between e = 40_000..60_000 above n = 30_000.
    let u = [
        enu(0, 0),
        enu(100_000, 0),
        enu(100_000, 100_000),
        enu(60_000, 100_000),
        enu(60_000, 30_000),
        enu(40_000, 30_000),
        enu(40_000, 100_000),
        enu(0, 100_000),
    ];
    assert_eq!(
        point_in_polygon(&u, enu(20_000, 80_000)).unwrap(),
        PipResult::Inside
    );
    assert_eq!(
        point_in_polygon(&u, enu(80_000, 80_000)).unwrap(),
        PipResult::Inside
    );
    assert_eq!(
        point_in_polygon(&u, enu(50_000, 15_000)).unwrap(),
        PipResult::Inside
    );
    assert_eq!(
        point_in_polygon(&u, enu(50_000, 80_000)).unwrap(),
        PipResult::Outside,
        "in the notch"
    );
    assert_eq!(
        point_in_polygon(&u, enu(50_000, 30_000)).unwrap(),
        PipResult::Boundary
    );
    assert_eq!(
        point_in_polygon(&u, enu(40_000, 60_000)).unwrap(),
        PipResult::Boundary
    );
    // Depth in the notch is negative and measured to the nearest wall (10 m each side).
    assert_eq!(
        signed_depth_polygon_mm(&u, enu(50_000, 80_000)).unwrap(),
        -10_000
    );
}

#[test]
fn polygon_input_validation() {
    assert_eq!(
        point_in_polygon(&[], enu(0, 0)),
        Err(EngineError::DegenerateGeometry)
    );
    assert_eq!(
        point_in_polygon(&[enu(0, 0), enu(1, 1)], enu(0, 0)),
        Err(EngineError::DegenerateGeometry)
    );
    let big: Vec<Enu> = (0..257).map(|i| enu(i, i * i % 1000)).collect();
    assert_eq!(point_in_polygon(&big, enu(0, 0)), Err(EngineError::Limit));
    assert_eq!(
        signed_depth_polygon_mm(&big, enu(0, 0)),
        Err(EngineError::Limit)
    );
    let ok: Vec<Enu> = (0..256).map(|i| enu(i, 0)).collect();
    assert!(point_in_polygon(&ok, enu(0, 5)).is_ok());
}

#[test]
fn polygon_depth_values() {
    let sq = square();
    let depth = |e, n| signed_depth_polygon_mm(&sq, enu(e, n)).unwrap();
    assert_eq!(depth(50_000, 50_000), 50_000);
    assert_eq!(depth(10_000, 50_000), 10_000);
    assert_eq!(depth(50_000, 99_000), 1_000);
    assert_eq!(depth(0, 50_000), 0);
    assert_eq!(depth(0, 0), 0);
    assert_eq!(depth(-10_000, 50_000), -10_000);
    // Diagonal outside a corner: sqrt(3000^2 + 4000^2) = 5000.
    assert_eq!(depth(-3_000, -4_000), -5_000);
    assert_eq!(depth(103_000, 104_000), -5_000);
    // Floor, not round: sqrt(1 + 1) = 1.41 -> 1.
    assert_eq!(depth(-1, -1), -1);
    assert_eq!(depth(-2, -2), -2);
}

#[test]
fn circle_depth_and_the_150m_example_from_the_spec() {
    let c = enu(0, 0);
    let r = 150_000;
    assert_eq!(signed_depth_circle_mm(c, r, enu(0, 0)).unwrap(), 150_000);
    assert_eq!(signed_depth_circle_mm(c, r, enu(150_000, 0)).unwrap(), 0);
    assert_eq!(
        signed_depth_circle_mm(c, r, enu(0, -160_000)).unwrap(),
        -10_000
    );
    assert_eq!(
        signed_depth_circle_mm(c, r, enu(90_000, 120_000)).unwrap(),
        0
    );
    assert_eq!(
        signed_depth_circle_mm(c, -1, enu(0, 0)),
        Err(EngineError::DegenerateGeometry)
    );
    // Spec 11: radius 150 m, acc_eff 10 m: ENTER (Inside) at <= 140 m, EXIT (Outside) at >= 160 m.
    let acc = acc_eff_mm(0, 1000).unwrap();
    assert_eq!(acc, 10_000);
    let class = |dist_mm: i32| {
        classify(
            signed_depth_circle_mm(c, r, enu(dist_mm, 0)).unwrap(),
            acc,
            1000,
        )
        .unwrap()
    };
    assert_eq!(class(140_000), Depth::Inside);
    assert_eq!(class(139_999), Depth::Inside);
    assert_eq!(class(140_001), Depth::Straddle);
    assert_eq!(class(150_000), Depth::Straddle);
    assert_eq!(class(159_999), Depth::Straddle);
    assert_eq!(class(160_000), Depth::Outside);
    assert_eq!(class(160_001), Depth::Outside);
    // "jitter around the edge never ENTERs": nothing between 140.001 m and 160 m is Inside.
    for d in (140_001..160_000).step_by(37) {
        assert_ne!(class(d), Depth::Inside, "{d}");
    }
}

#[test]
fn classify_boundaries_and_arguments() {
    // 1000*d >= k*acc  <=>  Inside; <= -k*acc <=> Outside.
    assert_eq!(classify(10_000, 10_000, 1000), Ok(Depth::Inside));
    assert_eq!(classify(9_999, 10_000, 1000), Ok(Depth::Straddle));
    assert_eq!(classify(-9_999, 10_000, 1000), Ok(Depth::Straddle));
    assert_eq!(classify(-10_000, 10_000, 1000), Ok(Depth::Outside));
    assert_eq!(classify(0, 10_000, 1000), Ok(Depth::Straddle));
    // k scales the band: k = 2000 doubles the required depth, k = 500 halves it.
    assert_eq!(classify(19_999, 10_000, 2000), Ok(Depth::Straddle));
    assert_eq!(classify(20_000, 10_000, 2000), Ok(Depth::Inside));
    assert_eq!(classify(5_000, 10_000, 500), Ok(Depth::Inside));
    assert_eq!(classify(-5_000, 10_000, 500), Ok(Depth::Outside));
    // Degenerate: zero band. Away from zero the sign decides; exactly on the line it is a straddle.
    assert_eq!(classify(1, 0, 1000), Ok(Depth::Inside));
    assert_eq!(classify(-1, 0, 1000), Ok(Depth::Outside));
    assert_eq!(classify(0, 0, 1000), Ok(Depth::Straddle));
    assert_eq!(classify(0, 10_000, 0), Ok(Depth::Straddle));
    assert_eq!(classify(1, 10_000, 0), Ok(Depth::Inside));
    // Extreme depths cannot overflow.
    assert_eq!(classify(i64::MAX, i64::MAX, 1000), Ok(Depth::Inside));
    assert_eq!(classify(i64::MIN, i64::MAX, 1000), Ok(Depth::Outside));
    assert_eq!(classify(i64::MAX, i64::MAX, u32::MAX), Ok(Depth::Straddle));
    assert_eq!(classify(i64::MAX, 0, u32::MAX), Ok(Depth::Inside));
    // Negative band is invalid input.
    assert_eq!(classify(0, -1, 1000), Err(EngineError::Parse));
}

#[test]
fn acc_eff_formula() {
    assert_eq!(acc_eff_mm(0, 1000), Ok(10_000)); // floored at 10 m
    assert_eq!(acc_eff_mm(9_999, 1000), Ok(10_000));
    assert_eq!(acc_eff_mm(10_000, 1000), Ok(10_000));
    assert_eq!(acc_eff_mm(65_000, 1000), Ok(65_000));
    assert_eq!(acc_eff_mm(20_000, 1500), Ok(30_000)); // kappa 1.5
    assert_eq!(acc_eff_mm(10_001, 1001), Ok(10_011)); // floor(10_001 * 1001 / 1000)
    assert_eq!(acc_eff_mm(u32::MAX, u32::MAX), Ok(18_446_744_065_119_617)); // no overflow
    assert_eq!(acc_eff_mm(5, 0), Ok(0));
}

// ---- properties ---------------------------------------------------------------------------

/// Convex hull (Andrew's monotone chain) with exact integer cross products, CCW, no collinear points.
fn hull(mut pts: Vec<(i64, i64)>) -> Vec<(i64, i64)> {
    pts.sort_unstable();
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let cross = |o: (i64, i64), a: (i64, i64), b: (i64, i64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut lower: Vec<(i64, i64)> = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<(i64, i64)> = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn to_enu(v: &[(i64, i64)]) -> Vec<Enu> {
    v.iter()
        .map(|&(e, n)| enu(i32::try_from(e).unwrap(), i32::try_from(n).unwrap()))
        .collect()
}

/// Reference for a CCW convex polygon: all edge orientations > 0 inside, >= 0 with a zero on it.
fn reference(h: &[(i64, i64)], p: (i64, i64)) -> PipResult {
    let mut any_neg = false;
    let mut any_zero = false;
    for (a, b) in h.iter().zip(h.iter().cycle().skip(1)) {
        let o = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
        any_neg |= o < 0;
        any_zero |= o == 0;
    }
    if any_neg {
        PipResult::Outside
    } else if any_zero {
        PipResult::Boundary
    } else {
        PipResult::Inside
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(600))]

    #[test]
    fn pip_agrees_with_the_convex_reference(
        pts in prop::collection::vec((-2000_i64..2000, -2000_i64..2000), 3..24),
        qs in prop::collection::vec((-2200_i64..2200, -2200_i64..2200), 1..30),
    ) {
        let h = hull(pts);
        prop_assume!(h.len() >= 3);
        let ccw = to_enu(&h);
        let mut cw = ccw.clone();
        cw.reverse();
        // Rotating the start vertex must not change the answer either.
        let mut rot = ccw.clone();
        rot.rotate_left(1);
        let mut probes = qs;
        probes.extend(h.iter().copied()); // vertices are always Boundary
        for q in probes {
            let want = reference(&h, q);
            let qe = enu(i32::try_from(q.0).unwrap(), i32::try_from(q.1).unwrap());
            prop_assert_eq!(point_in_polygon(&ccw, qe).unwrap(), want);
            prop_assert_eq!(point_in_polygon(&cw, qe).unwrap(), want);
            prop_assert_eq!(point_in_polygon(&rot, qe).unwrap(), want);
        }
    }

    #[test]
    fn depth_sign_agrees_with_pip_and_is_bounded(
        pts in prop::collection::vec((-2000_i64..2000, -2000_i64..2000), 3..16),
        q in (-2500_i64..2500, -2500_i64..2500),
    ) {
        let h = hull(pts);
        prop_assume!(h.len() >= 3);
        let poly = to_enu(&h);
        let qe = enu(i32::try_from(q.0).unwrap(), i32::try_from(q.1).unwrap());
        let d = signed_depth_polygon_mm(&poly, qe).unwrap();
        match point_in_polygon(&poly, qe).unwrap() {
            PipResult::Boundary => prop_assert_eq!(d, 0),
            PipResult::Inside => prop_assert!(d >= 0),
            PipResult::Outside => prop_assert!(d <= 0),
        }
        // |depth| never exceeds the distance to any vertex.
        for v in &poly {
            prop_assert!(d.abs() <= dist_mm(*v, qe).unwrap());
        }
    }

    /// Mirror symmetry: reflecting polygon and point through the n axis changes nothing.
    #[test]
    fn pip_is_mirror_invariant(
        pts in prop::collection::vec((-2000_i64..2000, -2000_i64..2000), 3..16),
        q in (-2500_i64..2500, -2500_i64..2500),
    ) {
        let h = hull(pts);
        prop_assume!(h.len() >= 3);
        let poly = to_enu(&h);
        let mir: Vec<Enu> = poly.iter().map(|v| enu(-v.e_mm, v.n_mm)).collect();
        let q1 = enu(i32::try_from(q.0).unwrap(), i32::try_from(q.1).unwrap());
        let q2 = enu(-q1.e_mm, q1.n_mm);
        prop_assert_eq!(point_in_polygon(&poly, q1).unwrap(), point_in_polygon(&mir, q2).unwrap());
        prop_assert_eq!(
            signed_depth_polygon_mm(&poly, q1).unwrap(),
            signed_depth_polygon_mm(&mir, q2).unwrap()
        );
    }

    /// classify is monotone in depth and consistent with its definition.
    #[test]
    fn classify_matches_its_definition(d in -1_000_000_i64..1_000_000, acc in 0_i64..200_000, k in 0_u32..3000) {
        let got = classify(d, acc, k).unwrap();
        let lhs = i128::from(d) * 1000;
        let rhs = i128::from(k) * i128::from(acc);
        let want = if lhs >= rhs && lhs <= -rhs { Depth::Straddle }
            else if lhs >= rhs { Depth::Inside }
            else if lhs <= -rhs { Depth::Outside }
            else { Depth::Straddle };
        prop_assert_eq!(got, want);
        // Never Inside and Outside at once, and deeper is never less inside.
        let deeper = classify(d.saturating_add(1), acc, k).unwrap();
        let rank = |x: Depth| match x { Depth::Outside => 0, Depth::Straddle => 1, Depth::Inside => 2 };
        prop_assert!(rank(deeper) >= rank(got));
    }

    /// Circle: Inside / Outside classification agrees with exact integer comparison.
    #[test]
    fn circle_classification_matches_exact_distance(
        r in 150_000_i64..600_000, e in -700_000_i32..700_000, n in -700_000_i32..700_000,
        acc in 0_u32..70_000,
    ) {
        let c = enu(0, 0);
        let depth = signed_depth_circle_mm(c, r, enu(e, n)).unwrap();
        let acc_eff = acc_eff_mm(acc, 1000).unwrap();
        let class = classify(depth, acc_eff, 1000).unwrap();
        let d2 = i128::from(e) * i128::from(e) + i128::from(n) * i128::from(n);
        // Inside requires the whole accuracy circle inside: dist + acc_eff <= r (dist exact floor).
        let inside_exact = {
            let need = i128::from(r) - i128::from(acc_eff);
            need >= 0 && d2 <= need * need
        };
        let dist_floor = i128::from(isqrt(u128::try_from(d2).unwrap()));
        if class == Depth::Inside {
            // floor(dist) <= r - acc_eff, so dist < r - acc_eff + 1.
            prop_assert!(dist_floor <= i128::from(r) - i128::from(acc_eff));
        }
        if inside_exact {
            prop_assert_eq!(class, Depth::Inside);
        }
        if class == Depth::Outside {
            prop_assert!(dist_floor >= i128::from(r) + i128::from(acc_eff));
        }
    }

    #[test]
    fn orient2d_is_antisymmetric_and_cyclic(
        a in (-1_000_000_i64..1_000_000, -1_000_000_i64..1_000_000),
        b in (-1_000_000_i64..1_000_000, -1_000_000_i64..1_000_000),
        c in (-1_000_000_i64..1_000_000, -1_000_000_i64..1_000_000),
    ) {
        let (a, b, c) = ([a.0, a.1], [b.0, b.1], [c.0, c.1]);
        let o = orient2d(a, b, c).unwrap();
        prop_assert_eq!(o, orient2d(b, c, a).unwrap());
        prop_assert_eq!(o, orient2d(c, a, b).unwrap());
        prop_assert_eq!(o, -orient2d(a, c, b).unwrap());
    }
}
