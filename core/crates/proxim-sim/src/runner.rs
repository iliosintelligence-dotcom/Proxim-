//! Drives the engine through a scenario, one fix at a time, threading state exactly as an SDK
//! does (spec 7), and aggregates the oracle's verdicts into a canonical JSON report that is
//! byte-identical on rerun and across operating systems (integers only, sorted keys, no time).

use crate::geom::{SimResult, put};
use crate::noise;
use crate::oracle::{RunRec, RunVerdict, StepRec, judge};
use crate::scenario::{Scenario, default_consent};
use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::canon::{to_canonical_json, value_to_canonical};
use proxim_types::{Clock, Consent, DetState, EngineError, Fix, Output, Policy, Signpost, State};
use serde_json::{Value, json};

/// The decision function under test. [`RealEngine`] is `proxim_decide::decide`; tests wrap it to
/// prove that each kind of assertion can fail.
pub trait Engine {
    /// # Errors
    /// As `proxim_decide::decide`.
    fn decide(
        &mut self,
        state: &State,
        fix: &Fix,
        signposts: &[Signpost],
        policy: &Policy,
        consent: &Consent,
        clock: &Clock,
    ) -> Result<(Output, State), EngineError>;
}

/// The real engine (seed argument 0: carried and unused in M1).
#[derive(Clone, Copy, Debug, Default)]
pub struct RealEngine;

impl Engine for RealEngine {
    fn decide(
        &mut self,
        state: &State,
        fix: &Fix,
        signposts: &[Signpost],
        policy: &Policy,
        consent: &Consent,
        clock: &Clock,
    ) -> Result<(Output, State), EngineError> {
        proxim_decide::decide(state, fix, signposts, policy, consent, clock, 0)
    }
}

/// Everything learned from one scenario.
#[derive(Clone, Debug)]
pub struct ScenarioResult {
    pub id: String,
    pub title: String,
    pub source: String,
    pub sha256: String,
    pub runs: u64,
    pub fixes: u64,
    pub verdicts: Vec<RunVerdict>,
    /// SHA-256 over every canonical engine output of every run, in order.
    pub outputs_sha256: String,
    pub pass: bool,
    pub summary: Value,
}

fn consent_at(sc: &Scenario, t_rel_ms: i64) -> Consent {
    let mut c = sc.file.consent.clone().unwrap_or_else(default_consent);
    for ch in &sc.file.consent_changes {
        if t_rel_ms >= ch.at_s * 1000 {
            c = ch.consent.clone();
        }
    }
    c
}

fn clock_at(sc: &Scenario, fix_t: i64, t_rel_ms: i64) -> Clock {
    let ck = &sc.file.clock;
    let mut shift = ck.lag_ms;
    let mut tz = ck.tz_offset_min;
    for e in &ck.events {
        if t_rel_ms >= e.at_s * 1000 {
            shift = shift.saturating_add(e.shift_ms);
            if let Some(z) = e.tz_offset_min {
                tz = z;
            }
        }
    }
    Clock {
        now_ms: fix_t.saturating_add(shift),
        tz_offset_min: tz,
    }
}

/// Run one seed.
///
/// # Errors
/// Noise generation failed (a sample outside the frame) or output serialization failed.
pub fn run_seed(
    sc: &Scenario,
    samples: &[crate::trace::Sample],
    seed: u64,
    engine: &mut dyn Engine,
    outputs: &mut Vec<u8>,
) -> SimResult<RunRec> {
    let fixes = noise::apply(
        &sc.local,
        samples,
        &sc.noise,
        &sc.dropouts,
        sc.start_ms,
        sc.file.period_ms,
        seed,
    )?;
    let mut state = State::empty();
    let mut steps = Vec::with_capacity(fixes.len());
    let mut restarts: Vec<i64> = sc.file.restarts_s.iter().map(|s| s * 1000).collect();
    restarts.sort_unstable();
    let mut next_restart = 0_usize;
    for nf in &fixes {
        let t_rel = nf.fix.t_ms - sc.start_ms;
        while restarts.get(next_restart).is_some_and(|r| t_rel >= *r) {
            // A new process: only `caps` survives (spec 3.3).
            state = State {
                v: 1,
                caps: state.caps.clone(),
                mem: proxim_types::Mem::default(),
            };
            next_restart += 1;
        }
        let consent = consent_at(sc, t_rel);
        let clock = clock_at(sc, nf.fix.t_ms, t_rel);
        let res = engine.decide(&state, &nf.fix, &sc.signposts, &sc.policy, &consent, &clock);
        let rec = match res {
            Ok((out, next)) => {
                outputs.extend_from_slice(to_canonical_json(&out)?.as_bytes());
                outputs.push(b'\n');
                let inside = sc
                    .signposts
                    .iter()
                    .map(|s| {
                        next.mem
                            .det
                            .iter()
                            .any(|d| d.id == s.id && matches!(d.st, DetState::In { .. }))
                    })
                    .collect();
                let mem_empty = next.mem == proxim_types::Mem::default();
                state = next;
                StepRec {
                    fix: nf.fix,
                    sample: nf.sample,
                    output: Some(out),
                    mem_empty,
                    inside,
                }
            }
            Err(e) => {
                outputs.extend_from_slice(alloc::format!("error {e:?}\n").as_bytes());
                StepRec {
                    fix: nf.fix,
                    sample: nf.sample,
                    output: None,
                    mem_empty: state.mem == proxim_types::Mem::default(),
                    inside: sc.signposts.iter().map(|_| false).collect(),
                }
            }
        };
        steps.push(rec);
    }
    Ok(RunRec {
        seed,
        steps,
        duration_ms: sc.truth.duration_ms(),
    })
}

fn min_max(v: &mut Option<[i64; 2]>, x: i64) {
    *v = Some(match *v {
        None => [x, x],
        Some([a, b]) => [a.min(x), b.max(x)],
    });
}

/// Run a scenario over `runs` seeds (1..=runs) with `engine` and judge it.
///
/// # Errors
/// As [`run_seed`] and the oracle.
#[allow(clippy::too_many_lines)]
pub fn run_scenario(
    sc: &Scenario,
    runs: u64,
    engine: &mut dyn Engine,
) -> SimResult<ScenarioResult> {
    let samples = sc.truth.samples(sc.file.period_ms)?;
    let mut verdicts = Vec::new();
    let mut run_digests = Vec::new();
    let mut fixes = 0_u64;
    for seed in 1..=runs {
        let mut outputs = Vec::new();
        let run = run_seed(sc, &samples, seed, engine, &mut outputs)?;
        fixes += u64::try_from(run.steps.len()).unwrap_or(0);
        run_digests.extend_from_slice(&proxim_decide::sha256::sha256(&outputs));
        verdicts.push(judge(sc, &samples, &run)?);
    }
    let outputs_sha256 = proxim_decide::sha256::sha256_hex(&run_digests);

    // Aggregate.
    let mut pass = true;
    let mut sp_out = Vec::new();
    for (k, e) in sc.file.expect.signposts.iter().enumerate() {
        let need = e.min_runs_ok_permille.unwrap_or(1000);
        let mut runs_ok = 0_u64;
        let mut fires_total = 0_u64;
        let mut runs_fired = 0_u64;
        let (mut d_rem, mut xt, mut depth, mut lat, mut t_hat, mut fire_t) =
            (None, None, None, None, None, None);
        let mut max_tr = 0_u32;
        let mut max_naive = 0_u32;
        let mut sum_naive = 0_u64;
        let mut failures = Vec::new();
        for v in &verdicts {
            let Some(r) = v.signposts.get(k) else {
                continue;
            };
            if r.ok {
                runs_ok += 1;
            } else if failures.len() < 3 {
                for f in r.failures.iter().take(2) {
                    failures.push(Value::String(alloc::format!("seed {}: {f}", v.seed)));
                }
            }
            if !r.fires.is_empty() {
                runs_fired += 1;
            }
            fires_total += u64::try_from(r.fires.len()).unwrap_or(0);
            for f in &r.fires {
                if let Some(x) = f.d_rem_mm {
                    min_max(&mut d_rem, x);
                }
                if let Some(x) = f.xt_mm {
                    min_max(&mut xt, x.abs());
                }
                if let Some(x) = f.depth_mm {
                    min_max(&mut depth, x);
                }
                if let Some(x) = f.latency_ms {
                    min_max(&mut lat, x);
                }
                if let Some(x) = f.t_hat_ms {
                    min_max(&mut t_hat, x);
                }
                min_max(&mut fire_t, f.t_rel_ms);
            }
            max_tr = max_tr.max(r.transitions);
            max_naive = max_naive.max(r.naive_transitions);
            sum_naive += u64::from(r.naive_transitions);
        }
        let ok = u128::from(runs_ok) * 1000 >= u128::from(need) * u128::from(runs);
        pass &= ok;
        let mut o = json!({
            "signpost": e.signpost,
            "pass": ok,
            "runs_ok": runs_ok,
            "required_permille": need,
            "fires_total": fires_total,
            "runs_fired": runs_fired,
            "max_transitions": max_tr,
            "naive_transitions_max": max_naive,
            "naive_transitions_sum": sum_naive,
            "expect_fires": e.fires,
            "failures": failures,
        });
        for (key, val) in [
            ("true_d_rem_mm", d_rem),
            ("true_xt_abs_mm", xt),
            ("true_depth_mm", depth),
            ("latency_ms", lat),
            ("t_hat_ms", t_hat),
            ("fire_t_rel_ms", fire_t),
        ] {
            if let Some(v) = val {
                put(&mut o, key, json!(v));
            }
        }
        sp_out.push(o);
    }
    let mut global_fail_runs = 0_u64;
    let mut global_failures = Vec::new();
    for v in &verdicts {
        if !v.global_failures.is_empty() {
            global_fail_runs += 1;
            if global_failures.len() < 3 {
                for f in v.global_failures.iter().take(2) {
                    global_failures.push(Value::String(alloc::format!("seed {}: {f}", v.seed)));
                }
            }
        }
    }
    pass &= global_fail_runs == 0;
    let summary = json!({
        "id": sc.file.id,
        "title": sc.file.title,
        "pass": pass,
        "runs": runs,
        "fixes": fixes,
        "noise": sc.file.noise.profile,
        "scenario_sha256": sc.sha256,
        "outputs_sha256": outputs_sha256,
        "signposts": sp_out,
        "global_failed_runs": global_fail_runs,
        "global_failures": global_failures,
    });
    Ok(ScenarioResult {
        id: sc.file.id.clone(),
        title: sc.file.title.clone(),
        source: sc.file.source.clone(),
        sha256: sc.sha256.clone(),
        runs,
        fixes,
        verdicts,
        outputs_sha256,
        pass,
        summary,
    })
}

/// The canonical JSON report for a set of results (with a trailing newline).
///
/// # Errors
/// Serialization failure (cannot happen for the integer-only tree built here).
pub fn report_json(results: &[ScenarioResult], seeds_override: Option<u64>) -> SimResult<String> {
    let passed = results.iter().filter(|r| r.pass).count();
    let mut v = json!({
        "format": "proxim-sim-report/1",
        "engine_spec": proxim_types::ENGINE_SPEC,
        "synthetic": true,
        "scenarios": results.iter().map(|r| r.summary.clone()).collect::<Vec<_>>(),
        "passed": passed,
        "failed": results.len() - passed,
    });
    if let Some(s) = seeds_override {
        put(&mut v, "seeds_override", json!(s));
    }
    let mut s = value_to_canonical(&v)?;
    s.push('\n');
    Ok(s)
}
