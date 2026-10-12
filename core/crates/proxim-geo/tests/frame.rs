//! Frame, projection and origin rule (spec 1.2): analytic values, boundaries, antimeridian, poles,
//! error band against an independent ECEF reference, properties.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
#![allow(
    clippy::float_arithmetic,
    clippy::float_cmp,
    clippy::cast_possible_truncation
)]

mod common;

use common::{destination, pos, true_enu_mm};
use proptest::prelude::*;
use proxim_geo::{frame_for, trig};
use proxim_types::{EngineError, Enu, Position};

fn p(lat_e7: i32, lon_e7: i32) -> Position {
    Position { lat_e7, lon_e7 }
}

#[test]
fn frame_uses_the_rounded_latitude_grid() {
    let f = frame_for(p(450_000_000, 75_000_000)).unwrap();
    assert_eq!((f.kn_q32, f.ke_q32), trig::scale_for_lat_q(4500).unwrap());
    // div_round half-up: exactly 44.995 degrees rounds up to 45.00, 44.99499.. down.
    let up = frame_for(p(449_950_000, 0)).unwrap();
    assert_eq!((up.kn_q32, up.ke_q32), trig::scale_for_lat_q(4500).unwrap());
    let down = frame_for(p(449_949_999, 0)).unwrap();
    assert_eq!(
        (down.kn_q32, down.ke_q32),
        trig::scale_for_lat_q(4499).unwrap()
    );
    // South: -44.995 rounds to -44.99 (half toward +infinity); table is symmetric so same as 4499.
    let s = frame_for(p(-449_950_000, 0)).unwrap();
    assert_eq!((s.kn_q32, s.ke_q32), trig::scale_for_lat_q(4499).unwrap());
    let s2 = frame_for(p(-449_950_001, 0)).unwrap();
    assert_eq!((s2.kn_q32, s2.ke_q32), trig::scale_for_lat_q(4500).unwrap());
    // The frame keeps the origin it was given (never derived from a fix).
    assert_eq!(f.origin, p(450_000_000, 75_000_000));
}

#[test]
fn unsupported_latitude_is_a_typed_error_at_exactly_80_degrees() {
    assert!(frame_for(p(800_000_000, 0)).is_ok());
    assert!(frame_for(p(-800_000_000, 0)).is_ok());
    for lat in [
        800_000_001,
        -800_000_001,
        850_000_000,
        900_000_000,
        i32::MAX,
        i32::MIN,
    ] {
        assert_eq!(
            frame_for(p(lat, 0)),
            Err(EngineError::UnsupportedLatitude),
            "lat {lat}"
        );
    }
}

#[test]
fn invalid_origin_longitude_is_a_parse_error() {
    assert!(frame_for(p(0, 1_800_000_000)).is_ok());
    assert!(frame_for(p(0, -1_800_000_000)).is_ok());
    assert_eq!(frame_for(p(0, 1_800_000_001)), Err(EngineError::Parse));
    assert_eq!(frame_for(p(0, i32::MIN)), Err(EngineError::Parse));
}

#[test]
fn analytic_values_at_the_equator_and_45_degrees() {
    let f0 = frame_for(p(0, 0)).unwrap();
    // 1 degree north: 110.574 km; 1 degree east: 111.319 km (WGS84).
    assert_eq!(
        f0.project(p(10_000_000, 0)).unwrap(),
        Enu {
            e_mm: 0,
            n_mm: 110_574_276
        }
    );
    assert_eq!(
        f0.project(p(0, 10_000_000)).unwrap(),
        Enu {
            e_mm: 111_319_491,
            n_mm: 0
        }
    );
    assert_eq!(f0.project(p(0, 0)).unwrap(), Enu { e_mm: 0, n_mm: 0 });
    // Signs: south and west are negative.
    assert_eq!(f0.project(p(-10_000_000, 0)).unwrap().n_mm, -110_574_276);
    assert_eq!(f0.project(p(0, -10_000_000)).unwrap().e_mm, -111_319_491);
    // At 45 degrees 1 degree of longitude is 78.847 km: 0.5 degree is 39.423 km.
    let f45 = frame_for(p(450_000_000, 0)).unwrap();
    assert_eq!(
        f45.project(p(450_000_000, 5_000_000)).unwrap(),
        Enu {
            e_mm: 39_423_418,
            n_mm: 0
        }
    );
    assert_eq!(
        f45.project(p(447_000_000, 0)).unwrap(),
        Enu {
            e_mm: 0,
            n_mm: -33_339_533
        }
    );
    // One e7 unit east at 45 degrees is 7.88 mm: rounds to 8.
    assert_eq!(f45.project(p(450_000_000, 1)).unwrap().e_mm, 8);
}

#[test]
fn frame_boundary_150_km_is_exact_on_both_axes() {
    let f = frame_for(p(0, 0)).unwrap();
    // North: largest dlat that fits is 13_565_542 (n = 149_999_998 mm), the next unit is 150_000_009.
    assert_eq!(f.project(p(13_565_542, 0)).unwrap().n_mm, 149_999_998);
    assert_eq!(f.project(p(13_565_543, 0)), Err(EngineError::OutOfFrame));
    assert_eq!(f.project(p(-13_565_542, 0)).unwrap().n_mm, -149_999_998);
    assert_eq!(f.project(p(-13_565_543, 0)), Err(EngineError::OutOfFrame));
    // East: scan to find the exact edge instead of hard-coding it.
    let mut lo = 0_i32;
    let mut hi = 20_000_000_i32;
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        match f.project(p(0, mid)) {
            Ok(q) => {
                assert!(q.e_mm <= 150_000_000);
                lo = mid;
            }
            Err(e) => {
                assert_eq!(e, EngineError::OutOfFrame);
                hi = mid;
            }
        }
    }
    let ok = f.project(p(0, lo)).unwrap().e_mm;
    assert!(ok <= 150_000_000 && ok > 149_999_980, "{ok}");
    assert_eq!(f.project(p(0, hi)), Err(EngineError::OutOfFrame));
}

#[test]
fn exactly_150_000_000_mm_is_inside_and_one_more_is_out() {
    // Find origins and offsets whose projection lands on exactly +-150_000_000 mm on each axis. That
    // value must be accepted (the bound is inclusive) and the next e7 unit (>= 1.9 mm) rejected.
    let mut hits_e = 0;
    let mut hits_n = 0;
    for lat in (0..=800_000_000_i32).step_by(250_000) {
        let f = frame_for(p(lat, 0)).unwrap();
        let de = (150_000_000_i128 << 32) / i128::from(f.ke_q32);
        let dn = (150_000_000_i128 << 32) / i128::from(f.kn_q32);
        for d in -2..=2_i128 {
            let e = i32::try_from(de + d).unwrap();
            let n = i32::try_from(dn + d).unwrap();
            if let Ok(q) = f.project(p(lat, e))
                && q.e_mm == 150_000_000
            {
                hits_e += 1;
                assert_eq!(f.project(p(lat, -e)).unwrap().e_mm, -150_000_000);
                assert_eq!(f.project(p(lat, e + 1)), Err(EngineError::OutOfFrame));
                assert_eq!(f.project(p(lat, -e - 1)), Err(EngineError::OutOfFrame));
            }
            if let Ok(q) = f.project(p(lat + n, 0))
                && q.n_mm == 150_000_000
            {
                hits_n += 1;
                assert_eq!(f.project(p(lat - n, 0)).unwrap().n_mm, -150_000_000);
                assert_eq!(f.project(p(lat + n + 1, 0)), Err(EngineError::OutOfFrame));
                assert_eq!(f.project(p(lat - n - 1, 0)), Err(EngineError::OutOfFrame));
            }
        }
    }
    assert!(
        hits_e >= 3 && hits_n >= 3,
        "bound never hit exactly: {hits_e} {hits_n}"
    );
}

#[test]
fn antimeridian_wraps_in_both_directions() {
    let east = frame_for(p(0, 1_799_999_000)).unwrap();
    // 2000 e7 units east across the date line: +22_264 mm, not -39_999 km.
    assert_eq!(
        east.project(p(0, -1_799_999_000)).unwrap(),
        Enu {
            e_mm: 22_264,
            n_mm: 0
        }
    );
    let west = frame_for(p(0, -1_799_999_000)).unwrap();
    assert_eq!(
        west.project(p(0, 1_799_999_000)).unwrap(),
        Enu {
            e_mm: -22_264,
            n_mm: 0
        }
    );
    // A fix on the same side is unaffected.
    assert_eq!(east.project(p(0, 1_799_998_000)).unwrap().e_mm, -11_132);
    // 180 degrees away wraps to -180 and is hopelessly out of frame.
    let o = frame_for(p(0, 0)).unwrap();
    assert_eq!(o.project(p(0, 1_800_000_000)), Err(EngineError::OutOfFrame));
    assert_eq!(
        o.project(p(0, -1_800_000_000)),
        Err(EngineError::OutOfFrame)
    );
}

#[test]
fn poles_and_invalid_inputs_are_typed_errors() {
    let f = frame_for(p(800_000_000, 0)).unwrap();
    // Five degrees north of the supported limit: 550 km, out of frame (not a panic).
    assert_eq!(f.project(p(850_000_000, 0)), Err(EngineError::OutOfFrame));
    assert_eq!(f.project(p(900_000_000, 0)), Err(EngineError::OutOfFrame));
    for bad in [
        p(900_000_001, 0),
        p(-900_000_001, 0),
        p(0, 1_800_000_001),
        p(i32::MAX, i32::MAX),
        p(i32::MIN, i32::MIN),
    ] {
        assert_eq!(f.project(bad), Err(EngineError::OutOfFrame), "{bad:?}");
    }
    // Longitude convergence near the pole: the east scale shrinks, so a 1 degree lon step at 80
    // degrees is only 19.4 km.
    assert_eq!(
        f.project(p(800_000_000, 10_000_000)).unwrap().e_mm,
        19_393_486
    );
}

#[test]
fn project_all_stops_at_the_first_error() {
    let f = frame_for(p(0, 0)).unwrap();
    let ok = f.project_all(&[p(0, 0), p(1000, 1000)]).unwrap();
    assert_eq!(ok.len(), 2);
    assert_eq!(
        f.project_all(&[p(0, 0), p(100_000_000, 0), p(1, 1)]),
        Err(EngineError::OutOfFrame)
    );
    assert_eq!(f.project_all(&[]).unwrap(), vec![]);
}

#[test]
fn unproject_inverts_project_within_one_unit() {
    let f = frame_for(p(450_000_000, 75_000_000)).unwrap();
    for (dlat, dlon) in [
        (0, 0),
        (1, 1),
        (-1234, 5678),
        (90_000, -250_000),
        (-300_000, 400_000),
    ] {
        let q = p(450_000_000 + dlat, 75_000_000 + dlon);
        let back = f.unproject(f.project(q).unwrap()).unwrap();
        assert!((back.lat_e7 - q.lat_e7).abs() <= 1, "{back:?} vs {q:?}");
        assert!((back.lon_e7 - q.lon_e7).abs() <= 1, "{back:?} vs {q:?}");
    }
    // Across the antimeridian it wraps back into range.
    let a = frame_for(p(0, 1_799_999_000)).unwrap();
    let back = a
        .unproject(Enu {
            e_mm: 22_264,
            n_mm: 0,
        })
        .unwrap();
    assert!((back.lon_e7 - (-1_799_999_000)).abs() <= 1);
    // Past the pole is a typed error, not a wrapped latitude.
    let n80 = frame_for(p(800_000_000, 0)).unwrap();
    assert_eq!(
        n80.unproject(Enu {
            e_mm: 0,
            n_mm: 2_000_000_000
        }),
        Err(EngineError::OutOfFrame)
    );
}

#[test]
fn error_band_against_ecef_reference_matches_the_spec_figures() {
    // Spec 4: about 0.1% within 5 km at latitude 45, 0.25% at 70, growing linearly. Measured on a
    // grid of bearings; the engine must stay within the stated figures.
    let mut report = Vec::new();
    // Each latitude is tried on the grid and at the worst quantization offset (+0.005 degrees).
    let mut cases = Vec::new();
    for off in [0.0, 0.005] {
        for (lat, limit) in [
            (0.0, 0.0005),
            (30.0, 0.001),
            (45.0, 0.001),
            (60.0, 0.0015),
            (70.0, 0.0025),
        ] {
            cases.push((lat + off, limit));
        }
    }
    for (lat, limit) in cases {
        let o = pos(lat, 10.0);
        let f = frame_for(o).unwrap();
        let mut worst = 0.0_f64;
        for dist in [200.0, 1000.0, 5000.0] {
            for b in (0..360).step_by(5) {
                let q = destination(o, f64::from(b), dist);
                let got = f.project(q).unwrap();
                let (te, tn) = true_enu_mm(o, q);
                let err = libm::hypot(f64::from(got.e_mm) - te, f64::from(got.n_mm) - tn);
                let d = libm::hypot(te, tn);
                // 40 mm absorbs the e7 rounding of the test point and of the origin.
                worst = worst.max(((err - 40.0).max(0.0)) / d);
            }
        }
        assert!(
            worst <= limit,
            "lat {lat}: worst relative error {worst} > {limit}"
        );
        report.push((lat, worst));
    }
    // The error does grow with latitude (sanity that the test measures something).
    assert!(report[4].1 > report[0].1);
    assert_eq!(report.len(), 10);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// Near-isometry in the spec's tolerance band: within 30 km of the origin the projected offset
    /// stays within 0.75 tan(lat) d/R + 0.005% (+ 40 mm) of the true tangent-plane offset.
    #[test]
    fn projection_is_near_isometric(
        lat in -80.0_f64..80.0, lon in -179.0_f64..179.0,
        bearing in 0.0_f64..360.0, dist in 1.0_f64..30_000.0,
    ) {
        let o = pos(lat, lon);
        let f = frame_for(o).unwrap();
        let q = destination(o, bearing, dist);
        let got = f.project(q).unwrap();
        let (te, tn) = true_enu_mm(o, q);
        let err = libm::hypot(f64::from(got.e_mm) - te, f64::from(got.n_mm) - tn);
        let d = libm::hypot(te, tn);
        let tan = libm::tan(deg_to_rad(lat)).abs();
        // 8.73e-5 rad = half of the 0.01 degree latitude grid: the quantization term tan(lat) * dphi.
        let band = 0.75 * tan * d / 6_371_008_800.0 + tan * 8.73e-5 + 5e-5;
        prop_assert!(err <= 40.0 + band * d, "lat {lat} d {d} err {err} band {}", band * d);
    }

    /// Monotone: moving north never decreases n and leaves e unchanged; moving east never
    /// decreases e and leaves n unchanged.
    #[test]
    fn projection_is_monotone_per_axis(
        lat in -800_000_000_i32..=800_000_000, lon in -1_700_000_000_i32..=1_700_000_000,
        a in -1_000_000_i32..1_000_000, b in -1_000_000_i32..1_000_000,
        da in 0_i32..1_000_000, db in 0_i32..1_000_000,
    ) {
        let f = frame_for(p(lat, lon)).unwrap();
        let q0 = p(lat + a, lon + b);
        let qn = p(lat + a + da, lon + b);
        let qe = p(lat + a, lon + b + db);
        let (Ok(e0), Ok(en), Ok(ee)) = (f.project(q0), f.project(qn), f.project(qe)) else {
            return Ok(());
        };
        prop_assert!(en.n_mm >= e0.n_mm);
        prop_assert_eq!(en.e_mm, e0.e_mm);
        prop_assert!(ee.e_mm >= e0.e_mm);
        prop_assert_eq!(ee.n_mm, e0.n_mm);
    }

    /// Never panics or wraps for any i32 pair; either a typed error or an in-frame result.
    #[test]
    fn project_total_function(olat in any::<i32>(), olon in any::<i32>(), plat in any::<i32>(), plon in any::<i32>()) {
        if let Ok(f) = frame_for(p(olat, olon)) {
            match f.project(p(plat, plon)) {
                Ok(q) => {
                    prop_assert!(i64::from(q.e_mm).abs() <= 150_000_000);
                    prop_assert!(i64::from(q.n_mm).abs() <= 150_000_000);
                }
                Err(e) => prop_assert!(matches!(e, EngineError::OutOfFrame | EngineError::Overflow)),
            }
        }
    }

    /// project / unproject round trip within 1 e7 unit (11 mm) anywhere a frame exists.
    #[test]
    fn unproject_round_trip(
        lat in -800_000_000_i32..=800_000_000, lon in -1_800_000_000_i32..=1_800_000_000,
        dlat in -1_200_000_i32..1_200_000, dlon in -1_200_000_i32..1_200_000,
    ) {
        let f = frame_for(p(lat, lon)).unwrap();
        let target = p(lat.saturating_add(dlat).clamp(-900_000_000, 900_000_000), lon.wrapping_add(dlon));
        let target = Position { lat_e7: target.lat_e7, lon_e7: wrap_lon(i64::from(lon) + i64::from(dlon)) };
        if let Ok(q) = f.project(target) {
            let back = f.unproject(q).unwrap();
            prop_assert!((i64::from(back.lat_e7) - i64::from(target.lat_e7)).abs() <= 1);
            let dl = (i64::from(back.lon_e7) - i64::from(target.lon_e7)).rem_euclid(3_600_000_000);
            prop_assert!(dl <= 1 || dl >= 3_599_999_999);
        }
    }
}

fn deg_to_rad(d: f64) -> f64 {
    d * core::f64::consts::PI / 180.0
}

fn wrap_lon(v: i64) -> i32 {
    let mut x = v;
    if x > 1_800_000_000 {
        x -= 3_600_000_000;
    }
    if x < -1_800_000_000 {
        x += 3_600_000_000;
    }
    i32::try_from(x).unwrap()
}
