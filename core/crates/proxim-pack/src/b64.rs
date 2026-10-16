//! Base64url without padding (RFC 4648 section 5), strict: no padding, no other character, and
//! the unused trailing bits must be zero, so every byte string has exactly one encoding.

use alloc::string::String;
use alloc::vec::Vec;

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn value(c: u8) -> Option<u32> {
    let v = match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'-' => 62,
        b'_' => 63,
        _ => return None,
    };
    Some(u32::from(v))
}

/// Decode; `None` for any non-canonical input.
#[must_use]
pub fn decode(s: &[u8]) -> Option<Vec<u8>> {
    if s.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3 + 2);
    for chunk in s.chunks(4) {
        let mut acc: u32 = 0;
        for &c in chunk {
            acc = (acc << 6) | value(c)?;
        }
        // Each chunk carries len*6 bits; keep the whole bytes, the rest must be zero.
        let (bytes, spare) = match chunk.len() {
            4 => (3, 0),
            3 => (2, 2),
            2 => (1, 4),
            _ => return None,
        };
        if acc & ((1 << spare) - 1) != 0 {
            return None;
        }
        let acc = acc >> spare;
        for i in (0..bytes).rev() {
            out.push(u8::try_from((acc >> (8 * i)) & 0xff).ok()?);
        }
    }
    Some(out)
}

/// Encode without padding.
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut acc: u32 = 0;
        for &b in chunk {
            acc = (acc << 8) | u32::from(b);
        }
        let (chars, shift) = match chunk.len() {
            3 => (4, 0),
            2 => (3, 2),
            _ => (2, 4),
        };
        acc <<= shift;
        for i in (0..chars).rev() {
            let idx = usize::try_from((acc >> (6 * i)) & 63).unwrap_or(0);
            out.push(char::from(ALPHABET.get(idx).copied().unwrap_or(b'A')));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc4648_vectors() {
        for (raw, enc) in [
            ("", ""),
            ("f", "Zg"),
            ("fo", "Zm8"),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg"),
            ("fooba", "Zm9vYmE"),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encode(raw.as_bytes()), enc);
            assert_eq!(decode(enc.as_bytes()).unwrap(), raw.as_bytes());
        }
        assert_eq!(encode(&[0xfb, 0xff]), "-_8");
    }

    #[test]
    fn rejects_non_canonical() {
        assert!(decode(b"Zg==").is_none(), "padding");
        assert!(decode(b"Zh").is_none(), "non-zero trailing bits");
        assert!(decode(b"Z").is_none(), "impossible length");
        assert!(decode(b"Zm9v+A").is_none(), "standard alphabet");
        assert!(decode(b"Zm9 v").is_none());
    }
}
