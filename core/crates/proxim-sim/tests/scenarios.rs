//! Scenario suite (`core/testdata/scenarios`, docs/simulator.md).
//!
//! - every scenario passes with its CI-sized seed count (or `PROXIM_SIM_SEEDS=N` for all);
//! - the canonical report equals the checked-in `core/testdata/sim/scenario-report.json`
//!   byte for byte (rerun and cross-OS identity; regenerate with `PROXIM_BLESS=1`);
//! - the stationary-outside statistic (0 false enters, <= 4 transitions per 10 min);
//! - every kind of assertion can fail: deliberately broken engines turn the oracle red.
//!
//! No threads are spawned and no clock is read, so this also runs on WebAssembly and mobile.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::print_stderr
)]

use proxim_sim::runner::{Engine, RealEngine, ScenarioResult, report_json, run_scenario};
use proxim_sim::scenario::{Scenario, list_dir};
use proxim_sim::stationary::StationaryStats;
use proxim_types::{
    Clock, Consent, Decision, DetEntry, DetState, EngineError, EventKind, Fix, Id, Outcome, Output,
    Policy, ReasonCode, Signpost, State,
};
use std::path::PathBuf;

fn core_dir() -> PathBuf {
    std::env::var_os("PROXIM_CORE_DIR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

fn seeds_override() -> Option<u64> {
    std::env::var("PROXIM_SIM_SEEDS").ok().map(|s| {
        s.parse()
            .unwrap_or_else(|_| panic!("PROXIM_SIM_SEEDS={s} is not a number"))
    })
}

fn load(id: &str) -> Scenario {
    Scenario::load(&core_dir().join(format!("testdata/scenarios/{id}.json"))).unwrap()
}

fn failures(r: &ScenarioResult) -> String {
    serde_json::to_string(&r.summary).unwrap()
}

#[test]
fn every_scenario_passes_and_the_report_is_byte_identical() {
    let seeds = seeds_override();
    let files = list_dir(&core_dir().join("testdata/scenarios")).unwrap();
    assert!(
        files.len() >= 30,
        "expected the full scenario set, found {}",
        files.len()
    );
    let mut results = Vec::new();
    for f in &files {
        let sc = Scenario::load(f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let r = run_scenario(&sc, sc.runs(seeds), &mut RealEngine).unwrap();
        results.push(r);
    }
    let failed: Vec<String> = results
        .iter()
        .filter(|r| !r.pass)
        .map(|r| format!("{}: {}", r.id, failures(r)))
        .collect();
    assert!(
        failed.is_empty(),
        "failing scenarios:\n{}",
        failed.join("\n")
    );

    // Stationary 25 m outside (ADDENDUM-01 M1 verify, [MODEL] targets).
    let st = results
        .iter()
        .find(|r| r.id == "stationary-outside")
        .unwrap();
    let s = StationaryStats::from_result(st, 0);
    eprintln!(
        "[stationary-outside] seeds {} false-enter runs {} max transitions/10 min {} naive edge crossings max {} mean x1000 {}",
        s.seeds,
        s.false_enter_runs,
        s.max_transitions,
        s.max_naive_transitions,
        s.naive_transitions_sum * 1000 / s.seeds
    );
    assert_eq!(s.false_enter_runs, 0);
    assert!(s.max_transitions <= 4);
    // The flap scenario shows the detector suppressing what a naive edge test would report.
    let fl = results.iter().find(|r| r.id == "flap").unwrap();
    let f = StationaryStats::from_result(fl, 0);
    eprintln!(
        "[flap] seeds {} false-enter runs {} max transitions {} naive max {} naive mean x1000 {}",
        f.seeds,
        f.false_enter_runs,
        f.max_transitions,
        f.max_naive_transitions,
        f.naive_transitions_sum * 1000 / f.seeds
    );
    assert!(f.max_naive_transitions > 4 * (f.max_transitions + 1));

    let text = report_json(&results, seeds).unwrap();
    // Rerun identity within this process for one noisy scenario.
    let again = run_scenario(&load("flap"), 3, &mut RealEngine).unwrap();
    let first = run_scenario(&load("flap"), 3, &mut RealEngine).unwrap();
    assert_eq!(again.summary, first.summary);
    if seeds.is_none() {
        let path = core_dir().join("testdata/sim/scenario-report.json");
        if std::env::var_os("PROXIM_BLESS").is_some() {
            std::fs::write(&path, &text).unwrap();
        }
        let old = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            old == text,
            "{} differs from this run (nondeterminism, or a behaviour change: rerun with PROXIM_BLESS=1 and review the diff)",
            path.display()
        );
    } else if let Some(out) = std::env::var_os("PROXIM_SIM_REPORT") {
        std::fs::write(out, &text).unwrap();
    }
}

// ---------------------------------------------------------------------------------------------
// Broken engines: each assertion kind must be able to fail.
// ---------------------------------------------------------------------------------------------

fn fire(sp: &Signpost, t_ms: i64) -> Decision {
    Decision {
        signpost: sp.id.clone(),
        rev: sp.rev,
        outcome: Outcome::Fire,
        reason: ReasonCode::Ok,
        event: sp.event,
        t_ms,
        path_tier: 0,
        path_conf_milli: 500,
        copy: proxim_types::Copy::NearYou,
        d_rem_mm: None,
        t_hat_ms: Some(60_000),
    }
}

/// Wraps the real engine and rewrites its output.
struct Broken<F: FnMut(&Fix, &[Signpost], &mut Output, &mut State)>(F);

impl<F: FnMut(&Fix, &[Signpost], &mut Output, &mut State)> Engine for Broken<F> {
    fn decide(
        &mut self,
        state: &State,
        fix: &Fix,
        signposts: &[Signpost],
        policy: &Policy,
        consent: &Consent,
        clock: &Clock,
    ) -> Result<(Output, State), EngineError> {
        let (mut o, mut s) =
            proxim_decide::decide(state, fix, signposts, policy, consent, clock, 0)?;
        (self.0)(fix, signposts, &mut o, &mut s);
        Ok((o, s))
    }
}

fn run_broken(
    id: &str,
    f: impl FnMut(&Fix, &[Signpost], &mut Output, &mut State),
) -> ScenarioResult {
    let sc = load(id);
    run_scenario(&sc, sc.runs(Some(2)), &mut Broken(f)).unwrap()
}

fn assert_red(r: &ScenarioResult, needle: &str) {
    let text = failures(r);
    assert!(!r.pass, "{} stayed green under a broken engine", r.id);
    assert!(
        text.contains(needle),
        "{}: expected {needle:?} in {text}",
        r.id
    );
}

#[test]
fn the_real_engine_passes_the_scenarios_used_below() {
    for id in [
        "expressway-inbound",
        "opposite-carriageway",
        "s01-drive-in-enter",
        "health-standoff",
        "stationary-outside",
        "signed-cross-fixture",
    ] {
        let sc = load(id);
        let r = run_scenario(&sc, sc.runs(Some(2)), &mut RealEngine).unwrap();
        assert!(r.pass, "{id}: {}", failures(&r));
    }
}

#[test]
fn a_silent_engine_fails_every_must_fire_scenario() {
    let strip = |_: &Fix, _: &[Signpost], o: &mut Output, _: &mut State| {
        o.decisions.retain(|d| d.outcome != Outcome::Fire);
    };
    assert_red(&run_broken("expressway-inbound", strip), "fired 0 time(s)");
    assert_red(&run_broken("s01-drive-in-enter", strip), "fired 0 time(s)");
}

#[test]
fn an_engine_that_fires_everywhere_fails_every_negative_scenario() {
    let all = |f: &Fix, sps: &[Signpost], o: &mut Output, _: &mut State| {
        for sp in sps {
            if !o
                .decisions
                .iter()
                .any(|d| d.signpost == sp.id && d.outcome == Outcome::Fire)
            {
                o.decisions.push(fire(sp, f.t_ms));
            }
        }
    };
    for id in [
        "opposite-carriageway",
        "stationary-outside",
        "signed-cross-fixture",
    ] {
        assert_red(&run_broken(id, all), "expected 0..=0");
    }
    // the standoff check is independent of the fire count
    assert_red(&run_broken("health-standoff", all), "from a no-fire point");
}

#[test]
fn a_double_fire_fails_the_once_only_and_gap_assertions() {
    let mut pending: Option<Decision> = None;
    let dbl = move |f: &Fix, _: &[Signpost], o: &mut Output, _: &mut State| {
        let real = o
            .decisions
            .iter()
            .find(|d| d.outcome == Outcome::Fire)
            .cloned();
        if let Some(mut d) = pending.take() {
            d.t_ms = f.t_ms;
            o.decisions.push(d);
        }
        pending = real;
    };
    assert_red(&run_broken("expressway-inbound", dbl), "fired 2 time(s)");
}

#[test]
fn a_fire_in_the_wrong_place_fails_the_window_assertions() {
    // Moves the real fire to the first fix: count stays 1, position is wrong.
    let mut first = true;
    let early = move |f: &Fix, sps: &[Signpost], o: &mut Output, _: &mut State| {
        o.decisions.retain(|d| d.outcome != Outcome::Fire);
        if first {
            first = false;
            o.decisions.push(fire(&sps[0], f.t_ms));
        }
    };
    assert_red(&run_broken("expressway-inbound", early), "true d_rem");
}

#[test]
fn a_late_or_false_enter_fails_the_latency_and_depth_assertions() {
    // Fire the enter 20 s late.
    let mut seen: Option<i64> = None;
    let late = move |f: &Fix, sps: &[Signpost], o: &mut Output, _: &mut State| {
        if o.decisions.iter().any(|d| d.outcome == Outcome::Fire) {
            seen = Some(f.t_ms);
        }
        o.decisions.retain(|d| d.outcome != Outcome::Fire);
        if seen.is_some_and(|t| f.t_ms == t + 20_000) {
            o.decisions.push(fire(&sps[0], f.t_ms));
        }
    };
    assert_red(
        &run_broken("s01-drive-in-enter", late),
        "after the true entry",
    );
    // Fire on the first fix, 600 m outside the zone.
    let mut first = true;
    let early = move |f: &Fix, sps: &[Signpost], o: &mut Output, _: &mut State| {
        o.decisions.retain(|d| d.outcome != Outcome::Fire);
        if first {
            first = false;
            o.decisions.push(fire(&sps[0], f.t_ms));
        }
    };
    assert_red(&run_broken("s01-drive-in-enter", early), "false enter");
}

#[test]
fn a_breadcrumb_inside_the_standoff_fails_the_memory_assertion() {
    let leak = |f: &Fix, _: &[Signpost], o: &mut Output, s: &mut State| {
        if o.fix.reason == ReasonCode::Suppressed {
            s.mem.last_t_ms = Some(f.t_ms);
        }
    };
    assert_red(&run_broken("health-standoff", leak), "mem not empty");
}

#[test]
fn a_flapping_detector_fails_the_transition_assertion() {
    let mut k = 0_u32;
    let flap = move |f: &Fix, sps: &[Signpost], _: &mut Output, s: &mut State| {
        k += 1;
        s.mem.det.retain(|d| d.id != sps[0].id);
        if k.is_multiple_of(2) {
            s.mem.det.push(DetEntry {
                id: sps[0].id.clone(),
                rev: sps[0].rev,
                st: DetState::In {
                    xn: 0,
                    x_first_t: None,
                    last_t: f.t_ms,
                },
            });
        }
    };
    assert_red(
        &run_broken("stationary-outside", flap),
        "detector transitions",
    );
}

#[test]
fn a_missing_reason_fails_the_reason_assertion() {
    let hide = |_: &Fix, _: &[Signpost], o: &mut Output, _: &mut State| {
        o.decisions
            .retain(|d| d.reason != ReasonCode::UnsupportedEvent);
    };
    assert_red(
        &run_broken("signed-cross-fixture", hide),
        "UnsupportedEvent",
    );
}

#[test]
fn the_cross_fixture_is_pinned_and_reserved() {
    let sc = load("signed-cross-fixture");
    assert_eq!(sc.signposts[0].event, EventKind::Cross);
    assert_eq!(sc.signposts[0].id, Id::new("gate-cross").unwrap());
    // a pin that does not match the fixture bytes is refused at load time
    let dir = core_dir().join("testdata/scenarios");
    let text = std::fs::read_to_string(dir.join("signed-cross-fixture.json")).unwrap();
    let pin = &sc.file.signposts[0].fixture.as_ref().unwrap().sha256;
    let flipped: String = pin
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i == 0 {
                if c == '0' { '1' } else { '0' }
            } else {
                c
            }
        })
        .collect();
    let bad = text.replace(pin.as_str(), &flipped);
    assert_ne!(bad, text);
    let err = Scenario::from_text(&bad, &dir).unwrap_err();
    assert!(err.0.contains("does not match the pinned"), "{err}");
}

#[test]
fn scenario_typos_are_rejected() {
    let dir = core_dir().join("testdata/scenarios");
    let src = std::fs::read_to_string(dir.join("expressway-inbound.json")).unwrap();
    assert!(Scenario::from_text(&src, &dir).is_ok());
    let bad = src.replace("\"true_d_rem_m\"", "\"true_drem_m\"");
    assert_ne!(bad, src);
    let err = Scenario::from_text(&bad, &dir).unwrap_err();
    assert!(err.0.contains("unknown field"), "{err}");
}
