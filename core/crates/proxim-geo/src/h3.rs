//! H3 cells via `h3o` (spec 4, 2.3). Second (and last) module permitted to touch floats.
//!
//! The only float operations are the e7 -> degrees division on the way in, `floor(deg * 1e7 + 0.5)`
//! on the way out, and the `h3o` calls; everything that decides anything (containment, distances)
//! is integer ENU millimetres through [`crate::Frame`]. Only integer cell indices leave this
//! module. A cell that leaves the device MUST be resolution <= 5 (I2): see [`MAX_EXPORT_RES`] and
//! [`is_exportable`].
#![allow(clippy::float_arithmetic)]

use crate::frame::frame_for;
use crate::seg::dist_point_segment_mm;
use crate::shape::signed_depth_polygon_mm;
use alloc::vec::Vec;
use h3o::{CellIndex, LatLng, Resolution};
use proxim_types::{EngineError, Enu, Position};

/// Finest resolution of a cell that may leave the device (I2).
pub const MAX_EXPORT_RES: u8 = 5;
/// Finest H3 resolution.
pub const MAX_RES: u8 = 15;
/// Largest ring count accepted by [`grid_disk`] (30301 cells).
pub const MAX_GRID_DISK_K: u32 = 100;
/// Largest radius accepted by [`cover_circle`]: 50 km.
pub const MAX_COVER_RADIUS_MM: i64 = 50_000_000;
/// Fixed safety margin added to every cover radius: 100 m.
pub const COVER_MARGIN_MM: i64 = 100_000;

fn resolution(res: u8) -> Result<Resolution, EngineError> {
    Resolution::try_from(res).map_err(|_| EngineError::Parse)
}

fn cell(raw: u64) -> Result<CellIndex, EngineError> {
    CellIndex::try_from(raw).map_err(|_| EngineError::Parse)
}

fn latlng(p: Position) -> Result<LatLng, EngineError> {
    if i64::from(p.lat_e7).abs() > 900_000_000 || i64::from(p.lon_e7).abs() > 1_800_000_000 {
        return Err(EngineError::Parse);
    }
    // One IEEE division per axis (spec 2.3).
    LatLng::new(f64::from(p.lat_e7) / 1e7, f64::from(p.lon_e7) / 1e7)
        .map_err(|_| EngineError::Parse)
}

/// `lat_e7 = floor(deg * 1e7 + 0.5)` (spec 1.1), saturating into `i32`.
fn to_e7(deg: f64) -> i32 {
    libm::floor(deg * 1e7 + 0.5) as i32
}

fn position(ll: LatLng) -> Position {
    Position {
        lat_e7: to_e7(ll.lat()),
        lon_e7: to_e7(ll.lng()),
    }
}

/// Cell index containing `p`, resolution 0..=15.
///
/// # Errors
/// [`EngineError::Parse`] for a bad resolution or an invalid coordinate.
pub fn cell_of(p: Position, res: u8) -> Result<u64, EngineError> {
    let r = resolution(res)?;
    Ok(u64::from(latlng(p)?.to_cell(r)))
}

/// Resolution of a cell index (0..=15).
///
/// # Errors
/// [`EngineError::Parse`] if `raw` is not a valid cell index.
pub fn cell_resolution(raw: u64) -> Result<u8, EngineError> {
    Ok(u8::from(cell(raw)?.resolution()))
}

/// True if `raw` is a valid cell of resolution <= [`MAX_EXPORT_RES`], the only kind that may leave
/// the device (I2).
#[must_use]
pub fn is_exportable(raw: u64) -> bool {
    cell_resolution(raw).is_ok_and(|r| r <= MAX_EXPORT_RES)
}

/// Centre of a cell, rounded to e7.
///
/// # Errors
/// [`EngineError::Parse`] if `raw` is not a valid cell index.
pub fn cell_center(raw: u64) -> Result<Position, EngineError> {
    Ok(position(LatLng::from(cell(raw)?)))
}

/// Boundary vertices of a cell (5 to 10 of them), rounded to e7, in `h3o` order.
///
/// # Errors
/// [`EngineError::Parse`] if `raw` is not a valid cell index.
pub fn cell_boundary(raw: u64) -> Result<Vec<Position>, EngineError> {
    Ok(cell(raw)?
        .boundary()
        .iter()
        .map(|ll| position(*ll))
        .collect())
}

/// Distance in mm from `p` to the nearest edge of the cell boundary (unsigned; 0 on an edge).
/// Edges are straight in the frame centred on `p`; the error against the true geodesic edge is
/// the sagitta (about 1.4 m for a res-5 edge). Used by the SDK to decide whether to prefetch
/// neighbours (within 3 km of an edge).
///
/// # Errors
/// [`EngineError::Parse`] for a bad cell; [`EngineError::UnsupportedLatitude`] above 80 degrees;
/// [`EngineError::OutOfFrame`] if a cell vertex is more than 150 km from `p` (resolution 3 or
/// coarser, mostly).
pub fn dist_to_cell_edge_mm(p: Position, raw: u64) -> Result<i64, EngineError> {
    let frame = frame_for(p)?;
    let poly = project_boundary(&frame, raw)?;
    let origin = Enu { e_mm: 0, n_mm: 0 };
    let mut best: Option<i64> = None;
    for (a, b) in poly
        .iter()
        .copied()
        .zip(poly.iter().copied().cycle().skip(1))
    {
        let d = dist_point_segment_mm(origin, a, b)?;
        best = Some(best.map_or(d, |m| m.min(d)));
    }
    best.ok_or(EngineError::DegenerateGeometry)
}

fn project_boundary(frame: &crate::Frame, raw: u64) -> Result<Vec<Enu>, EngineError> {
    let b = cell_boundary(raw)?;
    frame.project_all(&b)
}

/// Cells within `k` grid rings of `raw`, sorted ascending, no duplicates, including `raw`.
///
/// # Errors
/// [`EngineError::Parse`] for a bad cell; [`EngineError::Limit`] if `k > 100`.
pub fn grid_disk(raw: u64, k: u32) -> Result<Vec<u64>, EngineError> {
    if k > MAX_GRID_DISK_K {
        return Err(EngineError::Limit);
    }
    let mut v: Vec<u64> = cell(raw)?
        .grid_disk::<Vec<CellIndex>>(k)
        .into_iter()
        .map(u64::from)
        .collect();
    v.sort_unstable();
    v.dedup();
    Ok(v)
}

/// Compact regional candidate set: sorted ascending cells of resolution `res` that meet the circle
/// of radius `r_mm` around `center`, plus a margin of [`COVER_MARGIN_MM`] + `r_mm / 16` that
/// absorbs the frame scale error (about `tan(lat) * r / R`, 4.4% at 80 degrees and 50 km) and the
/// straight-edge approximation. A superset of the exact answer by design; a cell whose boundary
/// cannot be projected into the 150 km frame (very coarse resolutions) is included, because it
/// cannot be shown to miss the circle.
///
/// # Errors
/// [`EngineError::Parse`] for a bad resolution, coordinate or negative radius;
/// [`EngineError::Limit`] if `r_mm > 50 km` or the resolution is so fine that more than
/// [`MAX_GRID_DISK_K`] rings would be needed; [`EngineError::UnsupportedLatitude`] above 80 degrees.
pub fn cover_circle(center: Position, r_mm: i64, res: u8) -> Result<Vec<u64>, EngineError> {
    if r_mm < 0 {
        return Err(EngineError::Parse);
    }
    if r_mm > MAX_COVER_RADIUS_MM {
        return Err(EngineError::Limit);
    }
    let rs = resolution(res)?;
    let frame = frame_for(center)?;
    let home = latlng(center)?.to_cell(rs);
    let reach_mm = r_mm
        .checked_add(COVER_MARGIN_MM)
        .and_then(|x| x.checked_add(r_mm / 16))
        .ok_or(EngineError::Overflow)?;
    // Ring k holds cells whose centres are >= 1.5 k e_min from the home centre, and a cell meets the
    // circle only if its centre is within reach + e_max. With e_min = e/2 and e_max = 3e/2 (H3 edge
    // lengths vary far less than that) k = ceil((reach + 2e) / (0.75 e)) is a safe ring count.
    let edge_m = rs.edge_length_m();
    let need = ((reach_mm as f64) / 1000.0 + 2.0 * edge_m) / (0.75 * edge_m);
    if need.is_nan() || need > f64::from(MAX_GRID_DISK_K) {
        return Err(EngineError::Limit);
    }
    let k = libm::ceil(need) as u32;
    let mut out: Vec<u64> = Vec::new();
    let origin = Enu { e_mm: 0, n_mm: 0 };
    for cand in home.grid_disk::<Vec<CellIndex>>(k) {
        let keep = match frame.project_all(&cell_boundary(u64::from(cand))?) {
            Ok(poly) => signed_depth_polygon_mm(&poly, origin)
                .map(|depth| depth >= -reach_mm)
                // Degenerate or extreme geometry: cannot prove a miss, keep it.
                .unwrap_or(true),
            Err(_) => true,
        };
        if keep {
            out.push(u64::from(cand));
        }
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn h3o_links_without_std() {
        // Proves the dependency resolves with default-features = false.
        assert!(h3o::Resolution::try_from(5_u8).is_ok());
    }
}
