//! Strict lowercase hex (pack-spec 2): no prefix, no uppercase, even length.

use crate::error::PackError;
use alloc::vec::Vec;

fn nibble(c: u8) -> Result<u8, PackError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        _ => Err(PackError::Malformed),
    }
}

pub fn decode(s: &str) -> Result<Vec<u8>, PackError> {
    let raw = s.as_bytes();
    if !raw.len().is_multiple_of(2) {
        return Err(PackError::Malformed);
    }
    raw.chunks_exact(2)
        .map(|pair| match pair {
            [hi, lo] => Ok((nibble(*hi)? << 4) | nibble(*lo)?),
            _ => Err(PackError::Malformed),
        })
        .collect()
}

pub fn decode_array<const N: usize>(s: &str) -> Result<[u8; N], PackError> {
    decode(s)?.try_into().map_err(|_| PackError::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict() {
        assert_eq!(decode("00ff10"), Ok(alloc::vec![0, 255, 16]));
        assert_eq!(decode("0F"), Err(PackError::Malformed));
        assert_eq!(decode("abc"), Err(PackError::Malformed));
        assert_eq!(decode_array::<2>("0102"), Ok([1, 2]));
        assert_eq!(decode_array::<2>("01"), Err(PackError::Malformed));
    }
}
