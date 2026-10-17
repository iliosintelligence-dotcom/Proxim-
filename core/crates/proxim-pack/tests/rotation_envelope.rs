//! OI-ROT-1: the envelope v2 vectors of `core/testdata/golden/rotation/golden_v2.json` (19 entries,
//! including the 4-byte vid second preimage) pass through the M2 entry verifier. Their wire form is
//! a pack entry record followed by its 64-byte signature (pack-format 4.1).
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

use proxim_pack::pack::verify_envelope;
use proxim_pack::{ReasonCode, crypto};
use serde_json::Value;
use std::path::PathBuf;

fn golden() -> Value {
    let p = std::env::var_os("PROXIM_CORE_DIR")
        .map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            PathBuf::from,
        )
        .join("testdata/golden/rotation/golden_v2.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn every_golden_v2_envelope_entry_gets_its_verdict() {
    let g = golden();
    let env = &g["envelope"];
    let key = crypto::public_key(&unhex(env["pubkey_uncompressed_hex"].as_str().unwrap())).unwrap();
    let entries = env["entries"].as_array().unwrap();
    for e in entries {
        let name = e["name"].as_str().unwrap();
        let wire = unhex(e["wire_hex"].as_str().unwrap());
        let got = if wire.len() < 64 {
            Err(ReasonCode::SignatureInvalid)
        } else {
            let (entry, sig) = wire.split_at(wire.len() - 64);
            verify_envelope(&key, entry, sig)
        };
        match (e["expect"].as_str().unwrap(), got) {
            ("accept", Ok((_, bodies))) => {
                let idx: Vec<u64> = bodies.iter().map(|(i, _)| u64::from(*i)).collect();
                let want: Vec<u64> = e["shipped_idx"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_u64().unwrap())
                    .collect();
                assert_eq!(idx, want, "{name}");
                let hashes: Vec<String> = bodies
                    .iter()
                    .map(|(_, b)| crypto::hex(&crypto::sha256(&[b])))
                    .collect();
                let want: Vec<String> = e["body_sha256_hex"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_str().unwrap().into())
                    .collect();
                assert_eq!(hashes, want, "{name}");
            }
            ("reject", Err(code)) => assert_eq!(code, ReasonCode::SignatureInvalid, "{name}"),
            (want, got) => panic!("{name}: want {want}, got {:?}", got.map(|_| "accept")),
        }
    }
    assert_eq!(entries.len(), 19);
    // The fixture: two different bodies share a 4-byte vid; only the full digest tells them apart.
    let fx = &env["vid_collision"];
    let d = |h: &str| proxim_pack::encode::creative_digest(&unhex(h));
    let (legit, forged) = (
        d(fx["legit_hex"].as_str().unwrap()),
        d(fx["forged_hex"].as_str().unwrap()),
    );
    assert_eq!(legit[..4], forged[..4]);
    assert_ne!(legit, forged);
}
