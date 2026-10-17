//! Bounded big-endian reader. Every read is length-checked; nothing indexes a slice directly.

use crate::error::PackError;

pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub const fn pos(&self) -> usize {
        self.pos
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], PackError> {
        let end = self.pos.checked_add(n).ok_or(PackError::Malformed)?;
        let out = self.buf.get(self.pos..end).ok_or(PackError::Malformed)?;
        self.pos = end;
        Ok(out)
    }

    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], PackError> {
        self.take(N)?.try_into().map_err(|_| PackError::Malformed)
    }

    pub fn u8(&mut self) -> Result<u8, PackError> {
        Ok(u8::from_be_bytes(self.array()?))
    }

    pub fn u16(&mut self) -> Result<u16, PackError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    pub fn u32(&mut self) -> Result<u32, PackError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    pub fn u64(&mut self) -> Result<u64, PackError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    /// A `kid`: u8 length, then 1 to 64 bytes of `[a-z0-9._-]`.
    pub fn kid(&mut self) -> Result<&'a str, PackError> {
        let len = usize::from(self.u8()?);
        let raw = self.take(len)?;
        if !is_valid_kid(raw) {
            return Err(PackError::Malformed);
        }
        core::str::from_utf8(raw).map_err(|_| PackError::Malformed)
    }

    /// Fails unless every byte was consumed.
    pub const fn finish(&self) -> Result<(), PackError> {
        if self.pos == self.buf.len() {
            Ok(())
        } else {
            Err(PackError::Malformed)
        }
    }
}

/// pack-spec 2: 1 to 64 bytes of `[a-z0-9._-]`.
pub fn is_valid_kid(raw: &[u8]) -> bool {
    (1..=64).contains(&raw.len())
        && raw.iter().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_are_bounded() {
        let mut r = Reader::new(&[0, 1, 2]);
        assert_eq!(r.u16(), Ok(1));
        assert_eq!(r.u16(), Err(PackError::Malformed));
        assert_eq!(r.u8(), Ok(2));
        assert_eq!(r.finish(), Ok(()));
    }

    #[test]
    fn kid_grammar() {
        assert!(is_valid_kid(b"root-2026.a_1"));
        assert!(!is_valid_kid(b""));
        assert!(!is_valid_kid(b"Root"));
        assert!(!is_valid_kid(&[b'a'; 65]));
    }
}
