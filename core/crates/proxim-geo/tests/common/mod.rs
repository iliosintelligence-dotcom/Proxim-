//! Shared test support: an independent WGS84 reference (ECEF -> local tangent-plane ENU), a spherical
//! destination-point formula, and a tiny SHA-256. Floats are fine here: tests are not the engine.
#![allow(
    dead_code,
    clippy::float_arithmetic,
    clippy::float_cmp,
    clippy::cast_possible_truncation
)]

use proxim_types::{Enu, Position};

pub const A: f64 = 6_378_137.0;
pub const INV_F: f64 = 298.257_223_563;

pub fn e2() -> f64 {
    let f = 1.0 / INV_F;
    f * (2.0 - f)
}

pub fn deg(e7: i32) -> f64 {
    f64::from(e7) / 1e7
}

pub fn e7(deg: f64) -> i32 {
    libm::floor(deg * 1e7 + 0.5) as i32
}

pub fn pos(lat_deg: f64, lon_deg: f64) -> Position {
    Position {
        lat_e7: e7(lat_deg),
        lon_e7: e7(lon_deg),
    }
}

/// Geodetic degrees (h = 0) to ECEF metres.
pub fn ecef(lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let phi = lat_deg.to_radians();
    let lam = lon_deg.to_radians();
    let s = libm::sin(phi);
    let n = A / libm::sqrt(1.0 - e2() * s * s);
    [
        n * libm::cos(phi) * libm::cos(lam),
        n * libm::cos(phi) * libm::sin(lam),
        n * (1.0 - e2()) * s,
    ]
}

/// True local tangent-plane east/north in millimetres of `b` relative to `a`.
pub fn true_enu_mm(a: Position, b: Position) -> (f64, f64) {
    let (la, oa) = (deg(a.lat_e7), deg(a.lon_e7));
    let pa = ecef(la, oa);
    let pb = ecef(deg(b.lat_e7), deg(b.lon_e7));
    let d = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
    let (phi, lam) = (la.to_radians(), oa.to_radians());
    let (sp, cp, sl, cl) = (
        libm::sin(phi),
        libm::cos(phi),
        libm::sin(lam),
        libm::cos(lam),
    );
    let e = -sl * d[0] + cl * d[1];
    let n = -sp * cl * d[0] - sp * sl * d[1] + cp * d[2];
    (e * 1000.0, n * 1000.0)
}

/// Spherical destination point (R = 6371008.8 m) at `bearing_deg` and `dist_m` from `a`.
pub fn destination(a: Position, bearing_deg: f64, dist_m: f64) -> Position {
    const R: f64 = 6_371_008.8;
    let (lat1, lon1) = (deg(a.lat_e7).to_radians(), deg(a.lon_e7).to_radians());
    let br = bearing_deg.to_radians();
    let dr = dist_m / R;
    let lat2 = libm::asin(
        libm::sin(lat1) * libm::cos(dr) + libm::cos(lat1) * libm::sin(dr) * libm::cos(br),
    );
    let lon2 = lon1
        + libm::atan2(
            libm::sin(br) * libm::sin(dr) * libm::cos(lat1),
            libm::cos(dr) - libm::sin(lat1) * libm::sin(lat2),
        );
    let mut lon = lon2.to_degrees();
    while lon > 180.0 {
        lon -= 360.0;
    }
    while lon < -180.0 {
        lon += 360.0;
    }
    pos(lat2.to_degrees(), lon)
}

pub fn enu(e: i32, n: i32) -> Enu {
    Enu { e_mm: e, n_mm: n }
}

/// SHA-256 (FIPS 180-4), lowercase hex. Verified against the standard vectors in `trig_tables.rs`.
pub fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}
