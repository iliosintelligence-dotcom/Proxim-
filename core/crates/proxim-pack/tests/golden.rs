//! Pack golden vectors: the checked-in file is current, and every case replays from the file alone.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use proxim_pack::keyset::accept_keyset;
use proxim_pack::manifest::{accept_manifest, parse_cell};
use proxim_pack::revocation::accept_revocations;
use proxim_pack::testkit::summary;
use proxim_pack::{Guard, RevocationList, RootKey, b64, crypto, open_pack};
use serde_json::{Value, json};

fn load() -> Value {
    let text = std::fs::read_to_string(common::vectors::golden_path()).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn guard(v: &Value) -> Option<Guard> {
    v.get("prev").map(|p| Guard {
        seq: p["seq"].as_u64().unwrap(),
        sha256: crypto::parse_hex(p["sha256"].as_str().unwrap()).unwrap(),
    })
}

fn err(r: proxim_pack::ReasonCode) -> Value {
    json!({ "error": r })
}

#[test]
fn checked_in_vectors_are_current() {
    let text = common::vectors::render();
    let old = std::fs::read_to_string(common::vectors::golden_path()).unwrap();
    assert!(
        old == text,
        "stale: PROXIM_BLESS=1 cargo run -p proxim-pack --features testkit --example gen_pack_vectors"
    );
}

#[test]
fn every_case_replays_from_the_file() {
    let g = load();
    let now = g["now_ms"].as_i64().unwrap();
    let cell = parse_cell(g["cell"].as_str().unwrap()).unwrap();
    let roots: Vec<RootKey> = g["roots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let pubkey = b64::decode(r["pub"].as_str().unwrap().as_bytes()).unwrap();
            RootKey::new(r["kid"].as_str().unwrap(), &pubkey).unwrap()
        })
        .collect();
    for c in g["keyset_cases"].as_array().unwrap() {
        let got = accept_keyset(
            c["token"].as_str().unwrap().as_bytes(),
            &roots,
            guard(c).as_ref(),
            now,
        )
        .map_or_else(err, |k| json!({"version": k.guard.seq}));
        assert_eq!(got, c["expect"], "keyset {}", c["name"]);
    }
    let ks = accept_keyset(g["keyset"].as_str().unwrap().as_bytes(), &roots, None, now).unwrap();
    for c in g["manifest_cases"].as_array().unwrap() {
        let got = accept_manifest(
            c["token"].as_str().unwrap().as_bytes(),
            &ks,
            cell,
            guard(c).as_ref(),
            now,
        )
        .map_or_else(err, |m| json!({"seq": m.guard.seq}));
        assert_eq!(got, c["expect"], "manifest {}", c["name"]);
    }
    let man = accept_manifest(
        g["manifest"].as_str().unwrap().as_bytes(),
        &ks,
        cell,
        None,
        now,
    )
    .unwrap();
    let rl = accept_revocations(
        g["revocations"].as_str().unwrap().as_bytes(),
        &ks,
        None,
        now,
    )
    .unwrap();
    for c in g["pack_cases"].as_array().unwrap() {
        let bytes = b64::decode(c["pack"].as_str().unwrap().as_bytes()).unwrap();
        let list = if c["revocations"] == true {
            rl.clone()
        } else {
            RevocationList::default()
        };
        let got = open_pack(&bytes, &man, 0, &ks, &list, now).map_or_else(err, |p| summary(&p));
        assert_eq!(got, c["expect"], "pack {}", c["name"]);
    }
}

/// The TypeScript compiler's pack (`packages/pack`, `compiled_v1.json`) opens in Rust with the same
/// verdict per signpost as the Rust-built golden pack.
#[test]
fn the_typescript_compiled_pack_opens_with_the_same_verdicts() {
    let g = load();
    let path = common::vectors::golden_path().with_file_name("compiled_v1.json");
    let c: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let bytes = b64::decode(c["pack"].as_str().unwrap().as_bytes()).unwrap();
    assert_eq!(
        crypto::hex(&crypto::sha256(&[&bytes])),
        c["sha256"].as_str().unwrap()
    );
    let now = g["now_ms"].as_i64().unwrap();
    let ks = common::default_keyset();
    let doc = common::manifest_doc(9, &bytes, now - 60_000, now + common::DAY);
    let man = accept_manifest(
        common::manifest_token(&doc).as_bytes(),
        &ks,
        common::CELL,
        None,
        now,
    )
    .unwrap();
    let rl = accept_revocations(
        g["revocations"].as_str().unwrap().as_bytes(),
        &ks,
        None,
        now,
    )
    .unwrap();
    let opened = summary(&open_pack(&bytes, &man, 0, &ks, &rl, now).unwrap());
    let want = g["pack_cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "opens")
        .unwrap()["expect"]
        .clone();
    // Verdicts keyed by signpost id (the compiler orders by rank, so indices differ).
    let ids_ts: Vec<&str> = c["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let ids_rust: Vec<String> = g["snapshot"]["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            serde_json::from_str::<Value>(r["core"].as_str().unwrap()).unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    let verdicts = |s: &Value, ids: &dyn Fn(usize) -> String| {
        let mut v: Vec<(String, Value)> = s["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    e["id"].as_str().unwrap().to_string(),
                    json!({"creatives": e["creatives"], "rev": e["rev"], "kid": e["kid"]}),
                )
            })
            .chain(s["dropped"].as_array().unwrap().iter().map(|d| {
                (
                    ids(usize::try_from(d["index"].as_u64().unwrap()).unwrap()),
                    d["reason"].clone(),
                )
            }))
            .collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    };
    assert_eq!(
        verdicts(&opened, &|i| ids_ts[i].to_string()),
        verdicts(&want, &|i| ids_rust[i].clone())
    );
}
