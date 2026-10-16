//! Compact JWS, ES256 only (pack-format 2.1).

use crate::doc::MAX_JWS_BYTES;
use crate::{PackResult, b64, crypto, valid_kid};
use alloc::string::String;
use alloc::vec::Vec;
use p256::ecdsa::VerifyingKey;
use proxim_types::ReasonCode;
use proxim_types::canon::value_to_canonical;
use proxim_types::strict::{JsonLimits, parse_value};
use serde_json::Value;

/// Document types; a token is only ever accepted as the type the caller asked for.
pub const TYP_KEYSET: &str = "proxim-keyset+jws";
pub const TYP_MANIFEST: &str = "proxim-manifest+jws";
pub const TYP_REVOCATIONS: &str = "proxim-revocations+jws";

/// A parsed, not yet verified token.
#[derive(Debug)]
pub struct Jws<'a> {
    pub kid: String,
    pub payload: Vec<u8>,
    signing_input: &'a [u8],
    sig: Vec<u8>,
}

impl Jws<'_> {
    /// Check the signature with `key`.
    #[must_use]
    pub fn verify(&self, key: &VerifyingKey) -> bool {
        crypto::verify_es256(key, self.signing_input, &self.sig)
    }

    /// SHA-256 of the payload bytes (the rollback guard's identity of the document).
    #[must_use]
    pub fn payload_sha256(&self) -> [u8; 32] {
        crypto::sha256(&[&self.payload])
    }
}

/// Split and decode a token of type `typ`; the header must be exactly
/// `{"alg":"ES256","kid":…,"typ":typ}` in canonical form.
///
/// # Errors
/// [`ReasonCode::SignatureInvalid`] for anything malformed or of another type.
pub fn parse<'a>(token: &'a [u8], typ: &str) -> PackResult<Jws<'a>> {
    let bad = ReasonCode::SignatureInvalid;
    if token.len() > MAX_JWS_BYTES {
        return Err(bad);
    }
    let mut parts = token.split(|&c| c == b'.');
    let (Some(h), Some(p), Some(s), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(bad);
    };
    let header = b64::decode(h).ok_or(bad)?;
    let payload = b64::decode(p).ok_or(bad)?;
    let sig = b64::decode(s).ok_or(bad)?;
    if sig.len() != 64 {
        return Err(bad);
    }
    let tree = parse_value(&header, &JsonLimits::default()).map_err(|_| bad)?;
    if value_to_canonical(&tree).map_err(|_| bad)?.as_bytes() != header.as_slice() {
        return Err(bad);
    }
    let Value::Object(map) = tree else {
        return Err(bad);
    };
    let field = |k: &str| map.get(k).and_then(Value::as_str);
    let kid = field("kid").ok_or(bad)?;
    if map.len() != 3
        || field("alg") != Some("ES256")
        || field("typ") != Some(typ)
        || !valid_kid(kid)
    {
        return Err(bad);
    }
    let signing_len = h.len() + 1 + p.len();
    Ok(Jws {
        kid: kid.into(),
        payload,
        signing_input: token.get(..signing_len).ok_or(bad)?,
        sig,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::TestKey;

    #[test]
    fn round_trip_and_type_binding() {
        let k = TestKey::from_label("jws-test");
        let token = k.jws(TYP_MANIFEST, br#"{"a":1}"#);
        let j = parse(token.as_bytes(), TYP_MANIFEST).unwrap();
        assert_eq!(j.kid, "jws-test");
        assert_eq!(j.payload, br#"{"a":1}"#);
        assert!(j.verify(&k.verifying_key()));
        assert!(!j.verify(&TestKey::from_label("other").verifying_key()));
        assert_eq!(
            parse(token.as_bytes(), TYP_KEYSET).unwrap_err(),
            ReasonCode::SignatureInvalid
        );
    }

    #[test]
    fn rejects_header_variants() {
        let k = TestKey::from_label("jws-test");
        let good = k.jws(TYP_MANIFEST, b"{}");
        let (_, rest) = good.split_once('.').unwrap();
        for header in [
            r#"{"alg":"none","kid":"jws-test","typ":"proxim-manifest+jws"}"#,
            r#"{"alg":"ES256","kid":"jws-test","typ":"proxim-manifest+jws","x":1}"#,
            r#"{"kid":"jws-test","alg":"ES256","typ":"proxim-manifest+jws"}"#,
            r#"{"alg":"ES256", "kid":"jws-test","typ":"proxim-manifest+jws"}"#,
            r#"{"alg":"ES256","kid":"BAD","typ":"proxim-manifest+jws"}"#,
        ] {
            let t = alloc::format!("{}.{rest}", b64::encode(header.as_bytes()));
            assert!(parse(t.as_bytes(), TYP_MANIFEST).is_err(), "{header}");
        }
        assert!(parse(b"a.b", TYP_MANIFEST).is_err());
        assert!(parse(alloc::format!("{good}.x").as_bytes(), TYP_MANIFEST).is_err());
    }
}
