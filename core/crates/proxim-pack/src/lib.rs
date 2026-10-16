//! proxim-pack: signed content verification (M2). Normative format: `docs/pack-format.md`.
//!
//! Trust chain: pinned root keys → [`keyset`] → approve and manifest keys → [`manifest`],
//! [`revocation`] lists and every [`pack`] entry. All functions are pure: the caller supplies the
//! bytes, the previously accepted state (rollback guards) and `now`. Any failure is a
//! [`ReasonCode`] from engine spec 8 row 500 (or `NOT_YET_LIVE` for a revision before `live_at`).
#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod b64;
pub mod crypto;
pub mod encode;
pub mod jws;
pub mod keyset;
pub mod manifest;
pub mod pack;
pub mod revocation;
#[cfg(feature = "testkit")]
pub mod testkit;

mod doc;

pub use keyset::{Keyset, Purpose, RootKey};
pub use manifest::{Manifest, PackKind};
pub use pack::{DroppedEntry, OpenedPack, VerifiedEntry, open_pack};
pub use proxim_types::ReasonCode;
pub use revocation::RevocationList;

/// Result of every verification step.
pub type PackResult<T> = Result<T, ReasonCode>;

/// Rollback guard for a versioned document (key set version, revocation seq, manifest seq per
/// cell): the highest number accepted and the SHA-256 of that document's payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guard {
    pub seq: u64,
    pub sha256: [u8; 32],
}

impl Guard {
    /// Accept `seq` with payload hash `sha256` after `prev`: a higher number, or the same number
    /// with the same bytes (a re-fetch). Anything else is `SEQ_ROLLBACK`.
    ///
    /// # Errors
    /// [`ReasonCode::SeqRollback`].
    pub fn check(prev: Option<&Self>, seq: u64, sha256: &[u8; 32]) -> PackResult<Self> {
        match prev {
            Some(p) if seq < p.seq || (seq == p.seq && &p.sha256 != sha256) => {
                Err(ReasonCode::SeqRollback)
            }
            _ => Ok(Self {
                seq,
                sha256: *sha256,
            }),
        }
    }
}

/// `kid`: 1 to 32 characters from `a-z 0-9 -` (pack-format 2).
#[must_use]
pub fn valid_kid(kid: &str) -> bool {
    (1..=32).contains(&kid.len())
        && kid
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_accepts_newer_or_identical_only() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let g = Guard::check(None, 5, &a).unwrap();
        assert_eq!(Guard::check(Some(&g), 5, &a), Ok(g));
        assert_eq!(Guard::check(Some(&g), 5, &b), Err(ReasonCode::SeqRollback));
        assert_eq!(Guard::check(Some(&g), 4, &a), Err(ReasonCode::SeqRollback));
        assert_eq!(Guard::check(Some(&g), 6, &b).unwrap().seq, 6);
    }

    #[test]
    fn kid_charset() {
        assert!(valid_kid("approve-2026-1"));
        assert!(!valid_kid(""));
        assert!(!valid_kid("Approve"));
        assert!(!valid_kid("a_b"));
        assert!(!valid_kid(&"a".repeat(33)));
    }
}
