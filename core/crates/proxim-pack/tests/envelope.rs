//! Envelope v2 entries (pack-spec 5, A02 4.3, invariant I17).
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_pack::{PackError, verify_entry};

#[test]
fn valid_entry_verifies_with_every_creative() {
    let w = world();
    let e = verify_entry(&w.entry, &w.keyset(), NOW).unwrap();
    assert_eq!(e.kid, "online-1");
    assert_eq!(e.signpost.id.as_str(), "a-1");
    assert_eq!(e.declared, 3);
    assert_eq!(e.creatives.len(), 3);
    for (i, c) in e.creatives.iter().enumerate() {
        assert_eq!(usize::from(c.index), i);
        assert_eq!(c.digest, creative_digest(bodies()[i].as_bytes()));
        assert_eq!(c.vid, c.digest[..4]);
    }
    assert_eq!(e.creatives[0].creative.headline, "Fresh coffee ahead");
}

/// I17: the signature covers the core (every field the kernel reads) and the full digest of every
/// declared creative, so changing any bit of an entry drops it.
#[test]
fn flip_of_any_signed_field_is_rejected() {
    let w = world();
    let ks = w.keyset();
    for i in 0..w.entry.len() {
        for bit in 0..8 {
            let mut bad = w.entry.clone();
            bad[i] ^= 1 << bit;
            assert!(
                verify_entry(&bad, &ks, NOW).is_err(),
                "flip at byte {i} bit {bit} was accepted"
            );
        }
    }
}

#[test]
fn every_signpost_field_change_is_rejected_without_a_new_signature() {
    let w = world();
    let ks = w.keyset();
    let sp = signpost("a-1", 1, NOW + 60 * DAY);
    let good = core(&sp, NOW + 14 * DAY);
    let signed = entry(w.k1.kid, &w.k1.sk, &good, &bodies(), 0b111);
    for (from, to) in [
        ("\"radius_m\":200", "\"radius_m\":900"),
        ("\"category\":\"cafe\"", "\"category\":\"bank\""),
        ("\"sensitivity\":0", "\"sensitivity\":2"),
        ("\"rank_milli\":1000", "\"rank_milli\":9000"),
        ("\"rev\":1", "\"rev\":2"),
    ] {
        let forged_core = good.replace(from, to);
        assert_ne!(forged_core, good);
        let mut bad = signed.clone();
        let at = bad
            .windows(good.len())
            .position(|w| w == good.as_bytes())
            .unwrap();
        assert_eq!(forged_core.len(), good.len(), "{from}");
        bad[at..at + good.len()].copy_from_slice(forged_core.as_bytes());
        assert_eq!(
            verify_entry(&bad, &ks, NOW),
            Err(PackError::BadSignature),
            "{from}"
        );
    }
}

#[test]
fn high_s_signature_is_rejected() {
    let w = world();
    let mut bad = w.entry.clone();
    let n = bad.len();
    let sig: [u8; 64] = bad[n - 64..].try_into().unwrap();
    bad[n - 64..].copy_from_slice(&high_s(&sig));
    assert_eq!(
        verify_entry(&bad, &w.keyset(), NOW),
        Err(PackError::BadSignature)
    );
}

#[test]
fn withdrawn_creative_still_verifies_and_is_not_rendered() {
    let w = world();
    let sp = signpost("a-1", 1, NOW + 60 * DAY);
    let e = entry(
        w.k1.kid,
        &w.k1.sk,
        &core(&sp, NOW + 14 * DAY),
        &bodies(),
        0b101,
    );
    let v = verify_entry(&e, &w.keyset(), NOW).unwrap();
    assert_eq!(v.declared, 3);
    let shown: Vec<u8> = v.creatives.iter().map(|c| c.index).collect();
    assert_eq!(shown, vec![0, 2]);
}

#[test]
fn ship_mask_rules() {
    let w = world();
    let ks = w.keyset();
    let c = core(&signpost("a-1", 1, NOW + 60 * DAY), NOW + 14 * DAY);
    // Creative 0 always ships.
    let e = entry(w.k1.kid, &w.k1.sk, &c, &bodies(), 0b110);
    assert_eq!(verify_entry(&e, &ks, NOW), Err(PackError::Malformed));
    // No mask bit at or above n.
    let mut e = entry(w.k1.kid, &w.k1.sk, &c, &bodies()[..1], 0b1);
    let mask_at = 1 + 1 + w.k1.kid.len() + 2 + c.len() + 1;
    e[mask_at] = 0b11;
    assert_eq!(verify_entry(&e, &ks, NOW), Err(PackError::Malformed));
}

#[test]
fn duplicate_vid_drops_the_entry() {
    let w = world();
    let b = bodies();
    let d0 = creative_digest(b[0].as_bytes());
    // Creative 1 is withdrawn and its signed digest repeats creative 0's: same vid.
    let digests = [d0, d0, creative_digest(b[2].as_bytes())];
    let c = core(&signpost("a-1", 1, NOW + 60 * DAY), NOW + 14 * DAY);
    let e = entry_with(w.k1.kid, &w.k1.sk, &c, &b, &digests, 0b101);
    assert_eq!(
        verify_entry(&e, &w.keyset(), NOW),
        Err(PackError::Duplicate)
    );
}

#[test]
fn body_with_matching_vid_but_different_digest_is_rejected() {
    // A body is authentic only if its FULL digest is the signed one; a vid match is not enough.
    let w = world();
    let b = bodies();
    let c = core(&signpost("a-1", 1, NOW + 60 * DAY), NOW + 14 * DAY);
    let mut forged_digest = creative_digest(b[1].as_bytes());
    forged_digest[31] ^= 1; // same 4-byte vid, different digest
    let digests = [
        creative_digest(b[0].as_bytes()),
        forged_digest,
        creative_digest(b[2].as_bytes()),
    ];
    let e = entry_with(w.k1.kid, &w.k1.sk, &c, &b, &digests, 0b111);
    assert_eq!(
        verify_entry(&e, &w.keyset(), NOW),
        Err(PackError::BadSignature)
    );
}

#[test]
fn non_canonical_or_out_of_range_content_is_rejected_even_when_signed() {
    let w = world();
    let ks = w.keyset();
    let sp = signpost("a-1", 1, NOW + 60 * DAY);
    let spaced = core(&sp, NOW + 14 * DAY).replacen(':', ": ", 1);
    let e = entry(w.k1.kid, &w.k1.sk, &spaced, &bodies(), 0b111);
    assert_eq!(verify_entry(&e, &ks, NOW), Err(PackError::Malformed));

    let long = vec![creative(&"x".repeat(41), "", "Ahead")];
    let e = entry(w.k1.kid, &w.k1.sk, &core(&sp, NOW + 14 * DAY), &long, 1);
    assert_eq!(verify_entry(&e, &ks, NOW), Err(PackError::Malformed));

    let wordy = vec![creative(
        "Hi",
        "",
        "one two three four five six seven eight",
    )];
    let e = entry(w.k1.kid, &w.k1.sk, &core(&sp, NOW + 14 * DAY), &wordy, 1);
    assert_eq!(verify_entry(&e, &ks, NOW), Err(PackError::Malformed));

    let extra = vec![String::from(
        "{\"body\":\"\",\"glanceable\":\"Go\",\"headline\":\"Hi\",\"x\":1}",
    )];
    let e = entry(w.k1.kid, &w.k1.sk, &core(&sp, NOW + 14 * DAY), &extra, 1);
    assert_eq!(verify_entry(&e, &ks, NOW), Err(PackError::Malformed));
}

#[test]
fn entry_expiry_rules() {
    let w = world();
    let ks = w.keyset();
    // Expiry past the signpost's own validity is a signer bug: rejected.
    let sp = signpost("a-1", 1, NOW + DAY);
    let e = entry(
        w.k1.kid,
        &w.k1.sk,
        &core(&sp, NOW + 2 * DAY),
        &bodies(),
        0b111,
    );
    assert_eq!(verify_entry(&e, &ks, NOW), Err(PackError::Malformed));
    // Expired at now.
    let sp = signpost("a-1", 1, NOW + DAY);
    let e = entry(w.k1.kid, &w.k1.sk, &core(&sp, NOW - 1), &bodies(), 0b111);
    assert_eq!(verify_entry(&e, &ks, NOW), Err(PackError::Expired));
    assert!(verify_entry(&e, &ks, NOW - 1).is_ok());
}

#[test]
fn trailing_or_truncated_bytes_are_rejected() {
    let w = world();
    let ks = w.keyset();
    let mut longer = w.entry.clone();
    longer.push(0);
    assert_eq!(verify_entry(&longer, &ks, NOW), Err(PackError::Malformed));
    for cut in 0..w.entry.len() {
        assert!(
            verify_entry(&w.entry[..cut], &ks, NOW).is_err(),
            "cut {cut}"
        );
    }
}
