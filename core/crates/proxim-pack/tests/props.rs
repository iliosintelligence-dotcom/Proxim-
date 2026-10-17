//! Fuzz-style properties: the parsers never panic, and no mutation of a valid entry, manifest or
//! keyset is ever accepted as something different (A02 N-35: "0 accepted-but-different").
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proptest::prelude::*;
use proxim_pack::{verify_entry, verify_keyset, verify_manifest, verify_pack};

proptest! {
    #![proptest_config(ProptestConfig { cases: 2000, ..ProptestConfig::default() })]

    #[test]
    fn random_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        let w = world();
        let ks = w.keyset();
        let _ = verify_entry(&bytes, &ks, NOW);
        let _ = verify_keyset(&w.roots, &bytes, NOW, None);
        let _ = verify_manifest(&ks, &bytes, CELL, NOW, None);
        let _ = verify_pack(&w.manifest(), 0, &bytes, &ks, NOW);
    }

    #[test]
    fn mutated_documents_are_never_accepted(
        which in 0usize..3,
        edits in proptest::collection::vec((any::<usize>(), any::<u8>()), 1..4),
        trunc in proptest::option::of(any::<usize>()),
    ) {
        let w = world();
        let ks = w.keyset();
        let original = match which { 0 => w.entry.clone(), 1 => w.manifest_bytes.clone(), _ => w.keyset_bytes.clone() };
        let mut m = original.clone();
        for (at, x) in edits {
            let i = at % m.len();
            m[i] ^= x;
        }
        if let Some(t) = trunc {
            m.truncate(t % (m.len() + 1));
        }
        prop_assume!(m != original);
        let accepted = match which {
            0 => verify_entry(&m, &ks, NOW).is_ok(),
            1 => verify_manifest(&ks, &m, CELL, NOW, None).is_ok(),
            _ => verify_keyset(&w.roots, &m, NOW, None).is_ok(),
        };
        prop_assert!(!accepted);
    }
}
