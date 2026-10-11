//! Engine errors (spec 2.6). A per-signpost defect is a drop reason, never an `Err`.

use core::fmt;

/// Failure of a whole call. A shell MUST treat any `Err` from `decide` as
/// "no decision, state unchanged".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EngineError {
    Parse,
    Overflow,
    OutOfFrame,
    UnsupportedLatitude,
    DegenerateGeometry,
    Limit,
    /// Skeleton marker: the function exists in the contract but is not built yet.
    NotImplemented,
}

impl EngineError {
    /// Stable wire string.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Parse => "E_PARSE",
            Self::Overflow => "E_OVERFLOW",
            Self::OutOfFrame => "E_OUT_OF_FRAME",
            Self::UnsupportedLatitude => "E_UNSUPPORTED_LATITUDE",
            Self::DegenerateGeometry => "E_DEGENERATE_GEOMETRY",
            Self::Limit => "E_LIMIT",
            Self::NotImplemented => "E_NOT_IMPLEMENTED",
        }
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl core::error::Error for EngineError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable() {
        assert_eq!(EngineError::Parse.code(), "E_PARSE");
        assert_eq!(EngineError::NotImplemented.code(), "E_NOT_IMPLEMENTED");
    }
}
