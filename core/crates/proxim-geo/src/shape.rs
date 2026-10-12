//! Polygons and circles: exact point-in-polygon, signed depth, the straddle classifier (spec 1.4, 4).

use crate::prim::{dist2, isqrt, orient_enu};
use crate::seg::seg_dist2;
use proxim_types::defaults::{ACC_FLOOR_MM, MAX_POLYGON_VERTICES};
use proxim_types::{EngineError, Enu};

/// Result of a point-in-polygon test (spec 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipResult {
    Inside,
    Boundary,
    Outside,
}

/// Straddle classification (spec 1.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    Inside,
    Straddle,
    Outside,
}

fn check_polygon(poly: &[Enu]) -> Result<(), EngineError> {
    if poly.len() < 3 {
        return Err(EngineError::DegenerateGeometry);
    }
    if poly.len() > MAX_POLYGON_VERTICES {
        return Err(EngineError::Limit);
    }
    Ok(())
}

/// Edges `(v[i], v[i+1])` with the closing edge `(v[n-1], v[0])`; no indexing.
fn edges(poly: &[Enu]) -> impl Iterator<Item = (Enu, Enu)> + '_ {
    poly.iter()
        .copied()
        .zip(poly.iter().copied().cycle().skip(1))
}

fn on_segment(a: Enu, b: Enu, p: Enu) -> bool {
    orient_enu(a, b, p) == 0
        && p.e_mm >= a.e_mm.min(b.e_mm)
        && p.e_mm <= a.e_mm.max(b.e_mm)
        && p.n_mm >= a.n_mm.min(b.n_mm)
        && p.n_mm <= a.n_mm.max(b.n_mm)
}

/// Exact even-odd point-in-polygon. `poly` has no closing repeat; either winding works; a
/// self-intersecting polygon follows the even-odd rule (the compiler rejects those). `Boundary`
/// (on an edge or vertex) is tested first over all edges, then crossings are counted with the
/// half-open rule (an edge counts when exactly one endpoint is strictly above `p`), so a ray
/// through a vertex is counted once, never twice or zero times.
///
/// # Errors
/// [`EngineError::DegenerateGeometry`] for fewer than 3 vertices, [`EngineError::Limit`] for more
/// than 256.
pub fn point_in_polygon(poly: &[Enu], p: Enu) -> Result<PipResult, EngineError> {
    check_polygon(poly)?;
    if edges(poly).any(|(a, b)| on_segment(a, b, p)) {
        return Ok(PipResult::Boundary);
    }
    let mut inside = false;
    for (a, b) in edges(poly) {
        if (a.n_mm > p.n_mm) != (b.n_mm > p.n_mm) {
            let o = orient_enu(a, b, p);
            // Upward edge: p left of it means the edge is to p's right (the +e ray crosses it).
            let crosses = if b.n_mm > a.n_mm { o > 0 } else { o < 0 };
            if crosses {
                inside = !inside;
            }
        }
    }
    Ok(if inside {
        PipResult::Inside
    } else {
        PipResult::Outside
    })
}

/// Signed depth: `+` distance to the nearest edge (floor mm) if inside, `-` if outside, `0` on the
/// boundary. A strictly interior point closer than 1 mm to an edge also reports 0, which
/// [`classify`] treats as a straddle.
///
/// # Errors
/// As [`point_in_polygon`]; [`EngineError::Overflow`] for coordinates so extreme that the exact
/// squared distance does not fit.
pub fn signed_depth_polygon_mm(poly: &[Enu], p: Enu) -> Result<i64, EngineError> {
    let pip = point_in_polygon(poly, p)?;
    if pip == PipResult::Boundary {
        return Ok(0);
    }
    let mut best: Option<u128> = None;
    for (a, b) in edges(poly) {
        let d2 = seg_dist2(p, a, b)?;
        best = Some(best.map_or(d2, |m| m.min(d2)));
    }
    let d = i64::try_from(isqrt(best.ok_or(EngineError::DegenerateGeometry)?))
        .map_err(|_| EngineError::Overflow)?;
    Ok(if pip == PipResult::Inside { d } else { -d })
}

/// Signed depth for a circle: `r - dist(center, p)` (negative outside).
///
/// # Errors
/// [`EngineError::DegenerateGeometry`] for a negative radius; [`EngineError::Overflow`] if the
/// difference does not fit `i64`.
pub fn signed_depth_circle_mm(center: Enu, radius_mm: i64, p: Enu) -> Result<i64, EngineError> {
    if radius_mm < 0 {
        return Err(EngineError::DegenerateGeometry);
    }
    let d = i64::try_from(isqrt(dist2(center, p))).map_err(|_| EngineError::Overflow)?;
    radius_mm.checked_sub(d).ok_or(EngineError::Overflow)
}

/// `acc_eff = max(acc_mm, 10_000) x kappa_permille / 1000` (spec 1.4), floor division.
///
/// # Errors
/// [`EngineError::Overflow`] if the result does not fit `i64` (unreachable for `u32` inputs).
pub fn acc_eff_mm(acc_mm: u32, kappa_permille: u32) -> Result<i64, EngineError> {
    let floor = acc_mm.max(ACC_FLOOR_MM);
    let prod = u64::from(floor)
        .checked_mul(u64::from(kappa_permille))
        .ok_or(EngineError::Overflow)?;
    i64::try_from(prod / 1000).map_err(|_| EngineError::Overflow)
}

/// Classify a signed depth against `k x acc_eff` (spec 1.4): `Inside` iff `1000 d >= k acc_eff`,
/// `Outside` iff `1000 d <= -k acc_eff`, else `Straddle`.
///
/// When both inequalities hold (`d == 0` and `k x acc_eff == 0`: zero accuracy exactly on the
/// boundary) the spec's literal order would say `Inside`; this function returns `Straddle`, the
/// fail-closed reading. The engine floors accuracy at 10 m, so the case is unreachable from
/// `decide`.
///
/// # Errors
/// [`EngineError::Parse`] for a negative `acc_eff_mm`; [`EngineError::Overflow`] cannot occur for
/// `i64` depth (products are formed in `i128`) but is kept for signature stability.
pub fn classify(depth_mm: i64, acc_eff_mm: i64, k_permille: u32) -> Result<Depth, EngineError> {
    if acc_eff_mm < 0 {
        return Err(EngineError::Parse);
    }
    let lhs = i128::from(depth_mm)
        .checked_mul(1000)
        .ok_or(EngineError::Overflow)?;
    let rhs = i128::from(k_permille)
        .checked_mul(i128::from(acc_eff_mm))
        .ok_or(EngineError::Overflow)?;
    let inside = lhs >= rhs;
    let outside = lhs <= -rhs;
    Ok(match (inside, outside) {
        (true, false) => Depth::Inside,
        (false, true) => Depth::Outside,
        _ => Depth::Straddle,
    })
}
