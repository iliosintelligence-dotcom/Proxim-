//! The omniscient oracle: it knows the true, noise-free path and judges each run of the engine
//! against the scenario's expectation. Engine outputs never contain a position (I2), so every
//! spatial judgement ("fired 1.2 km before the anchor", "was truly inside when it entered",
//! "never fired within 1 km of the hospital") comes from the truth sample of the fix that fired.

use crate::geom::SimResult;
use crate::scenario::{Expect, Geometry, Scenario, SignpostExpect, route_truth};
use crate::trace::Sample;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use proxim_geo::{isqrt, signed_depth_polygon_mm};
use proxim_types::{Enu, Fix, Outcome, Output, ReasonCode};

/// What the runner recorded for one fix.
#[derive(Clone, Debug)]
pub struct StepRec {
    pub fix: Fix,
    /// Index of the fix's truth sample.
    pub sample: usize,
    /// `None` when the engine returned an error (state unchanged, spec 2.6).
    pub output: Option<Output>,
    pub mem_empty: bool,
    /// Per signpost (scenario order): the detector declares the phone inside (`in` state).
    pub inside: Vec<bool>,
}

/// One run (one seed).
#[derive(Clone, Debug)]
pub struct RunRec {
    pub seed: u64,
    pub steps: Vec<StepRec>,
    pub duration_ms: i64,
}

/// One fire as the oracle sees it.
#[derive(Clone, Copy, Debug)]
pub struct FireTruth {
    pub t_rel_ms: i64,
    pub t_hat_ms: Option<i64>,
    pub d_rem_mm: Option<i64>,
    pub xt_mm: Option<i64>,
    pub depth_mm: Option<i64>,
    pub latency_ms: Option<i64>,
}

/// Per-signpost result of one run.
#[derive(Clone, Debug, Default)]
pub struct SignpostRun {
    pub ok: bool,
    pub fires: Vec<FireTruth>,
    pub transitions: u32,
    pub naive_transitions: u32,
    pub failures: Vec<String>,
}

/// Judged run.
#[derive(Clone, Debug, Default)]
pub struct RunVerdict {
    pub seed: u64,
    pub signposts: Vec<SignpostRun>,
    pub global_failures: Vec<String>,
    pub engine_errors: u32,
}

fn dist(a: (i64, i64), b: (i64, i64)) -> i64 {
    let de = i128::from(a.0) - i128::from(b.0);
    let dn = i128::from(a.1) - i128::from(b.1);
    i64::try_from(isqrt(u128::try_from(de * de + dn * dn).unwrap_or(u128::MAX))).unwrap_or(i64::MAX)
}

/// True signed depth (inside positive, mm) of a point in a zone geometry.
///
/// # Errors
/// Degenerate polygon.
pub fn zone_depth(g: &Geometry, p: (i64, i64)) -> SimResult<Option<i64>> {
    Ok(match g {
        Geometry::Circle { center, radius_mm } => Some(radius_mm - dist(*center, p)),
        Geometry::Polygon { vertices } => {
            let q = Enu {
                e_mm: i32::try_from(p.0).unwrap_or(i32::MAX),
                n_mm: i32::try_from(p.1).unwrap_or(i32::MAX),
            };
            Some(signed_depth_polygon_mm(vertices, q)?)
        }
        _ => None,
    })
}

fn pt(s: &Sample) -> (i64, i64) {
    (s.e_mm, s.n_mm)
}

fn in_range(v: i64, r: [i64; 2]) -> bool {
    let [lo, hi] = r;
    v >= lo && v <= hi
}

/// Judge one run.
///
/// # Errors
/// Geometry errors in the oracle's own truth computations.
#[allow(clippy::too_many_lines)]
pub fn judge(sc: &Scenario, samples: &[Sample], run: &RunRec) -> SimResult<RunVerdict> {
    let ex: &Expect = &sc.file.expect;
    let mut v = RunVerdict {
        seed: run.seed,
        ..RunVerdict::default()
    };
    v.engine_errors = u32::try_from(run.steps.iter().filter(|s| s.output.is_none()).count())
        .unwrap_or(u32::MAX);
    if v.engine_errors > 0 {
        v.global_failures.push(alloc::format!(
            "engine returned an error on {} fix(es)",
            v.engine_errors
        ));
    }

    // Global checks.
    let mut fix_reasons = BTreeSet::new();
    let mut total_fires = 0_u32;
    for st in &run.steps {
        let Some(out) = &st.output else { continue };
        fix_reasons.insert(out.fix.reason);
        if ex.mem_empty_after.contains(&out.fix.reason) && !st.mem_empty {
            v.global_failures.push(alloc::format!(
                "t+{} s: mem not empty after {:?}",
                (st.fix.t_ms - sc.start_ms) / 1000,
                out.fix.reason
            ));
        }
        for d in out.decisions.iter().filter(|d| d.outcome == Outcome::Fire) {
            total_fires += 1;
            let Some(s) = samples.get(st.sample) else { continue };
            for z in &ex.no_fire_within {
                let c = (z.center_m[0] * 1000, z.center_m[1] * 1000);
                let dd = dist(c, pt(s));
                if dd <= z.radius_m * 1000 {
                    v.global_failures.push(alloc::format!(
                        "t+{} s: {} fired {} m from a no-fire point (limit {} m)",
                        s.t_rel_ms / 1000,
                        d.signpost,
                        dd / 1000,
                        z.radius_m
                    ));
                }
            }
        }
    }
    for r in &ex.fix_reasons_include {
        if !fix_reasons.contains(r) {
            v.global_failures
                .push(alloc::format!("fix reason {r:?} never seen"));
        }
    }
    if let Some(m) = ex.max_total_fires
        && total_fires > m
    {
        v.global_failures
            .push(alloc::format!("{total_fires} fires in total (max {m})"));
    }

    // Per-signpost checks, in expectation order.
    for e in &ex.signposts {
        v.signposts.push(judge_signpost(sc, samples, run, e)?);
    }
    Ok(v)
}

#[allow(clippy::too_many_lines)]
fn judge_signpost(
    sc: &Scenario,
    samples: &[Sample],
    run: &RunRec,
    e: &SignpostExpect,
) -> SimResult<SignpostRun> {
    let mut r = SignpostRun::default();
    let idx = sc.index_of(&e.signpost).unwrap_or(usize::MAX);
    let geom = sc.geometry.get(idx).unwrap_or(&Geometry::Opaque);
    let mut reasons = BTreeSet::new();
    let mut prev_inside = false;
    let mut prev_naive: Option<bool> = None;

    for st in &run.steps {
        let inside = st.inside.get(idx).copied().unwrap_or(false);
        if inside != prev_inside {
            r.transitions += 1;
            prev_inside = inside;
        }
        // naive classifier: is the reported centre inside the zone? (what flapping looks like
        // without accuracy-aware hysteresis)
        if !matches!(geom, Geometry::Route { .. } | Geometry::Opaque)
            && let Ok(q) = sc.local.enu(st.fix.position())
            && let Some(d) = zone_depth(geom, (i64::from(q.e_mm), i64::from(q.n_mm)))?
        {
            let n = d >= 0;
            if prev_naive.is_some_and(|p| p != n) {
                r.naive_transitions += 1;
            }
            prev_naive = Some(n);
        }
        let Some(out) = &st.output else { continue };
        for d in out.decisions.iter().filter(|d| d.signpost.as_str() == e.signpost) {
            reasons.insert(d.reason);
            if d.outcome != Outcome::Fire {
                continue;
            }
            let Some(s) = samples.get(st.sample) else { continue };
            let mut f = FireTruth {
                t_rel_ms: st.fix.t_ms - sc.start_ms,
                t_hat_ms: d.t_hat_ms,
                d_rem_mm: None,
                xt_mm: None,
                depth_mm: None,
                latency_ms: None,
            };
            match geom {
                Geometry::Route {
                    line, anchor_mm, ..
                } => {
                    let (d_rem, xt) = route_truth(line, *anchor_mm, pt(s))?;
                    f.d_rem_mm = Some(d_rem);
                    f.xt_mm = Some(xt);
                }
                Geometry::Circle { .. } | Geometry::Polygon { .. } => {
                    f.depth_mm = zone_depth(geom, pt(s))?;
                    // latest true entry (outside -> inside) at or before this fix
                    let mut entry: Option<i64> = None;
                    let mut was_in: Option<bool> = None;
                    for ts in samples.iter().take(st.sample.saturating_add(1)) {
                        let inn = zone_depth(geom, pt(ts))?.is_some_and(|x| x >= 0);
                        if inn && was_in != Some(true) {
                            entry = Some(ts.t_rel_ms);
                        }
                        was_in = Some(inn);
                    }
                    f.latency_ms = entry.map(|t| s.t_rel_ms - t);
                }
                Geometry::Opaque => {}
            }
            r.fires.push(f);
        }
        if let Some(notes) = &out.notes {
            for n in notes.iter().filter(|n| n.signpost.as_str() == e.signpost) {
                reasons.insert(n.reason);
            }
        }
    }

    let n = u32::try_from(r.fires.len()).unwrap_or(u32::MAX);
    let [lo, hi] = e.fires;
    let mut failures: Vec<String> = Vec::new();
    let mut fail = |msg: String| failures.push(msg);
    if n < lo || n > hi {
        fail(alloc::format!("{} fired {n} time(s), expected {lo}..={hi}", e.signpost));
    }
    let mut prev_t: Option<i64> = None;
    for f in &r.fires {
        let at = f.t_rel_ms / 1000;
        if let (Some(w), Some(x)) = (e.true_d_rem_m, f.d_rem_mm)
            && !in_range(x, [w[0] * 1000, w[1] * 1000])
        {
            fail(alloc::format!("fire at t+{at} s: true d_rem {} m outside {w:?}", x / 1000));
        }
        if let (Some(w), Some(x)) = (e.true_xt_max_m, f.xt_mm)
            && x.abs() > w * 1000
        {
            fail(alloc::format!("fire at t+{at} s: truly {} m off the route (max {w})", x / 1000));
        }
        if let Some(w) = e.true_depth_min_m {
            match f.depth_mm {
                Some(x) if x >= w * 1000 => {}
                Some(x) => fail(alloc::format!(
                    "fire at t+{at} s: true depth {} mm < {} m (false enter)",
                    x,
                    w
                )),
                None => fail(alloc::format!("fire at t+{at} s: no zone geometry to judge depth")),
            }
        }
        if let Some(w) = e.latency_s {
            match f.latency_ms {
                Some(x) if in_range(x, [w[0] * 1000, w[1] * 1000]) => {}
                Some(x) => fail(alloc::format!(
                    "fire at t+{at} s: {x} ms after the true entry, expected {w:?} s"
                )),
                None => fail(alloc::format!("fire at t+{at} s: the truth never entered")),
            }
        }
        if let Some(w) = e.fire_time_s
            && !in_range(f.t_rel_ms, [w[0] * 1000, w[1] * 1000])
        {
            fail(alloc::format!("fire at t+{at} s outside {w:?} s"));
        }
        if let Some(w) = e.t_hat_min_s
            && f.t_hat_ms.is_none_or(|t| t < w * 1000)
        {
            fail(alloc::format!(
                "fire at t+{at} s: t_hat {:?} ms below {w} s",
                f.t_hat_ms
            ));
        }
        if let (Some(w), Some(p)) = (e.min_gap_s, prev_t)
            && f.t_rel_ms - p < w * 1000
        {
            fail(alloc::format!(
                "fires {} s apart (min {w} s)",
                (f.t_rel_ms - p) / 1000
            ));
        }
        prev_t = Some(f.t_rel_ms);
    }
    for c in &e.reasons_include {
        if !reasons.contains(c) {
            fail(alloc::format!("{}: reason {c:?} never seen", e.signpost));
        }
    }
    if let Some(m) = e.max_transitions_per_10min {
        let dur = run.duration_ms.max(1);
        if i128::from(r.transitions) * 600_000 > i128::from(m) * i128::from(dur) {
            fail(alloc::format!(
                "{} detector transitions in {} s (max {m} per 10 min)",
                r.transitions,
                dur / 1000
            ));
        }
    }
    r.failures = failures;
    r.ok = r.failures.is_empty();
    Ok(r)
}

/// Every reason code the engine reported for any signpost in a run (used in reports).
#[must_use]
pub fn reasons_seen(run: &RunRec) -> BTreeSet<ReasonCode> {
    let mut s = BTreeSet::new();
    for st in &run.steps {
        if let Some(o) = &st.output {
            s.insert(o.fix.reason);
            for d in &o.decisions {
                s.insert(d.reason);
            }
        }
    }
    s
}
