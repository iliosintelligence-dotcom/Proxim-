//! Signed document framing (pack-spec 3): keysets and manifests.

use crate::error::PackError;
use crate::wire::Reader;
use alloc::vec::Vec;

pub const KIND_KEYSET: u8 = 1;
pub const KIND_MANIFEST: u8 = 2;
const MAGIC: &[u8; 4] = b"PXSD";
const DOMAIN: &[u8] = b"PROXIM-DOC-V1\x00";
/// Largest payload accepted (pack-spec 3).
pub const MAX_PAYLOAD: usize = 1 << 20;

pub struct Doc<'a> {
    pub kid: &'a str,
    pub payload: &'a [u8],
    pub signed_msg: Vec<u8>,
    pub sig: [u8; 64],
}

/// Parse a signed document of the expected kind. Does not verify the signature.
pub fn parse(bytes: &[u8], kind: u8) -> Result<Doc<'_>, PackError> {
    let mut r = Reader::new(bytes);
    if r.take(4)? != MAGIC || r.u8()? != 1 || r.u8()? != kind {
        return Err(PackError::Malformed);
    }
    let kid_start = r.pos();
    let kid = r.kid()?;
    let len = usize::try_from(r.u32()?).map_err(|_| PackError::Limit)?;
    if len > MAX_PAYLOAD {
        return Err(PackError::Limit);
    }
    let payload = r.take(len)?;
    let signed_end = r.pos();
    let sig = r.array::<64>()?;
    r.finish()?;
    let header = bytes
        .get(kid_start..signed_end)
        .ok_or(PackError::Malformed)?;
    let mut signed_msg = Vec::with_capacity(DOMAIN.len() + 1 + header.len());
    signed_msg.extend_from_slice(DOMAIN);
    signed_msg.push(kind);
    signed_msg.extend_from_slice(header);
    Ok(Doc {
        kid,
        payload,
        signed_msg,
        sig,
    })
}
