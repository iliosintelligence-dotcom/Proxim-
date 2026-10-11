//! Speed filter, heading filter, path tiers and time to anchor (engine-spec 5). Skeleton.
#![no_std]

#[cfg(test)]
extern crate std;

use proxim_types::{EngineError, Fix, Mem, PathInfo};

/// Output of one `path_update` call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathSummary {
    pub info: PathInfo,
    /// Effective speed after the mode floor (spec 5.1).
    pub v_eff_mmps: u32,
    /// Heading unit vector (east, north), Q30, when valid.
    pub heading_q30: Option<[i32; 2]>,
}

/// Update filters with an accepted fix (spec 5).
///
/// # Errors
/// Skeleton: `NotImplemented`.
pub fn path_update(_mem: &Mem, _fix: &Fix) -> Result<(Mem, PathSummary), EngineError> {
    Err(EngineError::NotImplemented)
}

/// Dead-reckoned remaining distance (spec 5.4), floor 0.
///
/// # Errors
/// `Overflow`. Skeleton: `NotImplemented`.
pub fn d_rem_mm(_rem_along_mm: i64, _v_eff_mmps: u32, _age_ms: i64) -> Result<i64, EngineError> {
    Err(EngineError::NotImplemented)
}

/// Time to anchor in ms (spec 5.4), `v_eff >= 700`.
///
/// # Errors
/// `Overflow`. Skeleton: `NotImplemented`.
pub fn t_hat_ms(_d_rem_mm: i64, _v_eff_mmps: u32) -> Result<i64, EngineError> {
    Err(EngineError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_returns_typed_error() {
        assert_eq!(t_hat_ms(1000, 1000), Err(EngineError::NotImplemented));
        assert_eq!(d_rem_mm(1000, 1000, 0), Err(EngineError::NotImplemented));
    }
}
