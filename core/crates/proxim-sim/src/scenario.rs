//! Scenario files (`core/testdata/scenarios/*.json`): signposts, the true path, noise, consent,
//! policy, clock events and the expected outcome. See `docs/simulator.md` for the format.
//!
//! JSON rather than the YAML the plan names: the maintained YAML crates for serde are either
//! archived (`serde_yaml`, unmaintained since 2024) or carry a RustSec advisory history
//! (`serde_yml`), and the workspace already depends on `serde_json` with strict parsing. Unknown
//! fields are rejected everywhere so a typo in a scenario fails loudly instead of being ignored.
//!
//! Geometry is authored in whole metres east (`e`) and north (`n`) of the scenario `origin`; the
//! loader converts it to e7 with the engine's frame. Trigger origins follow the compiler rule
//! (spec 1.2): route anchor, circle centre, polygon bounding-box centre.

use crate::geom::{Local, SimError, SimResult, put};
use crate::noise::NoiseParams;
use crate::trace::{Truth, TruthBuilder, geojson_points, gpx_points, truth_from_points};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use proxim_geo::{Polyline, project_to_polyline};
use proxim_types::{Class, Consent, ConsentValue, Enu, EventKind, Policy, Position, ReasonCode, Signpost};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};

/// Monday 2026-10-12 08:00:00 UTC: the default start of every scenario (no real date matters).
pub const DEFAULT_START_MS: i64 = 1_791_763_200_000 + 8 * 3_600_000;

fn default_category() -> String {
    "food.cafe".to_string()
}
fn default_sender() -> String {
    "sender-1".to_string()
}
const fn default_class() -> Class {
    Class::General
}
const fn default_period() -> i64 {
    1000
}
fn default_profile() -> String {
    "none".to_string()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioFile {
    pub id: String,
    pub title: String,
    /// Where the scenario comes from (plan row, addendum claim, spec section).
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub origin: Position,
    #[serde(default)]
    pub start_ms: Option<i64>,
    #[serde(default = "default_period")]
    pub period_ms: i64,
    pub signposts: Vec<SignpostSpec>,
    pub trace: TraceSpec,
    #[serde(default)]
    pub noise: NoiseSpec,
    #[serde(default)]
    pub dropouts: Vec<DropoutSpec>,
    /// Engine consent at the start (default: location granted, everything else unknown).
    #[serde(default)]
    pub consent: Option<Consent>,
    #[serde(default)]
    pub consent_changes: Vec<ConsentChange>,
    /// Engine policy JSON (strict), without `standoff_zones` (use `standoff_zones_m`).
    /// Default `{"explain": true}` so reasons are visible to the oracle.
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub standoff_zones_m: Vec<ZoneSpec>,
    #[serde(default)]
    pub clock: ClockSpec,
    /// Process restarts (state reset to `caps` only, spec 3.3) at these times, s from start.
    #[serde(default)]
    pub restarts_s: Vec<i64>,
    /// Runs in the default (CI-sized) sweep; ignored for a deterministic noise model (1 run).
    #[serde(default)]
    pub seeds: Option<u64>,
    pub expect: Expect,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignpostSpec {
    pub id: String,
    #[serde(default = "default_class")]
    pub class: Class,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default = "default_sender")]
    pub sender: String,
    #[serde(default)]
    pub sensitivity: u8,
    pub event: Option<EventKind>,
    pub shape: Option<ShapeSpec>,
    /// Extra trigger fields (`lead_s`, `mode`, `d_hi_m`, ...), merged into the trigger.
    #[serde(default)]
    pub trigger: Map<String, Value>,
    /// Extra signpost fields (`rev`, `min_path_conf_milli`, ...), merged into the signpost.
    #[serde(default)]
    pub extra: Map<String, Value>,
    /// Use a verbatim signpost file instead (relative to the scenario), pinned by SHA-256.
    #[serde(default)]
    pub fixture: Option<FixtureRef>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureRef {
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ShapeSpec {
    Circle {
        center_m: [i64; 2],
        radius_m: u32,
    },
    Polygon {
        vertices_m: Vec<[i64; 2]>,
        #[serde(default)]
        inradius_m: Option<u32>,
    },
    Route {
        vertices_m: Vec<[i64; 2]>,
        buffer_m: u32,
        #[serde(default)]
        anchor_along_m: Option<u32>,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TraceSpec {
    /// Synthetic legs from `start_m`.
    Path { start_m: [i64; 2], moves: Vec<Move> },
    /// First `LineString` of a GeoJSON file (relative to the scenario), at constant speed.
    Geojson { file: String, speed_mmps: u32 },
    /// A GPX track: its own timestamps, or constant `speed_mmps` if given.
    Gpx {
        file: String,
        #[serde(default)]
        speed_mmps: Option<u32>,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Move {
    Go { to_m: [i64; 2], speed_mmps: u32 },
    Wait { wait_s: i64 },
    Gap {
        gap_s: i64,
        #[serde(default)]
        then_at_m: Option<[i64; 2]>,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoiseSpec {
    #[serde(default = "default_profile")]
    pub profile: String,
    /// Field overrides on top of the profile (any `NoiseParams` field).
    #[serde(default)]
    pub set: Map<String, Value>,
}

impl Default for NoiseSpec {
    fn default() -> Self {
        Self {
            profile: default_profile(),
            set: Map::new(),
        }
    }
}

/// A span without fixes: by time (s from start) or by true distance travelled (m).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DropoutSpec {
    #[serde(default)]
    pub from_s: Option<i64>,
    #[serde(default)]
    pub to_s: Option<i64>,
    #[serde(default)]
    pub along_from_m: Option<i64>,
    #[serde(default)]
    pub along_to_m: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsentChange {
    pub at_s: i64,
    pub consent: Consent,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneSpec {
    pub id: String,
    pub center_m: [i64; 2],
    #[serde(default)]
    pub standoff_m: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockSpec {
    #[serde(default)]
    pub tz_offset_min: i16,
    /// `clock.now_ms - fix.t_ms` (a lagging host clock is negative... or an aged fix positive).
    #[serde(default)]
    pub lag_ms: i64,
    #[serde(default)]
    pub events: Vec<ClockEvent>,
}

/// From `at_s` on, the host clock is shifted by `shift_ms` more and/or reports a new zone.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockEvent {
    pub at_s: i64,
    #[serde(default)]
    pub shift_ms: i64,
    #[serde(default)]
    pub tz_offset_min: Option<i16>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    #[serde(default)]
    pub signposts: Vec<SignpostExpect>,
    /// No signpost may fire while the true position is within `radius_m` of `center_m`.
    #[serde(default)]
    pub no_fire_within: Vec<NoFireWithin>,
    /// Every listed code must appear as `fix.reason` at least once per run.
    #[serde(default)]
    pub fix_reasons_include: Vec<ReasonCode>,
    /// After a step whose `fix.reason` is one of these, `state.mem` must be empty (I4, I6).
    #[serde(default)]
    pub mem_empty_after: Vec<ReasonCode>,
    /// At most this many fires (all signposts) per run.
    #[serde(default)]
    pub max_total_fires: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoFireWithin {
    pub center_m: [i64; 2],
    pub radius_m: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignpostExpect {
    pub signpost: String,
    /// Fires per run, inclusive `[min, max]`.
    pub fires: [u32; 2],
    /// Share of runs that must meet every per-run check below (default 1000 = all of them).
    #[serde(default)]
    pub min_runs_ok_permille: Option<u32>,
    /// Route: true remaining distance along the route at each fire, metres.
    #[serde(default)]
    pub true_d_rem_m: Option<[i64; 2]>,
    /// Route: true cross-track distance at each fire at most this, metres.
    #[serde(default)]
    pub true_xt_max_m: Option<i64>,
    /// Zone: true signed depth (inside positive) at each fire at least this, metres.
    #[serde(default)]
    pub true_depth_min_m: Option<i64>,
    /// Zone: fire time minus the true entry time, seconds, inclusive.
    #[serde(default)]
    pub latency_s: Option<[i64; 2]>,
    /// Each fire's time, seconds from start, inclusive.
    #[serde(default)]
    pub fire_time_s: Option<[i64; 2]>,
    /// Engine-reported `t_hat_ms` at each fire at least this many seconds.
    #[serde(default)]
    pub t_hat_min_s: Option<i64>,
    /// Consecutive fires at least this far apart, seconds.
    #[serde(default)]
    pub min_gap_s: Option<i64>,
    /// Each code must appear for this signpost (decision or note) at least once per run.
    #[serde(default)]
    pub reasons_include: Vec<ReasonCode>,
    /// Detector inside/outside transitions, at most this many per 10 minutes of run.
    #[serde(default)]
    pub max_transitions_per_10min: Option<u32>,
}

/// A loaded, built scenario: everything the runner needs.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub file: ScenarioFile,
    pub path: PathBuf,
    /// SHA-256 of the scenario file bytes.
    pub sha256: String,
    pub local: Local,
    pub signposts: Vec<Signpost>,
    /// Local geometry of each signpost (same order), for the oracle.
    pub geometry: Vec<Geometry>,
    pub truth: Truth,
    pub noise: NoiseParams,
    /// Dropouts as inclusive `t_rel_ms` ranges.
    pub dropouts: Vec<(i64, i64)>,
    pub policy: Policy,
    pub start_ms: i64,
}

/// A signpost's shape in the scenario's local plane (mm).
#[derive(Clone, Debug)]
pub enum Geometry {
    Circle { center: (i64, i64), radius_mm: i64 },
    Polygon { vertices: Vec<Enu> },
    Route { line: Polyline, anchor_mm: i64, buffer_mm: i64 },
    /// A fixture signpost the oracle does not model (only fire counts are judged).
    Opaque,
}

fn m_to_mm(p: [i64; 2]) -> (i64, i64) {
    let [e, n] = p;
    (e.saturating_mul(1000), n.saturating_mul(1000))
}

fn enu(p: (i64, i64)) -> SimResult<Enu> {
    Ok(Enu {
        e_mm: i32::try_from(p.0).map_err(|_| SimError::new("coordinate beyond 150 km"))?,
        n_mm: i32::try_from(p.1).map_err(|_| SimError::new("coordinate beyond 150 km"))?,
    })
}

fn pos_json(p: Position) -> Value {
    json!({"lat_e7": p.lat_e7, "lon_e7": p.lon_e7})
}

/// Point at `along_mm` on a polyline (clamped to its ends).
fn point_along(line: &Polyline, along_mm: i64) -> (i64, i64) {
    let mut out = line
        .vertices
        .last()
        .map_or((0, 0), |v| (i64::from(v.e_mm), i64::from(v.n_mm)));
    for (i, w) in line.cum_mm.windows(2).enumerate() {
        let (Some(c0), Some(c1)) = (w.first(), w.get(1)) else {
            continue;
        };
        if along_mm >= *c0 && along_mm <= *c1 {
            let (Some(a), Some(b)) = (line.vertices.get(i), line.vertices.get(i + 1)) else {
                continue;
            };
            let span = (c1 - c0).max(1);
            let t = along_mm - c0;
            let lerp = |x: i32, y: i32| {
                i64::from(x) + (i64::from(y) - i64::from(x)) * t / span
            };
            out = (lerp(a.e_mm, b.e_mm), lerp(a.n_mm, b.n_mm));
            break;
        }
    }
    out
}

fn build_signpost(
    spec: &SignpostSpec,
    local: &Local,
    dir: &Path,
) -> SimResult<(Signpost, Geometry)> {
    if let Some(fx) = &spec.fixture {
        let path = dir.join(&fx.path);
        let bytes = std::fs::read(&path)
            .map_err(|e| SimError::new(alloc::format!("{}: {e}", path.display())))?;
        let got = proxim_decide::sha256::sha256_hex(&bytes);
        if got != fx.sha256 {
            return Err(SimError::new(alloc::format!(
                "fixture {} digest {got} does not match the pinned {}",
                fx.path, fx.sha256
            )));
        }
        let text = core::str::from_utf8(&bytes).map_err(|_| SimError::new("fixture not UTF-8"))?;
        let sp: Signpost = serde_json::from_str(text)?;
        if sp.id.as_str() != spec.id {
            return Err(SimError::new("fixture id differs from the scenario's signpost id"));
        }
        return Ok((sp, Geometry::Opaque));
    }
    let shape = spec
        .shape
        .as_ref()
        .ok_or_else(|| SimError::new(alloc::format!("signpost {} needs a shape", spec.id)))?;
    let event = spec
        .event
        .ok_or_else(|| SimError::new(alloc::format!("signpost {} needs an event", spec.id)))?;
    let (shape_json, origin, geom) = match shape {
        ShapeSpec::Circle { center_m, radius_m } => {
            let c = m_to_mm(*center_m);
            let cp = local.pos(c.0, c.1)?;
            (
                json!({"kind": "circle", "center": pos_json(cp), "radius_m": radius_m}),
                cp,
                Geometry::Circle {
                    center: c,
                    radius_mm: i64::from(*radius_m) * 1000,
                },
            )
        }
        ShapeSpec::Polygon {
            vertices_m,
            inradius_m,
        } => {
            let mm: Vec<(i64, i64)> = vertices_m.iter().map(|v| m_to_mm(*v)).collect();
            let (mut lo_e, mut lo_n, mut hi_e, mut hi_n) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
            for (e, n) in &mm {
                lo_e = lo_e.min(*e);
                hi_e = hi_e.max(*e);
                lo_n = lo_n.min(*n);
                hi_n = hi_n.max(*n);
            }
            let centre = local.pos((lo_e + hi_e) / 2, (lo_n + hi_n) / 2)?;
            let verts = mm
                .iter()
                .map(|p| local.pos(p.0, p.1).map(pos_json))
                .collect::<SimResult<Vec<_>>>()?;
            let mut s = json!({"kind": "polygon", "vertices": verts});
            if let Some(r) = inradius_m {
                put(&mut s, "inradius_m", json!(r));
            }
            (
                s,
                centre,
                Geometry::Polygon {
                    vertices: mm.iter().map(|p| enu(*p)).collect::<SimResult<Vec<_>>>()?,
                },
            )
        }
        ShapeSpec::Route {
            vertices_m,
            buffer_m,
            anchor_along_m,
        } => {
            let mm: Vec<(i64, i64)> = vertices_m.iter().map(|v| m_to_mm(*v)).collect();
            let line = Polyline::new(&mm.iter().map(|p| enu(*p)).collect::<SimResult<Vec<_>>>()?)?;
            let anchor_mm = anchor_along_m.map_or_else(
                || line.length_mm(),
                |a| i64::from(a) * 1000,
            );
            let ap = point_along(&line, anchor_mm);
            let origin = local.pos(ap.0, ap.1)?;
            let verts = mm
                .iter()
                .map(|p| local.pos(p.0, p.1).map(pos_json))
                .collect::<SimResult<Vec<_>>>()?;
            let mut s = json!({"kind": "route", "vertices": verts, "buffer_m": buffer_m});
            if let Some(a) = anchor_along_m {
                put(&mut s, "anchor_along_m", json!(a));
            }
            (
                s,
                origin,
                Geometry::Route {
                    line,
                    anchor_mm,
                    buffer_mm: i64::from(*buffer_m) * 1000,
                },
            )
        }
    };
    let mut trigger = json!({"origin": pos_json(origin), "shape": shape_json});
    for (k, v) in &spec.trigger {
        put(&mut trigger, k, v.clone());
    }
    let mut sp = json!({
        "id": spec.id, "rev": 1, "class": spec.class, "category": spec.category,
        "sender": spec.sender, "sensitivity": spec.sensitivity, "event": event,
        "valid_from_ms": 0, "valid_until_ms": i64::MAX, "trigger": trigger
    });
    for (k, v) in &spec.extra {
        put(&mut sp, k, v.clone());
    }
    let sp: Signpost = serde_json::from_value(sp)
        .map_err(|e| SimError::new(alloc::format!("signpost {}: {e}", spec.id)))?;
    Ok((sp, geom))
}

fn build_truth(t: &TraceSpec, local: &Local, dir: &Path) -> SimResult<Truth> {
    match t {
        TraceSpec::Path { start_m, moves } => {
            let mut b = TruthBuilder::new(m_to_mm(*start_m));
            for m in moves {
                match m {
                    Move::Go { to_m, speed_mmps } => {
                        b.go_to(m_to_mm(*to_m), *speed_mmps)?;
                    }
                    Move::Wait { wait_s } => {
                        b.wait(wait_s.saturating_mul(1000));
                    }
                    Move::Gap { gap_s, then_at_m } => {
                        b.gap(gap_s.saturating_mul(1000), then_at_m.map(m_to_mm));
                    }
                }
            }
            Ok(b.build())
        }
        TraceSpec::Geojson { file, speed_mmps } => {
            let text = read_text(&dir.join(file))?;
            let pts: Vec<(Position, Option<i64>)> =
                geojson_points(&text)?.into_iter().map(|p| (p, None)).collect();
            truth_from_points(local, &pts, Some(*speed_mmps))
        }
        TraceSpec::Gpx { file, speed_mmps } => {
            let text = read_text(&dir.join(file))?;
            truth_from_points(local, &gpx_points(&text)?, *speed_mmps)
        }
    }
}

fn read_text(p: &Path) -> SimResult<String> {
    std::fs::read_to_string(p).map_err(|e| SimError::new(alloc::format!("{}: {e}", p.display())))
}

fn noise_params(spec: &NoiseSpec) -> SimResult<NoiseParams> {
    let base = NoiseParams::profile(&spec.profile)?;
    if spec.set.is_empty() {
        return Ok(base);
    }
    let mut v = serde_json::to_value(NoiseParamsSer(&base))?;
    for (k, x) in &spec.set {
        if v.get(k).is_none() {
            return Err(SimError::new(alloc::format!("unknown noise field {k:?}")));
        }
        put(&mut v, k, x.clone());
    }
    Ok(serde_json::from_value(v)?)
}

/// Serialize helper (the params type only derives `Deserialize` publicly).
struct NoiseParamsSer<'a>(&'a NoiseParams);

impl serde::Serialize for NoiseParamsSer<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let p = self.0;
        json!({
            "sigma_mm": p.sigma_mm, "rho_milli": p.rho_milli, "acc_mm": p.acc_mm,
            "acc_sigma_mm": p.acc_sigma_mm, "acc_min_mm": p.acc_min_mm, "jump_ppm": p.jump_ppm,
            "jump_mm": p.jump_mm, "jump_fixes": p.jump_fixes,
            "jump_acc_permille": p.jump_acc_permille, "speed_sigma_mmps": p.speed_sigma_mmps,
            "course_sigma_ddeg": p.course_sigma_ddeg,
            "course_min_speed_mmps": p.course_min_speed_mmps,
            "reacquire_fixes": p.reacquire_fixes, "reacquire_acc_mm": p.reacquire_acc_mm,
            "reacquire_sigma_mm": p.reacquire_sigma_mm, "doppler": p.doppler
        })
        .serialize(s)
    }
}

fn dropouts(specs: &[DropoutSpec], truth: &Truth, period_ms: i64) -> SimResult<Vec<(i64, i64)>> {
    let mut out = Vec::new();
    for d in specs {
        match (d.from_s, d.to_s, d.along_from_m, d.along_to_m) {
            (Some(a), Some(b), None, None) => out.push((a * 1000, b * 1000)),
            (None, None, Some(a), Some(b)) => {
                // first and last sample whose true travelled distance lies in [a, b]
                let samples = truth.samples(period_ms)?;
                let inside: Vec<i64> = samples
                    .iter()
                    .filter(|s| s.along_mm >= a * 1000 && s.along_mm <= b * 1000)
                    .map(|s| s.t_rel_ms)
                    .collect();
                if let (Some(lo), Some(hi)) = (inside.first(), inside.last()) {
                    out.push((*lo, *hi));
                }
            }
            _ => {
                return Err(SimError::new(
                    "a dropout needs from_s and to_s, or along_from_m and along_to_m",
                ));
            }
        }
    }
    Ok(out)
}

/// The consent at the start of a run: location granted, everything else unknown.
#[must_use]
pub fn default_consent() -> Consent {
    Consent {
        location_matching: ConsentValue::Granted,
        ..Consent::default()
    }
}

impl Scenario {
    /// Load and build a scenario file.
    ///
    /// # Errors
    /// Unreadable or invalid file, invalid geometry, a fixture whose digest moved.
    pub fn load(path: &Path) -> SimResult<Self> {
        let bytes = std::fs::read(path)
            .map_err(|e| SimError::new(alloc::format!("{}: {e}", path.display())))?;
        let text =
            core::str::from_utf8(&bytes).map_err(|_| SimError::new("scenario is not UTF-8"))?;
        let file: ScenarioFile = serde_json::from_str(text)
            .map_err(|e| SimError::new(alloc::format!("{}: {e}", path.display())))?;
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        Self::build(file, path.to_path_buf(), dir, &bytes)
    }

    fn build(file: ScenarioFile, path: PathBuf, dir: &Path, bytes: &[u8]) -> SimResult<Self> {
        let local = Local::new(file.origin)?;
        let mut signposts = Vec::new();
        let mut geometry = Vec::new();
        for s in &file.signposts {
            let (sp, g) = build_signpost(s, &local, dir)?;
            signposts.push(sp);
            geometry.push(g);
        }
        for e in &file.expect.signposts {
            if !file.signposts.iter().any(|s| s.id == e.signpost) {
                return Err(SimError::new(alloc::format!(
                    "expectation for unknown signpost {}",
                    e.signpost
                )));
            }
        }
        let truth = build_truth(&file.trace, &local, dir)?;
        if truth.pieces.is_empty() {
            return Err(SimError::new("the trace is empty"));
        }
        let noise = noise_params(&file.noise)?;
        let dropouts = dropouts(&file.dropouts, &truth, file.period_ms)?;
        let mut policy = file.policy.clone().unwrap_or_else(|| json!({"explain": true}));
        if !file.standoff_zones_m.is_empty() {
            let zones = file
                .standoff_zones_m
                .iter()
                .map(|z| {
                    let c = m_to_mm(z.center_m);
                    let p = local.pos(c.0, c.1)?;
                    let mut v = json!({"id": z.id, "center": pos_json(p)});
                    if let Some(s) = z.standoff_m {
                        put(&mut v, "standoff_m", json!(s));
                    }
                    Ok(v)
                })
                .collect::<SimResult<Vec<_>>>()?;
            put(&mut policy, "standoff_zones", Value::Array(zones));
        }
        let policy: Policy = serde_json::from_value(policy)
            .map_err(|e| SimError::new(alloc::format!("policy: {e}")))?;
        let start_ms = file.start_ms.unwrap_or(DEFAULT_START_MS);
        Ok(Self {
            sha256: proxim_decide::sha256::sha256_hex(bytes),
            file,
            path,
            local,
            signposts,
            geometry,
            truth,
            noise,
            dropouts,
            policy,
            start_ms,
        })
    }

    /// Runs for this scenario: 1 for a deterministic model, else `override_seeds` or the file's
    /// `seeds` (default 20).
    #[must_use]
    pub fn runs(&self, override_seeds: Option<u64>) -> u64 {
        if self.noise.is_deterministic() {
            1
        } else {
            override_seeds.or(self.file.seeds).unwrap_or(20).max(1)
        }
    }

    /// Index of a signpost by id.
    #[must_use]
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.signposts.iter().position(|s| s.id.as_str() == id)
    }
}

/// True cross-track / remaining distance of a point against a route geometry.
///
/// # Errors
/// Engine geometry errors (degenerate route).
pub fn route_truth(line: &Polyline, anchor_mm: i64, p: (i64, i64)) -> SimResult<(i64, i64)> {
    let pr = project_to_polyline(line, enu(p)?, None)?;
    Ok((anchor_mm - pr.along_mm, pr.xt_mm))
}

/// All scenario files (`*.json`) directly in `dir`, sorted by file name.
///
/// # Errors
/// Unreadable directory.
pub fn list_dir(dir: &Path) -> SimResult<Vec<PathBuf>> {
    let rd = std::fs::read_dir(dir)
        .map_err(|e| SimError::new(alloc::format!("{}: {e}", dir.display())))?;
    let mut v: Vec<PathBuf> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "json"))
        .collect();
    v.sort();
    Ok(v)
}
