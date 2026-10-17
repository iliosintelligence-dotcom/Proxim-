//! Pack container and envelope v2 entries (pack-format 4 and 5).

use crate::encode::{CREATIVE_DOMAIN, ENTRY_AUTH_ES256, MAGIC, VERSION, signed_message};
use crate::keyset::{Keyset, Purpose};
use crate::manifest::{MAX_PACK_BYTES, Manifest};
use crate::revocation::RevocationList;
use crate::{PackResult, crypto, valid_kid};
use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::strict::{JsonLimits, from_canonical_json, parse_value};
use proxim_types::{ReasonCode, Signpost};
use serde_json::Value;

pub const MAX_ENTRIES: u32 = 4096;
pub const MAX_KIDS: u8 = 16;
pub const MAX_STREAM_BYTES: usize = 4_194_304;
pub const MAX_ENTRY_BYTES: u32 = 65_536;
pub const MAX_CORE_BYTES: u16 = 16_384;
pub const MAX_CREATIVES: u8 = 8;
pub const MAX_BODY_BYTES: u16 = 1024;

/// A creative that shipped and is not revoked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Creative {
    pub index: u8,
    pub vid: [u8; 4],
    pub body: Vec<u8>,
}

/// An entry that passed every check (pack-format 5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedEntry {
    pub index: u32,
    pub signpost: Signpost,
    pub kid: String,
    pub live_at_ms: Option<i64>,
    /// Shipped, unrevoked creatives in index order; creative 0 is always first.
    pub creatives: Vec<Creative>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroppedEntry {
    pub index: u32,
    /// The signpost id, when the core was readable.
    pub id: Option<String>,
    pub reason: ReasonCode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedPack {
    pub cell: u64,
    pub seq: u64,
    pub tier: u8,
    pub entries: Vec<VerifiedEntry>,
    pub dropped: Vec<DroppedEntry>,
}

/// Big-endian reader that fails with `SIGNATURE_INVALID` past the end.
pub(crate) struct Reader<'a> {
    b: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) const fn new(b: &'a [u8]) -> Self {
        Self { b }
    }
    pub(crate) fn take(&mut self, n: usize) -> PackResult<&'a [u8]> {
        if n > self.b.len() {
            return Err(ReasonCode::SignatureInvalid);
        }
        let (head, tail) = self.b.split_at(n);
        self.b = tail;
        Ok(head)
    }
    fn array<const N: usize>(&mut self) -> PackResult<[u8; N]> {
        self.take(N)?
            .try_into()
            .map_err(|_| ReasonCode::SignatureInvalid)
    }
    pub(crate) fn u8(&mut self) -> PackResult<u8> {
        Ok(u8::from_be_bytes(self.array()?))
    }
    pub(crate) fn u16(&mut self) -> PackResult<u16> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    pub(crate) fn u32(&mut self) -> PackResult<u32> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    pub(crate) fn u64(&mut self) -> PackResult<u64> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    pub(crate) const fn is_empty(&self) -> bool {
        self.b.is_empty()
    }
}

fn usize_of(n: u32) -> PackResult<usize> {
    usize::try_from(n).map_err(|_| ReasonCode::SignatureInvalid)
}

/// One creative slot as shipped.
struct Slot<'a> {
    body: Option<&'a [u8]>,
    digest: [u8; 32],
}

struct RawEntry<'a> {
    core: &'a [u8],
    slots: Vec<Slot<'a>>,
}

/// Parse one entry record and recompute the digests of shipped bodies (pack-format 4.1 and step a).
/// Layout (golden_v2 envelope): the digests of unshipped creatives in index order, then the
/// shipped bodies in index order.
fn parse_entry(bytes: &[u8]) -> PackResult<RawEntry<'_>> {
    let bad = ReasonCode::SignatureInvalid;
    let mut r = Reader::new(bytes);
    let core_len = r.u16()?;
    if core_len == 0 || core_len > MAX_CORE_BYTES {
        return Err(bad);
    }
    let core = r.take(usize::from(core_len))?;
    let n = r.u8()?;
    let mask = r.u8()?;
    if n == 0 || n > MAX_CREATIVES || mask & 1 == 0 || u16::from(mask) >> n != 0 {
        return Err(bad);
    }
    let shipped = |i: u8| mask & (1 << i) != 0;
    let mut slots: Vec<Slot<'_>> = Vec::with_capacity(usize::from(n));
    for i in 0..n {
        let digest = if shipped(i) { [0; 32] } else { r.array()? };
        slots.push(Slot { body: None, digest });
    }
    for (i, slot) in (0u8..).zip(slots.iter_mut()) {
        if shipped(i) {
            let len = r.u16()?;
            if len == 0 || len > MAX_BODY_BYTES {
                return Err(bad);
            }
            let body = r.take(usize::from(len))?;
            slot.body = Some(body);
            slot.digest = crypto::sha256(&[CREATIVE_DOMAIN, body]);
        }
    }
    if !r.is_empty() {
        return Err(bad);
    }
    // vid = first 4 bytes of the digest; pairwise distinct (Addendum 02 4.1).
    let mut vids: Vec<&[u8]> = slots.iter().filter_map(|s| s.digest.get(..4)).collect();
    vids.sort_unstable();
    if vids.windows(2).any(|w| w.first() == w.get(1)) {
        return Err(bad);
    }
    Ok(RawEntry { core, slots })
}

/// A verified envelope: the signed core and the shipped bodies by creative index.
pub type Envelope<'a> = (&'a [u8], Vec<(u8, &'a [u8])>);

/// Steps a and c for one entry with its signature: parse, recompute digests, verify ES256 over the
/// envelope v2 message. Returns the core and the shipped bodies by index. Used by the golden_v2
/// envelope vectors (`core/testdata/golden/rotation/`), whose entries carry the signature inline.
///
/// # Errors
/// [`ReasonCode::SignatureInvalid`] for any parse, duplicate-vid or signature failure.
pub fn verify_envelope<'a>(
    key: &p256::ecdsa::VerifyingKey,
    entry: &'a [u8],
    sig: &[u8],
) -> PackResult<Envelope<'a>> {
    let raw = parse_entry(entry)?;
    let digests: Vec<[u8; 32]> = raw.slots.iter().map(|s| s.digest).collect();
    if !crypto::verify_es256(key, &signed_message(raw.core, &digests)?, sig) {
        return Err(ReasonCode::SignatureInvalid);
    }
    let bodies = (0u8..)
        .zip(&raw.slots)
        .filter_map(|(i, s)| s.body.map(|b| (i, b)))
        .collect();
    Ok((raw.core, bodies))
}

/// Read the pack-layer fields of a core: `pack_kind` (required, must match) and `live_at_ms`.
fn core_fields(core: &[u8], pack_kind: &str) -> PackResult<Option<i64>> {
    let bad = ReasonCode::SignatureInvalid;
    let tree = parse_value(core, &JsonLimits::default()).map_err(|_| bad)?;
    // `wordings` is filled in by this layer from the verified entry, never taken from a core.
    if tree.get("pack_kind").and_then(Value::as_str) != Some(pack_kind)
        || tree.get("wordings").is_some()
    {
        return Err(bad);
    }
    match tree.get("live_at_ms") {
        None => Ok(None),
        Some(v) => v.as_i64().map(Some).ok_or(bad),
    }
}

/// Open a pack: whole-pack checks, then each entry (pack-format 5).
///
/// # Errors
/// [`ReasonCode::SignatureInvalid`] when the whole pack is rejected (hash, header, structure,
/// duplicate ids). Per-entry failures are listed in [`OpenedPack::dropped`] instead.
pub fn open_pack(
    bytes: &[u8],
    manifest: &Manifest,
    tier: u8,
    keyset: &Keyset,
    revocations: &RevocationList,
    now_ms: i64,
) -> PackResult<OpenedPack> {
    let bad = ReasonCode::SignatureInvalid;
    // 1. Bytes equal the manifest's tier entry.
    let want = manifest.tier(tier).ok_or(bad)?;
    if bytes.len() > usize_of(MAX_PACK_BYTES)?
        || u32::try_from(bytes.len()) != Ok(want.bytes)
        || crypto::sha256(&[bytes]) != want.sha256
    {
        return Err(bad);
    }
    // 2. Header equals the manifest.
    let mut r = Reader::new(bytes);
    let header_ok = r.take(4)? == MAGIC
        && r.u8()? == VERSION
        && r.u8()? == manifest.pack_kind.byte()
        && r.u8()? == ENTRY_AUTH_ES256
        && r.u8()? == tier
        && r.u64()? == manifest.cell
        && r.u64()? == manifest.guard.seq;
    if !header_ok {
        return Err(bad);
    }
    // 3. Structure.
    let n_kids = r.u8()?;
    if n_kids == 0 || n_kids > MAX_KIDS {
        return Err(bad);
    }
    let mut kids: Vec<String> = Vec::with_capacity(usize::from(n_kids));
    for _ in 0..n_kids {
        let len = r.u8()?;
        let kid = core::str::from_utf8(r.take(usize::from(len))?).map_err(|_| bad)?;
        if !valid_kid(kid) || kids.iter().any(|k| k == kid) {
            return Err(bad);
        }
        kids.push(kid.into());
    }
    let n_entries = r.u32()?;
    if n_entries > MAX_ENTRIES {
        return Err(bad);
    }
    let stream_len = r.u32()?;
    let compressed = r.take(usize_of(stream_len)?)?;
    let mut sigs: Vec<(&String, &[u8])> = Vec::with_capacity(usize_of(n_entries)?);
    for _ in 0..n_entries {
        let kid = kids.get(usize::from(r.u8()?)).ok_or(bad)?;
        sigs.push((kid, r.take(64)?));
    }
    if !r.is_empty() {
        return Err(bad);
    }
    let stream = miniz_oxide::inflate::decompress_to_vec_with_limit(compressed, MAX_STREAM_BYTES)
        .map_err(|_| bad)?;
    let mut s = Reader::new(&stream);
    let mut records: Vec<&[u8]> = Vec::with_capacity(sigs.len());
    for _ in 0..n_entries {
        let len = s.u32()?;
        if len == 0 || len > MAX_ENTRY_BYTES {
            return Err(bad);
        }
        records.push(s.take(usize_of(len)?)?);
    }
    if !s.is_empty() {
        return Err(bad);
    }

    let kind = manifest.pack_kind.name();
    let mut out = OpenedPack {
        cell: manifest.cell,
        seq: manifest.guard.seq,
        tier,
        entries: Vec::new(),
        dropped: Vec::new(),
    };
    let mut seen: Vec<String> = Vec::new();
    for (index, (record, (kid, sig))) in (0u32..).zip(records.into_iter().zip(sigs)) {
        let mut id = None;
        let result = verify_entry(record, kid, sig, keyset, kind, &mut id, &mut seen, now_ms)
            .and_then(|(signpost, live_at_ms, slots)| {
                finish_entry(index, signpost, kid, live_at_ms, slots, revocations, now_ms)
            });
        match result {
            Ok(e) => out.entries.push(e),
            Err(reason) => out.dropped.push(DroppedEntry { index, id, reason }),
        }
    }
    // 4. No two entries share a signpost id (a compiler fault: the whole pack is refused).
    let mut ids = seen;
    ids.sort_unstable();
    if ids.windows(2).any(|w| w.first() == w.get(1)) {
        return Err(bad);
    }
    Ok(out)
}

type Checked<'a> = (Signpost, Option<i64>, Vec<Slot<'a>>);

/// Steps a to d.
#[allow(clippy::too_many_arguments)]
fn verify_entry<'a>(
    record: &'a [u8],
    kid: &str,
    sig: &[u8],
    keyset: &Keyset,
    pack_kind: &str,
    id: &mut Option<String>,
    seen: &mut Vec<String>,
    now_ms: i64,
) -> PackResult<Checked<'a>> {
    let raw = parse_entry(record)?;
    let key = keyset.usable(kid, Purpose::Approve, now_ms)?;
    let digests: Vec<[u8; 32]> = raw.slots.iter().map(|s| s.digest).collect();
    let msg = signed_message(raw.core, &digests)?;
    if !crypto::verify_es256(key, &msg, sig) {
        return Err(ReasonCode::SignatureInvalid);
    }
    let signpost: Signpost =
        from_canonical_json(raw.core).map_err(|_| ReasonCode::SignatureInvalid)?;
    *id = Some(signpost.id.as_str().into());
    seen.push(signpost.id.as_str().into());
    let live_at_ms = core_fields(raw.core, pack_kind)?;
    Ok((signpost, live_at_ms, raw.slots))
}

/// Steps e and f.
fn finish_entry(
    index: u32,
    mut signpost: Signpost,
    kid: &str,
    live_at_ms: Option<i64>,
    slots: Vec<Slot<'_>>,
    revocations: &RevocationList,
    now_ms: i64,
) -> PackResult<VerifiedEntry> {
    let id = signpost.id.as_str();
    if revocations.revokes_entry(id, signpost.rev, &signpost.sender, kid) {
        return Err(ReasonCode::Revoked);
    }
    let mut creatives = Vec::new();
    let digests: Vec<alloc::string::String> =
        slots.iter().map(|s| crypto::hex(&s.digest)).collect();
    let mut avail_mask = 0u8;
    for (i, slot) in (0u8..).zip(slots) {
        let vid: [u8; 4] = slot
            .digest
            .get(..4)
            .and_then(|v| v.try_into().ok())
            .ok_or(ReasonCode::SignatureInvalid)?;
        if revocations.revokes_variant(id, signpost.rev, &vid) {
            if i == 0 {
                return Err(ReasonCode::Revoked);
            }
            continue;
        }
        if let Some(body) = slot.body {
            avail_mask |= 1 << i;
            creatives.push(Creative {
                index: i,
                vid,
                body: body.into(),
            });
        }
    }
    if live_at_ms.is_some_and(|t| now_ms < t) {
        return Err(ReasonCode::NotYetLive);
    }
    // Rotation inputs for `decide` (engine spec 7.3): every declared digest, and which wordings
    // shipped and are not variant-revoked. Locale filtering, if any, is the SDK's.
    signpost.wordings = Some(proxim_types::Wordings {
        digests,
        avail_mask,
    });
    Ok(VerifiedEntry {
        index,
        signpost,
        kid: kid.into(),
        live_at_ms,
        creatives,
    })
}
