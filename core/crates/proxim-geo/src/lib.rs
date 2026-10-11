//! Integer geometry (engine-spec 4). `no_std` + `alloc`. Skeleton: signatures only.
#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod h3;
pub mod prim;
pub mod trig;

use alloc::vec::Vec;
use proxim_types::{EngineError, Enu, Position};

/// Local plane: origin plus mm-per-e7-unit scales in Q32 (spec 1.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    pub origin: Position,
    pub kn_q32: i64,
    pub ke_q32: i64,
}

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

/// Polyline with cumulative lengths and Q30 tangents (spec 4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Polyline {
    pub vertices: Vec<Enu>,
    pub cum_mm: Vec<i64>,
    pub tangents_q30: Vec<[i32; 2]>,
}

/// Projection of a point onto a polyline (spec 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Projection {
    pub seg: u32,
    pub xt_mm: i64,
    pub along_mm: i64,
    pub dist_mm: i64,
    pub tangent_q30: [i32; 2],
}

/// Frame for an origin (latitude grid lookup).
///
/// # Errors
/// `UnsupportedLatitude` above 80 degrees. Skeleton: `NotImplemented`.
pub fn frame_for(_origin: Position) -> Result<Frame, EngineError> {
    Err(EngineError::NotImplemented)
}

impl Frame {
    /// Project into integer ENU mm.
    ///
    /// # Errors
    /// `OutOfFrame`, `Overflow`. Skeleton: `NotImplemented`.
    pub fn project(&self, _p: Position) -> Result<Enu, EngineError> {
        Err(EngineError::NotImplemented)
    }
}

/// Exact even-odd point-in-polygon with boundary detection.
///
/// # Errors
/// `Overflow`. Skeleton: `NotImplemented`.
pub fn point_in_polygon(_poly: &[Enu], _p: Enu) -> Result<PipResult, EngineError> {
    Err(EngineError::NotImplemented)
}

/// Signed depth: positive inside, negative outside, zero on the boundary.
///
/// # Errors
/// `Overflow`. Skeleton: `NotImplemented`.
pub fn signed_depth_polygon_mm(_poly: &[Enu], _p: Enu) -> Result<i64, EngineError> {
    Err(EngineError::NotImplemented)
}

/// Signed depth for a circle: `r - dist`.
///
/// # Errors
/// `Overflow`. Skeleton: `NotImplemented`.
pub fn signed_depth_circle_mm(_center: Enu, _radius_mm: i64, _p: Enu) -> Result<i64, EngineError> {
    Err(EngineError::NotImplemented)
}

/// Classify a signed depth against `k x acc_eff` (spec 1.4).
///
/// # Errors
/// `Overflow`. Skeleton: `NotImplemented`.
pub fn classify(_depth_mm: i64, _acc_eff_mm: i64, _k_permille: u32) -> Result<Depth, EngineError> {
    Err(EngineError::NotImplemented)
}

impl Polyline {
    /// # Errors
    /// `DegenerateGeometry` for < 2 vertices or zero-length segments. Skeleton: `NotImplemented`.
    pub fn new(_v: &[Enu]) -> Result<Self, EngineError> {
        Err(EngineError::NotImplemented)
    }
}

/// Nearest-segment projection, optionally restricted to an along window.
///
/// # Errors
/// `Overflow`. Skeleton: `NotImplemented`.
pub fn project_to_polyline(
    _pl: &Polyline,
    _p: Enu,
    _window_mm: Option<(i64, i64)>,
) -> Result<Projection, EngineError> {
    Err(EngineError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_returns_typed_error() {
        let p = Position {
            lat_e7: 0,
            lon_e7: 0,
        };
        assert_eq!(frame_for(p), Err(EngineError::NotImplemented));
        assert_eq!(classify(0, 0, 1000), Err(EngineError::NotImplemented));
        assert_eq!(Polyline::new(&[]), Err(EngineError::NotImplemented));
    }
}
