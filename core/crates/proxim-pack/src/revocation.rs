//! Revocation list (pack-format 3.2; scopes of Addendum 01 I20 and Addendum 02 9.2).

use crate::jws::{self, TYP_REVOCATIONS};
use crate::keyset::{Keyset, Purpose};
use crate::{Guard, PackResult, crypto, doc, valid_kid};
use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::{Id, ReasonCode};
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "proxim-revocations/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case", deny_unknown_fields)]
pub enum Scope {
    /// One wording of one revision; `vid` is 8 lowercase hex (4 bytes).
    Variant {
        id: String,
        rev: u32,
        vid: String,
    },
    SignpostRev {
        id: String,
        rev: u32,
    },
    Signpost {
        id: String,
    },
    SenderOrg {
        sender: String,
    },
    Kid {
        kid: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListDoc {
    format: String,
    seq: u64,
    issued_at_ms: i64,
    expires_at_ms: i64,
    entries: Vec<Scope>,
}

/// An accepted revocation list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RevocationList {
    pub guard: Option<Guard>,
    pub expires_at_ms: i64,
    pub entries: Vec<Scope>,
}

impl RevocationList {
    /// Whether the entry as a whole is revoked (every scope but `variant`).
    #[must_use]
    pub fn revokes_entry(&self, id: &str, rev: u32, sender: &str, kid: &str) -> bool {
        self.entries.iter().any(|s| match s {
            Scope::Variant { .. } => false,
            Scope::SignpostRev { id: i, rev: r } => i == id && *r == rev,
            Scope::Signpost { id: i } => i == id,
            Scope::SenderOrg { sender: s } => s == sender,
            Scope::Kid { kid: k } => k == kid,
        })
    }

    /// Whether wording `vid` of `id`/`rev` is revoked.
    #[must_use]
    pub fn revokes_variant(&self, id: &str, rev: u32, vid: &[u8; 4]) -> bool {
        let want = crypto::hex(vid);
        self.entries.iter().any(|s| {
            matches!(s, Scope::Variant { id: i, rev: r, vid: v } if i == id && *r == rev && *v == want)
        })
    }
}

/// Verify a revocation list against the key set and the previous guard.
///
/// # Errors
/// `SIGNATURE_INVALID`, `KID_REVOKED`, `PACK_EXPIRED`, `SEQ_ROLLBACK`.
pub fn accept_revocations(
    token: &[u8],
    keyset: &Keyset,
    prev: Option<&Guard>,
    now_ms: i64,
) -> PackResult<RevocationList> {
    let bad = ReasonCode::SignatureInvalid;
    let j = jws::parse(token, TYP_REVOCATIONS)?;
    let key = keyset.usable(&j.kid, Purpose::Manifest, now_ms)?;
    if !j.verify(key) {
        return Err(bad);
    }
    let d: ListDoc = doc::decode(&j.payload)?;
    let well_formed = d.entries.iter().all(|s| match s {
        Scope::Variant { id, vid, .. } => {
            Id::new(id).is_ok() && crypto::parse_hex::<4>(vid).is_some()
        }
        Scope::SignpostRev { id, .. } | Scope::Signpost { id } => Id::new(id).is_ok(),
        Scope::SenderOrg { sender } => !sender.is_empty(),
        Scope::Kid { kid } => valid_kid(kid),
    });
    if d.format != FORMAT || !well_formed {
        return Err(bad);
    }
    doc::check_window(d.issued_at_ms, d.expires_at_ms, now_ms)?;
    let guard = Guard::check(prev, d.seq, &j.payload_sha256())?;
    Ok(RevocationList {
        guard: Some(guard),
        expires_at_ms: d.expires_at_ms,
        entries: d.entries,
    })
}
