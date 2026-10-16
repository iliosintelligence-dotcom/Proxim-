//! SHA-256 (FIPS 180-4) for golden vectors (engine-spec 9): `state_sha256` and `trace_sha256`.
//!
//! Integer-only, no allocation beyond the padded tail, no `unsafe`. Checked against the NIST
//! example messages and a long input in the tests below. The engine itself never hashes anything;
//! this exists so that every binding can verify the golden bytes without a platform crypto API.

use alloc::string::String;
use core::fmt::Write as _;

const K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

const H0: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

fn compress(h: &mut [u32; 8], block: &[u8]) {
    let mut w = [0_u32; 64];
    for (slot, chunk) in w.iter_mut().zip(block.chunks_exact(4)) {
        let mut b = [0_u8; 4];
        b.copy_from_slice(chunk);
        *slot = u32::from_be_bytes(b);
    }
    for i in 16..64 {
        let w15 = w.get(i - 15).copied().unwrap_or(0);
        let w2 = w.get(i - 2).copied().unwrap_or(0);
        let w16 = w.get(i - 16).copied().unwrap_or(0);
        let w7 = w.get(i - 7).copied().unwrap_or(0);
        let s0 = w15.rotate_right(7) ^ w15.rotate_right(18) ^ (w15 >> 3);
        let s1 = w2.rotate_right(17) ^ w2.rotate_right(19) ^ (w2 >> 10);
        let v = w16.wrapping_add(s0).wrapping_add(w7).wrapping_add(s1);
        if let Some(slot) = w.get_mut(i) {
            *slot = v;
        }
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
    for (k, wi) in K.iter().zip(w.iter()) {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = hh
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(*k)
            .wrapping_add(*wi);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (slot, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
        *slot = slot.wrapping_add(v);
    }
}

/// SHA-256 digest of `data`.
#[must_use]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = H0;
    let mut chunks = data.chunks_exact(64);
    for block in &mut chunks {
        compress(&mut h, block);
    }
    let rem = chunks.remainder();
    // Padding: 0x80, zeros, 64-bit big-endian bit length. The length is a u64 of bits; an input of
    // 2^61 bytes or more cannot exist in memory, and the multiplication saturates rather than wraps.
    let bit_len = u64::try_from(data.len())
        .unwrap_or(u64::MAX)
        .saturating_mul(8);
    let mut tail = [0_u8; 128];
    for (slot, b) in tail.iter_mut().zip(rem.iter()) {
        *slot = *b;
    }
    let rem_len = rem.len();
    if let Some(slot) = tail.get_mut(rem_len) {
        *slot = 0x80;
    }
    let total = if rem_len < 56 { 64 } else { 128 };
    let len_bytes = bit_len.to_be_bytes();
    if let Some(dst) = tail.get_mut(total - 8..total) {
        dst.copy_from_slice(&len_bytes);
    }
    for block in tail.chunks_exact(64).take(total / 64) {
        compress(&mut h, block);
    }
    let mut out = [0_u8; 32];
    for (dst, word) in out.chunks_exact_mut(4).zip(h.iter()) {
        dst.copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Lowercase hex of a digest.
#[must_use]
pub fn hex(digest: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in digest {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// `hex(sha256(data))`.
#[must_use]
pub fn sha256_hex(data: &[u8]) -> String {
    hex(&sha256(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nist_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn million_a() {
        let data = alloc::vec![b'a'; 1_000_000];
        assert_eq!(
            sha256_hex(&data),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn padding_boundaries_55_56_63_64_65_bytes() {
        // Reference values computed with Python hashlib (see the test log).
        let cases: [(usize, &str); 5] = [
            (
                55,
                "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
            ),
            (
                56,
                "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
            ),
            (
                63,
                "7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34",
            ),
            (
                64,
                "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
            ),
            (
                65,
                "635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0",
            ),
        ];
        for (n, want) in cases {
            let data = alloc::vec![b'a'; n];
            assert_eq!(sha256_hex(&data), want, "{n} bytes");
        }
    }
}
