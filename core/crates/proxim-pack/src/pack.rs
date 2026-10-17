//! Pack (tier file) verification (pack-spec 4).

use crate::entry::{VerifiedEntry, verify_entry};
use crate::error::PackError;
use crate::keyset::Keyset;
use crate::manifest::Manifest;
use crate::sha256;
use crate::wire::Reader;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::defaults::MAX_SIGNPOSTS;

const MAGIC: &[u8; 4] = b"PXPK";

/// An entry the verifier dropped, by its position in the pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dropped {
    pub index: u16,
    pub error: PackError,
}

/// The usable entries of a pack, in pack order, and the dropped ones with their reasons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedPack {
    pub cell: u64,
    pub seq: u64,
    pub tier: u8,
    pub entries: Vec<VerifiedEntry>,
    pub dropped: Vec<Dropped>,
}

/// Verify one tier file against its verified manifest.
///
/// # Errors
/// [`PackError::Mismatch`] if the bytes, length, cell, seq or tier differ from the manifest;
/// [`PackError::Malformed`] or [`PackError::Limit`] for a framing error. Entry failures are not
/// errors: they are listed in [`VerifiedPack::dropped`].
pub fn verify_pack(
    manifest: &Manifest,
    tier: u8,
    bytes: &[u8],
    keyset: &Keyset,
    now_ms: i64,
) -> Result<VerifiedPack, PackError> {
    let rec = manifest.tier(tier).ok_or(PackError::Mismatch)?;
    if u32::try_from(bytes.len()).ok() != Some(rec.bytes) || sha256(bytes) != rec.sha256 {
        return Err(PackError::Mismatch);
    }
    let mut r = Reader::new(bytes);
    if r.take(4)? != MAGIC || r.u8()? != 1 {
        return Err(PackError::Malformed);
    }
    if r.u64()? != manifest.cell || r.u64()? != manifest.seq || r.u8()? != tier {
        return Err(PackError::Mismatch);
    }
    let count = r.u16()?;
    if usize::from(count) > MAX_SIGNPOSTS {
        return Err(PackError::Limit);
    }
    let mut raw = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let len = usize::try_from(r.u32()?).map_err(|_| PackError::Limit)?;
        raw.push(r.take(len)?);
    }
    r.finish()?;

    let mut results: Vec<Result<VerifiedEntry, PackError>> = raw
        .iter()
        .map(|e| verify_entry(e, keyset, now_ms))
        .collect();

    // Two entries for one signpost revision: both are dropped (pack-spec 5).
    let mut seen: BTreeMap<(String, u32), usize> = BTreeMap::new();
    let mut dup = Vec::new();
    for (i, res) in results.iter().enumerate() {
        if let Ok(e) = res {
            let key = (String::from(e.signpost.id.as_str()), e.signpost.rev);
            if let Some(first) = seen.insert(key, i) {
                dup.push(first);
                dup.push(i);
            }
        }
    }
    for i in dup {
        if let Some(slot) = results.get_mut(i) {
            *slot = Err(PackError::Duplicate);
        }
    }

    let mut entries = Vec::new();
    let mut dropped = Vec::new();
    for (i, res) in results.into_iter().enumerate() {
        match res {
            Ok(e) => entries.push(e),
            Err(error) => dropped.push(Dropped {
                index: u16::try_from(i).map_err(|_| PackError::Limit)?,
                error,
            }),
        }
    }
    Ok(VerifiedPack {
        cell: manifest.cell,
        seq: manifest.seq,
        tier,
        entries,
        dropped,
    })
}
