//! Manifest verification (pack-spec 3.2): one per H3 res-5 cell, signed by an online key.

use crate::doc::{self, KIND_MANIFEST};
use crate::error::PackError;
use crate::hex;
use crate::keyset::Keyset;
use crate::{SeqMark, sha256};
use alloc::vec::Vec;
use proxim_types::strict::from_canonical_json;
use serde::{Deserialize, Serialize};

/// PLAN 5.2 hard TTL: 72 hours.
pub const MAX_MANIFEST_TTL_MS: i64 = 72 * 3_600_000;
/// Largest tier file accepted (pack-spec 3.2).
pub const MAX_TIER_BYTES: u32 = 8 << 20;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TierWire {
    bytes: u32,
    sha256: alloc::string::String,
    tier: u8,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestPayload {
    built_ms: i64,
    cell: u64,
    expires_ms: i64,
    seq: u64,
    tiers: Vec<TierWire>,
    v: u32,
}

/// One tier file the manifest vouches for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TierRecord {
    pub tier: u8,
    pub bytes: u32,
    pub sha256: [u8; 32],
}

/// A verified manifest for one cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub cell: u64,
    pub seq: u64,
    pub built_ms: i64,
    pub expires_ms: i64,
    pub tiers: Vec<TierRecord>,
    digest: [u8; 32],
}

impl Manifest {
    /// The rollback mark to persist for this cell after accepting the manifest.
    #[must_use]
    pub const fn mark(&self) -> SeqMark {
        SeqMark {
            seq: self.seq,
            digest: self.digest,
        }
    }

    /// The record for `tier`, if the manifest lists it.
    #[must_use]
    pub fn tier(&self, tier: u8) -> Option<&TierRecord> {
        self.tiers.iter().find(|t| t.tier == tier)
    }
}

/// Verify the manifest for `cell` (the cell the device asked for).
///
/// # Errors
/// Any [`PackError`]; see pack-spec 3.2.
pub fn verify_manifest(
    keyset: &Keyset,
    bytes: &[u8],
    cell: u64,
    now_ms: i64,
    last: Option<SeqMark>,
) -> Result<Manifest, PackError> {
    let d = doc::parse(bytes, KIND_MANIFEST)?;
    keyset
        .key_for(d.kid, now_ms)?
        .verify(&d.signed_msg, &d.sig)?;
    let p: ManifestPayload = from_canonical_json(d.payload)?;
    if p.v != 1 || p.built_ms > p.expires_ms || p.tiers.is_empty() || p.tiers.len() > 3 {
        return Err(PackError::Malformed);
    }
    if p.expires_ms.saturating_sub(p.built_ms) > MAX_MANIFEST_TTL_MS {
        return Err(PackError::Malformed);
    }
    if p.cell != cell {
        return Err(PackError::Mismatch);
    }
    let mut tiers = Vec::with_capacity(p.tiers.len());
    for (i, t) in p.tiers.iter().enumerate() {
        if usize::from(t.tier) != i {
            return Err(PackError::Malformed);
        }
        if t.bytes > MAX_TIER_BYTES {
            return Err(PackError::Limit);
        }
        tiers.push(TierRecord {
            tier: t.tier,
            bytes: t.bytes,
            sha256: hex::decode_array(&t.sha256)?,
        });
    }
    let digest = sha256(bytes);
    SeqMark::admit(last, SeqMark { seq: p.seq, digest })?;
    if now_ms < p.built_ms || now_ms > p.expires_ms {
        return Err(PackError::Expired);
    }
    Ok(Manifest {
        cell: p.cell,
        seq: p.seq,
        built_ms: p.built_ms,
        expires_ms: p.expires_ms,
        tiers,
        digest,
    })
}
