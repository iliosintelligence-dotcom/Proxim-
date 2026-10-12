//! The ONLY module (with `h3`) permitted to touch floats (spec 2.1-2.2).
//!
//! Two quantized tables, computed on demand with `libm` and converted to integers at once:
//! - `SCALE[lat_q]`, `lat_q` in -8000..=8000 (0.01 degree grid, 16001 entries): mm per e7 unit in
//!   Q32 for north (`kn`) and east (`ke`), WGS84.
//! - `SINCOS[course_ddeg]`, 0..=3599 (0.1 degree grid, 3600 entries): `[east, north] = [sin, cos]`
//!   of the clockwise-from-north course, Q30, round-half-away.
//!
//! `libm` is a pure-Rust software implementation, so a given input gives the same bits on every
//! target. The results are frozen in `core/testdata/golden/{frame_scale,sincos}_table.json`; the
//! test `tables_match_golden_json` fails if a single entry changes. Ports embed those tables or
//! verify their own computation against them exhaustively. Only `libm::{sin, cos, sqrt, floor}`
//! are called, only on the finite quantized domains below.
//!
//! Exact operation order (so a port can reproduce the bits, though the golden digest is the
//! contract): see [`scale_for_lat_q`] and [`unit_from_course`].
#![allow(clippy::float_arithmetic)]

use proxim_types::EngineError;

/// Largest `|lat_q|` (80 degrees).
pub const SCALE_LAT_Q_MAX: i32 = 8000;
/// Number of `SCALE` entries (`lat_q` -8000..=8000).
pub const SCALE_LEN: usize = 16_001;
/// Number of `SINCOS` entries.
pub const SINCOS_LEN: usize = 3600;

const WGS84_A_M: f64 = 6_378_137.0;
const WGS84_INV_F: f64 = 298.257_223_563;
const PI_OVER_180: f64 = core::f64::consts::PI / 180.0;
/// metres per radian -> mm per e7 unit: `pi/180 * 1e-4`.
const MM_PER_E7_PER_M_PER_DEG: f64 = PI_OVER_180 * 1e-4;
const Q32: f64 = 4_294_967_296.0;
const Q30: f64 = 1_073_741_824.0;

/// Round half away from zero (`floor(x + 0.5)` for x >= 0, mirrored below zero), saturating.
fn round_half_away(x: f64) -> i64 {
    if x >= 0.0 {
        libm::floor(x + 0.5) as i64
    } else {
        -(libm::floor(-x + 0.5) as i64)
    }
}

/// `(kn_q32, ke_q32)`: mm per e7 unit, north and east, for a 0.01 degree latitude grid index.
///
/// With `phi = (|lat_q| / 100) * (pi / 180)`, `f = 1 / 298.257223563`, `e2 = f * (2 - f)`,
/// `w = 1 - e2 * sin(phi)^2`, `sw = sqrt(w)`:
/// `M = a * (1 - e2) / (w * sw)`, `N = a / sw`,
/// `kn = M * K`, `ke = N * cos(phi) * K` with `K = (pi / 180) * 1e-4`, and each result is
/// `round_half_away(k * 2^32)`. The table is symmetric in `lat_q` by construction.
///
/// # Errors
/// [`EngineError::UnsupportedLatitude`] if `|lat_q| > 8000`.
pub fn scale_for_lat_q(lat_q: i32) -> Result<(i64, i64), EngineError> {
    if !(-SCALE_LAT_Q_MAX..=SCALE_LAT_Q_MAX).contains(&lat_q) {
        return Err(EngineError::UnsupportedLatitude);
    }
    let phi = (f64::from(lat_q.unsigned_abs()) / 100.0) * PI_OVER_180;
    let f = 1.0 / WGS84_INV_F;
    let e2 = f * (2.0 - f);
    let s = libm::sin(phi);
    let c = libm::cos(phi);
    let w = 1.0 - e2 * s * s;
    let sw = libm::sqrt(w);
    let m = WGS84_A_M * (1.0 - e2) / (w * sw);
    let n = WGS84_A_M / sw;
    let kn = m * MM_PER_E7_PER_M_PER_DEG;
    let ke = n * c * MM_PER_E7_PER_M_PER_DEG;
    Ok((round_half_away(kn * Q32), round_half_away(ke * Q32)))
}

fn q30(x: f64) -> i32 {
    // |x| <= 1 so the product is at most 2^30: the conversion cannot fail.
    i32::try_from(round_half_away(x * Q30)).unwrap_or(0)
}

/// `[east, north]` unit vector in Q30 for a course in 0.1 degree units (0 = north, clockwise).
///
/// The course is reduced to its quadrant (`ddeg / 900`) and a remainder `r` in 0..900; with
/// `theta = (r / 10) * (pi / 180)`, `s = q30(sin theta)`, `c = q30(cos theta)` and
/// `q30(x) = round_half_away(x * 2^30)`, the quadrants map `(east, north)` to
/// `(s, c)`, `(c, -s)`, `(-s, -c)`, `(-c, s)`. Cardinal directions are therefore exact.
///
/// # Errors
/// [`EngineError::Parse`] if `course_ddeg >= 3600` (outside the wire range).
pub fn unit_from_course(course_ddeg: u16) -> Result<[i32; 2], EngineError> {
    if usize::from(course_ddeg) >= SINCOS_LEN {
        return Err(EngineError::Parse);
    }
    let quad = course_ddeg / 900;
    let r = course_ddeg % 900;
    let theta = (f64::from(r) / 10.0) * PI_OVER_180;
    let s = q30(libm::sin(theta));
    let c = q30(libm::cos(theta));
    Ok(match quad {
        0 => [s, c],
        1 => [c, -s],
        2 => [-s, -c],
        _ => [-c, s],
    })
}

/// `cos(tol)` in Q30 for a tolerance in 0.1 degree units: `SINCOS[tol].north` (spec 2.2).
///
/// # Errors
/// [`EngineError::Parse`] if `tol_ddeg > 900`.
pub fn cos_tol_q30(tol_ddeg: u16) -> Result<i32, EngineError> {
    if tol_ddeg > 900 {
        return Err(EngineError::Parse);
    }
    Ok(unit_from_course(tol_ddeg)?[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_matches_independently_computed_values() {
        // Reference values from an independent Python float computation of the same WGS84
        // formulas (see the spec errata); tolerance 2 Q32 LSB covers libm-vs-glibc ulp noise.
        for (lat_q, kn, ke) in [
            (0, 47_491_289_843_i64, 47_811_357_236_i64),
            (4500, 47_730_734_954, 33_864_457_812),
            (6000, 47_851_213_100, 23_965_918_187),
            (7000, 47_915_524_929, 16_400_994_592),
            (8000, 47_957_579_235, 8_329_438_610),
        ] {
            let (gn, ge) = scale_for_lat_q(lat_q).unwrap();
            assert!((gn - kn).abs() <= 2, "kn at {lat_q}: {gn} vs {kn}");
            assert!((ge - ke).abs() <= 2, "ke at {lat_q}: {ge} vs {ke}");
        }
    }

    #[test]
    fn scale_physical_sanity() {
        // One degree of latitude at the equator is 110.574 km, of longitude 111.319 km.
        let (kn, ke) = scale_for_lat_q(0).unwrap();
        let deg_lat_m = (kn as i128 * 10_000_000) >> 32; // mm
        let deg_lon_m = (ke as i128 * 10_000_000) >> 32;
        assert_eq!(deg_lat_m, 110_574_275);
        assert_eq!(deg_lon_m, 111_319_490);
        // Symmetric in latitude.
        for q in [1, 77, 4500, 8000] {
            assert_eq!(scale_for_lat_q(q), scale_for_lat_q(-q));
        }
    }

    #[test]
    fn scale_monotone_in_latitude() {
        let mut prev = scale_for_lat_q(0).unwrap();
        for q in 1..=SCALE_LAT_Q_MAX {
            let cur = scale_for_lat_q(q).unwrap();
            assert!(cur.0 >= prev.0, "kn must not shrink toward the pole at {q}");
            assert!(cur.1 < prev.1, "ke must shrink strictly at {q}");
            prev = cur;
        }
    }

    #[test]
    fn scale_domain_is_exactly_pm_8000() {
        assert!(scale_for_lat_q(8000).is_ok());
        assert!(scale_for_lat_q(-8000).is_ok());
        for bad in [8001, -8001, i32::MAX, i32::MIN, 9000] {
            assert_eq!(scale_for_lat_q(bad), Err(EngineError::UnsupportedLatitude));
        }
    }

    #[test]
    fn sincos_cardinals_are_exact() {
        let one = 1 << 30;
        assert_eq!(unit_from_course(0), Ok([0, one]));
        assert_eq!(unit_from_course(900), Ok([one, 0]));
        assert_eq!(unit_from_course(1800), Ok([0, -one]));
        assert_eq!(unit_from_course(2700), Ok([-one, 0]));
    }

    #[test]
    fn sincos_known_angles() {
        // 30, 45, 60 degrees (independent float reference: 536870912, 759250125, 929887697).
        assert_eq!(unit_from_course(300), Ok([536_870_912, 929_887_697]));
        assert_eq!(unit_from_course(450), Ok([759_250_125, 759_250_125]));
        assert_eq!(unit_from_course(600), Ok([929_887_697, 536_870_912]));
        // North-west is negative east, positive north.
        assert_eq!(unit_from_course(3150), Ok([-759_250_125, 759_250_125]));
        // 0.1 degree: sin = 1.745329e-3 * 2^30 = 1874032.06.
        assert_eq!(unit_from_course(1), Ok([1_874_032, 1_073_740_189]));
    }

    #[test]
    fn sincos_is_unit_length_everywhere() {
        let one2: i128 = 1 << 60;
        for c in 0..3600_u16 {
            let [e, n] = unit_from_course(c).unwrap();
            let len2 = i128::from(e) * i128::from(e) + i128::from(n) * i128::from(n);
            // Each component is within 0.5 LSB, so |len2 - 2^60| <= 2^30 * (|e| + |n|) <= 2^32.
            assert!(
                (len2 - one2).abs() <= 1 << 32,
                "course {c}: {}",
                len2 - one2
            );
        }
    }

    #[test]
    fn sincos_rejects_out_of_range_and_cos_tol_domain() {
        assert_eq!(unit_from_course(3600), Err(EngineError::Parse));
        assert_eq!(unit_from_course(u16::MAX), Err(EngineError::Parse));
        assert_eq!(cos_tol_q30(901), Err(EngineError::Parse));
        assert_eq!(cos_tol_q30(900), Ok(0));
        assert_eq!(cos_tol_q30(0), Ok(1 << 30));
        assert_eq!(cos_tol_q30(450), Ok(759_250_125)); // cos 45 deg
        assert_eq!(cos_tol_q30(300), Ok(929_887_697)); // cos 30 deg: the default bearing_tol
        assert_eq!(cos_tol_q30(150), Ok(1_037_154_959)); // cos 15 deg
    }
}
