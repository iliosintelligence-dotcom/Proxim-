//! Polylines: cumulative length, Q30 tangents, projection with signed cross-track (spec 4).

use crate::prim::{div_round_i128, isqrt, orient_enu};
use crate::seg::{Foot, seg_dist2_foot};
use alloc::vec::Vec;
use proxim_types::defaults::MAX_ROUTE_VERTICES;
use proxim_types::{EngineError, Enu};

/// Polyline with cumulative lengths and Q30 tangents (spec 4).
///
/// Invariants (established by [`Polyline::new`]): `vertices.len() >= 2`;
/// `cum_mm.len() == vertices.len()`, `cum_mm[0] == 0`, `cum_mm[i+1] - cum_mm[i]` is
/// `isqrt(len2)` of segment `i` (>= 1), so `cum_mm.last()` IS the route length;
/// `tangents_q30.len() == vertices.len() - 1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Polyline {
    pub vertices: Vec<Enu>,
    pub cum_mm: Vec<i64>,
    pub tangents_q30: Vec<[i32; 2]>,
}

/// Projection of a point onto a polyline (spec 4).
///
/// `xt_mm` is signed (left of travel positive) and `|xt_mm| == dist_mm`: it is the distance to the
/// nearest point of the nearest segment, which equals the perpendicular distance except beyond a
/// segment end, where it is larger (a point 1 km past the end of a route is 1 km off it, not 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Projection {
    pub seg: u32,
    pub xt_mm: i64,
    pub along_mm: i64,
    pub dist_mm: i64,
    pub tangent_q30: [i32; 2],
}

/// Unit vector in Q30 of `(x, y)`: `n = isqrt(x^2 + y^2)`, `div_round(x << 30, n)`; `None` if
/// `n == 0`. Inputs up to the full `i64` range are supported.
#[must_use]
pub fn normalize_q30(x: i64, y: i64) -> Option<[i32; 2]> {
    let xx = i128::from(x);
    let yy = i128::from(y);
    // |x|,|y| <= 2^63 so each square is at most 2^126 and the sum at most 2^127: that does not fit
    // i128 (max 2^127 - 1) but does fit u128, so square the magnitudes in unsigned arithmetic.
    let (ux, uy) = (xx.unsigned_abs(), yy.unsigned_abs());
    let n = isqrt(ux * ux + uy * uy);
    if n == 0 {
        return None;
    }
    let n = i128::from(n);
    // |x << 30| <= 2^93: no overflow; the quotient is within [-2^30, 2^30].
    let cx = div_round_i128(xx << 30, n).ok()?;
    let cy = div_round_i128(yy << 30, n).ok()?;
    Some([i32::try_from(cx).ok()?, i32::try_from(cy).ok()?])
}

/// Dot product of two Q30 vectors, returned in Q30: `rsr(a.x*b.x + a.y*b.y, 30)`.
/// For unit vectors the result is in `[-2^30, 2^30]` and compares directly with
/// `trig::cos_tol_q30`.
#[must_use]
pub fn dot_q30(a: [i32; 2], b: [i32; 2]) -> i64 {
    let s = i128::from(a[0]) * i128::from(b[0]) + i128::from(a[1]) * i128::from(b[1]);
    // |s| <= 2^63, so adding 2^29 cannot overflow i128.
    let r = (s + (1_i128 << 29)) >> 30;
    i64::try_from(r).unwrap_or(if r < 0 { i64::MIN } else { i64::MAX })
}

/// `anchor - along` (negative once the anchor has been passed).
///
/// # Errors
/// [`EngineError::Overflow`].
pub fn rem_along_mm(anchor_mm: i64, along_mm: i64) -> Result<i64, EngineError> {
    anchor_mm.checked_sub(along_mm).ok_or(EngineError::Overflow)
}

impl Polyline {
    /// Build from projected vertices.
    ///
    /// # Errors
    /// [`EngineError::DegenerateGeometry`] for fewer than 2 vertices or any zero-length segment
    /// (consecutive duplicate vertices); [`EngineError::Limit`] above 1024 vertices;
    /// [`EngineError::Overflow`] if the total length does not fit `i64`.
    pub fn new(v: &[Enu]) -> Result<Self, EngineError> {
        if v.len() < 2 {
            return Err(EngineError::DegenerateGeometry);
        }
        if v.len() > MAX_ROUTE_VERTICES {
            return Err(EngineError::Limit);
        }
        let mut cum_mm: Vec<i64> = Vec::with_capacity(v.len());
        let mut tangents_q30: Vec<[i32; 2]> = Vec::with_capacity(v.len().saturating_sub(1));
        let mut total: i64 = 0;
        cum_mm.push(0);
        for (a, b) in v.iter().zip(v.iter().skip(1)) {
            let dx = i64::from(b.e_mm) - i64::from(a.e_mm);
            let dy = i64::from(b.n_mm) - i64::from(a.n_mm);
            let len2 =
                (i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy)).unsigned_abs();
            if len2 == 0 {
                return Err(EngineError::DegenerateGeometry);
            }
            let len = i64::try_from(isqrt(len2)).map_err(|_| EngineError::Overflow)?;
            total = total.checked_add(len).ok_or(EngineError::Overflow)?;
            cum_mm.push(total);
            tangents_q30.push(normalize_q30(dx, dy).ok_or(EngineError::DegenerateGeometry)?);
        }
        Ok(Self {
            vertices: v.to_vec(),
            cum_mm,
            tangents_q30,
        })
    }

    /// Route length in mm (sum of the floor segment lengths).
    #[must_use]
    pub fn length_mm(&self) -> i64 {
        self.cum_mm.last().copied().unwrap_or(0)
    }

    /// Number of segments.
    #[must_use]
    pub fn segments(&self) -> usize {
        self.vertices.len().saturating_sub(1)
    }
}

/// Nearest-segment projection, optionally restricted to an along-route window `[lo, hi]` (mm):
/// only segments whose span `[cum[i], cum[i+1]]` overlaps the window are candidates. The nearest
/// segment is the one with the smallest integer squared distance, ties to the lowest index.
/// `along = cum[seg] + clamp(floor(dot * seg_len / len2), 0, seg_len)`.
///
/// # Errors
/// [`EngineError::DegenerateGeometry`] if the polyline is malformed, the window is inverted, or no
/// segment overlaps the window; [`EngineError::Overflow`] for extreme coordinates.
pub fn project_to_polyline(
    pl: &Polyline,
    p: Enu,
    window_mm: Option<(i64, i64)>,
) -> Result<Projection, EngineError> {
    if let Some((lo, hi)) = window_mm
        && lo > hi
    {
        return Err(EngineError::DegenerateGeometry);
    }
    let mut best: Option<(usize, u128, Foot)> = None;
    let segs = pl.vertices.iter().zip(pl.vertices.iter().skip(1));
    let spans = pl.cum_mm.iter().zip(pl.cum_mm.iter().skip(1));
    for (i, ((a, b), (c0, c1))) in segs.zip(spans).enumerate() {
        if let Some((lo, hi)) = window_mm
            && (*c0 > hi || *c1 < lo)
        {
            continue;
        }
        let (d2, foot) = seg_dist2_foot(p, *a, *b)?;
        // Strictly smaller wins, so equal distances keep the lowest segment index.
        if best.is_none_or(|(_, bd2, _)| d2 < bd2) {
            best = Some((i, d2, foot));
        }
    }
    let (seg, d2, foot) = best.ok_or(EngineError::DegenerateGeometry)?;
    let a = *pl
        .vertices
        .get(seg)
        .ok_or(EngineError::DegenerateGeometry)?;
    let b = *pl
        .vertices
        .get(seg.saturating_add(1))
        .ok_or(EngineError::DegenerateGeometry)?;
    let cum0 = *pl.cum_mm.get(seg).ok_or(EngineError::DegenerateGeometry)?;
    let cum1 = *pl
        .cum_mm
        .get(seg.saturating_add(1))
        .ok_or(EngineError::DegenerateGeometry)?;
    let seg_len = cum1 - cum0;
    let t = match foot {
        Foot::Start => 0,
        Foot::End => seg_len,
        Foot::Interior { dot, len2 } => {
            let num = dot
                .checked_mul(u128::try_from(seg_len).map_err(|_| EngineError::Overflow)?)
                .ok_or(EngineError::Overflow)?;
            // floor(dot * seg_len / len2) < seg_len because dot < len2.
            i64::try_from(num / len2)
                .map_err(|_| EngineError::Overflow)?
                .clamp(0, seg_len)
        }
    };
    let dist = i64::try_from(isqrt(d2)).map_err(|_| EngineError::Overflow)?;
    // Left of travel (counter-clockwise from the direction a -> b) is positive; on the line, +.
    let xt = if orient_enu(a, b, p) < 0 { -dist } else { dist };
    Ok(Projection {
        seg: u32::try_from(seg).map_err(|_| EngineError::Overflow)?,
        xt_mm: xt,
        along_mm: cum0.checked_add(t).ok_or(EngineError::Overflow)?,
        dist_mm: dist,
        tangent_q30: *pl
            .tangents_q30
            .get(seg)
            .ok_or(EngineError::DegenerateGeometry)?,
    })
}
