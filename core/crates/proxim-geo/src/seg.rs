//! Point-to-segment distance shared by polygons and polylines (spec 1.3).

use crate::prim::{dist2, isqrt};
use proxim_types::{EngineError, Enu};

/// Where on a segment the nearest point lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Foot {
    /// Nearest point is the start vertex.
    Start,
    /// Nearest point is the end vertex.
    End,
    /// Nearest point is interior: `dot` (>0, <len2) and `len2` of the segment.
    Interior { dot: u128, len2: u128 },
}

/// Exact-integer squared distance from `p` to segment `a -> b`, plus the foot classification.
///
/// Clamped cases use `dx^2 + dy^2`; the perpendicular case uses `floor(cross^2 / len2)`, whose
/// floor square root equals the floor of the true distance. A zero-length segment is a point.
pub(crate) fn seg_dist2_foot(p: Enu, a: Enu, b: Enu) -> Result<(u128, Foot), EngineError> {
    let abx = i128::from(b.e_mm) - i128::from(a.e_mm);
    let aby = i128::from(b.n_mm) - i128::from(a.n_mm);
    let apx = i128::from(p.e_mm) - i128::from(a.e_mm);
    let apy = i128::from(p.n_mm) - i128::from(a.n_mm);
    let len2 = (abx * abx + aby * aby).unsigned_abs();
    let dot = abx * apx + aby * apy;
    if len2 == 0 || dot <= 0 {
        return Ok((dist2(p, a), Foot::Start));
    }
    let dot_u = dot.unsigned_abs();
    if dot_u >= len2 {
        return Ok((dist2(p, b), Foot::End));
    }
    let cross = abx * apy - aby * apx;
    let c = cross.unsigned_abs();
    let c2 = c.checked_mul(c).ok_or(EngineError::Overflow)?;
    Ok((c2 / len2, Foot::Interior { dot: dot_u, len2 }))
}

/// Squared distance only.
pub(crate) fn seg_dist2(p: Enu, a: Enu, b: Enu) -> Result<u128, EngineError> {
    seg_dist2_foot(p, a, b).map(|(d2, _)| d2)
}

/// Distance from `p` to the segment `a -> b` in mm (floor).
///
/// # Errors
/// [`EngineError::Overflow`] only for coordinates so extreme that `cross^2` exceeds `u128`
/// (spans beyond about 4e9 mm); engine geometry is within 150 km of the frame origin.
pub fn dist_point_segment_mm(p: Enu, a: Enu, b: Enu) -> Result<i64, EngineError> {
    let d2 = seg_dist2(p, a, b)?;
    i64::try_from(isqrt(d2)).map_err(|_| EngineError::Overflow)
}
