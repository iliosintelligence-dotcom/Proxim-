//! Keyset verification (pack-spec 3.1): pinned roots sign the list of online keys and revoked kids.

use crate::doc::{self, KIND_KEYSET};
use crate::error::PackError;
use crate::es256::PublicKey;
use crate::wire::is_valid_kid;
use crate::{SeqMark, sha256};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::strict::from_canonical_json;
use serde::{Deserialize, Serialize};

const MAX_KEYS: usize = 32;
const MAX_REVOKED: usize = 256;

/// Root public keys pinned in the SDK build. Only these may sign a keyset.
#[derive(Clone, Debug, Default)]
pub struct TrustRoots {
    keys: BTreeMap<String, PublicKey>,
}

impl TrustRoots {
    /// # Errors
    /// [`PackError::Malformed`] for a bad kid or key, [`PackError::Duplicate`] for a repeated kid.
    pub fn new(roots: &[(&str, &[u8])]) -> Result<Self, PackError> {
        let mut keys = BTreeMap::new();
        for (kid, sec1) in roots {
            if !is_valid_kid(kid.as_bytes()) {
                return Err(PackError::Malformed);
            }
            if keys
                .insert(String::from(*kid), PublicKey::from_sec1(sec1)?)
                .is_some()
            {
                return Err(PackError::Duplicate);
            }
        }
        Ok(Self { keys })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyRecord {
    kid: String,
    not_after_ms: i64,
    not_before_ms: i64,
    #[serde(rename = "pub")]
    public: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeysetPayload {
    expires_ms: i64,
    issued_ms: i64,
    keys: Vec<KeyRecord>,
    revoked_kids: Vec<String>,
    seq: u64,
    v: u32,
}

#[derive(Clone, Debug)]
struct OnlineKey {
    key: PublicKey,
    not_before_ms: i64,
    not_after_ms: i64,
}

/// A verified keyset: the online keys that may sign manifests and entries.
#[derive(Clone, Debug)]
pub struct Keyset {
    pub seq: u64,
    pub issued_ms: i64,
    pub expires_ms: i64,
    digest: [u8; 32],
    keys: BTreeMap<String, OnlineKey>,
    revoked: BTreeSet<String>,
}

impl Keyset {
    /// The rollback mark to persist after accepting this keyset.
    #[must_use]
    pub const fn mark(&self) -> SeqMark {
        SeqMark {
            seq: self.seq,
            digest: self.digest,
        }
    }

    /// Whether `kid` is on the revoked list.
    #[must_use]
    pub fn is_revoked(&self, kid: &str) -> bool {
        self.revoked.contains(kid)
    }

    /// The online key that may verify a signature by `kid` at `now_ms`.
    ///
    /// # Errors
    /// [`PackError::KidRevoked`], [`PackError::UnknownKid`] or [`PackError::KeyNotValid`].
    pub fn key_for(&self, kid: &str, now_ms: i64) -> Result<&PublicKey, PackError> {
        if self.revoked.contains(kid) {
            return Err(PackError::KidRevoked);
        }
        let k = self.keys.get(kid).ok_or(PackError::UnknownKid)?;
        if now_ms < k.not_before_ms || now_ms > k.not_after_ms {
            return Err(PackError::KeyNotValid);
        }
        Ok(&k.key)
    }
}

/// Verify a keyset document against the pinned roots.
///
/// # Errors
/// Any [`PackError`]; see pack-spec 3.1 for the order of checks.
pub fn verify_keyset(
    roots: &TrustRoots,
    bytes: &[u8],
    now_ms: i64,
    last: Option<SeqMark>,
) -> Result<Keyset, PackError> {
    let d = doc::parse(bytes, KIND_KEYSET)?;
    let root = roots.keys.get(d.kid).ok_or(PackError::UnknownKid)?;
    root.verify(&d.signed_msg, &d.sig)?;
    let p: KeysetPayload = from_canonical_json(d.payload)?;
    if p.v != 1
        || p.keys.is_empty()
        || p.issued_ms > p.expires_ms
        || !p.revoked_kids.iter().all(|k| is_valid_kid(k.as_bytes()))
    {
        return Err(PackError::Malformed);
    }
    if p.keys.len() > MAX_KEYS || p.revoked_kids.len() > MAX_REVOKED {
        return Err(PackError::Limit);
    }
    let digest = sha256(bytes);
    SeqMark::admit(last, SeqMark { seq: p.seq, digest })?;
    if now_ms < p.issued_ms || now_ms > p.expires_ms {
        return Err(PackError::Expired);
    }
    let mut revoked = BTreeSet::new();
    for kid in p.revoked_kids {
        if !revoked.insert(kid) {
            return Err(PackError::Duplicate);
        }
    }
    let mut keys = BTreeMap::new();
    for rec in p.keys {
        if !is_valid_kid(rec.kid.as_bytes()) || rec.not_before_ms > rec.not_after_ms {
            return Err(PackError::Malformed);
        }
        if revoked.contains(&rec.kid) || roots.keys.contains_key(&rec.kid) {
            return Err(PackError::Malformed);
        }
        let key = OnlineKey {
            key: PublicKey::from_hex(&rec.public)?,
            not_before_ms: rec.not_before_ms,
            not_after_ms: rec.not_after_ms,
        };
        if keys.insert(rec.kid, key).is_some() {
            return Err(PackError::Duplicate);
        }
    }
    Ok(Keyset {
        seq: p.seq,
        issued_ms: p.issued_ms,
        expires_ms: p.expires_ms,
        digest,
        keys,
        revoked,
    })
}
