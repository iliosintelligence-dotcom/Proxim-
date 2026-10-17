//! Encoders for the formats in `docs/pack-format.md`. The production compiler is
//! `packages/pack` (TypeScript); these exist so tests and golden vectors can build every shape,
//! including the broken ones, and so both implementations can be checked against each other.

use crate::{PackResult, b64, crypto};
use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::ReasonCode;

pub const MAGIC: &[u8; 4] = b"PXPK";
pub const VERSION: u8 = 1;
pub const ENTRY_AUTH_ES256: u8 = 1;
pub const ENVELOPE_DOMAIN: &[u8] = b"PROXIM-ENV-V2\x00";
pub const CREATIVE_DOMAIN: &[u8] = b"PROXIM-CREATIVE-V2\x00";

fn u16_len(n: usize) -> PackResult<[u8; 2]> {
    u16::try_from(n)
        .map(u16::to_be_bytes)
        .map_err(|_| ReasonCode::SignatureInvalid)
}

fn u32_len(n: usize) -> PackResult<[u8; 4]> {
    u32::try_from(n)
        .map(u32::to_be_bytes)
        .map_err(|_| ReasonCode::SignatureInvalid)
}

/// `d_i = SHA-256("PROXIM-CREATIVE-V2" 0x00 || body)`.
#[must_use]
pub fn creative_digest(body: &[u8]) -> [u8; 32] {
    crypto::sha256(&[CREATIVE_DOMAIN, body])
}

/// `"PROXIM-ENV-V2" 0x00 || u16(len core) || core || u8(n) || d_0 .. d_(n-1)`.
///
/// # Errors
/// `SIGNATURE_INVALID` if the core or the digest count does not fit its length field.
pub fn signed_message(core: &[u8], digests: &[[u8; 32]]) -> PackResult<Vec<u8>> {
    let n = u8::try_from(digests.len()).map_err(|_| ReasonCode::SignatureInvalid)?;
    let mut m = Vec::with_capacity(ENVELOPE_DOMAIN.len() + 3 + core.len() + 32 * digests.len());
    m.extend_from_slice(ENVELOPE_DOMAIN);
    m.extend_from_slice(&u16_len(core.len())?);
    m.extend_from_slice(core);
    m.push(n);
    for d in digests {
        m.extend_from_slice(d);
    }
    Ok(m)
}

/// Entry bytes (pack-format 4.1): core, n, mask, the digests of withheld creatives, then the
/// shipped bodies, each group in index order. No validation: tests use this to build malformed entries too.
///
/// # Errors
/// `SIGNATURE_INVALID` if a length does not fit its field.
pub fn entry(core: &[u8], bodies: &[&[u8]], ship_mask: u8) -> PackResult<Vec<u8>> {
    let mut e = Vec::new();
    e.extend_from_slice(&u16_len(core.len())?);
    e.extend_from_slice(core);
    e.push(u8::try_from(bodies.len()).map_err(|_| ReasonCode::SignatureInvalid)?);
    e.push(ship_mask);
    let shipped = |i: usize| i < 8 && ship_mask & (1 << i) != 0;
    for (i, body) in bodies.iter().enumerate() {
        if !shipped(i) {
            e.extend_from_slice(&creative_digest(body));
        }
    }
    for (i, body) in bodies.iter().enumerate() {
        if shipped(i) {
            e.extend_from_slice(&u16_len(body.len())?);
            e.extend_from_slice(body);
        }
    }
    Ok(e)
}

/// One pack row: entry bytes, kid and signature.
#[derive(Clone, Debug)]
pub struct Row {
    pub entry: Vec<u8>,
    pub kid: String,
    pub sig: [u8; 64],
}

/// Pack header values.
#[derive(Clone, Copy, Debug)]
pub struct Header {
    pub pack_kind: u8,
    pub tier: u8,
    pub cell: u64,
    pub seq: u64,
}

/// The raw DEFLATE'd entry stream: `u32 len || entry` per row.
///
/// # Errors
/// `SIGNATURE_INVALID` if a length does not fit.
pub fn stream(rows: &[Row]) -> PackResult<Vec<u8>> {
    let mut s = Vec::new();
    for r in rows {
        s.extend_from_slice(&u32_len(r.entry.len())?);
        s.extend_from_slice(&r.entry);
    }
    Ok(s)
}

/// Assemble a pack from a header, the compressed stream and the rows (kid table in first-use
/// order). Compression is the caller's: [`deflate`] here, zlib in `packages/pack`.
///
/// # Errors
/// `SIGNATURE_INVALID` if a count or length does not fit.
pub fn pack(h: &Header, compressed: &[u8], rows: &[Row]) -> PackResult<Vec<u8>> {
    let mut kids: Vec<&str> = Vec::new();
    for r in rows {
        if !kids.contains(&r.kid.as_str()) {
            kids.push(&r.kid);
        }
    }
    let mut p = Vec::new();
    p.extend_from_slice(MAGIC);
    p.extend_from_slice(&[VERSION, h.pack_kind, ENTRY_AUTH_ES256, h.tier]);
    p.extend_from_slice(&h.cell.to_be_bytes());
    p.extend_from_slice(&h.seq.to_be_bytes());
    p.push(u8::try_from(kids.len()).map_err(|_| ReasonCode::SignatureInvalid)?);
    for k in &kids {
        p.push(u8::try_from(k.len()).map_err(|_| ReasonCode::SignatureInvalid)?);
        p.extend_from_slice(k.as_bytes());
    }
    p.extend_from_slice(&u32_len(rows.len())?);
    p.extend_from_slice(&u32_len(compressed.len())?);
    p.extend_from_slice(compressed);
    for r in rows {
        let idx = kids.iter().position(|k| *k == r.kid).unwrap_or(0);
        p.push(u8::try_from(idx).map_err(|_| ReasonCode::SignatureInvalid)?);
        p.extend_from_slice(&r.sig);
    }
    Ok(p)
}

/// Raw DEFLATE at level 9 (miniz). Valid for the verifier; byte-identical output with zlib is
/// not expected, which is why the manifest hashes the compressed bytes.
#[must_use]
pub fn deflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec(data, 9)
}

/// The signing input of a compact JWS: `b64u(header) "." b64u(payload)`.
#[must_use]
pub fn jws_signing_input(typ: &str, kid: &str, payload: &[u8]) -> String {
    let header = alloc::format!(r#"{{"alg":"ES256","kid":"{kid}","typ":"{typ}"}}"#);
    let mut s = b64::encode(header.as_bytes());
    s.push('.');
    s.push_str(&b64::encode(payload));
    s
}
