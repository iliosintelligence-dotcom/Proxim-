//! Verification errors (pack-spec 6).

use core::fmt;

/// Why a keyset, manifest, pack or entry was rejected. Never "partially accepted".
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum PackError {
    Malformed,
    Limit,
    UnknownKid,
    KidRevoked,
    KeyNotValid,
    BadSignature,
    Expired,
    Rollback,
    SeqConflict,
    Mismatch,
    Duplicate,
}

impl PackError {
    /// Stable wire string.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Malformed => "P_MALFORMED",
            Self::Limit => "P_LIMIT",
            Self::UnknownKid => "P_UNKNOWN_KID",
            Self::KidRevoked => "P_KID_REVOKED",
            Self::KeyNotValid => "P_KEY_NOT_VALID",
            Self::BadSignature => "P_BAD_SIGNATURE",
            Self::Expired => "P_EXPIRED",
            Self::Rollback => "P_ROLLBACK",
            Self::SeqConflict => "P_SEQ_CONFLICT",
            Self::Mismatch => "P_MISMATCH",
            Self::Duplicate => "P_DUPLICATE",
        }
    }
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl core::error::Error for PackError {}

impl From<proxim_types::EngineError> for PackError {
    fn from(e: proxim_types::EngineError) -> Self {
        match e {
            proxim_types::EngineError::Limit => Self::Limit,
            _ => Self::Malformed,
        }
    }
}
