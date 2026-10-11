//! H3 cells via `h3o` (spec 4, 2.3). Second permitted float module. Skeleton.
//! A cell leaving the device MUST be res <= 5 (I2).
#![allow(clippy::float_arithmetic)]

use alloc::vec::Vec;
use proxim_types::{EngineError, Position};

/// Cell index containing `p`, resolution 0..=15.
///
/// # Errors
/// `Parse` for a bad resolution. Skeleton: `NotImplemented`.
pub fn cell_of(_p: Position, _res: u8) -> Result<u64, EngineError> {
    Err(EngineError::NotImplemented)
}

/// Sorted ascending cells within `k` rings.
///
/// # Errors
/// Skeleton: `NotImplemented`.
pub fn grid_disk(_cell: u64, _k: u32) -> Result<Vec<u64>, EngineError> {
    Err(EngineError::NotImplemented)
}

/// Sorted ascending cells meeting a circle (compact regional set).
///
/// # Errors
/// Skeleton: `NotImplemented`.
pub fn cover_circle(_center: Position, _r_mm: i64, _res: u8) -> Result<Vec<u64>, EngineError> {
    Err(EngineError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_returns_typed_error() {
        assert_eq!(grid_disk(0, 1), Err(EngineError::NotImplemented));
    }

    #[test]
    fn h3o_links_without_std() {
        // Proves the dependency resolves with default-features = false.
        assert!(h3o::Resolution::try_from(5_u8).is_ok());
    }
}
