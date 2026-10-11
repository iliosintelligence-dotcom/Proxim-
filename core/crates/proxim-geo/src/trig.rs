//! The ONLY module (with `h3`) permitted to touch floats (spec 2.1-2.2). Skeleton.
//!
//! Real implementation: `SCALE[lat_q]` (16001 entries) and `SINCOS[course_ddeg]` (3600 entries,
//! Q30), computed with `libm` on quantized domains and converted to integers at once.
#![allow(clippy::float_arithmetic)]

use proxim_types::EngineError;

/// `(kn_q32, ke_q32)` for a 0.01 degree latitude grid index.
///
/// # Errors
/// `UnsupportedLatitude` beyond 80 degrees. Skeleton: `NotImplemented`.
pub fn scale_for_lat_q(_lat_q: i32) -> Result<(i64, i64), EngineError> {
    Err(EngineError::NotImplemented)
}

/// `(east, north)` unit vector in Q30 for a course in 0.1 degree units.
///
/// # Errors
/// Skeleton: `NotImplemented`.
pub fn unit_from_course(_course_ddeg: u16) -> Result<[i32; 2], EngineError> {
    Err(EngineError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_returns_typed_error() {
        assert_eq!(scale_for_lat_q(4500), Err(EngineError::NotImplemented));
        assert_eq!(unit_from_course(0), Err(EngineError::NotImplemented));
    }
}
