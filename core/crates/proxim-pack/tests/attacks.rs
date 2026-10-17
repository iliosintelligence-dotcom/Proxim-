//! M2 exit checks (PLAN M2; Addendum 01 11.3): tamper, rollback, wrong kid, revoked kid, splice,
//! dropped entry, swapped signatures, expiry, replay, live_at, reserved events, determinism.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_pack::jws::{TYP_KEYSET, TYP_MANIFEST};
use proxim_pack::keyset::accept_keyset;
use proxim_pack::manifest::accept_manifest;
use proxim_pack::revocation::RevocationList;
use proxim_pack::testkit::TestKey;
use proxim_pack::{Guard, ReasonCode, open_pack};
use serde_json::json;

fn reasons(p: &proxim_pack::OpenedPack) -> Vec<(u32, ReasonCode)> {
    p.dropped.iter().map(|d| (d.index, d.reason)).collect()
}

#[test]
fn a_well_formed_pack_opens_with_every_entry_and_creative() {
    let revs = three_revs();
    let rows: Vec<_> = revs.iter().map(all_shipped).collect();
    let p = open_rows(&rows).unwrap();
    assert!(p.dropped.is_empty(), "{:?}", p.dropped);
    let ids: Vec<_> = p.entries.iter().map(|e| e.signpost.id.as_str()).collect();
    assert_eq!(ids, ["cafe-1", "fuel-1", "deli-1"]);
    assert_eq!(p.entries[2].creatives.len(), 3);
    assert_eq!(p.entries[2].creatives[1].body, b"Deli: bread");
    assert_eq!((p.cell, p.seq, p.tier), (CELL, 7, 0));
}

#[test]
fn flipping_any_byte_of_the_pack_is_rejected_by_the_manifest_hash() {
    let rows: Vec<_> = three_revs().iter().map(all_shipped).collect();
    let ks = default_keyset();
    let pack = build(&header(7), &rows);
    let m = manifest_for(7, &pack, &ks);
    let none = RevocationList::default();
    for i in 0..pack.len() {
        let mut bad = pack.clone();
        bad[i] ^= 0x01;
        assert_eq!(
            open_pack(&bad, &m, 0, &ks, &none, NOW),
            Err(ReasonCode::SignatureInvalid),
            "byte {i}"
        );
    }
    // Truncation and extension too.
    assert!(open_pack(&pack[..pack.len() - 1], &m, 0, &ks, &none, NOW).is_err());
    let mut longer = pack.clone();
    longer.push(0);
    assert!(open_pack(&longer, &m, 0, &ks, &none, NOW).is_err());
}

/// The manifest key alone (a compromised CDN signer) cannot change signed content: flip every
/// byte of an entry's signed bytes, re-hash in a fresh manifest, and the entry is dropped.
#[test]
fn any_signed_field_flip_drops_the_entry_even_under_a_valid_manifest() {
    let revs = three_revs();
    let target = &revs[0];
    let shipped = all_shipped(target);
    let mut checked = 0;
    for i in 0..shipped.entry.len() {
        let mut row = shipped.clone();
        row.entry[i] ^= 0x01;
        let rows = vec![row, all_shipped(&revs[1])];
        match open_rows(&rows) {
            // Structural damage (a length byte) can make the stream unreadable: whole pack refused.
            Err(code) => assert_eq!(code, ReasonCode::SignatureInvalid),
            Ok(p) => {
                assert_eq!(reasons(&p), [(0, ReasonCode::SignatureInvalid)], "byte {i}");
                assert_eq!(p.entries.len(), 1);
                assert_eq!(p.entries[0].signpost.id.as_str(), "fuel-1");
            }
        }
        checked += 1;
    }
    assert_eq!(checked, shipped.entry.len());
}

#[test]
fn swapped_signatures_and_spliced_rows_are_dropped() {
    let revs = three_revs();
    let mut a = all_shipped(&revs[0]);
    let mut b = all_shipped(&revs[1]);
    std::mem::swap(&mut a.sig, &mut b.sig);
    let p = open_rows(&[a, b]).unwrap();
    assert_eq!(
        reasons(&p),
        [
            (0, ReasonCode::SignatureInvalid),
            (1, ReasonCode::SignatureInvalid)
        ]
    );
    // Splice: rev 2's core under rev 1's signature (same id, both genuinely approved once).
    let v2 = sign_rev(
        &signpost("cafe-1", 2, &[]),
        &["Coffee ahead", "Fresh pastries"],
        &approve(),
    );
    let mut spliced = all_shipped(&v2);
    spliced.sig = revs[0].sig;
    let p = open_rows(&[spliced]).unwrap();
    assert_eq!(reasons(&p), [(0, ReasonCode::SignatureInvalid)]);
    // Splice a creative: rev 1 of cafe-1 with deli-1's wording.
    let mut swapped = revs[0].clone();
    swapped.bodies[1] = b"Deli: bread".to_vec();
    let p = open_rows(&[all_shipped(&swapped)]).unwrap();
    assert_eq!(reasons(&p), [(0, ReasonCode::SignatureInvalid)]);
}

#[test]
fn removing_an_entry_or_a_signature_row_breaks_the_manifest_hash() {
    let rows: Vec<_> = three_revs().iter().map(all_shipped).collect();
    let ks = default_keyset();
    let full = build(&header(7), &rows);
    let m = manifest_for(7, &full, &ks);
    let shorter = build(&header(7), &rows[..2]);
    assert_eq!(
        open_pack(&shorter, &m, 0, &ks, &RevocationList::default(), NOW),
        Err(ReasonCode::SignatureInvalid)
    );
}

#[test]
fn wrong_unknown_and_revoked_kids() {
    let revs = three_revs();
    // Signed by the manifest key: right signature, wrong purpose.
    let wrong_purpose = sign_rev(&signpost("x-1", 1, &[]), &["Hi"], &manifest_key());
    // Signed by a key the key set does not list.
    let unknown = sign_rev(
        &signpost("x-2", 1, &[]),
        &["Hi"],
        &TestKey::from_label("stranger"),
    );
    let p = open_rows(&[
        all_shipped(&wrong_purpose),
        all_shipped(&unknown),
        all_shipped(&revs[0]),
    ])
    .unwrap();
    assert_eq!(
        reasons(&p),
        [
            (0, ReasonCode::KidRevoked),
            (1, ReasonCode::SignatureInvalid)
        ]
    );
    assert_eq!(p.entries.len(), 1);

    // The key set revokes the approve kid: every entry it signed is dropped.
    let (a, m) = (approve(), manifest_key());
    let a2 = TestKey::from_label("test-approve-2");
    let doc = keyset_doc(2, &[(&a2, "approve"), (&m, "manifest")], &[&a.kid]);
    let ks = accept_keyset(keyset_token(&doc).as_bytes(), &roots(), None, NOW).unwrap();
    let pack = build(&header(7), &[all_shipped(&revs[0])]);
    let man = manifest_for(7, &pack, &ks);
    let p = open_pack(&pack, &man, 0, &ks, &RevocationList::default(), NOW).unwrap();
    assert_eq!(reasons(&p), [(0, ReasonCode::KidRevoked)]);
}

#[test]
fn rollback_replay_and_expiry() {
    let ks = default_keyset();
    let rows: Vec<_> = three_revs().iter().map(all_shipped).collect();
    let p8 = build(&header(8), &rows);
    let m8 = manifest_for(8, &p8, &ks);
    // Lower seq after 8 was accepted.
    let p7 = build(&header(7), &rows);
    let doc7 = manifest_doc(7, &p7, NOW - 60_000, NOW + DAY);
    assert_eq!(
        accept_manifest(
            manifest_token(&doc7).as_bytes(),
            &ks,
            CELL,
            Some(&m8.guard),
            NOW
        ),
        Err(ReasonCode::SeqRollback)
    );
    // Same seq, different content.
    let other = build(&header(8), &rows[..1]);
    let doc8b = manifest_doc(8, &other, NOW - 60_000, NOW + DAY);
    assert_eq!(
        accept_manifest(
            manifest_token(&doc8b).as_bytes(),
            &ks,
            CELL,
            Some(&m8.guard),
            NOW
        ),
        Err(ReasonCode::SeqRollback)
    );
    // The same manifest fetched again is fine.
    let doc8 = manifest_doc(8, &p8, NOW - 60_000, NOW + DAY);
    assert!(
        accept_manifest(
            manifest_token(&doc8).as_bytes(),
            &ks,
            CELL,
            Some(&m8.guard),
            NOW
        )
        .is_ok()
    );
    // Replayed pack: the seq-7 pack under the seq-8 manifest (hash and header both differ).
    assert_eq!(
        open_pack(&p7, &m8, 0, &ks, &RevocationList::default(), NOW),
        Err(ReasonCode::SignatureInvalid)
    );
    // Expired, or from the future.
    for (issued, expires) in [(NOW - 2 * DAY, NOW - DAY), (NOW, NOW), (NOW + 1, NOW + DAY)] {
        let d = manifest_doc(8, &p8, issued, expires);
        assert_eq!(
            accept_manifest(manifest_token(&d).as_bytes(), &ks, CELL, None, NOW),
            Err(ReasonCode::PackExpired)
        );
    }
    // Another cell's manifest.
    assert_eq!(
        accept_manifest(manifest_token(&doc8).as_bytes(), &ks, CELL + 1, None, NOW),
        Err(ReasonCode::SignatureInvalid)
    );
}

#[test]
fn key_set_rules() {
    let (a, m) = (approve(), manifest_key());
    let doc = keyset_doc(3, &[(&a, "approve"), (&m, "manifest")], &[]);
    let ks = accept_keyset(keyset_token(&doc).as_bytes(), &roots(), None, NOW).unwrap();
    // Older version.
    let d2 = keyset_doc(2, &[(&a, "approve"), (&m, "manifest")], &[]);
    assert_eq!(
        accept_keyset(keyset_token(&d2).as_bytes(), &roots(), Some(&ks.guard), NOW).unwrap_err(),
        ReasonCode::SeqRollback
    );
    // Signed by a non-root key, or presented as another type.
    let t = a.jws(TYP_KEYSET, &canon(&doc));
    assert_eq!(
        accept_keyset(t.as_bytes(), &roots(), None, NOW).unwrap_err(),
        ReasonCode::SignatureInvalid
    );
    let t = root().jws(TYP_MANIFEST, &canon(&doc));
    assert!(accept_keyset(t.as_bytes(), &roots(), None, NOW).is_err());
    // A key that reuses the root kid, a duplicate kid, or a listed kid also revoked.
    let r = root();
    for bad in [
        keyset_doc(4, &[(&r, "approve")], &[]),
        keyset_doc(4, &[(&a, "approve"), (&a, "manifest")], &[]),
        keyset_doc(4, &[(&a, "approve")], &[&a.kid]),
    ] {
        assert_eq!(
            accept_keyset(keyset_token(&bad).as_bytes(), &roots(), None, NOW).unwrap_err(),
            ReasonCode::SignatureInvalid
        );
    }
    // Not canonical JSON (whitespace) is refused even though the signature is good.
    let loose = br#"{"format": "proxim-keyset/1"}"#;
    let t = root().jws(TYP_KEYSET, loose);
    assert!(accept_keyset(t.as_bytes(), &roots(), None, NOW).is_err());
}

#[test]
fn revocation_scopes() {
    let revs = three_revs();
    let rows: Vec<_> = revs.iter().map(all_shipped).collect();
    let ks = default_keyset();
    let pack = build(&header(7), &rows);
    let m = manifest_for(7, &pack, &ks);
    let deli_vid = |i: usize| {
        let d = proxim_pack::encode::creative_digest(&revs[2].bodies[i]);
        proxim_pack::crypto::hex(&d[..4])
    };
    let open = |entries| {
        let list = revocations(entries, &ks);
        open_pack(&pack, &m, 0, &ks, &list, NOW).unwrap()
    };
    let p = open(json!([{"scope": "signpost_rev", "id": "cafe-1", "rev": 1}]));
    assert_eq!(reasons(&p), [(0, ReasonCode::Revoked)]);
    let p = open(json!([{"scope": "signpost_rev", "id": "cafe-1", "rev": 2}]));
    assert!(p.dropped.is_empty(), "another revision is untouched");
    let p = open(json!([{"scope": "signpost", "id": "fuel-1"}]));
    assert_eq!(reasons(&p), [(1, ReasonCode::Revoked)]);
    let p = open(json!([{"scope": "sender_org", "sender": "sender-1"}]));
    assert_eq!(
        reasons(&p),
        [(0, ReasonCode::Revoked), (2, ReasonCode::Revoked)]
    );
    let p = open(json!([{"scope": "kid", "kid": "test-approve-1"}]));
    assert_eq!(p.entries.len(), 0);
    // Variant scope withdraws one wording; revoking the base wording drops the entry.
    let p = open(json!([{"scope": "variant", "id": "deli-1", "rev": 2, "vid": deli_vid(1)}]));
    let deli = p
        .entries
        .iter()
        .find(|e| e.signpost.id.as_str() == "deli-1")
        .unwrap();
    let kept: Vec<u8> = deli.creatives.iter().map(|c| c.index).collect();
    assert_eq!(kept, [0, 2]);
    let p = open(json!([{"scope": "variant", "id": "deli-1", "rev": 2, "vid": deli_vid(0)}]));
    assert_eq!(reasons(&p), [(2, ReasonCode::Revoked)]);
}

#[test]
fn ship_mask_withholds_bodies_but_the_signature_still_covers_them() {
    let revs = three_revs();
    let p = open_rows(&[row(&revs[2], 0b101)]).unwrap();
    let kept: Vec<u8> = p.entries[0].creatives.iter().map(|c| c.index).collect();
    assert_eq!(kept, [0, 2]);
    // Creative 0 must ship; bits beyond n are invalid.
    for mask in [0b110, 0b1111] {
        let p = open_rows(&[row(&revs[2], mask)]).unwrap();
        assert_eq!(reasons(&p), [(0, ReasonCode::SignatureInvalid)], "{mask:b}");
    }
}

#[test]
fn duplicate_vids_and_duplicate_ids() {
    let a = approve();
    let twin = sign_rev(&signpost("twin-1", 1, &[]), &["Same", "Same"], &a);
    let p = open_rows(&[all_shipped(&twin)]).unwrap();
    assert_eq!(reasons(&p), [(0, ReasonCode::SignatureInvalid)]);
    let revs = three_revs();
    let again = sign_rev(&signpost("cafe-1", 9, &[]), &["Other"], &a);
    assert_eq!(
        open_rows(&[all_shipped(&revs[0]), all_shipped(&again)]),
        Err(ReasonCode::SignatureInvalid)
    );
}

#[test]
fn pack_kind_is_signed_and_live_at_is_enforced() {
    let a = approve();
    let no_kind = {
        let mut v = signpost("k-1", 1, &[]);
        v.as_object_mut().unwrap().remove("pack_kind");
        sign_rev(&v, &["Hi"], &a)
    };
    let owner = sign_rev(
        &signpost("k-2", 1, &[("pack_kind", json!("owner_bundle"))]),
        &["Hi"],
        &a,
    );
    let later = sign_rev(
        &signpost("k-3", 1, &[("live_at_ms", json!(NOW + 1))]),
        &["Hi"],
        &a,
    );
    let now = sign_rev(
        &signpost("k-4", 1, &[("live_at_ms", json!(NOW))]),
        &["Hi"],
        &a,
    );
    let p = open_rows(&[
        all_shipped(&no_kind),
        all_shipped(&owner),
        all_shipped(&later),
        all_shipped(&now),
    ])
    .unwrap();
    assert_eq!(
        reasons(&p),
        [
            (0, ReasonCode::SignatureInvalid),
            (1, ReasonCode::SignatureInvalid),
            (2, ReasonCode::NotYetLive)
        ]
    );
    assert_eq!(p.entries[0].live_at_ms, Some(NOW));
}

#[test]
fn a_core_that_is_not_canonical_is_dropped_even_if_signed() {
    let a = approve();
    let mut rev = sign_rev(&signpost("c-1", 1, &[]), &["Hi"], &a);
    let mut loose = b" ".to_vec();
    loose.extend_from_slice(&rev.core);
    rev.core = loose;
    let digests = [proxim_pack::encode::creative_digest(b"Hi")];
    rev.sig = a.sign(&proxim_pack::encode::signed_message(&rev.core, &digests).unwrap());
    let p = open_rows(&[all_shipped(&rev)]).unwrap();
    assert_eq!(reasons(&p), [(0, ReasonCode::SignatureInvalid)]);
}

#[test]
fn a_reserved_event_in_a_valid_pack_still_fails_closed_in_decide() {
    let a = approve();
    let cross = sign_rev(
        &signpost("cross-1", 1, &[("event", json!("cross"))]),
        &["Hi"],
        &a,
    );
    let p = open_rows(&[all_shipped(&cross)]).unwrap();
    assert!(p.dropped.is_empty(), "the pack layer does not judge events");
    let signposts = [p.entries[0].signpost.clone()];
    let fix: proxim_types::Fix = serde_json::from_value(json!({
        "t_ms": NOW, "lat_e7": 450000000, "lon_e7": 70000000, "acc_mm": 5000
    }))
    .unwrap();
    let policy: proxim_types::Policy = serde_json::from_value(json!({"explain": true})).unwrap();
    let consent: proxim_types::Consent =
        serde_json::from_value(json!({"location_matching": "granted"})).unwrap();
    let clock = proxim_types::Clock {
        now_ms: NOW,
        tz_offset_min: 0,
    };
    let (out, _) = proxim_decide::decide(
        &proxim_types::State::empty(),
        &fix,
        &signposts,
        &policy,
        &consent,
        &clock,
        0,
    )
    .unwrap();
    let text = serde_json::to_string(&out).unwrap();
    assert!(text.contains("UNSUPPORTED_EVENT"), "{text}");
    assert!(!text.contains("\"fire\""), "{text}");
}

#[test]
fn builds_are_deterministic() {
    let rows: Vec<_> = three_revs().iter().map(all_shipped).collect();
    assert_eq!(build(&header(7), &rows), build(&header(7), &rows));
    assert_eq!(
        three_revs()[0].sig,
        three_revs()[0].sig,
        "RFC 6979 signatures"
    );
}

#[test]
fn guard_type_is_shared() {
    let g = Guard {
        seq: 1,
        sha256: [0; 32],
    };
    assert_eq!(
        Guard::check(Some(&g), 0, &[0; 32]),
        Err(ReasonCode::SeqRollback)
    );
}

/// Pack to decide (engine spec 7.3): a verified entry carries its wordings, a withheld or
/// variant-revoked wording is unavailable, and the fire names the wording to show.
#[test]
fn verified_wordings_flow_into_decide() {
    let a = approve();
    let rev = sign_rev(&signpost("w-1", 1, &[]), &["One", "Two", "Three"], &a);
    let p = open_rows(&[row(&rev, 0b011)]).unwrap();
    let sp = &p.entries[0].signpost;
    let w = sp.wordings.as_ref().unwrap();
    assert_eq!(w.digests.len(), 3);
    assert_eq!(w.avail_mask, 0b011, "the withheld wording is not available");
    let fix: proxim_types::Fix = serde_json::from_value(json!({
        "t_ms": NOW, "lat_e7": 450000000, "lon_e7": 70000000, "acc_mm": 5000
    }))
    .unwrap();
    let policy: proxim_types::Policy = serde_json::from_value(json!({"explain": true})).unwrap();
    let consent: proxim_types::Consent =
        serde_json::from_value(json!({"location_matching": "granted"})).unwrap();
    let mut state = proxim_types::State::empty();
    let mut wordings = vec![];
    // Walk in from 400 m south (about 3,600 e-7 degrees of latitude) at 2 m/s, one fix a second.
    for k in 0..300i64 {
        let t = NOW + k * 1000;
        let lat = 449_996_400 + i32::try_from((k * 18).min(3_600)).unwrap();
        let fix = proxim_types::Fix {
            t_ms: t,
            lat_e7: lat,
            ..fix
        };
        let clock = proxim_types::Clock {
            now_ms: t,
            tz_offset_min: 0,
        };
        let (out, next) = proxim_decide::decide(
            &state,
            &fix,
            std::slice::from_ref(sp),
            &policy,
            &consent,
            &clock,
            0,
        )
        .unwrap();
        state = next;
        wordings.extend(
            out.decisions
                .iter()
                .filter(|d| d.outcome == proxim_types::Outcome::Fire)
                .map(|d| d.wording),
        );
    }
    assert_eq!(wordings.len(), 1, "one enter fires");
    assert!(matches!(wordings[0], Some(0 | 1)), "{wordings:?}");
    assert_eq!(state.rotation.rows.len(), 1);
}

#[test]
fn a_core_that_carries_its_own_wordings_is_refused() {
    let a = approve();
    let forged = sign_rev(
        &signpost(
            "w-2",
            1,
            &[(
                "wordings",
                json!({"digests": ["00".repeat(32)], "avail_mask": 1}),
            )],
        ),
        &["Hi"],
        &a,
    );
    let p = open_rows(&[all_shipped(&forged)]).unwrap();
    assert_eq!(reasons(&p), [(0, ReasonCode::SignatureInvalid)]);
}
