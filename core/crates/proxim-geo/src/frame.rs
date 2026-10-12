//! Frame and projection (spec 1.2): e7 degrees -> integer ENU millimetres around a signpost
//! origin. The origin is a descriptor field, never derived from the fix.

use crate::prim::{div_round, div_round_i128, rsr};
use crate::trig;
use alloc::vec::Vec;
use proxim_types::defaults::{MAX_ABS_LAT_E7, MAX_FRAME_MM};
use proxim_types::{EngineError, Enu, Position};

/// 180 degrees in e7 units.
const LON_HALF_E7: i64 = 1_800_000_000;
/// 360 degrees in e7 units.
const LON_FULL_E7: i64 = 3_600_000_000;
/// 90 degrees in e7 units.
const LAT_POLE_E7: i64 = 900_000_000;

/// Local plane: origin plus mm-per-e7-unit scales in Q32 (spec 1.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    pub origin: Position,
    pub kn_q32: i64,
    pub ke_q32: i64,
}

/// Wrap a longitude difference (e7) into `[-1.8e9, 1.8e9)`.
fn wrap_dlon(d: i64) -> i64 {
    if d >= LON_HALF_E7 {
        d - LON_FULL_E7
    } else if d < -LON_HALF_E7 {
        d + LON_FULL_E7
    } else {
        d
    }
}

/// Frame for an origin: `lat_q = div_round(lat_e7, 100000)` indexes the scale table.
///
/// The latitude limit is applied to the raw origin (`|lat_e7| > 800_000_000` is unsupported), which
/// is stricter than testing `lat_q` alone (spec 1.2 reads either way).
///
/// # Errors
/// [`EngineError::UnsupportedLatitude`] above 80 degrees; [`EngineError::Parse`] for a longitude
/// outside +-180 degrees.
pub fn frame_for(origin: Position) -> Result<Frame, EngineError> {
    if origin.lat_e7.unsigned_abs() > MAX_ABS_LAT_E7.unsigned_abs() {
        return Err(EngineError::UnsupportedLatitude);
    }
    if i64::from(origin.lon_e7).abs() > LON_HALF_E7 {
        return Err(EngineError::Parse);
    }
    let lat_q = div_round(i64::from(origin.lat_e7), 100_000)?;
    let lat_q = i32::try_from(lat_q).map_err(|_| EngineError::UnsupportedLatitude)?;
    let (kn_q32, ke_q32) = trig::scale_for_lat_q(lat_q)?;
    Ok(Frame {
        origin,
        kn_q32,
        ke_q32,
    })
}

impl Frame {
    /// Project into integer ENU mm: `e = rsr(dlon * ke_q32, 32)`, `n = rsr(dlat * kn_q32, 32)`
    /// with `dlon` wrapped across the antimeridian.
    ///
    /// # Errors
    /// [`EngineError::OutOfFrame`] if `p` is not a valid coordinate or either result exceeds
    /// 150 km. Never panics or wraps.
    pub fn project(&self, p: Position) -> Result<Enu, EngineError> {
        if i64::from(p.lat_e7).abs() > LAT_POLE_E7 || i64::from(p.lon_e7).abs() > LON_HALF_E7 {
            return Err(EngineError::OutOfFrame);
        }
        let dlat = i64::from(p.lat_e7) - i64::from(self.origin.lat_e7);
        let dlon = wrap_dlon(i64::from(p.lon_e7) - i64::from(self.origin.lon_e7));
        let e = scaled(dlon, self.ke_q32)?;
        let n = scaled(dlat, self.kn_q32)?;
        Ok(Enu { e_mm: e, n_mm: n })
    }

    /// Project a vertex list (route or polygon) in order.
    ///
    /// # Errors
    /// The first error of [`Frame::project`].
    pub fn project_all(&self, ps: &[Position]) -> Result<Vec<Enu>, EngineError> {
        ps.iter().map(|p| self.project(*p)).collect()
    }

    /// Inverse of [`Frame::project`] (tools and tests only; the engine never needs it). Round trips
    /// to within one e7 unit.
    ///
    /// # Errors
    /// [`EngineError::OutOfFrame`] if the result is not a valid coordinate.
    pub fn unproject(&self, q: Enu) -> Result<Position, EngineError> {
        let dn = div_round_i128(i128::from(q.n_mm) << 32, i128::from(self.kn_q32))?;
        let de = div_round_i128(i128::from(q.e_mm) << 32, i128::from(self.ke_q32))?;
        let lat = i128::from(self.origin.lat_e7) + dn;
        let mut lon = i128::from(self.origin.lon_e7) + de;
        if lon > i128::from(LON_HALF_E7) {
            lon -= i128::from(LON_FULL_E7);
        } else if lon < -i128::from(LON_HALF_E7) {
            lon += i128::from(LON_FULL_E7);
        }
        if lat.abs() > i128::from(LAT_POLE_E7) || lon.abs() > i128::from(LON_HALF_E7) {
            return Err(EngineError::OutOfFrame);
        }
        Ok(Position {
            lat_e7: i32::try_from(lat).map_err(|_| EngineError::OutOfFrame)?,
            lon_e7: i32::try_from(lon).map_err(|_| EngineError::OutOfFrame)?,
        })
    }
}

/// `rsr(delta * k_q32, 32)` bounded to the 150 km frame.
fn scaled(delta_e7: i64, k_q32: i64) -> Result<i32, EngineError> {
    let prod = i128::from(delta_e7)
        .checked_mul(i128::from(k_q32))
        .ok_or(EngineError::Overflow)?;
    let mm = rsr(prod, 32)?;
    if mm.abs() > i128::from(MAX_FRAME_MM) {
        return Err(EngineError::OutOfFrame);
    }
    i32::try_from(mm).map_err(|_| EngineError::OutOfFrame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_dlon_is_half_open_at_180_degrees() {
        assert_eq!(wrap_dlon(0), 0);
        assert_eq!(wrap_dlon(1_799_999_999), 1_799_999_999);
        assert_eq!(
            wrap_dlon(1_800_000_000),
            -1_800_000_000,
            "+180 wraps to -180: [-1.8e9, 1.8e9)"
        );
        assert_eq!(wrap_dlon(-1_800_000_000), -1_800_000_000, "-180 stays");
        assert_eq!(wrap_dlon(-1_800_000_001), 1_799_999_999);
        assert_eq!(wrap_dlon(3_600_000_000), 0);
        assert_eq!(wrap_dlon(-3_600_000_000), 0);
        assert_eq!(wrap_dlon(3_599_998_000), -2_000);
        assert_eq!(wrap_dlon(-3_599_998_000), 2_000);
    }
}
