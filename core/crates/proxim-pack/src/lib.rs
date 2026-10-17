//! proxim-pack: verification of signed keysets, manifests, packs and envelope v2 entries
//! (docs/pack-spec.md; invariants I10 and I17).
//!
//! Pure: every function is a function of its arguments. There is no clock (`now_ms` is an
//! argument), no network, no storage and no randomness. Rollback state ([`SeqMark`]) is passed in
//! and handed back so the SDK shell can persist it. Every input is untrusted: no `unwrap`, no
//! unchecked indexing, all lengths checked before use.
//!
//! Order of use: [`verify_keyset`] against the pinned [`TrustRoots`], then [`verify_manifest`] for
//! the device's cell, then [`verify_pack`] for each tier file. A framing error rejects a whole
//! document; a bad entry inside a pack is dropped on its own with a reason.
#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

mod doc;
mod entry;
mod error;
mod es256;
mod hex;
mod keyset;
mod manifest;
mod pack;
mod wire;

pub use entry::{Creative, ShippedCreative, VerifiedEntry, verify_entry};
pub use error::PackError;
pub use es256::PublicKey;
pub use keyset::{Keyset, TrustRoots, verify_keyset};
pub use manifest::{Manifest, TierRecord, verify_manifest};
pub use pack::{Dropped, VerifiedPack, verify_pack};

use sha2::{Digest as _, Sha256};

/// Highest accepted sequence number of a keyset or of one cell's manifest, with the digest of the
/// document that carried it. A later document with a lower `seq` is a rollback; one with the same
/// `seq` must be byte-identical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeqMark {
    pub seq: u64,
    pub digest: [u8; 32],
}

impl SeqMark {
    /// Apply the rollback rule (pack-spec 3.1 and 3.2) for a candidate document.
    ///
    /// # Errors
    /// [`PackError::Rollback`] for a lower `seq`, [`PackError::SeqConflict`] for the same `seq`
    /// with a different digest.
    pub fn admit(last: Option<Self>, next: Self) -> Result<(), PackError> {
        match last {
            Some(prev) if next.seq < prev.seq => Err(PackError::Rollback),
            Some(prev) if next.seq == prev.seq && next.digest != prev.digest => {
                Err(PackError::SeqConflict)
            }
            _ => Ok(()),
        }
    }
}

/// SHA-256 of `bytes`.
#[must_use]
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seq_mark_rules() {
        let a = SeqMark {
            seq: 5,
            digest: [1; 32],
        };
        assert_eq!(SeqMark::admit(None, a), Ok(()));
        assert_eq!(SeqMark::admit(Some(a), a), Ok(()));
        assert_eq!(
            SeqMark::admit(
                Some(a),
                SeqMark {
                    seq: 6,
                    digest: [2; 32]
                }
            ),
            Ok(())
        );
        assert_eq!(
            SeqMark::admit(
                Some(a),
                SeqMark {
                    seq: 4,
                    digest: [1; 32]
                }
            ),
            Err(PackError::Rollback)
        );
        assert_eq!(
            SeqMark::admit(
                Some(a),
                SeqMark {
                    seq: 5,
                    digest: [2; 32]
                }
            ),
            Err(PackError::SeqConflict)
        );
    }

    #[test]
    fn sha256_known_answer() {
        let d = sha256(b"abc");
        assert_eq!(d[0], 0xba);
        assert_eq!(d[31], 0xad);
    }
}
