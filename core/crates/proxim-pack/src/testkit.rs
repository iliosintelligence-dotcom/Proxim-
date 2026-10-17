//! Deterministic test keys (feature `testkit`). The secret of a key is derived from a public label,
//! so these keys are known to everyone and must never be trusted by a release build
//! (pack-format 9). Signatures are RFC 6979 deterministic, so golden vectors are reproducible.

use crate::encode::jws_signing_input;
use crate::{b64, crypto};
use alloc::string::String;
use p256::ecdsa::signature::Signer as _;
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};

#[derive(Clone, Debug)]
pub struct TestKey {
    pub kid: String,
    key: SigningKey,
}

impl TestKey {
    /// Secret = SHA-256("proxim-test-key/v1" 0x00 || label || counter), first counter that gives a
    /// valid scalar. The label is also the kid.
    ///
    /// # Panics
    /// Never in practice (a 2^-128 event per counter); it is test-only code.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn from_label(label: &str) -> Self {
        let key = (0u8..=255)
            .find_map(|c| {
                let seed = crypto::sha256(&[b"proxim-test-key/v1\x00", label.as_bytes(), &[c]]);
                SigningKey::from_slice(&seed).ok()
            })
            .expect("a valid scalar");
        Self {
            kid: label.into(),
            key,
        }
    }

    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        *self.key.verifying_key()
    }

    /// Uncompressed SEC1 public key.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn public_sec1(&self) -> [u8; 65] {
        self.key
            .verifying_key()
            .to_sec1_point(false)
            .as_bytes()
            .try_into()
            .expect("65-byte uncompressed point")
    }

    #[must_use]
    pub fn public_b64(&self) -> String {
        b64::encode(&self.public_sec1())
    }

    /// ES256 over `msg`, `r || s`.
    #[must_use]
    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        let s: Signature = self.key.sign(msg);
        s.to_bytes().into()
    }

    /// A compact JWS of type `typ` over `payload` (which the caller makes canonical).
    #[must_use]
    pub fn jws(&self, typ: &str, payload: &[u8]) -> String {
        let mut t = jws_signing_input(typ, &self.kid, payload);
        let sig = self.sign(t.as_bytes());
        t.push('.');
        t.push_str(&b64::encode(&sig));
        t
    }
}

/// The canonical summary of an opened pack that golden vectors record and every implementation
/// must reproduce: per entry its index, id, rev, kid, `live_at_ms` and kept creatives (index, vid);
/// per dropped entry its index, id (when read) and reason.
#[must_use]
pub fn summary(p: &crate::OpenedPack) -> serde_json::Value {
    use serde_json::json;
    let entries: alloc::vec::Vec<_> = p
        .entries
        .iter()
        .map(|e| {
            let mut v = json!({
                "index": e.index, "id": e.signpost.id.as_str(), "rev": e.signpost.rev, "kid": e.kid,
                "creatives": e.creatives.iter()
                    .map(|c| json!({"index": c.index, "vid": crypto::hex(&c.vid)}))
                    .collect::<alloc::vec::Vec<_>>(),
            });
            if let (Some(t), Some(o)) = (e.live_at_ms, v.as_object_mut()) {
                o.insert("live_at_ms".into(), json!(t));
            }
            v
        })
        .collect();
    let dropped: alloc::vec::Vec<_> = p
        .dropped
        .iter()
        .map(|d| {
            let mut v = json!({"index": d.index, "reason": d.reason});
            if let (Some(id), Some(o)) = (&d.id, v.as_object_mut()) {
                o.insert("id".into(), json!(id));
            }
            v
        })
        .collect();
    json!({"cell": alloc::format!("{:015x}", p.cell), "seq": p.seq, "tier": p.tier, "entries": entries, "dropped": dropped})
}
