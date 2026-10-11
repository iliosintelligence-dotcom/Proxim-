//! Integer primitives (spec 1.3). Skeleton.

use proxim_types::EngineError;

/// `floor((2a + b) / (2b))` for `b > 0`.
///
/// # Errors
/// `Overflow`. Skeleton: `NotImplemented`.
pub fn div_round(_a: i64, _b: i64) -> Result<i64, EngineError> {
    Err(EngineError::NotImplemented)
}

/// Exact floor square root.
#[must_use]
pub fn isqrt(_n: u128) -> u64 {
    0
}

/// `orient2d(a,b,c)`: exact cross-product sign.
///
/// # Errors
/// `Overflow`. Skeleton: `NotImplemented`.
pub fn orient2d(_a: [i64; 2], _b: [i64; 2], _c: [i64; 2]) -> Result<i8, EngineError> {
    Err(EngineError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_returns_typed_error() {
        assert_eq!(div_round(1, 2), Err(EngineError::NotImplemented));
        assert_eq!(
            orient2d([0, 0], [1, 0], [0, 1]),
            Err(EngineError::NotImplemented)
        );
    }
}
