//! Key set (pack-format 3.1): the root-signed list of approve and manifest keys.

use crate::jws::{self, TYP_KEYSET};
use crate::{Guard, PackResult, b64, crypto, doc, valid_kid};
use alloc::string::String;
use alloc::vec::Vec;
use p256::ecdsa::VerifyingKey;
use proxim_types::ReasonCode;
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "proxim-keyset/1";
const MAX_KEYS: usize = 64;

/// What a key may sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    /// Signpost revisions (pack entries).
    Approve,
    /// Tile manifests and revocation lists.
    Manifest,
}

/// A root key pinned in the SDK build.
#[derive(Clone, Debug)]
pub struct RootKey {
    pub kid: String,
    key: VerifyingKey,
}

impl RootKey {
    /// # Errors
    /// [`ReasonCode::SignatureInvalid`] for a bad kid or key encoding.
    pub fn new(kid: &str, sec1: &[u8]) -> PackResult<Self> {
        if !valid_kid(kid) {
            return Err(ReasonCode::SignatureInvalid);
        }
        let key = crypto::public_key(sec1).ok_or(ReasonCode::SignatureInvalid)?;
        Ok(Self {
            kid: kid.into(),
            key,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyDoc {
    kid: String,
    purpose: Purpose,
    #[serde(rename = "pub")]
    public: String,
    not_before_ms: i64,
    not_after_ms: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeysetDoc {
    format: String,
    version: u64,
    issued_at_ms: i64,
    expires_at_ms: i64,
    keys: Vec<KeyDoc>,
    revoked_kids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Key {
    pub kid: String,
    pub purpose: Purpose,
    pub not_before_ms: i64,
    pub not_after_ms: i64,
    key: VerifyingKey,
}

/// How a kid stands for one use at one moment.
#[derive(Clone, Copy, Debug)]
pub enum KeyStatus<'a> {
    Usable(&'a VerifyingKey),
    /// Listed, but revoked, outside its window or of another purpose.
    Unusable,
    Unknown,
}

/// An accepted key set.
#[derive(Clone, Debug)]
pub struct Keyset {
    pub guard: Guard,
    pub expires_at_ms: i64,
    pub keys: Vec<Key>,
    pub revoked_kids: Vec<String>,
}

impl Keyset {
    /// Look up `kid` for `purpose` at `now_ms`.
    #[must_use]
    pub fn lookup(&self, kid: &str, purpose: Purpose, now_ms: i64) -> KeyStatus<'_> {
        if self.revoked_kids.iter().any(|k| k == kid) {
            return KeyStatus::Unusable;
        }
        match self.keys.iter().find(|k| k.kid == kid) {
            None => KeyStatus::Unknown,
            Some(k)
                if k.purpose == purpose && k.not_before_ms <= now_ms && now_ms < k.not_after_ms =>
            {
                KeyStatus::Usable(&k.key)
            }
            Some(_) => KeyStatus::Unusable,
        }
    }

    /// The usable key, or the failure code for a document or entry signed with `kid`.
    ///
    /// # Errors
    /// [`ReasonCode::KidRevoked`] for a listed but unusable kid, else
    /// [`ReasonCode::SignatureInvalid`].
    pub fn usable(&self, kid: &str, purpose: Purpose, now_ms: i64) -> PackResult<&VerifyingKey> {
        match self.lookup(kid, purpose, now_ms) {
            KeyStatus::Usable(k) => Ok(k),
            KeyStatus::Unusable => Err(ReasonCode::KidRevoked),
            KeyStatus::Unknown => Err(ReasonCode::SignatureInvalid),
        }
    }
}

/// Verify a key set token against the pinned `roots` and the previously accepted one.
///
/// # Errors
/// `SIGNATURE_INVALID` (bad token, unknown root, bad content), `PACK_EXPIRED`, `SEQ_ROLLBACK`.
pub fn accept_keyset(
    token: &[u8],
    roots: &[RootKey],
    prev: Option<&Guard>,
    now_ms: i64,
) -> PackResult<Keyset> {
    let bad = ReasonCode::SignatureInvalid;
    let j = jws::parse(token, TYP_KEYSET)?;
    let root = roots.iter().find(|r| r.kid == j.kid).ok_or(bad)?;
    if !j.verify(&root.key) {
        return Err(bad);
    }
    let d: KeysetDoc = doc::decode(&j.payload)?;
    if d.format != FORMAT || d.keys.len() > MAX_KEYS {
        return Err(bad);
    }
    let mut keys: Vec<Key> = Vec::with_capacity(d.keys.len());
    for k in d.keys {
        let fresh = valid_kid(&k.kid)
            && !keys.iter().any(|x| x.kid == k.kid)
            && !roots.iter().any(|r| r.kid == k.kid)
            && !d.revoked_kids.contains(&k.kid)
            && k.not_before_ms < k.not_after_ms;
        if !fresh {
            return Err(bad);
        }
        let raw = b64::decode(k.public.as_bytes()).ok_or(bad)?;
        let key = crypto::public_key(&raw).ok_or(bad)?;
        keys.push(Key {
            kid: k.kid,
            purpose: k.purpose,
            not_before_ms: k.not_before_ms,
            not_after_ms: k.not_after_ms,
            key,
        });
    }
    if !d.revoked_kids.iter().all(|k| valid_kid(k)) {
        return Err(bad);
    }
    doc::check_window(d.issued_at_ms, d.expires_at_ms, now_ms)?;
    let guard = Guard::check(prev, d.version, &j.payload_sha256())?;
    Ok(Keyset {
        guard,
        expires_at_ms: d.expires_at_ms,
        keys,
        revoked_kids: d.revoked_kids,
    })
}
