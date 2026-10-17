//! Envelope v2 entries (pack-spec 5, A02 4.3): one signpost family, verified on its own.

use crate::error::PackError;
use crate::keyset::Keyset;
use crate::wire::Reader;
use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::Signpost;
use proxim_types::strict::from_canonical_json;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

const ENV_DOMAIN: &[u8] = b"PROXIM-ENV-V2\x00";
const CREATIVE_DOMAIN: &[u8] = b"PROXIM-CREATIVE-V2\x00";
/// Most creatives declared on the wire (A02 4.3; v1 compiles at most 6).
pub const MAX_CREATIVES: u8 = 8;
const MAX_HEADLINE_CHARS: usize = 40;
const MAX_BODY_CHARS: usize = 90;
const MAX_GLANCEABLE_WORDS: usize = 7;

/// One wording of a family (A02 3.3, text only in v1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Creative {
    pub body: String,
    pub glanceable: String,
    pub headline: String,
}

impl Creative {
    fn check(&self) -> Result<(), PackError> {
        let headline = self.headline.chars().count();
        let words = self.glanceable.split_whitespace().count();
        if headline == 0
            || headline > MAX_HEADLINE_CHARS
            || self.body.chars().count() > MAX_BODY_CHARS
            || words == 0
            || words > MAX_GLANCEABLE_WORDS
        {
            return Err(PackError::Malformed);
        }
        Ok(())
    }
}

/// A shipped creative whose full digest matched the signed digest at its index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShippedCreative {
    pub index: u8,
    pub vid: [u8; 4],
    pub digest: [u8; 32],
    pub creative: Creative,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Core {
    expires_ms: i64,
    signpost: Signpost,
}

/// A verified entry: safe to hand to the decision kernel and to render.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedEntry {
    pub kid: String,
    pub expires_ms: i64,
    pub signpost: Signpost,
    /// Declared creative count `n` (signed).
    pub declared: u8,
    pub creatives: Vec<ShippedCreative>,
}

fn creative_digest(body: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(CREATIVE_DOMAIN);
    h.update(body);
    h.finalize().into()
}

/// Verify one entry at `now_ms` against the verified keyset.
///
/// # Errors
/// Any [`PackError`]. Any failure means the whole entry is dropped (I10).
pub fn verify_entry(
    bytes: &[u8],
    keyset: &Keyset,
    now_ms: i64,
) -> Result<VerifiedEntry, PackError> {
    let mut r = Reader::new(bytes);
    if r.u8()? != 2 {
        return Err(PackError::Malformed);
    }
    let kid = r.kid()?;
    let core_len_at = r.pos();
    let core_len = usize::from(r.u16()?);
    let core_raw = r.take(core_len)?;
    let core_wire = bytes
        .get(core_len_at..r.pos())
        .ok_or(PackError::Malformed)?;
    let n = r.u8()?;
    let mask = r.u8()?;
    if n == 0 || n > MAX_CREATIVES || mask & 1 == 0 || u16::from(mask) >> n != 0 {
        return Err(PackError::Malformed);
    }
    let mut digests: Vec<[u8; 32]> = Vec::with_capacity(usize::from(n));
    let mut shipped = Vec::new();
    for i in 0..n {
        if mask & (1 << i) != 0 {
            let len = usize::from(r.u16()?);
            let body = r.take(len)?;
            let creative: Creative = from_canonical_json(body)?;
            creative.check()?;
            let digest = creative_digest(body);
            digests.push(digest);
            shipped.push((i, digest, creative));
        } else {
            digests.push(r.array::<32>()?);
        }
    }
    let sig = r.array::<64>()?;
    r.finish()?;

    let vids: Vec<[u8; 4]> = digests.iter().map(vid_of).collect();
    for (i, v) in vids.iter().enumerate() {
        if vids.iter().skip(i + 1).any(|w| w == v) {
            return Err(PackError::Duplicate);
        }
    }

    let mut msg = Vec::with_capacity(ENV_DOMAIN.len() + core_wire.len() + 1 + 32 * digests.len());
    msg.extend_from_slice(ENV_DOMAIN);
    msg.extend_from_slice(core_wire);
    msg.push(n);
    for d in &digests {
        msg.extend_from_slice(d);
    }
    keyset.key_for(kid, now_ms)?.verify(&msg, &sig)?;

    let core: Core = from_canonical_json(core_raw)?;
    if core.expires_ms > core.signpost.valid_until_ms {
        return Err(PackError::Malformed);
    }
    if now_ms > core.expires_ms {
        return Err(PackError::Expired);
    }
    let creatives = shipped
        .into_iter()
        .map(|(index, digest, creative)| ShippedCreative {
            index,
            vid: vid_of(&digest),
            digest,
            creative,
        })
        .collect();
    Ok(VerifiedEntry {
        kid: String::from(kid),
        expires_ms: core.expires_ms,
        signpost: core.signpost,
        declared: n,
        creatives,
    })
}

fn vid_of(d: &[u8; 32]) -> [u8; 4] {
    let [a, b, c, e, ..] = *d;
    [a, b, c, e]
}
