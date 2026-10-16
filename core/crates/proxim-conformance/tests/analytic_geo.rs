//! Spec 1-4 analytic values for the public geometry API, with an independent reference for the two
//! quantized trig tables (spec 2.2) and the integer primitives (spec 1.3).
//!
//! The float code below exists only to compute reference values; it uses basic arithmetic and a
//! hand-written Taylor series, never libm or `f64::sin`/`cos` (those are what the engine under test
//! uses). It is test code outside the engine's float confinement (spec 2.1 applies to the engine).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::unreachable,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::float_arithmetic,
    clippy::float_cmp,
    clippy::needless_range_loop
)]

use proxim_conformance::rng::Rng;
use proxim_conformance::sha::sha256_hex;
use proxim_geo::trig::scale_for_lat_q;
use proxim_geo::{
    Depth, PipResult, Polyline, acc_eff_mm, classify, cos_tol_q30, dist_mm, dist_point_segment_mm,
    div_round, frame_for, isqrt, normalize_q30, orient2d, point_in_polygon, project_to_polyline,
    rem_along_mm, rsr, signed_depth_circle_mm, signed_depth_polygon_mm, unit_from_course,
};
use proxim_types::{EngineError, Enu, Position};

const PI: f64 = core::f64::consts::PI;

// ---- reference trig (Taylor, no libm) ---------------------------------------------------------

fn sin_taylor(x: f64) -> f64 {
    // |x| <= pi/2 here
    let mut term = x;
    let mut sum = x;
    for n in 1..30 {
        term *= -x * x / (f64::from(2 * n) * f64::from(2 * n + 1));
        sum += term;
    }
    sum
}
fn cos_taylor(x: f64) -> f64 {
    let mut term = 1.0;
    let mut sum = 1.0;
    for n in 1..30 {
        term *= -x * x / (f64::from(2 * n - 1) * f64::from(2 * n));
        sum += term;
    }
    sum
}
fn sqrt_newton(x: f64) -> f64 {
    let mut r = if x > 1.0 { x / 2.0 } else { 1.0 };
    for _ in 0..80 {
        r = 0.5 * (r + x / r);
    }
    r
}
fn round_half_away(x: f64) -> i64 {
    if x >= 0.0 {
        (x + 0.5).floor() as i64
    } else {
        -((-x + 0.5).floor() as i64)
    }
}

/// Reference `SCALE[lat_q]` from the spec 1.2 formulas with WGS84.
fn ref_scale(lat_q: i32) -> (i64, i64) {
    let phi = f64::from(lat_q.abs()) * 0.01 * PI / 180.0;
    let f = 1.0 / 298.257_223_563;
    let e2 = f * (2.0 - f);
    let s = sin_taylor(phi);
    let c = cos_taylor(phi);
    let w = 1.0 - e2 * s * s;
    let sw = sqrt_newton(w);
    let m = 6_378_137.0 * (1.0 - e2) / (w * sw);
    let n = 6_378_137.0 / sw;
    let k = PI / 180.0 * 1e-4;
    let q32 = 4_294_967_296.0;
    (
        round_half_away(m * k * q32),
        round_half_away(n * c * k * q32),
    )
}

/// Reference `SINCOS[course_ddeg]` as (east=sin, north=cos) in Q30.
fn ref_unit(course_ddeg: u16) -> [i32; 2] {
    let d = i32::from(course_ddeg) % 3600;
    let (quad, r) = (d / 900, d % 900);
    let x = f64::from(r) * 0.1 * PI / 180.0;
    let (s, c) = (sin_taylor(x), cos_taylor(x));
    // rotate by quadrant: bearing measured clockwise from north; east = sin, north = cos
    let (e, n) = match quad {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    };
    let q30 = 1_073_741_824.0;
    [
        round_half_away(e * q30) as i32,
        round_half_away(n * q30) as i32,
    ]
}

// ---- tables -----------------------------------------------------------------------------------

#[test]
fn scale_table_matches_independent_wgs84_computation_for_all_16001_rows() {
    let mut exact = 0_u32;
    let mut worst = 0_i64;
    for lat_q in -8000..=8000 {
        let (kn, ke) = scale_for_lat_q(lat_q).unwrap();
        let (rkn, rke) = ref_scale(lat_q);
        let d = (kn - rkn).abs().max((ke - rke).abs());
        worst = worst.max(d);
        if d == 0 {
            exact += 1;
        }
        assert!(
            d <= 1,
            "lat_q {lat_q}: engine ({kn},{ke}) vs reference ({rkn},{rke})"
        );
    }
    // The reference is exact to ~1e-5 of one Q32 unit, so a correct table matches essentially everywhere.
    assert!(
        exact >= 15_990,
        "only {exact}/16001 rows match exactly (worst diff {worst})"
    );
}

#[test]
fn scale_table_is_symmetric_and_rejects_beyond_80_degrees() {
    for lat_q in 0..=8000 {
        assert_eq!(
            scale_for_lat_q(lat_q).unwrap(),
            scale_for_lat_q(-lat_q).unwrap()
        );
    }
    for bad in [8001, -8001, i32::MAX, i32::MIN] {
        assert_eq!(
            scale_for_lat_q(bad),
            Err(EngineError::UnsupportedLatitude),
            "{bad}"
        );
    }
}

#[test]
fn scale_values_agree_with_published_degree_lengths() {
    // Metres per degree on WGS84 (spec 1.2 formulas, evaluated offline): latitude 110 574.28 (0 deg),
    // 111 131.78 (45), 111 412.29 (60); longitude 111 319.49 (0), 78 846.84 (45), 55 800.00 (60). Q32 value = metres/degree x 1e-4 x 2^32.
    let q32 = 4_294_967_296_i128;
    let m_per_deg = |k: i64| i128::from(k) * 10_000 * 1000 / q32; // millimetres per degree (x 1000)
    let cases = [
        (0, 110_574_276_i128, 111_319_491_i128),
        (4500, 111_131_777, 78_846_835),
        (6000, 111_412_287, 55_800_002),
    ];
    for (lat_q, lat_mm_per_deg, lon_mm_per_deg) in cases {
        let (kn, ke) = scale_for_lat_q(lat_q).unwrap();
        let got_lat = m_per_deg(kn); // mm per degree
        let got_lon = m_per_deg(ke);
        let rel = |a: i128, b: i128| (a - b).abs() * 1_000_000 / b;
        assert!(
            rel(got_lat, lat_mm_per_deg) < 10,
            "lat_q {lat_q}: {got_lat} vs {lat_mm_per_deg}"
        );
        assert!(
            rel(got_lon, lon_mm_per_deg) < 10,
            "lat_q {lat_q}: {got_lon} vs {lon_mm_per_deg}"
        );
    }
}

fn load_table(name: &str) -> serde_json::Value {
    let path = format!(
        "{}/../../testdata/golden/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))).unwrap()
}

#[test]
fn scale_table_digest_matches_the_normative_file_and_the_errata_prefix() {
    let mut rows = String::new();
    for lat_q in -8000..=8000 {
        let (kn, ke) = scale_for_lat_q(lat_q).unwrap();
        rows.push_str(&format!("{},{kn},{ke}\n", lat_q + 8000));
    }
    let digest = sha256_hex(rows.as_bytes());
    let file = load_table("frame_scale_table.json");
    assert_eq!(
        file["sha256"].as_str().unwrap(),
        digest,
        "engine table differs from the golden file"
    );
    // errata E-G1 prints `539ff518...771a`
    assert!(
        digest.starts_with("539ff518") && digest.ends_with("771a"),
        "{digest}"
    );
    // every 100th row listed in the file equals the engine value
    for e in file["entries"].as_array().unwrap() {
        let a = e.as_array().unwrap();
        let idx = a[0].as_i64().unwrap() as i32;
        let (kn, ke) = scale_for_lat_q(idx - 8000).unwrap();
        assert_eq!(
            (a[1].as_i64().unwrap(), a[2].as_i64().unwrap()),
            (kn, ke),
            "row {idx}"
        );
    }
}

#[test]
fn sincos_table_matches_independent_computation_for_all_3600_rows() {
    let mut exact = 0;
    for c in 0..3600_u16 {
        let u = unit_from_course(c).unwrap();
        let r = ref_unit(c);
        let d = (u[0] - r[0]).abs().max((u[1] - r[1]).abs());
        assert!(d <= 1, "course {c}: engine {u:?} vs reference {r:?}");
        exact += i32::from(d == 0);
    }
    assert!(exact >= 3590, "only {exact}/3600 exact");
    // cardinal directions are exact: north, east, south, west
    assert_eq!(unit_from_course(0).unwrap(), [0, 1 << 30]);
    assert_eq!(unit_from_course(900).unwrap(), [1 << 30, 0]);
    assert_eq!(unit_from_course(1800).unwrap(), [0, -(1 << 30)]);
    assert_eq!(unit_from_course(2700).unwrap(), [-(1 << 30), 0]);
    assert_eq!(unit_from_course(3600), Err(EngineError::Parse));
    assert_eq!(unit_from_course(u16::MAX), Err(EngineError::Parse));
}

#[test]
fn sincos_table_digest_matches_the_normative_file() {
    let mut rows = String::new();
    for c in 0..3600_u16 {
        let u = unit_from_course(c).unwrap();
        rows.push_str(&format!("{c},{},{}\n", u[0], u[1]));
    }
    let digest = sha256_hex(rows.as_bytes());
    let file = load_table("sincos_table.json");
    assert_eq!(file["sha256"].as_str().unwrap(), digest);
    assert!(
        digest.starts_with("79bcdda5") && digest.ends_with("73e1"),
        "{digest}"
    );
}

#[test]
fn cos_tol_is_the_north_component_of_the_sincos_row() {
    // "cos(tol) = SINCOS[tol].cos, tol <= 900"
    for tol in 0..=900_u16 {
        assert_eq!(
            cos_tol_q30(tol).unwrap(),
            unit_from_course(tol).unwrap()[1],
            "tol {tol}"
        );
    }
    assert_eq!(cos_tol_q30(0).unwrap(), 1 << 30);
    assert_eq!(cos_tol_q30(900).unwrap(), 0);
    // cos 45 deg = 759250125 (round(2^30 / sqrt 2)); cos 30 = 929887697; cos 15 deg = 1037154959
    assert_eq!(cos_tol_q30(450).unwrap(), 759_250_125);
    assert_eq!(cos_tol_q30(300).unwrap(), 929_887_697);
    assert_eq!(cos_tol_q30(150).unwrap(), 1_037_154_959);
    for bad in [901, 3600, u16::MAX] {
        assert_eq!(cos_tol_q30(bad), Err(EngineError::Parse), "tol {bad}");
    }
}

// ---- frame ------------------------------------------------------------------------------------

#[test]
fn project_uses_the_table_scale_and_rounds_half_up() {
    let o = Position {
        lat_e7: 450_000_000,
        lon_e7: 70_000_000,
    };
    let fr = frame_for(o).unwrap();
    let (kn, ke) = scale_for_lat_q(4500).unwrap();
    assert_eq!((fr.kn_q32, fr.ke_q32), (kn, ke));
    // 1000 e7 units north: rsr(1000 x kn, 32)
    let p = fr
        .project(Position {
            lat_e7: 450_001_000,
            lon_e7: 70_000_000,
        })
        .unwrap();
    let want_n = ((i128::from(kn) * 1000 + (1 << 31)) >> 32) as i32;
    assert_eq!((p.e_mm, p.n_mm), (0, want_n));
    let p = fr
        .project(Position {
            lat_e7: 449_999_000,
            lon_e7: 70_002_000,
        })
        .unwrap();
    assert_eq!(p.n_mm, ((i128::from(kn) * -1000 + (1 << 31)) >> 32) as i32);
    assert_eq!(p.e_mm, ((i128::from(ke) * 2000 + (1 << 31)) >> 32) as i32);
    // one degree north is about 111.13 km at latitude 45
    let p = fr
        .project(Position {
            lat_e7: 460_000_000,
            lon_e7: 70_000_000,
        })
        .unwrap();
    assert!((111_000_000..111_300_000).contains(&p.n_mm), "{}", p.n_mm);
}

#[test]
fn frame_origin_latitude_rounding_uses_a_0_01_degree_grid() {
    // lat_q = div_round(lat_e7, 100000): 45.004999 deg -> 4500, 45.005000 deg -> 4501
    let a = frame_for(Position {
        lat_e7: 450_049_999,
        lon_e7: 0,
    })
    .unwrap();
    let b = frame_for(Position {
        lat_e7: 450_050_000,
        lon_e7: 0,
    })
    .unwrap();
    assert_eq!((a.kn_q32, a.ke_q32), scale_for_lat_q(4500).unwrap());
    assert_eq!((b.kn_q32, b.ke_q32), scale_for_lat_q(4501).unwrap());
    // negative: -45.005 -> -4500.5 rounds up (floor((2a+b)/2b)) to -4500; -45.0050001 -> -4501
    let c = frame_for(Position {
        lat_e7: -450_050_000,
        lon_e7: 0,
    })
    .unwrap();
    assert_eq!((c.kn_q32, c.ke_q32), scale_for_lat_q(-4500).unwrap());
}

#[test]
fn latitude_above_80_degrees_is_unsupported_and_fails_closed() {
    for lat in [
        800_000_001,
        850_000_000,
        900_000_000,
        -800_000_001,
        i32::MAX,
        i32::MIN,
    ] {
        let r = frame_for(Position {
            lat_e7: lat,
            lon_e7: 0,
        });
        assert!(
            matches!(
                r,
                Err(EngineError::UnsupportedLatitude | EngineError::Parse)
            ),
            "lat {lat}: {r:?}"
        );
    }
    frame_for(Position {
        lat_e7: 800_000_000,
        lon_e7: 0,
    })
    .unwrap();
    frame_for(Position {
        lat_e7: -800_000_000,
        lon_e7: 0,
    })
    .unwrap();
}

#[test]
fn project_wraps_longitude_across_the_antimeridian() {
    let o = Position {
        lat_e7: 0,
        lon_e7: 1_799_990_000,
    };
    let fr = frame_for(o).unwrap();
    let east = fr
        .project(Position {
            lat_e7: 0,
            lon_e7: -1_799_990_000,
        })
        .unwrap();
    // 0.002 degree east of the origin across the date line, about 222.6 m
    assert!((222_000..223_200).contains(&east.e_mm), "{east:?}");
    assert_eq!(east.n_mm, 0);
    let west = fr
        .project(Position {
            lat_e7: 0,
            lon_e7: 1_799_970_000,
        })
        .unwrap();
    assert!((-223_200..-222_000).contains(&west.e_mm), "{west:?}");
}

#[test]
fn project_rejects_points_beyond_150_km_of_the_origin() {
    let fr = frame_for(Position {
        lat_e7: 450_000_000,
        lon_e7: 70_000_000,
    })
    .unwrap();
    // 1.4 degrees north is ~155 km
    let r = fr.project(Position {
        lat_e7: 464_000_000,
        lon_e7: 70_000_000,
    });
    assert_eq!(r, Err(EngineError::OutOfFrame));
    // 1.3 degrees north is ~144 km: fine
    let ok = fr
        .project(Position {
            lat_e7: 463_000_000,
            lon_e7: 70_000_000,
        })
        .unwrap();
    assert!(ok.n_mm.abs() <= 150_000_000);
    // the bound is on each axis
    let r = fr.project(Position {
        lat_e7: 450_000_000,
        lon_e7: 90_000_000,
    });
    assert_eq!(r, Err(EngineError::OutOfFrame));
}

#[test]
fn project_never_panics_on_extreme_positions() {
    let fr = frame_for(Position {
        lat_e7: 450_000_000,
        lon_e7: 70_000_000,
    })
    .unwrap();
    for lat in [
        i32::MIN,
        -900_000_001,
        -900_000_000,
        0,
        900_000_000,
        900_000_001,
        i32::MAX,
    ] {
        for lon in [
            i32::MIN,
            -1_800_000_001,
            -1_800_000_000,
            0,
            1_800_000_000,
            1_800_000_001,
            i32::MAX,
        ] {
            let _ = fr.project(Position {
                lat_e7: lat,
                lon_e7: lon,
            });
        }
    }
}

// ---- integer primitives -----------------------------------------------------------------------

#[test]
fn div_round_is_floor_of_2a_plus_b_over_2b() {
    // floor((2a+b)/(2b)): halves round up (toward +infinity)
    let cases = [
        (7, 2, 4),
        (-7, 2, -3),
        (5, 10, 1),
        (-5, 10, 0),
        (1, 3, 0),
        (2, 3, 1),
        (-1, 3, 0),
        (-2, 3, -1),
        (0, 5, 0),
        (1, 2, 1),
        (-1, 2, 0),
        (3, 2, 2),
        (-3, 2, -1),
        (100_000, 100_000, 1),
        (450_049_999, 100_000, 4500),
        (450_050_000, 100_000, 4501),
        (-450_050_000, 100_000, -4500),
    ];
    for (a, b, want) in cases {
        assert_eq!(div_round(a, b), Ok(want), "div_round({a},{b})");
    }
    let mut rng = Rng::new(7);
    for _ in 0..5000 {
        let a = rng.range_i64(-1 << 40, 1 << 40);
        let b = rng.range_i64(1, 1 << 20);
        let want = (2 * i128::from(a) + i128::from(b)).div_euclid(2 * i128::from(b)) as i64;
        assert_eq!(div_round(a, b), Ok(want), "div_round({a},{b})");
    }
    // b <= 0 is not a valid divisor; extreme a must never wrap
    assert!(div_round(1, 0).is_err() && div_round(1, -1).is_err());
    for a in [i64::MIN, i64::MIN + 1, i64::MAX, i64::MAX - 1] {
        for b in [1, 2, 3, i64::MAX] {
            if let Ok(v) = div_round(a, b) {
                let want = (2 * i128::from(a) + i128::from(b)).div_euclid(2 * i128::from(b));
                assert_eq!(i128::from(v), want, "div_round({a},{b}) wrapped");
            }
        }
    }
}

#[test]
fn rsr_is_round_shift_right_with_arithmetic_shift() {
    // rsr(x, s) = (x + 2^(s-1)) >> s
    assert_eq!(rsr(5, 1), Ok(3));
    assert_eq!(rsr(-5, 1), Ok(-2));
    assert_eq!(rsr(3, 2), Ok(1));
    assert_eq!(rsr(2, 2), Ok(1));
    assert_eq!(rsr(1, 2), Ok(0));
    assert_eq!(rsr(-2, 2), Ok(0));
    assert_eq!(rsr(-3, 2), Ok(-1));
    let mut rng = Rng::new(11);
    for _ in 0..5000 {
        let x = i128::from(rng.range_i64(i64::MIN, i64::MAX))
            * i128::from(rng.range_i64(-1 << 30, 1 << 30));
        let s = rng.range_i64(1, 64) as u32;
        assert_eq!(
            rsr(x, s),
            Ok((x + (1_i128 << (s - 1))) >> s),
            "rsr({x},{s})"
        );
    }
}

#[test]
fn isqrt_is_the_exact_floor_square_root() {
    for n in 0..2000_u128 {
        let r = u128::from(isqrt(n));
        assert!(r * r <= n && (r + 1) * (r + 1) > n, "isqrt({n}) = {r}");
    }
    for r in [
        1_u64,
        2,
        3,
        46_340,
        65_535,
        1 << 31,
        (1 << 32) - 1,
        1 << 40,
        u64::MAX - 1,
        u64::MAX,
    ] {
        let sq = u128::from(r) * u128::from(r);
        assert_eq!(isqrt(sq), r, "isqrt({r}^2)");
        assert_eq!(isqrt(sq - 1), r - 1, "isqrt({r}^2 - 1)");
        if r < u64::MAX {
            assert_eq!(isqrt(sq + 2 * u128::from(r)), r, "isqrt({r}^2 + 2r)");
        }
    }
    assert_eq!(isqrt(u128::MAX), u64::MAX);
    assert_eq!(isqrt(0), 0);
    let mut rng = Rng::new(3);
    for _ in 0..20_000 {
        let n = (u128::from(rng.next_u64()) << 64 | u128::from(rng.next_u64())) >> rng.below(120);
        let r = u128::from(isqrt(n));
        assert!(r * r <= n, "isqrt({n}) = {r} too large");
        assert!(
            r == u128::from(u64::MAX) || (r + 1) * (r + 1) > n,
            "isqrt({n}) = {r} too small"
        );
    }
}

#[test]
fn orient2d_is_the_exact_sign_of_the_cross_product() {
    assert_eq!(orient2d([0, 0], [10, 0], [10, 10]), Ok(1)); // counter-clockwise
    assert_eq!(orient2d([0, 0], [10, 0], [10, -10]), Ok(-1));
    assert_eq!(orient2d([0, 0], [10, 10], [20, 20]), Ok(0));
    // exactness where an f64 cross product rounds to zero or flips sign
    let b = [1_000_000_007_i64, 1_000_000_000];
    let c = [2_000_000_014_i64, 2_000_000_001];
    assert_eq!(orient2d([0, 0], b, c), Ok(1)); // cross = 1_000_000_007
    let b = [3_000_000_000_i64, 1_000_000_000];
    assert_eq!(orient2d([0, 0], b, [6_000_000_000, 2_000_000_000]), Ok(0));
    assert_eq!(orient2d([0, 0], b, [6_000_000_000, 2_000_000_001]), Ok(1));
    let mut rng = Rng::new(5);
    for _ in 0..10_000 {
        let r = |rng: &mut Rng| rng.range_i64(-(1 << 40), 1 << 40);
        let (a, b, c) = (
            [r(&mut rng), r(&mut rng)],
            [r(&mut rng), r(&mut rng)],
            [r(&mut rng), r(&mut rng)],
        );
        let cross = i128::from(b[0] - a[0]) * i128::from(c[1] - a[1])
            - i128::from(b[1] - a[1]) * i128::from(c[0] - a[0]);
        assert_eq!(orient2d(a, b, c), Ok(cross.signum() as i8));
    }
    // near-overflow inputs: must be an error or the right sign, never a wrapped answer
    let m = i64::MAX / 2;
    if let Ok(s) = orient2d([-m, -m], [m, -m], [m, m]) {
        assert_eq!(s, 1);
    }
}

fn e(x: i32, y: i32) -> Enu {
    Enu { e_mm: x, n_mm: y }
}

#[test]
fn distances_are_floor_square_roots_of_integer_squares() {
    assert_eq!(dist_mm(e(0, 0), e(3000, 4000)), Ok(5000));
    assert_eq!(dist_mm(e(0, 0), e(1, 1)), Ok(1)); // floor(1.414)
    assert_eq!(dist_mm(e(0, 0), e(2, 2)), Ok(2)); // floor(2.83)
    assert_eq!(dist_mm(e(-5, -5), e(5, 5)), Ok(14));
    assert_eq!(dist_mm(e(7, 7), e(7, 7)), Ok(0));
    // segment, perpendicular case
    assert_eq!(
        dist_point_segment_mm(e(5000, 3000), e(0, 0), e(10_000, 0)),
        Ok(3000)
    );
    // clamped to the end points
    assert_eq!(
        dist_point_segment_mm(e(13_000, 4000), e(0, 0), e(10_000, 0)),
        Ok(5000)
    );
    assert_eq!(
        dist_point_segment_mm(e(-3000, -4000), e(0, 0), e(10_000, 0)),
        Ok(5000)
    );
    // perpendicular case floors: isqrt(cross^2 / len2) with cross = 1 x 2 (point (1,1), seg (0,0)-(2,0))
    assert_eq!(dist_point_segment_mm(e(1, 1), e(0, 0), e(2, 0)), Ok(1));
    // symmetric in the segment direction
    assert_eq!(
        dist_point_segment_mm(e(4000, 9000), e(10_000, 0), e(0, 0)),
        dist_point_segment_mm(e(4000, 9000), e(0, 0), e(10_000, 0))
    );
    // degenerate segment collapses to a point or is rejected, never panics
    let _ = dist_point_segment_mm(e(3, 4), e(0, 0), e(0, 0));
    // extremes: never panic
    for p in [
        e(i32::MIN, i32::MIN),
        e(i32::MAX, i32::MAX),
        e(i32::MIN, i32::MAX),
    ] {
        let _ = dist_mm(p, e(0, 0));
        let _ = dist_mm(p, e(i32::MAX, i32::MIN));
        let _ = dist_point_segment_mm(p, e(i32::MIN, 0), e(i32::MAX, 0));
    }
}

// ---- polygons, circles, classify ---------------------------------------------------------------

fn square() -> Vec<Enu> {
    vec![e(0, 0), e(10_000, 0), e(10_000, 10_000), e(0, 10_000)]
}

#[test]
fn point_in_polygon_inside_boundary_outside() {
    let sq = square();
    assert_eq!(point_in_polygon(&sq, e(5000, 5000)), Ok(PipResult::Inside));
    assert_eq!(
        point_in_polygon(&sq, e(11_000, 5000)),
        Ok(PipResult::Outside)
    );
    assert_eq!(point_in_polygon(&sq, e(-1, 5000)), Ok(PipResult::Outside));
    for b in [
        e(10_000, 5000),
        e(5000, 10_000),
        e(5000, 0),
        e(0, 5000),
        e(0, 0),
        e(10_000, 10_000),
    ] {
        assert_eq!(point_in_polygon(&sq, b), Ok(PipResult::Boundary), "{b:?}");
    }
    // a ray through a vertex: diamond, query level with the left/right vertices
    let diamond = vec![e(0, 10_000), e(10_000, 0), e(0, -10_000), e(-10_000, 0)];
    assert_eq!(
        point_in_polygon(&diamond, e(-20_000, 0)),
        Ok(PipResult::Outside)
    );
    assert_eq!(
        point_in_polygon(&diamond, e(-5000, 0)),
        Ok(PipResult::Inside)
    );
    assert_eq!(
        point_in_polygon(&diamond, e(20_000, 0)),
        Ok(PipResult::Outside)
    );
    assert_eq!(point_in_polygon(&diamond, e(0, 0)), Ok(PipResult::Inside));
    // orientation independence
    let mut rev = sq.clone();
    rev.reverse();
    assert_eq!(point_in_polygon(&rev, e(5000, 5000)), Ok(PipResult::Inside));
    // concave (U shape): the notch is outside
    let u = vec![
        e(0, 0),
        e(30_000, 0),
        e(30_000, 30_000),
        e(20_000, 30_000),
        e(20_000, 10_000),
        e(10_000, 10_000),
        e(10_000, 30_000),
        e(0, 30_000),
    ];
    assert_eq!(
        point_in_polygon(&u, e(15_000, 20_000)),
        Ok(PipResult::Outside)
    );
    assert_eq!(point_in_polygon(&u, e(5000, 20_000)), Ok(PipResult::Inside));
    assert_eq!(
        point_in_polygon(&u, e(25_000, 20_000)),
        Ok(PipResult::Inside)
    );
    assert_eq!(point_in_polygon(&u, e(15_000, 5000)), Ok(PipResult::Inside));
    assert_eq!(
        point_in_polygon(&u, e(15_000, 10_000)),
        Ok(PipResult::Boundary)
    );
}

/// Independent even-odd half-open reference (spec 4): boundary first, then crossing parity.
fn ref_pip(poly: &[Enu], p: Enu) -> PipResult {
    let n = poly.len();
    let (px, py) = (i64::from(p.e_mm), i64::from(p.n_mm));
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (ax, ay, bx, by) = (
            i64::from(a.e_mm),
            i64::from(a.n_mm),
            i64::from(b.e_mm),
            i64::from(b.n_mm),
        );
        let cross = (bx - ax) * (py - ay) - (by - ay) * (px - ax);
        if cross == 0
            && px >= ax.min(bx)
            && px <= ax.max(bx)
            && py >= ay.min(by)
            && py <= ay.max(by)
        {
            return PipResult::Boundary;
        }
    }
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (ax, ay, bx, by) = (
            i64::from(a.e_mm),
            i64::from(a.n_mm),
            i64::from(b.e_mm),
            i64::from(b.n_mm),
        );
        if (ay > py) != (by > py) {
            // x of the intersection compared to px, exactly
            let lhs = (px - ax) * (by - ay);
            let rhs = (bx - ax) * (py - ay);
            let right = if by > ay { lhs < rhs } else { lhs > rhs };
            if right {
                inside = !inside;
            }
        }
    }
    if inside {
        PipResult::Inside
    } else {
        PipResult::Outside
    }
}

#[test]
fn point_in_polygon_matches_an_independent_even_odd_reference_on_a_dense_grid() {
    let mut rng = Rng::new(99);
    let mut boundary_hits = 0;
    for _ in 0..600 {
        let n = 3 + rng.below(6) as usize;
        let poly: Vec<Enu> = (0..n)
            .map(|_| e(rng.range_i64(0, 8) as i32, rng.range_i64(0, 8) as i32))
            .collect();
        // skip degenerate polygons with coincident consecutive vertices only if the engine rejects them
        for x in -1..=9 {
            for y in -1..=9 {
                let p = e(x, y);
                let want = ref_pip(&poly, p);
                if let Ok(got) = point_in_polygon(&poly, p) {
                    assert_eq!(got, want, "poly {poly:?} point {p:?}");
                    boundary_hits += i32::from(want == PipResult::Boundary);
                }
            }
        }
    }
    assert!(
        boundary_hits > 1000,
        "grid must exercise boundaries, got {boundary_hits}"
    );
}

#[test]
fn signed_depths_and_classification() {
    let sq = square();
    // circle: r - dist, floor distance
    assert_eq!(
        signed_depth_circle_mm(e(0, 0), 150_000, e(100_000, 0)),
        Ok(50_000)
    );
    assert_eq!(
        signed_depth_circle_mm(e(0, 0), 150_000, e(200_000, 0)),
        Ok(-50_000)
    );
    assert_eq!(
        signed_depth_circle_mm(e(0, 0), 150_000, e(150_000, 0)),
        Ok(0)
    );
    assert_eq!(
        signed_depth_circle_mm(e(0, 0), 150_000, e(1, 1)),
        Ok(149_999)
    );
    // polygon: + distance to the nearest edge inside, - outside, 0 on the boundary
    assert_eq!(signed_depth_polygon_mm(&sq, e(5000, 5000)), Ok(5000));
    assert_eq!(signed_depth_polygon_mm(&sq, e(2000, 5000)), Ok(2000));
    assert_eq!(signed_depth_polygon_mm(&sq, e(10_000, 5000)), Ok(0));
    assert_eq!(signed_depth_polygon_mm(&sq, e(15_000, 5000)), Ok(-5000));
    assert_eq!(signed_depth_polygon_mm(&sq, e(13_000, 14_000)), Ok(-5000)); // nearest is the corner

    // classify (spec 1.4), k = 1000: Inside iff 1000 d >= k acc; Outside iff 1000 d <= -k acc
    assert_eq!(classify(10_000, 10_000, 1000), Ok(Depth::Inside));
    assert_eq!(classify(9_999, 10_000, 1000), Ok(Depth::Straddle));
    assert_eq!(classify(0, 10_000, 1000), Ok(Depth::Straddle));
    assert_eq!(classify(-9_999, 10_000, 1000), Ok(Depth::Straddle));
    assert_eq!(classify(-10_000, 10_000, 1000), Ok(Depth::Outside));
    assert_eq!(classify(20_000, 10_000, 2000), Ok(Depth::Inside));
    assert_eq!(classify(19_999, 10_000, 2000), Ok(Depth::Straddle));
    assert_eq!(classify(-20_000, 10_000, 2000), Ok(Depth::Outside));
    assert_eq!(classify(5_000, 10_000, 500), Ok(Depth::Inside));
    // extremes never wrap
    for d in [i64::MIN, i64::MAX, 0] {
        for a in [0, 10_000, i64::MAX] {
            let _ = classify(d, a, 1000);
        }
    }
    // acc_eff = max(acc, 10 m) x kappa / 1000
    assert_eq!(acc_eff_mm(5000, 1000), Ok(10_000));
    assert_eq!(acc_eff_mm(0, 1000), Ok(10_000));
    assert_eq!(acc_eff_mm(10_000, 1000), Ok(10_000));
    assert_eq!(acc_eff_mm(50_000, 1000), Ok(50_000));
    assert_eq!(acc_eff_mm(50_000, 1500), Ok(75_000));
    assert_eq!(acc_eff_mm(5000, 2000), Ok(20_000));
}

// ---- polyline ----------------------------------------------------------------------------------

#[test]
fn polyline_validation_length_and_projection() {
    assert!(Polyline::new(&[]).is_err());
    assert!(Polyline::new(&[e(0, 0)]).is_err());
    assert_eq!(
        Polyline::new(&[e(0, 0), e(0, 0)]).err(),
        Some(EngineError::DegenerateGeometry)
    );
    assert_eq!(
        Polyline::new(&[e(0, 0), e(1000, 0), e(1000, 0), e(2000, 0)]).err(),
        Some(EngineError::DegenerateGeometry)
    );
    // 3-4-5 then a 6 m leg
    let pl = Polyline::new(&[e(0, 0), e(3000, 4000), e(3000, 10_000)]).unwrap();
    assert_eq!(pl.length_mm(), 11_000);
    assert_eq!(pl.cum_mm, vec![0, 5000, 11_000]);
    // the route length IS the sum of floor-sqrt segment lengths
    let pl2 = Polyline::new(&[e(0, 0), e(1000, 1000), e(2000, 2000)]).unwrap();
    assert_eq!(pl2.cum_mm, vec![0, 1414, 2828]);
    assert_eq!(pl2.length_mm(), 2828);

    // straight northward route: xt left of travel is positive (west of a north-going route)
    let n = Polyline::new(&[e(0, 0), e(0, 100_000)]).unwrap();
    let p = project_to_polyline(&n, e(-3000, 40_000), None).unwrap();
    assert_eq!((p.seg, p.along_mm, p.dist_mm), (0, 40_000, 3000));
    assert_eq!(p.xt_mm, 3000, "left of travel is positive");
    let p = project_to_polyline(&n, e(3000, 40_000), None).unwrap();
    assert_eq!(p.xt_mm, -3000);
    // beyond the end: along clamps, distance is to the end point (E-G4)
    let p = project_to_polyline(&n, e(3000, 104_000), None).unwrap();
    assert_eq!((p.along_mm, p.dist_mm), (100_000, 5000));
    assert_eq!(p.xt_mm.abs(), p.dist_mm);
    let p = project_to_polyline(&n, e(0, -7000), None).unwrap();
    assert_eq!((p.along_mm, p.dist_mm), (0, 7000));
    // tangent of a north-going segment is north in Q30
    assert_eq!(p.tangent_q30, [0, 1 << 30]);
    // along adds cum[seg]: second leg of the 3-4-5 route
    let p = project_to_polyline(&pl, e(4000, 8000), None).unwrap();
    assert_eq!((p.seg, p.along_mm, p.dist_mm), (1, 5000 + 4000, 1000));
}

#[test]
fn nearest_segment_ties_go_to_the_lowest_index_and_windows_restrict_candidates() {
    // an out-and-back route: both legs are the same distance from a point halfway between them
    let pl = Polyline::new(&[e(0, 0), e(0, 100_000), e(10_000, 100_000), e(10_000, 0)]).unwrap();
    let p = project_to_polyline(&pl, e(5000, 100_000), None).unwrap();
    // the top segment (seg 1) contains the point
    assert_eq!((p.seg, p.dist_mm), (1, 0));
    // equidistant from seg 0 (x=0) and seg 2 (x=10_000): point (5000, 50_000)
    let p = project_to_polyline(&pl, e(5000, 50_000), None).unwrap();
    assert_eq!(p.seg, 0, "tie -> lowest segment index");
    // a window covering only the return leg (along 110_000..210_000) selects seg 2
    let p = project_to_polyline(&pl, e(5000, 50_000), Some((120_000, 200_000))).unwrap();
    assert_eq!(p.seg, 2);
    // inverted window is an error, not a panic
    assert!(project_to_polyline(&pl, e(0, 0), Some((200_000, 100_000))).is_err());
}

#[test]
fn rem_along_and_normalize() {
    assert_eq!(rem_along_mm(100_000, 40_000), Ok(60_000));
    assert_eq!(
        rem_along_mm(100_000, 130_000),
        Ok(-30_000),
        "negative once passed"
    );
    assert_eq!(normalize_q30(0, 0), None);
    assert_eq!(normalize_q30(0, 5), Some([0, 1 << 30]));
    assert_eq!(normalize_q30(-7000, 0), Some([-(1 << 30), 0]));
    // n = isqrt(x^2 + y^2); component = div_round(x << 30, n): (3,4) -> (0.6, 0.8)
    let v = normalize_q30(3, 4).unwrap();
    assert_eq!(v, [644_245_094, 858_993_459]);
    let v = normalize_q30(-3, 4).unwrap();
    assert_eq!(v, [-644_245_094, 858_993_459]);
    // floor sqrt effect: (1,1) has n = 1, so the "unit" vector is (2^30, 2^30)
    assert_eq!(normalize_q30(1, 1), Some([1 << 30, 1 << 30]));
    // extremes never panic
    let _ = normalize_q30(i64::MAX, i64::MAX);
    let _ = normalize_q30(i64::MIN, i64::MIN);
}
