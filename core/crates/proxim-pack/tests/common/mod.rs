//! Test-side encoder for pack-spec documents. Written independently of the verifier from
//! docs/pack-spec.md, so a shared misreading of the spec shows up as a failing test.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use p256::ecdsa::signature::Signer as _;
use p256::ecdsa::{Signature, SigningKey};
use proxim_pack::{Keyset, Manifest, TrustRoots, verify_keyset, verify_manifest};
use proxim_types::Signpost;
use proxim_types::canon::to_canonical_json;
use proxim_types::strict::from_json;
use sha2::{Digest as _, Sha256};

pub const NOW: i64 = 1_791_763_200_000; // 2026-10-12T00:00:00Z
pub const HOUR: i64 = 3_600_000;
pub const DAY: i64 = 24 * HOUR;
pub const CELL: u64 = 0x0851_fb43_ffff_ffff;

pub fn key(seed: u8) -> SigningKey {
    SigningKey::from_slice(&[seed; 32]).unwrap()
}

pub fn sec1(sk: &SigningKey) -> Vec<u8> {
    sk.verifying_key().to_sec1_point(false).as_ref().to_vec()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Low-S raw signature.
pub fn sign(sk: &SigningKey, msg: &[u8]) -> [u8; 64] {
    let sig: Signature = sk.sign(msg);
    sig.normalize_s().to_bytes().into()
}

/// The same signature with S replaced by n - S (valid ECDSA, rejected as high-S).
pub fn high_s(sig: &[u8; 64]) -> [u8; 64] {
    let s = Signature::from_slice(sig).unwrap();
    // n - s, computed through the curve's own negation of the scalar.
    let neg = Signature::from_scalars(s.r(), -*s.s()).unwrap();
    neg.to_bytes().into()
}

pub fn doc(kind: u8, kid: &str, payload: &str, sk: &SigningKey) -> Vec<u8> {
    let mut header = vec![kid.len() as u8];
    header.extend_from_slice(kid.as_bytes());
    header.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    header.extend_from_slice(payload.as_bytes());
    let mut msg = b"PROXIM-DOC-V1\x00".to_vec();
    msg.push(kind);
    msg.extend_from_slice(&header);
    let mut out = b"PXSD".to_vec();
    out.push(1);
    out.push(kind);
    out.extend_from_slice(&header);
    out.extend_from_slice(&sign(sk, &msg));
    out
}

pub struct OnlineKey {
    pub kid: &'static str,
    pub sk: SigningKey,
    pub not_before_ms: i64,
    pub not_after_ms: i64,
}

pub fn online(kid: &'static str, seed: u8) -> OnlineKey {
    OnlineKey {
        kid,
        sk: key(seed),
        not_before_ms: NOW - 30 * DAY,
        not_after_ms: NOW + 30 * DAY,
    }
}

/// Keyset payload; `keys` are written in the given order (keys sort inside each record).
pub fn keyset_payload(
    seq: u64,
    keys: &[&OnlineKey],
    revoked: &[&str],
    issued: i64,
    expires: i64,
) -> String {
    let keys: Vec<String> = keys
        .iter()
        .map(|k| {
            format!(
                "{{\"kid\":\"{}\",\"not_after_ms\":{},\"not_before_ms\":{},\"pub\":\"{}\"}}",
                k.kid,
                k.not_after_ms,
                k.not_before_ms,
                hex(&sec1(&k.sk))
            )
        })
        .collect();
    let revoked: Vec<String> = revoked.iter().map(|k| format!("\"{k}\"")).collect();
    format!(
        "{{\"expires_ms\":{expires},\"issued_ms\":{issued},\"keys\":[{}],\"revoked_kids\":[{}],\"seq\":{seq},\"v\":1}}",
        keys.join(","),
        revoked.join(",")
    )
}

pub fn manifest_payload(cell: u64, seq: u64, built: i64, expires: i64, tiers: &[&[u8]]) -> String {
    let tiers: Vec<String> = tiers
        .iter()
        .enumerate()
        .map(|(i, t)| {
            format!(
                "{{\"bytes\":{},\"sha256\":\"{}\",\"tier\":{i}}}",
                t.len(),
                hex(&sha(t))
            )
        })
        .collect();
    format!(
        "{{\"built_ms\":{built},\"cell\":{cell},\"expires_ms\":{expires},\"seq\":{seq},\"tiers\":[{}],\"v\":1}}",
        tiers.join(",")
    )
}

pub fn signpost(id: &str, rev: u32, valid_until_ms: i64) -> Signpost {
    let json = format!(
        "{{\"category\":\"cafe\",\"class\":\"general\",\"event\":\"enter\",\"id\":\"{id}\",\"rank_milli\":1000,\"rev\":{rev},\"sender\":\"sender-1\",\"sensitivity\":0,\"trigger\":{{\"origin\":{{\"lat_e7\":450000000,\"lon_e7\":70000000}},\"shape\":{{\"center\":{{\"lat_e7\":450000000,\"lon_e7\":70000000}},\"kind\":\"circle\",\"radius_m\":200}}}},\"valid_from_ms\":0,\"valid_until_ms\":{valid_until_ms}}}"
    );
    from_json(json.as_bytes()).unwrap()
}

pub fn core(sp: &Signpost, expires_ms: i64) -> String {
    format!(
        "{{\"expires_ms\":{expires_ms},\"signpost\":{}}}",
        to_canonical_json(sp).unwrap()
    )
}

pub fn creative(headline: &str, body: &str, glanceable: &str) -> String {
    format!("{{\"body\":\"{body}\",\"glanceable\":\"{glanceable}\",\"headline\":\"{headline}\"}}")
}

pub fn creative_digest(body: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"PROXIM-CREATIVE-V2\x00");
    h.update(body);
    h.finalize().into()
}

/// Envelope v2 entry. `digests` gives the signed digest of each declared creative (normally the
/// digest of `bodies[i]`); bodies whose mask bit is clear are replaced by their digest on the wire.
pub fn entry_with(
    kid: &str,
    sk: &SigningKey,
    core: &str,
    bodies: &[String],
    digests: &[[u8; 32]],
    mask: u8,
) -> Vec<u8> {
    let n = bodies.len() as u8;
    let mut core_wire = (core.len() as u16).to_be_bytes().to_vec();
    core_wire.extend_from_slice(core.as_bytes());
    let mut msg = b"PROXIM-ENV-V2\x00".to_vec();
    msg.extend_from_slice(&core_wire);
    msg.push(n);
    for d in digests {
        msg.extend_from_slice(d);
    }
    let mut out = vec![2, kid.len() as u8];
    out.extend_from_slice(kid.as_bytes());
    out.extend_from_slice(&core_wire);
    out.push(n);
    out.push(mask);
    for (i, b) in bodies.iter().enumerate() {
        if mask & (1 << i) != 0 {
            out.extend_from_slice(&(b.len() as u16).to_be_bytes());
            out.extend_from_slice(b.as_bytes());
        } else {
            out.extend_from_slice(&digests[i]);
        }
    }
    out.extend_from_slice(&sign(sk, &msg));
    out
}

pub fn entry(kid: &str, sk: &SigningKey, core: &str, bodies: &[String], mask: u8) -> Vec<u8> {
    let digests: Vec<[u8; 32]> = bodies
        .iter()
        .map(|b| creative_digest(b.as_bytes()))
        .collect();
    entry_with(kid, sk, core, bodies, &digests, mask)
}

pub fn pack(cell: u64, seq: u64, tier: u8, entries: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"PXPK".to_vec();
    out.push(1);
    out.extend_from_slice(&cell.to_be_bytes());
    out.extend_from_slice(&seq.to_be_bytes());
    out.push(tier);
    out.extend_from_slice(&(entries.len() as u16).to_be_bytes());
    for e in entries {
        out.extend_from_slice(&(e.len() as u32).to_be_bytes());
        out.extend_from_slice(e);
    }
    out
}

/// A complete, valid world: one root, one online key, one cell with a three-creative entry.
pub struct World {
    pub root: SigningKey,
    pub roots: TrustRoots,
    pub k1: OnlineKey,
    pub keyset_bytes: Vec<u8>,
    pub entry: Vec<u8>,
    pub pack: Vec<u8>,
    pub manifest_bytes: Vec<u8>,
}

pub fn bodies() -> Vec<String> {
    vec![
        creative(
            "Fresh coffee ahead",
            "Two minutes from your route, open until 6pm",
            "Coffee ahead",
        ),
        creative(
            "Pastries baked today",
            "Warm croissants every morning",
            "Pastries ahead",
        ),
        creative(
            "Quiet corner seats",
            "Wi-Fi and plugs at every table",
            "Work spot ahead",
        ),
    ]
}

pub fn world() -> World {
    let root = key(1);
    let roots = TrustRoots::new(&[("root-1", &sec1(&root))]).unwrap();
    let k1 = online("online-1", 2);
    let keyset_bytes = doc(
        1,
        "root-1",
        &keyset_payload(1, &[&k1], &[], NOW - DAY, NOW + 7 * DAY),
        &root,
    );
    let sp = signpost("a-1", 1, NOW + 60 * DAY);
    let entry = entry(k1.kid, &k1.sk, &core(&sp, NOW + 14 * DAY), &bodies(), 0b111);
    let pack = pack(CELL, 7, 0, std::slice::from_ref(&entry));
    let manifest_bytes = doc(
        2,
        k1.kid,
        &manifest_payload(CELL, 7, NOW - HOUR, NOW + 23 * HOUR, &[&pack]),
        &k1.sk,
    );
    World {
        root,
        roots,
        k1,
        keyset_bytes,
        entry,
        pack,
        manifest_bytes,
    }
}

impl World {
    pub fn keyset(&self) -> Keyset {
        verify_keyset(&self.roots, &self.keyset_bytes, NOW, None).unwrap()
    }

    pub fn manifest(&self) -> Manifest {
        verify_manifest(&self.keyset(), &self.manifest_bytes, CELL, NOW, None).unwrap()
    }
}
