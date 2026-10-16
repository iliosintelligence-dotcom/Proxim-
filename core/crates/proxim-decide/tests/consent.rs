//! I4 (consent fails closed) and I12 (health class needs a consent flag and a receipt), F01 and F15.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_decide::decide;
use proxim_types::canon::to_canonical_json;
use proxim_types::strict::from_json;
use proxim_types::{Class, Consent, ConsentValue, Mem, Outcome, ReasonCode, State};

/// A state with caps and a live detector, so that wiping and keeping are both observable.
fn warm_state() -> (Sim, Vec<proxim_types::Fix>) {
    let mut sim = Sim::new(vec![circle("c-1", 200), route("r-1", 2500, 30)]);
    let mut fixes = drive_north(T0, 1_400_000, 15_000, 20, 0);
    fixes.extend(visit(T0 + 60_000, 0, 0, 3, 0));
    for f in &fixes {
        sim.step(f);
    }
    assert!(!sim.state.mem.det.is_empty() && !sim.state.mem.trace.is_empty());
    (sim, fixes)
}

#[test]
fn consent_missing_or_unknown_denies_everything() {
    // Every consent that is not exactly "granted" for location_matching must deny, whatever else
    // is granted. Parsed from JSON so the wire forms are exercised, not only the Rust enum.
    let wire = [
        "{}",
        r#"{"location_matching":"unknown"}"#,
        r#"{"location_matching":"denied"}"#,
        r#"{"location_matching":"Granted"}"#,
        r#"{"location_matching":"GRANTED"}"#,
        r#"{"location_matching":" granted"}"#,
        r#"{"location_matching":"granted "}"#,
        r#"{"location_matching":"yes"}"#,
        r#"{"location_matching":""}"#,
        r#"{"location_matching":true}"#,
        r#"{"location_matching":1}"#,
        r#"{"location_matching":["granted"]}"#,
        r#"{"location_matching":{"granted":true}}"#,
        r#"{"location_matching":"granted\u0000"}"#,
        r#"{"nearby_health_events":"granted","aggregate_reporting":"granted","local_notifications":"granted","health_receipt":"r"}"#,
    ];
    for text in wire {
        let consent: Consent = from_json(text.as_bytes()).unwrap();
        assert_ne!(consent.location_matching, ConsentValue::Granted, "{text}");
        let (mut sim, _) = warm_state();
        let caps_before = to_canonical_json(&sim.state.caps).unwrap();
        sim.consent = consent;
        // A fix that would otherwise have been processed (inside the circle).
        let out = sim.step(&still(T0 + 70_000, 0, 0));
        assert!(!out.fix.ok, "{text}");
        assert_eq!(out.fix.reason, ReasonCode::ConsentDenied, "{text}");
        assert!(out.decisions.is_empty() && out.notes.is_none(), "{text}");
        assert_eq!(
            out.path.speed_mmps, None,
            "nothing derived from the fix is reported"
        );
        // Nothing processed or retained: mem wiped, caps untouched byte for byte.
        assert_eq!(sim.state.mem, Mem::default(), "{text}");
        assert_eq!(
            to_canonical_json(&sim.state.caps).unwrap(),
            caps_before,
            "{text}"
        );
    }
}

#[test]
fn only_the_exact_string_granted_grants() {
    let c: Consent = from_json(br#"{"location_matching":"granted"}"#).unwrap();
    assert_eq!(c.location_matching, ConsentValue::Granted);
    let (mut sim, _) = warm_state();
    sim.consent = c;
    assert!(sim.step(&still(T0 + 70_000, 0, 0)).fix.ok);
}

#[test]
fn denied_then_granted_starts_from_nothing() {
    // Withdrawal wipes the memory: after re-granting, no candidate count survives.
    let mut sim = Sim::new(vec![circle("c-1", 200)]);
    let a = visit(T0, 0, 0, 2, 0); // two inside fixes: candidate n = 2
    sim.run(&a);
    assert!(!sim.state.mem.det.is_empty());
    sim.consent = Consent::default();
    sim.step(&still(T0 + 9_000, 0, 0));
    sim.consent = granted();
    // Without the wipe the next fix would complete the 3 fixes + 3 s rule; with it, n restarts.
    let o = sim.step(&still(T0 + 13_000, 0, 0));
    assert!(fires(&o).is_empty());
    assert_eq!(sim.state.mem.det.len(), 1);
}

#[test]
fn consent_is_checked_before_the_fix_is_even_validated() {
    let (mut sim, _) = warm_state();
    sim.consent = Consent::default();
    let mut bad = still(T0 + 70_000, 0, 0);
    bad.lat_e7 = 2_000_000_000; // invalid
    let o = sim.step(&bad);
    assert_eq!(
        o.fix.reason,
        ReasonCode::ConsentDenied,
        "consent is F01, validity F03"
    );
}

#[test]
fn child_directed_is_checked_after_consent() {
    let (mut sim, _) = warm_state();
    sim.consent = Consent::default();
    sim.policy.child_directed = Some(true);
    assert_eq!(
        sim.step(&still(T0 + 70_000, 0, 0)).fix.reason,
        ReasonCode::ConsentDenied
    );
}

fn health_circle() -> proxim_types::Signpost {
    let mut sp = circle("h-1", 200);
    sp.class = Class::Health;
    sp
}

fn health_fires(consent: Consent) -> (bool, Vec<ReasonCode>) {
    let mut sim = Sim::new(vec![health_circle()]);
    sim.consent = consent;
    let outs = sim.run(&visit(T0, 0, 0, 5, 0));
    let fired = outs.iter().any(|o| !fires(o).is_empty());
    let reasons = outs
        .iter()
        .flat_map(|o| o.decisions.iter())
        .filter(|d| d.outcome == Outcome::Drop)
        .map(|d| d.reason)
        .collect();
    (fired, reasons)
}

#[test]
fn health_class_requires_sdk_issued_receipt() {
    // Flag and receipt: fires.
    assert!(health_fires(granted_health()).0);
    // Missing receipt, empty receipt, flag denied/unknown/absent, location-only consent: never.
    let cases = [
        Consent {
            health_receipt: None,
            ..granted_health()
        },
        Consent {
            health_receipt: Some(String::new()),
            ..granted_health()
        },
        Consent {
            nearby_health_events: ConsentValue::Denied,
            ..granted_health()
        },
        Consent {
            nearby_health_events: ConsentValue::Unknown,
            ..granted_health()
        },
        granted(),
        Consent {
            health_receipt: Some("r".into()),
            ..granted()
        },
    ];
    for (i, c) in cases.into_iter().enumerate() {
        let (fired, reasons) = health_fires(c);
        assert!(!fired, "case {i}");
        assert!(
            reasons.contains(&ReasonCode::HealthConsentMissing),
            "case {i}: {reasons:?}"
        );
    }
}

#[test]
fn general_class_needs_no_health_consent() {
    let mut sim = Sim::new(vec![circle("c-1", 200)]);
    let outs = sim.run(&visit(T0, 0, 0, 5, 0));
    assert!(outs.iter().any(|o| !fires(o).is_empty()));
}

#[test]
fn the_state_type_cannot_smuggle_a_grant() {
    // Defence in depth: a State parsed from JSON has no consent field at all.
    let text = r#"{"v":1,"caps":{"hwm_ms":0,"signposts":[]},"mem":{"jump_rejects":0,"trace":[],"det":[]},"consent":"granted"}"#;
    assert!(from_json::<State>(text.as_bytes()).is_err());
    // And decide never reads consent from anywhere but its argument.
    let (o, _) = decide(
        &State::empty(),
        &still(T0, 0, 0),
        &[circle("c-1", 200)],
        &explain(),
        &Consent::default(),
        &clock(T0),
        0,
    )
    .unwrap();
    assert_eq!(o.fix.reason, ReasonCode::ConsentDenied);
}
