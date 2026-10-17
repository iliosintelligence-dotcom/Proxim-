//! A small signed world: pinned root, key set, revocation list, manifest and one tier-0 pack.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use proxim_pack::encode::{self, Header, Row};
use proxim_pack::jws::{TYP_KEYSET, TYP_MANIFEST, TYP_REVOCATIONS};
use proxim_pack::keyset::accept_keyset;
use proxim_pack::manifest::accept_manifest;
use proxim_pack::revocation::accept_revocations;
use proxim_pack::testkit::TestKey;
use proxim_pack::{Keyset, Manifest, OpenedPack, PackResult, RevocationList, RootKey, crypto};
use serde_json::{Value, json};

pub const NOW: i64 = 1_791_705_600_000;
pub const DAY: i64 = 86_400_000;
/// A synthetic res-5 cell id (the verifier only compares it with the requested cell).
pub const CELL: u64 = 0x0851_f9a8_3fff_ffff;
pub const CELL_HEX: &str = "851f9a83fffffff";

pub fn root() -> TestKey {
    TestKey::from_label("test-root-1")
}
pub fn approve() -> TestKey {
    TestKey::from_label("test-approve-1")
}
pub fn manifest_key() -> TestKey {
    TestKey::from_label("test-manifest-1")
}

pub fn roots() -> Vec<RootKey> {
    let r = root();
    vec![RootKey::new(&r.kid, &r.public_sec1()).unwrap()]
}

/// Canonical JSON bytes of a value (keys sorted by `serde_json`'s default map).
pub fn canon(v: &Value) -> Vec<u8> {
    proxim_types::canon::value_to_canonical(v)
        .unwrap()
        .into_bytes()
}

pub fn keyset_doc(version: u64, keys: &[(&TestKey, &str)], revoked: &[&str]) -> Value {
    json!({
        "format": "proxim-keyset/1", "version": version,
        "issued_at_ms": NOW - DAY, "expires_at_ms": NOW + 30 * DAY,
        "keys": keys.iter().map(|(k, p)| json!({
            "kid": k.kid, "purpose": p, "pub": k.public_b64(),
            "not_before_ms": NOW - 365 * DAY, "not_after_ms": NOW + 365 * DAY,
        })).collect::<Vec<_>>(),
        "revoked_kids": revoked,
    })
}

pub fn keyset_token(doc: &Value) -> String {
    root().jws(TYP_KEYSET, &canon(doc))
}

pub fn default_keyset() -> Keyset {
    let (a, m) = (approve(), manifest_key());
    let doc = keyset_doc(1, &[(&a, "approve"), (&m, "manifest")], &[]);
    accept_keyset(keyset_token(&doc).as_bytes(), &roots(), None, NOW).unwrap()
}

pub fn signpost(id: &str, rev: u32, extra: &[(&str, Value)]) -> Value {
    let mut v = json!({
        "category": "food.cafe", "class": "general", "event": "enter", "id": id, "rev": rev,
        "sender": "sender-1", "sensitivity": 0, "pack_kind": "hosted_tile",
        "trigger": {"origin": {"lat_e7": 450000000, "lon_e7": 70000000},
            "shape": {"center": {"lat_e7": 450000000, "lon_e7": 70000000}, "kind": "circle", "radius_m": 200}},
        "valid_from_ms": 0, "valid_until_ms": NOW + 30 * DAY,
    });
    for (k, x) in extra {
        v[*k] = x.clone();
    }
    v
}

/// One signed revision: core bytes, creative bodies and the approve signature.
#[derive(Clone)]
pub struct Rev {
    pub core: Vec<u8>,
    pub bodies: Vec<Vec<u8>>,
    pub kid: String,
    pub sig: [u8; 64],
}

pub fn sign_rev(core: &Value, bodies: &[&str], key: &TestKey) -> Rev {
    let core = canon(core);
    let bodies: Vec<Vec<u8>> = bodies.iter().map(|b| b.as_bytes().to_vec()).collect();
    let digests: Vec<[u8; 32]> = bodies.iter().map(|b| encode::creative_digest(b)).collect();
    let sig = key.sign(&encode::signed_message(&core, &digests).unwrap());
    Rev {
        core,
        bodies,
        kid: key.kid.clone(),
        sig,
    }
}

pub fn row(r: &Rev, ship_mask: u8) -> Row {
    let bodies: Vec<&[u8]> = r.bodies.iter().map(Vec::as_slice).collect();
    Row {
        entry: encode::entry(&r.core, &bodies, ship_mask).unwrap(),
        kid: r.kid.clone(),
        sig: r.sig,
    }
}

pub fn all_shipped(r: &Rev) -> Row {
    row(r, u8::try_from((1u16 << r.bodies.len()) - 1).unwrap())
}

pub fn header(seq: u64) -> Header {
    Header {
        pack_kind: 1,
        tier: 0,
        cell: CELL,
        seq,
    }
}

pub fn build(h: &Header, rows: &[Row]) -> Vec<u8> {
    let stream = encode::stream(rows).unwrap();
    encode::pack(h, &encode::deflate(&stream), rows).unwrap()
}

pub fn manifest_doc(seq: u64, pack: &[u8], issued: i64, expires: i64) -> Value {
    json!({
        "format": "proxim-manifest/1", "cell": CELL_HEX, "seq": seq, "pack_kind": "hosted_tile",
        "issued_at_ms": issued, "expires_at_ms": expires,
        "tiers": [{"tier": 0, "bytes": pack.len(), "sha256": crypto::hex(&crypto::sha256(&[pack]))}],
    })
}

pub fn manifest_token(doc: &Value) -> String {
    manifest_key().jws(TYP_MANIFEST, &canon(doc))
}

pub fn manifest_for(seq: u64, pack: &[u8], ks: &Keyset) -> Manifest {
    let doc = manifest_doc(seq, pack, NOW - 60_000, NOW + DAY);
    accept_manifest(manifest_token(&doc).as_bytes(), ks, CELL, None, NOW).unwrap()
}

pub fn revocations(entries: Value, ks: &Keyset) -> RevocationList {
    let doc = json!({
        "format": "proxim-revocations/1", "seq": 1, "issued_at_ms": NOW - 60_000,
        "expires_at_ms": NOW + DAY, "entries": entries,
    });
    let t = manifest_key().jws(TYP_REVOCATIONS, &canon(&doc));
    accept_revocations(t.as_bytes(), ks, None, NOW).unwrap()
}

/// Build a pack from rows, sign a matching manifest and open it with no revocations.
pub fn open_rows(rows: &[Row]) -> PackResult<OpenedPack> {
    let ks = default_keyset();
    let pack = build(&header(7), rows);
    let m = manifest_for(7, &pack, &ks);
    proxim_pack::open_pack(&pack, &m, 0, &ks, &RevocationList::default(), NOW)
}

pub fn three_revs() -> Vec<Rev> {
    let a = approve();
    vec![
        sign_rev(
            &signpost("cafe-1", 1, &[]),
            &["Coffee ahead", "Fresh pastries"],
            &a,
        ),
        sign_rev(
            &signpost("fuel-1", 3, &[("sender", json!("sender-2"))]),
            &["Fuel in 2 km"],
            &a,
        ),
        sign_rev(
            &signpost("deli-1", 2, &[]),
            &["Deli: soup", "Deli: bread", "Deli: cheese"],
            &a,
        ),
    ]
}
pub mod vectors;
