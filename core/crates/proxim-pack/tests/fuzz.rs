//! Parser fuzz (PLAN M2 "fuzz run on the parser"; Addendum 02 N-34 "0 accepted-but-different").
//!
//! Mutations are applied under a manifest re-signed over the mutated bytes, so the hash check
//! passes and the container, stream and entry parsers see the damage. The verifier must never
//! panic, and anything it accepts must be exactly what was signed (a ship mask may only withhold).
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed};
use proxim_pack::encode::{self, Row};
use proxim_pack::jws::{TYP_KEYSET, TYP_MANIFEST, TYP_REVOCATIONS};
use proxim_pack::{OpenedPack, RevocationList, open_pack};

fn cfg(cases: u32) -> Config {
    Config {
        cases,
        failure_persistence: None,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5EED_2026),
        max_shrink_iters: 100,
        ..Config::default()
    }
}

#[derive(Clone, Debug)]
enum Edit {
    Flip(usize, u8),
    Insert(usize, u8),
    Delete(usize),
    Truncate(usize),
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        (any::<usize>(), 1u8..=255).prop_map(|(i, x)| Edit::Flip(i, x)),
        (any::<usize>(), any::<u8>()).prop_map(|(i, x)| Edit::Insert(i, x)),
        any::<usize>().prop_map(Edit::Delete),
        any::<usize>().prop_map(Edit::Truncate),
    ]
}

fn apply(b: &mut Vec<u8>, edits: &[Edit]) {
    for e in edits {
        let n = b.len().max(1);
        match *e {
            Edit::Flip(i, x) if !b.is_empty() => b[i % n] ^= x,
            Edit::Insert(i, x) => b.insert(i % (b.len() + 1), x),
            Edit::Delete(i) if !b.is_empty() => {
                b.remove(i % n);
            }
            Edit::Truncate(i) => b.truncate(i % (b.len() + 1)),
            _ => {}
        }
    }
}

fn rows() -> (Vec<Rev>, Vec<Row>) {
    let revs = three_revs();
    let rows = revs.iter().map(all_shipped).collect();
    (revs, rows)
}

/// Open `pack` under a manifest re-signed for exactly these bytes.
fn open_resigned(pack: &[u8]) -> Option<OpenedPack> {
    let ks = default_keyset();
    let m = manifest_for(7, pack, &ks);
    open_pack(pack, &m, 0, &ks, &RevocationList::default(), NOW).ok()
}

/// Everything accepted equals the signed original (by id and rev), body for body.
fn accepted_is_original(p: &OpenedPack, revs: &[Rev]) {
    for e in &p.entries {
        let orig = revs
            .iter()
            .find(|r| {
                let v: serde_json::Value = serde_json::from_slice(&r.core).unwrap();
                v["id"] == e.signpost.id.as_str() && v["rev"] == e.signpost.rev
            })
            .expect("accepted entry was signed");
        let orig_sp: proxim_types::Signpost =
            proxim_types::strict::from_canonical_json(&orig.core).unwrap();
        let mut signed = e.signpost.clone();
        let w = signed
            .wordings
            .take()
            .expect("the pack layer fills in the wordings");
        assert_eq!(signed, orig_sp);
        // The wordings are the signed digests of the original bodies, nothing else.
        let want: Vec<String> = orig
            .bodies
            .iter()
            .map(|b| proxim_pack::crypto::hex(&encode::creative_digest(b)))
            .collect();
        assert_eq!(w.digests, want);
        for c in &e.creatives {
            assert_eq!(c.body, orig.bodies[usize::from(c.index)]);
        }
        assert_eq!(e.creatives.first().map(|c| c.index), Some(0));
    }
}

proptest! {
    #![proptest_config(cfg(300))]

    #[test]
    fn mutated_pack_bytes_never_panic_and_never_accept_different_content(
        edits in proptest::collection::vec(edit(), 1..6)
    ) {
        let (revs, rows) = rows();
        let mut pack = build(&header(7), &rows);
        apply(&mut pack, &edits);
        if let Some(p) = open_resigned(&pack) {
            accepted_is_original(&p, &revs);
        }
    }

    #[test]
    fn mutated_entry_streams_never_panic_and_never_accept_different_content(
        edits in proptest::collection::vec(edit(), 1..6)
    ) {
        let (revs, rows) = rows();
        let mut stream = encode::stream(&rows).unwrap();
        apply(&mut stream, &edits);
        let pack = encode::pack(&header(7), &encode::deflate(&stream), &rows).unwrap();
        if let Some(p) = open_resigned(&pack) {
            accepted_is_original(&p, &revs);
        }
    }

    #[test]
    fn arbitrary_bytes_as_a_pack_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..400)) {
        let _ = open_resigned(&bytes);
    }

    #[test]
    fn mutated_tokens_never_panic(edits in proptest::collection::vec(edit(), 1..4), which in 0u8..3) {
        let ks = default_keyset();
        let pack = build(&header(7), &rows().1);
        let doc = manifest_doc(7, &pack, NOW - 60_000, NOW + DAY);
        let mut t = match which {
            0 => manifest_token(&doc).into_bytes(),
            1 => keyset_token(&keyset_doc(1, &[(&approve(), "approve")], &[])).into_bytes(),
            _ => manifest_key().jws(TYP_REVOCATIONS, br#"{"entries":[],"expires_at_ms":1,"format":"proxim-revocations/1","issued_at_ms":0,"seq":1}"#).into_bytes(),
        };
        apply(&mut t, &edits);
        let _ = proxim_pack::manifest::accept_manifest(&t, &ks, CELL, None, NOW);
        let _ = proxim_pack::keyset::accept_keyset(&t, &roots(), None, NOW);
        let _ = proxim_pack::revocation::accept_revocations(&t, &ks, None, NOW);
        let _ = proxim_pack::jws::parse(&t, TYP_KEYSET);
        let _ = proxim_pack::jws::parse(&t, TYP_MANIFEST);
    }
}
