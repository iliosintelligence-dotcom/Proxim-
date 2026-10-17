//! Invariant I10: unsigned, expired, rolled-back or revoked content is never rendered. Covers the
//! keyset, manifest, pack and entry layers (pack-spec 3 to 5). Signpost-scope revocation is I20 (M3).
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_pack::{
    PackError, SeqMark, TrustRoots, verify_entry, verify_keyset, verify_manifest, verify_pack,
};

#[test]
fn happy_path_end_to_end() {
    let w = world();
    let ks = w.keyset();
    let m = w.manifest();
    let p = verify_pack(&m, 0, &w.pack, &ks, NOW).unwrap();
    assert_eq!(p.entries.len(), 1);
    assert!(p.dropped.is_empty());
    assert_eq!((p.cell, p.seq, p.tier), (CELL, 7, 0));
}

#[test]
fn unsigned_expired_rolled_back_or_revoked_is_never_rendered() {
    let w = world();
    let ks = w.keyset();
    let sp = signpost("a-1", 1, NOW + 60 * DAY);
    let c = core(&sp, NOW + 14 * DAY);

    // Unsigned: a key the keyset does not list, and a wrong key under a listed kid.
    let stranger = online("stranger", 9);
    assert_eq!(
        verify_entry(
            &entry(stranger.kid, &stranger.sk, &c, &bodies(), 0b111),
            &ks,
            NOW
        ),
        Err(PackError::UnknownKid)
    );
    assert_eq!(
        verify_entry(
            &entry(w.k1.kid, &stranger.sk, &c, &bodies(), 0b111),
            &ks,
            NOW
        ),
        Err(PackError::BadSignature)
    );

    // Expired: entry, manifest and keyset.
    assert_eq!(
        verify_entry(&w.entry, &ks, NOW + 15 * DAY),
        Err(PackError::Expired)
    );
    assert_eq!(
        verify_entry(&w.entry, &ks, NOW + 31 * DAY),
        Err(PackError::KeyNotValid)
    );
    assert_eq!(
        verify_manifest(&ks, &w.manifest_bytes, CELL, NOW + DAY, None),
        Err(PackError::Expired)
    );
    assert_eq!(
        verify_keyset(&w.roots, &w.keyset_bytes, NOW + 8 * DAY, None).map(|k| k.seq),
        Err(PackError::Expired)
    );

    // Rolled back: an older manifest after a newer one, and an older keyset.
    let newer = doc(
        2,
        w.k1.kid,
        &manifest_payload(CELL, 8, NOW - HOUR, NOW + HOUR, &[&w.pack]),
        &w.k1.sk,
    );
    let mark = verify_manifest(&ks, &newer, CELL, NOW, None)
        .unwrap()
        .mark();
    assert_eq!(
        verify_manifest(&ks, &w.manifest_bytes, CELL, NOW, Some(mark)),
        Err(PackError::Rollback)
    );
    let ks2_bytes = doc(
        1,
        "root-1",
        &keyset_payload(2, &[&w.k1], &[], NOW - DAY, NOW + DAY),
        &w.root,
    );
    let ks2 = verify_keyset(&w.roots, &ks2_bytes, NOW, Some(ks.mark())).unwrap();
    assert_eq!(
        verify_keyset(&w.roots, &w.keyset_bytes, NOW, Some(ks2.mark())).map(|k| k.seq),
        Err(PackError::Rollback)
    );

    // Revoked: a keyset that revokes the online key stops its manifests and entries.
    let k2 = online("online-2", 3);
    let ks3_bytes = doc(
        1,
        "root-1",
        &keyset_payload(3, &[&k2], &["online-1"], NOW - DAY, NOW + DAY),
        &w.root,
    );
    let ks3 = verify_keyset(&w.roots, &ks3_bytes, NOW, Some(ks2.mark())).unwrap();
    assert!(ks3.is_revoked("online-1"));
    assert_eq!(
        verify_entry(&w.entry, &ks3, NOW),
        Err(PackError::KidRevoked)
    );
    assert_eq!(
        verify_manifest(&ks3, &w.manifest_bytes, CELL, NOW, None),
        Err(PackError::KidRevoked)
    );
    // ... and inside a pack the entry is dropped, never returned.
    let m = doc(
        2,
        k2.kid,
        &manifest_payload(
            CELL,
            9,
            NOW - HOUR,
            NOW + HOUR,
            &[&pack(CELL, 9, 0, std::slice::from_ref(&w.entry))],
        ),
        &k2.sk,
    );
    let m = verify_manifest(&ks3, &m, CELL, NOW, None).unwrap();
    let p = verify_pack(
        &m,
        0,
        &pack(CELL, 9, 0, std::slice::from_ref(&w.entry)),
        &ks3,
        NOW,
    )
    .unwrap();
    assert!(p.entries.is_empty());
    assert_eq!(p.dropped[0].error, PackError::KidRevoked);
}

#[test]
fn same_seq_with_different_bytes_is_a_conflict() {
    let w = world();
    let ks = w.keyset();
    let mark = w.manifest().mark();
    assert!(verify_manifest(&ks, &w.manifest_bytes, CELL, NOW, Some(mark)).is_ok());
    let other = doc(
        2,
        w.k1.kid,
        &manifest_payload(CELL, 7, NOW - HOUR, NOW + 2 * HOUR, &[&w.pack]),
        &w.k1.sk,
    );
    assert_eq!(
        verify_manifest(&ks, &other, CELL, NOW, Some(mark)),
        Err(PackError::SeqConflict)
    );
    assert_eq!(
        SeqMark::admit(
            Some(mark),
            SeqMark {
                seq: 8,
                digest: [0; 32]
            }
        ),
        Ok(())
    );
}

#[test]
fn keyset_must_be_root_signed_and_documents_cannot_swap_kind() {
    let w = world();
    // An online key cannot sign a keyset.
    let by_online = doc(
        1,
        w.k1.kid,
        &keyset_payload(5, &[&w.k1], &[], NOW - DAY, NOW + DAY),
        &w.k1.sk,
    );
    assert_eq!(
        verify_keyset(&w.roots, &by_online, NOW, None).map(|k| k.seq),
        Err(PackError::UnknownKid)
    );
    // A root kid with another key's signature.
    let forged = doc(
        1,
        "root-1",
        &keyset_payload(5, &[&w.k1], &[], NOW - DAY, NOW + DAY),
        &w.k1.sk,
    );
    assert_eq!(
        verify_keyset(&w.roots, &forged, NOW, None).map(|k| k.seq),
        Err(PackError::BadSignature)
    );
    // A manifest offered as a keyset, and the reverse.
    assert_eq!(
        verify_keyset(&w.roots, &w.manifest_bytes, NOW, None).map(|k| k.seq),
        Err(PackError::Malformed)
    );
    assert_eq!(
        verify_manifest(&w.keyset(), &w.keyset_bytes, CELL, NOW, None),
        Err(PackError::Malformed)
    );
    // Relabelling the kind byte breaks the signature (the kind is signed).
    let mut relabel = w.manifest_bytes.clone();
    relabel[5] = 1;
    assert!(verify_keyset(&w.roots, &relabel, NOW, None).is_err());
}

#[test]
fn online_key_window_is_enforced() {
    let w = world();
    let late = OnlineKey {
        not_before_ms: NOW + HOUR,
        ..online("online-9", 4)
    };
    let ks_bytes = doc(
        1,
        "root-1",
        &keyset_payload(2, &[&late], &[], NOW - DAY, NOW + DAY),
        &w.root,
    );
    let ks = verify_keyset(&w.roots, &ks_bytes, NOW, None).unwrap();
    let c = core(&signpost("a-1", 1, NOW + 60 * DAY), NOW + DAY);
    let e = entry(late.kid, &late.sk, &c, &bodies(), 1 | 2 | 4);
    assert_eq!(verify_entry(&e, &ks, NOW), Err(PackError::KeyNotValid));
    assert!(verify_entry(&e, &ks, NOW + HOUR).is_ok());
}

#[test]
fn pack_must_match_its_manifest() {
    let w = world();
    let ks = w.keyset();
    let m = w.manifest();
    // Different bytes for the same tier.
    let mut swapped = w.pack.clone();
    let last = swapped.len() - 1;
    swapped[last] ^= 1;
    assert_eq!(
        verify_pack(&m, 0, &swapped, &ks, NOW),
        Err(PackError::Mismatch)
    );
    // Tier the manifest does not list.
    assert_eq!(
        verify_pack(&m, 1, &w.pack, &ks, NOW),
        Err(PackError::Mismatch)
    );
    // A pack from another cell, even when the manifest vouches for its bytes.
    let other = pack(CELL + 1, 7, 0, std::slice::from_ref(&w.entry));
    let m2 = doc(
        2,
        w.k1.kid,
        &manifest_payload(CELL, 7, NOW - HOUR, NOW + HOUR, &[&other]),
        &w.k1.sk,
    );
    let m2 = verify_manifest(&ks, &m2, CELL, NOW, None).unwrap();
    assert_eq!(
        verify_pack(&m2, 0, &other, &ks, NOW),
        Err(PackError::Mismatch)
    );
    // The manifest for another cell is refused for this one.
    assert_eq!(
        verify_manifest(&ks, &w.manifest_bytes, CELL + 1, NOW, None),
        Err(PackError::Mismatch)
    );
}

#[test]
fn a_bad_entry_drops_alone_and_duplicates_drop_together() {
    let w = world();
    let ks = w.keyset();
    let good_b = entry(
        w.k1.kid,
        &w.k1.sk,
        &core(&signpost("b-1", 1, NOW + 60 * DAY), NOW + DAY),
        &bodies(),
        0b111,
    );
    let mut bad = w.entry.clone();
    bad[40] ^= 1;
    let dup_a = entry(
        w.k1.kid,
        &w.k1.sk,
        &core(&signpost("c-1", 3, NOW + 60 * DAY), NOW + DAY),
        &bodies(),
        0b111,
    );
    let dup_b = entry(
        w.k1.kid,
        &w.k1.sk,
        &core(&signpost("c-1", 3, NOW + 60 * DAY), NOW + 2 * DAY),
        &bodies(),
        0b111,
    );
    let p = pack(CELL, 7, 0, &[bad, good_b, dup_a, dup_b]);
    let m = doc(
        2,
        w.k1.kid,
        &manifest_payload(CELL, 7, NOW - HOUR, NOW + HOUR, &[&p]),
        &w.k1.sk,
    );
    let m = verify_manifest(&ks, &m, CELL, NOW, None).unwrap();
    let v = verify_pack(&m, 0, &p, &ks, NOW).unwrap();
    let ids: Vec<&str> = v.entries.iter().map(|e| e.signpost.id.as_str()).collect();
    assert_eq!(ids, vec!["b-1"]);
    let dropped: Vec<(u16, PackError)> = v.dropped.iter().map(|d| (d.index, d.error)).collect();
    assert_eq!(dropped[0].0, 0);
    assert_eq!(
        &dropped[1..],
        &[(2, PackError::Duplicate), (3, PackError::Duplicate)]
    );
}

#[test]
fn manifest_ttl_and_tier_shape_are_checked() {
    let w = world();
    let ks = w.keyset();
    let long = doc(
        2,
        w.k1.kid,
        &manifest_payload(CELL, 7, NOW - HOUR, NOW + 72 * HOUR, &[&w.pack]),
        &w.k1.sk,
    );
    assert_eq!(
        verify_manifest(&ks, &long, CELL, NOW, None),
        Err(PackError::Malformed)
    );
    let gap = manifest_payload(CELL, 7, NOW - HOUR, NOW + HOUR, &[&w.pack])
        .replace("\"tier\":0", "\"tier\":1");
    let gap = doc(2, w.k1.kid, &gap, &w.k1.sk);
    assert_eq!(
        verify_manifest(&ks, &gap, CELL, NOW, None),
        Err(PackError::Malformed)
    );
}

#[test]
fn roots_reject_duplicates_and_bad_keys() {
    let k = sec1(&key(1));
    assert!(TrustRoots::new(&[("r", &k), ("r", &k)]).is_err());
    assert!(TrustRoots::new(&[("r", &k[..64])]).is_err());
    assert!(TrustRoots::new(&[("R", &k)]).is_err());
}
