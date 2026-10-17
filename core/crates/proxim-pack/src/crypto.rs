//! SHA-256, ES256 verification and hex helpers.

use alloc::string::String;
use p256::ecdsa::signature::Verifier as _;
use p256::ecdsa::{Signature, VerifyingKey};
use sha2::{Digest as _, Sha256};

/// SHA-256 over the concatenation of `parts`.
#[must_use]
pub fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    h.finalize().into()
}

/// Parse an uncompressed SEC1 P-256 public key (65 bytes, `0x04 || X || Y`, on the curve).
#[must_use]
pub fn public_key(sec1: &[u8]) -> Option<VerifyingKey> {
    if sec1.len() != 65 || sec1.first() != Some(&0x04) {
        return None;
    }
    VerifyingKey::from_sec1_bytes(sec1).ok()
}

/// ES256: ECDSA P-256 with SHA-256 over `msg`, signature `r || s` (64 bytes). High-S is accepted.
#[must_use]
pub fn verify_es256(key: &VerifyingKey, msg: &[u8], sig: &[u8]) -> bool {
    Signature::from_slice(sig).is_ok_and(|s| key.verify(msg, &s).is_ok())
}

/// Lowercase hex.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        for nib in [b >> 4, b & 0xf] {
            s.push(char::from(
                HEX.get(usize::from(nib)).copied().unwrap_or(b'0'),
            ));
        }
    }
    s
}

/// Parse exactly `N` bytes of lowercase hex.
#[must_use]
pub fn parse_hex<const N: usize>(s: &str) -> Option<[u8; N]> {
    let b = s.as_bytes();
    if b.len() != N * 2 {
        return None;
    }
    let nib = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; N];
    for (o, pair) in out.iter_mut().zip(b.chunks_exact(2)) {
        let [hi, lo] = pair else { return None };
        *o = (nib(*hi)? << 4) | nib(*lo)?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_answer() {
        assert_eq!(
            hex(&sha256(&[b"ab", b"c"])),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hex_round_trip_and_strictness() {
        let h = hex(&[0x00, 0xab, 0xff]);
        assert_eq!(h, "00abff");
        assert_eq!(parse_hex::<3>(&h), Some([0x00, 0xab, 0xff]));
        assert_eq!(parse_hex::<3>("00ABff"), None);
        assert_eq!(parse_hex::<2>(&h), None);
    }

    #[test]
    fn rejects_compressed_or_short_keys() {
        assert!(public_key(&[0x04; 64]).is_none());
        let mut k = [0u8; 65];
        k[0] = 0x02;
        assert!(public_key(&k).is_none());
        k[0] = 0x04;
        assert!(public_key(&k).is_none(), "not on the curve");
    }
}
