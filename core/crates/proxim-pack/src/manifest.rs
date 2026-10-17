//! Tile manifest (pack-format 3.3).

use crate::jws::{self, TYP_MANIFEST};
use crate::keyset::{Keyset, Purpose};
use crate::{Guard, PackResult, crypto, doc};
use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::ReasonCode;
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "proxim-manifest/1";
/// Largest pack the verifier will hash or open.
pub const MAX_PACK_BYTES: u32 = 4_194_304;

/// The pipe a pack belongs to. Only `hosted_tile` exists before M15.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackKind {
    HostedTile,
}

impl PackKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::HostedTile => "hosted_tile",
        }
    }
    pub const fn byte(self) -> u8 {
        match self {
            Self::HostedTile => 1,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TierDoc {
    tier: u8,
    bytes: u32,
    sha256: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestDoc {
    format: String,
    cell: String,
    seq: u64,
    pack_kind: String,
    issued_at_ms: i64,
    expires_at_ms: i64,
    tiers: Vec<TierDoc>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tier {
    pub tier: u8,
    pub bytes: u32,
    pub sha256: [u8; 32],
}

/// An accepted manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub cell: u64,
    pub guard: Guard,
    pub pack_kind: PackKind,
    pub expires_at_ms: i64,
    pub tiers: Vec<Tier>,
}

impl Manifest {
    #[must_use]
    pub fn tier(&self, tier: u8) -> Option<&Tier> {
        self.tiers.iter().find(|t| t.tier == tier)
    }
}

/// H3 index as 15 lowercase hex digits.
#[must_use]
pub fn parse_cell(s: &str) -> Option<u64> {
    if s.len() != 15
        || !s
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return None;
    }
    u64::from_str_radix(s, 16).ok()
}

/// Verify a manifest for `cell` against the key set and the cell's previous guard.
///
/// # Errors
/// `SIGNATURE_INVALID`, `KID_REVOKED`, `PACK_EXPIRED`, `SEQ_ROLLBACK`.
pub fn accept_manifest(
    token: &[u8],
    keyset: &Keyset,
    cell: u64,
    prev: Option<&Guard>,
    now_ms: i64,
) -> PackResult<Manifest> {
    let bad = ReasonCode::SignatureInvalid;
    let j = jws::parse(token, TYP_MANIFEST)?;
    let key = keyset.usable(&j.kid, Purpose::Manifest, now_ms)?;
    if !j.verify(key) {
        return Err(bad);
    }
    let d: ManifestDoc = doc::decode(&j.payload)?;
    let pack_kind = match d.pack_kind.as_str() {
        "hosted_tile" => PackKind::HostedTile,
        _ => return Err(bad),
    };
    if d.format != FORMAT || parse_cell(&d.cell) != Some(cell) || !(1..=3).contains(&d.tiers.len())
    {
        return Err(bad);
    }
    let mut tiers = Vec::with_capacity(d.tiers.len());
    for t in &d.tiers {
        let ascending = tiers.last().is_none_or(|p: &Tier| p.tier < t.tier);
        if t.tier > 2 || !ascending || t.bytes > MAX_PACK_BYTES {
            return Err(bad);
        }
        tiers.push(Tier {
            tier: t.tier,
            bytes: t.bytes,
            sha256: crypto::parse_hex(&t.sha256).ok_or(bad)?,
        });
    }
    doc::check_window(d.issued_at_ms, d.expires_at_ms, now_ms)?;
    let guard = Guard::check(prev, d.seq, &j.payload_sha256())?;
    Ok(Manifest {
        cell,
        guard,
        pack_kind,
        expires_at_ms: d.expires_at_ms,
        tiers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells() {
        assert_eq!(parse_cell("85283473fffffff"), Some(0x0852_8347_3fff_ffff));
        assert_eq!(parse_cell("85283473FFFFFFF"), None);
        assert_eq!(parse_cell("085283473fffffff"), None);
    }
}
