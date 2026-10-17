//! Invariant I10 (docs/privacy-invariants.md): unsigned, expired, rolled-back or revoked content is
//! never rendered. The per-class freshness window of the revocation list is applied by the SDK
//! (M3 onwards); this covers everything the pack layer decides.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_pack::manifest::accept_manifest;
use proxim_pack::testkit::TestKey;
use proxim_pack::{ReasonCode, RevocationList, open_pack};
use serde_json::json;

#[test]
fn unsigned_expired_rolled_back_or_revoked_is_never_rendered() {
    let a = approve();
    let good = sign_rev(&signpost("ok-1", 1, &[]), &["Hello"], &a);
    // Unsigned: a zero signature, and a signature by a key nobody approved.
    let mut unsigned = good.clone();
    unsigned.sig = [0; 64];
    let stranger = sign_rev(
        &signpost("st-1", 1, &[]),
        &["Hello"],
        &TestKey::from_label("stranger"),
    );
    // A creative whose body differs from the signed digest.
    let mut swapped = sign_rev(&signpost("sw-1", 1, &[]), &["Hello", "World"], &a);
    swapped.bodies[1] = b"Wordl".to_vec();
    let p = open_rows(&[
        all_shipped(&unsigned),
        all_shipped(&stranger),
        all_shipped(&swapped),
        all_shipped(&good),
    ])
    .unwrap();
    let rendered: Vec<_> = p.entries.iter().map(|e| e.signpost.id.as_str()).collect();
    assert_eq!(rendered, ["ok-1"]);
    assert!(
        p.dropped
            .iter()
            .all(|d| d.reason == ReasonCode::SignatureInvalid)
    );

    let ks = default_keyset();
    let pack = build(&header(5), &[all_shipped(&good)]);
    // Expired manifest: nothing from it can be opened.
    let expired = manifest_doc(5, &pack, NOW - 2 * DAY, NOW - 1);
    assert_eq!(
        accept_manifest(manifest_token(&expired).as_bytes(), &ks, CELL, None, NOW),
        Err(ReasonCode::PackExpired)
    );
    // Rolled back: seq 4 after seq 5.
    let m5 = manifest_for(5, &pack, &ks);
    let older = build(&header(4), &[all_shipped(&good)]);
    let doc4 = manifest_doc(4, &older, NOW - 60_000, NOW + DAY);
    assert_eq!(
        accept_manifest(
            manifest_token(&doc4).as_bytes(),
            &ks,
            CELL,
            Some(&m5.guard),
            NOW
        ),
        Err(ReasonCode::SeqRollback)
    );
    // Revoked.
    let list = revocations(json!([{"scope": "signpost", "id": "ok-1"}]), &ks);
    let p = open_pack(&pack, &m5, 0, &ks, &list, NOW).unwrap();
    assert!(p.entries.is_empty());
    assert_eq!(p.dropped[0].reason, ReasonCode::Revoked);
    // And with no list at all it renders, so the list is what removed it.
    assert_eq!(
        open_pack(&pack, &m5, 0, &ks, &RevocationList::default(), NOW)
            .unwrap()
            .entries
            .len(),
        1
    );
}
