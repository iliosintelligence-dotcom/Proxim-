//! The pack golden vectors (`core/testdata/golden/pack/pack_v1.json`): built here, checked by
//! `tests/golden.rs` in Rust and by `packages/pack` in TypeScript.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use super::*;
use proxim_pack::jws::{TYP_KEYSET, TYP_MANIFEST, TYP_REVOCATIONS};
use proxim_pack::keyset::accept_keyset;
use proxim_pack::manifest::accept_manifest;
use proxim_pack::revocation::accept_revocations;
use proxim_pack::testkit::{TestKey, summary};
use proxim_pack::{ReasonCode, b64, open_pack};
use serde_json::{Value, json};

fn code(r: ReasonCode) -> Value {
    serde_json::to_value(r).unwrap()
}

fn rev_json(r: &Rev) -> Value {
    json!({
        "core": String::from_utf8(r.core.clone()).unwrap(),
        "bodies": r.bodies.iter().map(|b| String::from_utf8(b.clone()).unwrap()).collect::<Vec<_>>(),
        "kid": r.kid, "sig": b64::encode(&r.sig),
    })
}

/// The snapshot both compilers build from: signed revisions as the database would hold them.
pub fn snapshot() -> Vec<Rev> {
    let a = approve();
    let mut v = three_revs();
    v.push(sign_rev(
        &signpost("soon-1", 1, &[("live_at_ms", json!(NOW + 3_600_000))]),
        &["Opening at noon"],
        &a,
    ));
    v.push(sign_rev(
        &signpost("rank-1", 1, &[("rank_milli", json!(2500))]),
        &["Top pick", "Top pick, today only"],
        &a,
    ));
    v.push(sign_rev(
        &signpost("stray-1", 1, &[]),
        &["Not approved by a listed key"],
        &TestKey::from_label("stranger"),
    ));
    v
}

pub fn vectors() -> Value {
    let (a, m) = (approve(), manifest_key());
    let ks_doc = keyset_doc(1, &[(&a, "approve"), (&m, "manifest")], &[]);
    let ks_token = keyset_token(&ks_doc);
    let ks = accept_keyset(ks_token.as_bytes(), &roots(), None, NOW).unwrap();

    let revs = snapshot();
    let deli_vid1 = proxim_pack::crypto::hex(&encode::creative_digest(&revs[2].bodies[1])[..4]);
    let rl_doc = json!({
        "format": "proxim-revocations/1", "seq": 4, "issued_at_ms": NOW - 60_000,
        "expires_at_ms": NOW + DAY,
        "entries": [
            {"scope": "signpost_rev", "id": "fuel-1", "rev": 3},
            {"scope": "variant", "id": "deli-1", "rev": 2, "vid": deli_vid1},
        ],
    });
    let rl_token = m.jws(TYP_REVOCATIONS, &canon(&rl_doc));
    let rl = accept_revocations(rl_token.as_bytes(), &ks, None, NOW).unwrap();

    // Pack rows: everything shipped except rank-1, which withholds its second wording.
    let rows: Vec<Row> = revs
        .iter()
        .map(|r| {
            if r.core.windows(6).any(|w| w == b"rank-1") {
                row(r, 0b01)
            } else {
                all_shipped(r)
            }
        })
        .collect();
    let pack = build(&header(9), &rows);
    let man_doc = manifest_doc(9, &pack, NOW - 60_000, NOW + DAY);
    let man_token = manifest_token(&man_doc);
    let man = accept_manifest(man_token.as_bytes(), &ks, CELL, None, NOW).unwrap();
    let opened = open_pack(&pack, &man, 0, &ks, &rl, NOW).unwrap();

    let mut flipped = pack.clone();
    let mid = flipped.len() / 2;
    flipped[mid] ^= 0x80;
    let old_pack = build(&header(8), &rows);

    let pack_cases = vec![
        json!({"name": "opens", "pack": b64::encode(&pack), "revocations": true, "expect": summary(&opened)}),
        json!({"name": "opens_without_revocations", "pack": b64::encode(&pack), "revocations": false,
               "expect": summary(&open_pack(&pack, &man, 0, &ks, &Default::default(), NOW).unwrap())}),
        json!({"name": "byte_flipped", "pack": b64::encode(&flipped), "revocations": true,
               "expect": {"error": code(ReasonCode::SignatureInvalid)}}),
        json!({"name": "replayed_older_pack", "pack": b64::encode(&old_pack), "revocations": true,
               "expect": {"error": code(ReasonCode::SignatureInvalid)}}),
    ];
    for c in &pack_cases[2..] {
        let bytes = b64::decode(c["pack"].as_str().unwrap().as_bytes()).unwrap();
        assert!(open_pack(&bytes, &man, 0, &ks, &rl, NOW).is_err());
    }

    let man_prev =
        json!({"seq": man.guard.seq, "sha256": proxim_pack::crypto::hex(&man.guard.sha256)});
    let older = manifest_token(&manifest_doc(8, &old_pack, NOW - 60_000, NOW + DAY));
    let expired = manifest_token(&manifest_doc(9, &pack, NOW - 2 * DAY, NOW - DAY));
    let as_keyset = root().jws(TYP_KEYSET, &canon(&man_doc));
    let by_approve = a.jws(TYP_MANIFEST, &canon(&man_doc));
    let manifest_cases = vec![
        json!({"name": "accepted", "token": man_token, "expect": {"seq": 9}}),
        json!({"name": "same_again", "token": man_token, "prev": man_prev, "expect": {"seq": 9}}),
        json!({"name": "rollback", "token": older, "prev": man_prev, "expect": {"error": code(ReasonCode::SeqRollback)}}),
        json!({"name": "expired", "token": expired, "expect": {"error": code(ReasonCode::PackExpired)}}),
        json!({"name": "wrong_type", "token": as_keyset, "expect": {"error": code(ReasonCode::SignatureInvalid)}}),
        json!({"name": "wrong_purpose", "token": by_approve, "expect": {"error": code(ReasonCode::KidRevoked)}}),
    ];

    let ks_prev = json!({"seq": 2, "sha256": "00".repeat(32)});
    let by_non_root = a.jws(TYP_KEYSET, &canon(&ks_doc));
    let keyset_cases = vec![
        json!({"name": "accepted", "token": ks_token, "expect": {"version": 1}}),
        json!({"name": "rollback", "token": ks_token, "prev": ks_prev, "expect": {"error": code(ReasonCode::SeqRollback)}}),
        json!({"name": "not_root", "token": by_non_root, "expect": {"error": code(ReasonCode::SignatureInvalid)}}),
    ];

    json!({
        "header": {"format": "proxim-golden/1", "kind": "pack", "id": "pack_v1", "engine_spec": 1, "synthetic": true},
        "pack_format": "docs/pack-format.md",
        "now_ms": NOW, "cell": CELL_HEX, "tier": 0,
        "roots": [{"kid": root().kid, "pub": root().public_b64()}],
        "keyset": ks_token, "revocations": rl_token, "manifest": man_token,
        "snapshot": {"cell": CELL_HEX, "seq": 9, "tier": 0, "pack_kind": "hosted_tile",
                     "ship_masks": {"rank-1": 1},
                     "revisions": revs.iter().map(rev_json).collect::<Vec<_>>()},
        "pack_cases": pack_cases, "manifest_cases": manifest_cases, "keyset_cases": keyset_cases,
    })
}

/// Canonical text with a trailing newline (engine spec 9).
pub fn render() -> String {
    let mut s = proxim_types::canon::value_to_canonical(&vectors()).unwrap();
    s.push('\n');
    s
}

pub fn golden_path() -> std::path::PathBuf {
    std::env::var_os("PROXIM_CORE_DIR")
        .map_or_else(
            || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            std::path::PathBuf::from,
        )
        .join("testdata/golden/pack/pack_v1.json")
}
