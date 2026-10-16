//! Scenario world: coordinates, builders for wire JSON (spec 3.3) and the `Rig` that drives
//! `decide_json` one fix at a time while threading state.

use crate::canon::{check_canonical, render};
use proxim_geo::trig::scale_for_lat_q;
use serde_json::{Value, json};

/// Monday 2026-10-12 00:00:00 UTC. `day_idx = floor(t / 86_400_000)`, weeks start on Monday
/// (`week_idx = floor_div(day_idx + 3, 7)`, spec 7), so this instant is the first ms of a week.
pub const T0: i64 = 1_791_763_200_000;
/// 08:00 UTC the same day: default fix time, far from any day boundary.
pub const BASE_T: i64 = T0 + 8 * 3_600_000;
pub const DAY_MS: i64 = 86_400_000;
/// Latitude 45.0, longitude 7.0 (synthetic, mid-latitude).
pub const ORIGIN: (i32, i32) = (450_000_000, 70_000_000);

pub type P = (i32, i32);

// ---------------------------------------------------------------------------------------------
// Geometry helpers (spec 1.2 via the public SCALE table; no engine internals)
// ---------------------------------------------------------------------------------------------

fn round_div(a: i128, b: i128) -> i128 {
    // floor((2a + b) / (2b)) for b > 0, spec 1.3
    (2 * a + b).div_euclid(2 * b)
}

/// `(kn_q32, ke_q32)` for a latitude in e7 degrees.
pub fn scale_at(lat_e7: i32) -> (i64, i64) {
    let lat_q = round_div(i128::from(lat_e7), 100_000) as i32;
    scale_for_lat_q(lat_q).expect("lat in range")
}

/// Position `e_mm` east and `n_mm` north of `origin`, using the frame of `origin` (spec 1.2
/// inverse). Accurate to a few mm over a few km.
pub fn offset(origin: P, e_mm: i64, n_mm: i64) -> P {
    let (kn, ke) = scale_at(origin.0);
    let dlat = round_div(i128::from(n_mm) << 32, i128::from(kn));
    let dlon = round_div(i128::from(e_mm) << 32, i128::from(ke));
    (
        (i128::from(origin.0) + dlat) as i32,
        (i128::from(origin.1) + dlon) as i32,
    )
}

/// Straight-line distance in mm between two positions near `a` (planar frame of `a`).
pub fn dist_mm(a: P, b: P) -> i64 {
    let (kn, ke) = scale_at(a.0);
    let dn = round_div(
        (i128::from(b.0) - i128::from(a.0)) * i128::from(kn),
        1 << 32,
    );
    let de = round_div(
        (i128::from(b.1) - i128::from(a.1)) * i128::from(ke),
        1 << 32,
    );
    let sq = (dn * dn + de * de) as u128;
    proxim_geo::isqrt(sq) as i64
}

pub fn pos(p: P) -> Value {
    json!({"lat_e7": p.0, "lon_e7": p.1})
}

// ---------------------------------------------------------------------------------------------
// Wire builders
// ---------------------------------------------------------------------------------------------

pub fn empty_state() -> String {
    r#"{"caps":{"hwm_ms":0,"signposts":[]},"mem":{"det":[],"jump_rejects":0,"trace":[]},"v":1}"#
        .to_string()
}

pub fn empty_mem() -> Value {
    json!({"det": [], "jump_rejects": 0, "trace": []})
}

pub fn consent_granted() -> Value {
    json!({
        "location_matching": "granted",
        "nearby_health_events": "unknown",
        "aggregate_reporting": "unknown",
        "local_notifications": "unknown"
    })
}

pub fn clock(now_ms: i64, tz: i16) -> Value {
    json!({"now_ms": now_ms, "tz_offset_min": tz})
}

/// A good fix: accuracy 5 m (so `acc_eff` = 10 m), Doppler speed 10 m/s, course north.
pub fn fix_at(t_ms: i64, p: P) -> Value {
    json!({
        "t_ms": t_ms, "lat_e7": p.0, "lon_e7": p.1, "acc_mm": 5000,
        "speed_mmps": 10000, "course_ddeg": 0
    })
}

fn signpost_common(id: &str, event: &str, trigger: Value) -> Value {
    json!({
        "id": id, "rev": 1, "class": "general", "category": "cafe", "sender": "sender-1",
        "sensitivity": 0, "event": event, "valid_from_ms": 0,
        "valid_until_ms": i64::MAX, "trigger": trigger
    })
}

/// ENTER signpost: circle of `radius_m` centred on `centre`.
pub fn enter_circle_at(id: &str, centre: P, radius_m: i64) -> Value {
    signpost_common(
        id,
        "enter",
        json!({"origin": pos(centre), "shape": {"kind": "circle", "center": pos(centre), "radius_m": radius_m}}),
    )
}

/// ENTER signpost centred on [`ORIGIN`].
pub fn enter_circle(id: &str, radius_m: i64) -> Value {
    enter_circle_at(id, ORIGIN, radius_m)
}

/// A route that ends at the anchor [`ORIGIN`] and runs `len_m` metres due north to it. Following
/// the compiler rule (spec 1.2) the trigger origin is the route anchor.
pub fn approach_route(id: &str, len_m: i64) -> Value {
    let start = offset(ORIGIN, 0, -len_m * 1000);
    signpost_common(
        id,
        "approach",
        json!({
            "origin": pos(ORIGIN),
            "shape": {"kind": "route", "buffer_m": 30, "vertices": [pos(start), pos(ORIGIN)]},
            "lead_s": 40, "mode": "vehicle"
        }),
    )
}

/// Set a dotted path (`"trigger.shape.radius_m"`) in a JSON value, creating objects on the way.
pub fn set(v: &mut Value, path: &str, val: Value) {
    let mut cur = v;
    let parts: Vec<&str> = path.split('.').collect();
    for (i, k) in parts.iter().enumerate() {
        if i + 1 == parts.len() {
            cur[*k] = val;
            return;
        }
        if cur.get(*k).is_none() {
            cur[*k] = json!({});
        }
        cur = &mut cur[*k];
    }
}

pub fn unset(v: &mut Value, path: &str) {
    let parts: Vec<&str> = path.split('.').collect();
    let mut cur = v;
    for k in &parts[..parts.len() - 1] {
        cur = &mut cur[*k];
    }
    cur.as_object_mut().unwrap().remove(parts[parts.len() - 1]);
}

/// Local day index for `now` at `tz` (spec 7).
pub fn day_idx(now_ms: i64, tz_min: i16) -> i64 {
    (now_ms + i64::from(tz_min) * 60_000).div_euclid(DAY_MS)
}
pub fn week_idx(now_ms: i64, tz_min: i16) -> i64 {
    (day_idx(now_ms, tz_min) + 3).div_euclid(7)
}

// ---------------------------------------------------------------------------------------------
// Rig
// ---------------------------------------------------------------------------------------------

/// One `decide_json` response, parsed.
#[derive(Clone, Debug)]
pub struct Step {
    pub text: String,
    pub error: Option<Value>,
    pub output: Value,
    pub state: Value,
}

impl Step {
    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }
    pub fn fix_ok(&self) -> bool {
        self.output["fix"]["ok"].as_bool().unwrap()
    }
    pub fn fix_reason(&self) -> String {
        self.output["fix"]["reason"].as_str().unwrap().to_string()
    }
    pub fn decisions(&self) -> Vec<Value> {
        self.output["decisions"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    }
    pub fn fires(&self) -> Vec<Value> {
        self.decisions()
            .into_iter()
            .filter(|d| d["outcome"] == "fire")
            .collect()
    }
    pub fn fired(&self, id: &str) -> bool {
        self.fires().iter().any(|d| d["signpost"] == id)
    }
    /// Reason codes of `drop` decisions for `id`.
    pub fn drops(&self, id: &str) -> Vec<String> {
        self.decisions()
            .iter()
            .filter(|d| d["outcome"] == "drop" && d["signpost"] == id)
            .map(|d| d["reason"].as_str().unwrap().to_string())
            .collect()
    }
    /// Reason codes of notes for `id` (explain only).
    pub fn notes(&self, id: &str) -> Vec<String> {
        self.output
            .get("notes")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter(|n| n["signpost"] == id)
                    .map(|n| n["reason"].as_str().unwrap().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
    /// Every reason (drops and notes) reported for `id`.
    pub fn reasons(&self, id: &str) -> Vec<String> {
        let mut r = self.drops(id);
        r.extend(self.notes(id));
        r
    }
    /// The detector state entry of `id`.
    pub fn det(&self, id: &str) -> Option<Value> {
        self.state["mem"]["det"]
            .as_array()?
            .iter()
            .find(|e| e["id"] == id)
            .cloned()
    }
    pub fn caps(&self) -> &Value {
        &self.state["caps"]
    }
    pub fn mem(&self) -> &Value {
        &self.state["mem"]
    }
    pub fn trace_len(&self) -> usize {
        self.state["mem"]["trace"].as_array().map_or(0, Vec::len)
    }
    pub fn cap_entry(&self, id: &str) -> Option<Value> {
        self.state["caps"]["signposts"]
            .as_array()?
            .iter()
            .find(|e| e["id"] == id)
            .cloned()
    }
}

/// Drives `decide_json` and threads the state. Every response is checked to be canonical by the
/// independent checker (spec 3.2) before anything else looks at it.
#[derive(Clone, Debug)]
pub struct Rig {
    pub state: String,
    pub signposts: Vec<Value>,
    pub policy: Value,
    pub consent: Value,
    pub tz: i16,
    pub seed: u64,
    pub history: Vec<Step>,
}

impl Default for Rig {
    fn default() -> Self {
        Self::new()
    }
}

impl Rig {
    pub fn new() -> Self {
        Self {
            state: empty_state(),
            signposts: Vec::new(),
            policy: json!({"explain": true}),
            consent: consent_granted(),
            tz: 0,
            seed: 0,
            history: Vec::new(),
        }
    }
    pub fn with_signpost(mut self, sp: Value) -> Self {
        self.signposts.push(sp);
        self
    }
    pub fn with_policy(mut self, key: &str, v: Value) -> Self {
        self.policy[key] = v;
        self
    }

    /// Raw call with the rig's current state, without advancing it.
    pub fn peek(&self, fix: &Value, now_ms: i64) -> Step {
        self.call_with(&self.state, fix, &clock(now_ms, self.tz))
    }

    pub fn call_with(&self, state: &str, fix: &Value, clock: &Value) -> Step {
        let text = proxim_ffi::decide_json(
            state,
            &render(fix),
            &render(&Value::Array(self.signposts.clone())),
            &render(&self.policy),
            &render(&self.consent),
            &render(clock),
            self.seed,
        );
        parse_response(text)
    }

    /// One engine call; advances the state on success.
    pub fn call(&mut self, fix: &Value, now_ms: i64) -> Step {
        let c = clock(now_ms, self.tz);
        self.call_clock(fix, &c)
    }

    pub fn call_clock(&mut self, fix: &Value, clock: &Value) -> Step {
        let step = self.call_with(&self.state.clone(), fix, clock);
        if step.is_ok() {
            self.state = render(&step.state);
        }
        self.history.push(step.clone());
        step
    }

    /// Clock equal to the fix time.
    pub fn step(&mut self, fix: &Value) -> Step {
        let t = fix["t_ms"].as_i64().unwrap_or(0);
        self.call(fix, t)
    }

    /// The state as a JSON value.
    pub fn state_json(&self) -> Value {
        serde_json::from_str(&self.state).unwrap()
    }
}

pub fn parse_response(text: String) -> Step {
    if let Err(e) = check_canonical(&text) {
        panic!(
            "decide_json returned non-canonical JSON ({e}): {}",
            &text[..text.len().min(400)]
        );
    }
    let v: Value = serde_json::from_str(&text).expect("canonical JSON parses");
    let obj = v.as_object().expect("top-level object");
    if let Some(err) = obj.get("error") {
        assert_eq!(
            obj.len(),
            1,
            "error response must hold only `error`: {text}"
        );
        return Step {
            text,
            error: Some(err.clone()),
            output: Value::Null,
            state: Value::Null,
        };
    }
    assert_eq!(
        obj.len(),
        2,
        "success response is exactly {{output,state}}: {text}"
    );
    Step {
        error: None,
        output: obj["output"].clone(),
        state: obj["state"].clone(),
        text,
    }
}

// ---------------------------------------------------------------------------------------------
// Driving along the straight north route that ends at ORIGIN
// ---------------------------------------------------------------------------------------------

/// A vehicle moving north along `approach_route(_, len_m)` (or parallel to it, `lateral_mm` east).
#[derive(Clone, Debug)]
pub struct Drive {
    pub len_mm: i64,
    pub start_along_mm: i64,
    pub speed_mmps: i64,
    pub lateral_mm: i64,
    pub acc_mm: u32,
    pub dt_ms: i64,
    pub t0: i64,
    pub doppler: bool,
    pub course: Option<u16>,
}

impl Drive {
    pub fn new(len_m: i64, speed_mmps: i64) -> Self {
        Self {
            len_mm: len_m * 1000,
            start_along_mm: 0,
            speed_mmps,
            lateral_mm: 0,
            acc_mm: 5000,
            dt_ms: 1000,
            t0: BASE_T,
            doppler: true,
            course: Some(0),
        }
    }
    pub fn along_mm(&self, k: i64) -> i64 {
        self.start_along_mm + self.speed_mmps * self.dt_ms * k / 1000
    }
    pub fn t_ms(&self, k: i64) -> i64 {
        self.t0 + k * self.dt_ms
    }
    pub fn point(&self, k: i64) -> P {
        offset(ORIGIN, self.lateral_mm, self.along_mm(k) - self.len_mm)
    }
    pub fn fix(&self, k: i64) -> Value {
        let p = self.point(k);
        let mut f =
            json!({"t_ms": self.t_ms(k), "lat_e7": p.0, "lon_e7": p.1, "acc_mm": self.acc_mm});
        if self.doppler {
            f["speed_mmps"] = json!(self.speed_mmps);
        }
        if let Some(c) = self.course {
            f["course_ddeg"] = json!(c);
        }
        f
    }
}

/// Run `steps` fixes of the drive; returns the index and decision of the first fire of `id`.
pub fn drive_until_fire(rig: &mut Rig, d: &Drive, id: &str, steps: i64) -> Option<(i64, Value)> {
    for k in 0..steps {
        let s = rig.step(&d.fix(k));
        assert!(s.is_ok(), "engine error during drive: {}", s.text);
        if let Some(f) = s.fires().into_iter().find(|f| f["signpost"] == id) {
            return Some((k, f));
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------
// One-shot call description
// ---------------------------------------------------------------------------------------------

/// All seven arguments of `decide_json` as JSON values, for table-driven single-call tests.
#[derive(Clone, Debug)]
pub struct Call {
    pub state: String,
    pub fix: Value,
    pub signposts: Vec<Value>,
    pub policy: Value,
    pub consent: Value,
    pub clock: Value,
    pub seed: u64,
}

impl Call {
    /// Empty state, one ENTER circle `c-1` (200 m radius) at [`ORIGIN`], a good fix on it,
    /// explain on, consent granted, clock equal to the fix time.
    pub fn base() -> Self {
        Self {
            state: empty_state(),
            fix: fix_at(BASE_T, ORIGIN),
            signposts: vec![enter_circle("c-1", 200)],
            policy: json!({"explain": true}),
            consent: consent_granted(),
            clock: clock(BASE_T, 0),
            seed: 0,
        }
    }
    pub fn run(&self) -> Step {
        let text = proxim_ffi::decide_json(
            &self.state,
            &render(&self.fix),
            &render(&Value::Array(self.signposts.clone())),
            &render(&self.policy),
            &render(&self.consent),
            &render(&self.clock),
            self.seed,
        );
        parse_response(text)
    }
    /// Like [`Call::run`], but argument `arg` (`state|fix|signposts|policy|consent|clock`) is rendered
    /// canonically with the marker string `"@@"` standing in for `token`, which is spliced in
    /// verbatim (so it may be a float, `null`, `NaN`, a huge integer or any other non-canonical text).
    pub fn run_with_raw(&self, arg: &str, path: &str, token: &str) -> Step {
        let mut c = self.clone();
        let target: &mut Value = match arg {
            "fix" => &mut c.fix,
            "policy" => &mut c.policy,
            "consent" => &mut c.consent,
            "clock" => &mut c.clock,
            other => panic!("unsupported arg {other}"),
        };
        set(target, path, json!("@@"));
        let text = |v: &Value| render(v).replace("\"@@\"", token);
        let r = proxim_ffi::decide_json(
            &c.state,
            &if arg == "fix" {
                text(&c.fix)
            } else {
                render(&c.fix)
            },
            &render(&Value::Array(c.signposts.clone())),
            &if arg == "policy" {
                text(&c.policy)
            } else {
                render(&c.policy)
            },
            &if arg == "consent" {
                text(&c.consent)
            } else {
                render(&c.consent)
            },
            &if arg == "clock" {
                text(&c.clock)
            } else {
                render(&c.clock)
            },
            c.seed,
        );
        parse_response(r)
    }
    pub fn with_fix(mut self, path: &str, v: Value) -> Self {
        set(&mut self.fix, path, v);
        self
    }
    pub fn with_policy(mut self, path: &str, v: Value) -> Self {
        set(&mut self.policy, path, v);
        self
    }
    pub fn with_consent(mut self, path: &str, v: Value) -> Self {
        set(&mut self.consent, path, v);
        self
    }
    pub fn with_now(mut self, now_ms: i64) -> Self {
        self.clock["now_ms"] = json!(now_ms);
        self
    }
}

/// A state with populated `caps` and `mem` (an ENTER already fired at `c-1`), plus the last fix time.
pub fn fired_state() -> (Rig, i64) {
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 200));
    let mut last = BASE_T;
    for k in 0..4 {
        last = BASE_T + k * 1000;
        rig.step(&fix_at(last, ORIGIN));
    }
    assert_eq!(
        rig.history.last().unwrap().fires().len(),
        1,
        "setup must have fired"
    );
    (rig, last)
}

// ---------------------------------------------------------------------------------------------
// Seeded random worlds (for determinism / invariant sweeps)
// ---------------------------------------------------------------------------------------------

use crate::rng::Rng;

/// A reproducible mixed signpost set around [`ORIGIN`]: circles, polygons, routes, a health
/// signpost, statically invalid ones, revisions and duplicates.
pub fn mixed_signposts(rng: &mut Rng) -> Vec<Value> {
    let mut v = Vec::new();
    for i in 0..10 {
        let c = offset(
            ORIGIN,
            rng.range_i64(-3000, 3000) * 1000,
            rng.range_i64(-3000, 3000) * 1000,
        );
        let mut sp = enter_circle_at(&format!("c-{i:02}"), c, rng.range_i64(150, 600));
        sp["rank_milli"] = json!(rng.range_i64(500, 1500));
        v.push(sp);
    }
    for i in 0..4 {
        let c = offset(
            ORIGIN,
            rng.range_i64(-2000, 2000) * 1000,
            rng.range_i64(-2000, 2000) * 1000,
        );
        let mut sp = enter_circle_at(&format!("p-{i:02}"), c, 200);
        let sq = [(-250, -250), (250, -250), (250, 250), (-250, 250)]
            .map(|(e, n)| pos(offset(c, e * 1000, n * 1000)));
        set(
            &mut sp,
            "trigger.shape",
            json!({"kind": "polygon", "vertices": sq}),
        );
        v.push(sp);
    }
    for i in 0..4 {
        let len = rng.range_i64(800, 2500);
        let mut sp = approach_route(&format!("r-{i:02}"), len);
        let dx = rng.range_i64(-600, 600) * 1000;
        let dy = rng.range_i64(-600, 600) * 1000;
        // shift the whole route (origin, vertices) by (dx, dy)
        let shift = |p: &Value| -> Value {
            let q = (
                p["lat_e7"].as_i64().unwrap() as i32,
                p["lon_e7"].as_i64().unwrap() as i32,
            );
            pos(offset(q, dx, dy))
        };
        let o = shift(&sp["trigger"]["origin"]);
        sp["trigger"]["origin"] = o;
        let vs: Vec<Value> = sp["trigger"]["shape"]["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .map(shift)
            .collect();
        sp["trigger"]["shape"]["vertices"] = json!(vs);
        v.push(sp);
    }
    let mut h = enter_circle_at("h-00", offset(ORIGIN, 400_000, 400_000), 200);
    h["class"] = json!("health");
    v.push(h);
    v.push(enter_circle("bad-radius", 10));
    let mut old = enter_circle("dup-0", 200);
    old["rev"] = json!(1);
    v.push(old);
    let mut newer = enter_circle("dup-0", 300);
    newer["rev"] = json!(2);
    v.push(newer);
    let mut ev = enter_circle("evt-x", 200);
    ev["event"] = json!("dwell");
    v.push(ev);
    v
}

/// A reproducible drive: random walk with speed/course noise, occasional gaps and one jump.
pub fn random_trace(rng: &mut Rng, n: usize) -> Vec<(Value, i64)> {
    let mut p = offset(
        ORIGIN,
        rng.range_i64(-2000, 2000) * 1000,
        rng.range_i64(-2000, 2000) * 1000,
    );
    let mut t = BASE_T;
    let mut heading = rng.below(3600) as i64;
    let mut out = Vec::new();
    for i in 0..n {
        t += if rng.chance(1, 40) {
            rng.range_i64(2000, 90_000)
        } else {
            1000
        };
        heading = (heading + rng.range_i64(-120, 120)).rem_euclid(3600);
        let speed = rng.range_i64(0, 30_000);
        // advance along the heading (east = sin, north = cos), 1 s of travel
        let (e, nn) = {
            let u = proxim_geo::unit_from_course(heading as u16).unwrap();
            (
                i64::from(u[0]) * speed / (1 << 30),
                i64::from(u[1]) * speed / (1 << 30),
            )
        };
        p = offset(p, e, nn);
        let mut f = fix_at(t, p);
        f["speed_mmps"] = json!(speed);
        f["course_ddeg"] = json!(heading);
        f["acc_mm"] = json!(rng.range_i64(3000, 70_000));
        if rng.chance(1, 25) {
            unset(&mut f, "speed_mmps");
        }
        if rng.chance(1, 25) {
            unset(&mut f, "course_ddeg");
        }
        if rng.chance(1, 60) {
            f["is_mock"] = json!(true);
        }
        let mut now = t;
        if rng.chance(1, 50) {
            now += rng.range_i64(-6000, 12_000);
        }
        if i == n / 2 && rng.chance(1, 2) {
            // a teleport
            let q = offset(p, 20_000_000, 20_000_000);
            f["lat_e7"] = json!(q.0);
            f["lon_e7"] = json!(q.1);
        }
        out.push((f, now.max(0)));
    }
    out
}

/// Integer bearing (0.1 degree, clockwise from north) of the vector `(e, n)`: the table row whose
/// unit vector has the largest dot product. Test-side only; avoids any float trig.
pub fn bearing_ddeg(e: i64, n: i64) -> u16 {
    let mut best = (i128::MIN, 0_u16);
    for c in 0..3600_u16 {
        let u = proxim_geo::unit_from_course(c).unwrap();
        let d = i128::from(u[0]) * i128::from(e) + i128::from(u[1]) * i128::from(n);
        if d > best.0 {
            best = (d, c);
        }
    }
    best.1
}

/// A purposeful trace: drives to, dwells in and drives along the given signposts (so fires, caps and
/// cool-downs actually happen), with noise, gaps, 31-minute pauses, mock fixes and clock skew.
pub fn scripted_trace(rng: &mut Rng, signposts: &[Value], visits: usize) -> Vec<(Value, i64)> {
    let mut out: Vec<(Value, i64)> = Vec::new();
    let mut p = offset(ORIGIN, -4_000_000, -4_000_000);
    let mut t = BASE_T;
    let push =
        |out: &mut Vec<(Value, i64)>, rng: &mut Rng, t: i64, p: P, speed: i64, course: u16| {
            let q = offset(p, rng.range_i64(-3000, 3000), rng.range_i64(-3000, 3000));
            let mut f = fix_at(t, q);
            f["speed_mmps"] = json!(speed);
            f["course_ddeg"] = json!(course);
            f["acc_mm"] = json!(rng.range_i64(3000, 45_000));
            if rng.chance(1, 80) {
                f["is_mock"] = json!(true);
            }
            if rng.chance(1, 30) {
                unset(&mut f, "course_ddeg");
            }
            let now = if rng.chance(1, 60) {
                t + rng.range_i64(-5000, 9000)
            } else {
                t
            };
            out.push((f, now.max(0)));
        };
    let mut targets: Vec<Value> = signposts.to_vec();
    rng.shuffle(&mut targets);
    for sp in targets.into_iter().cycle().take(visits) {
        let to_pos = |v: &Value| -> P {
            (
                v["lat_e7"].as_i64().unwrap() as i32,
                v["lon_e7"].as_i64().unwrap() as i32,
            )
        };
        let is_route = sp["trigger"]["shape"]["kind"] == "route";
        let dest = if is_route {
            to_pos(&sp["trigger"]["shape"]["vertices"][0])
        } else {
            to_pos(&sp["trigger"]["origin"])
        };
        // travel in steps of at most 25 m/s
        let (kn, ke) = scale_at(p.0);
        let dn = ((i128::from(dest.0) - i128::from(p.0)) * i128::from(kn)) >> 32;
        let de = ((i128::from(dest.1) - i128::from(p.1)) * i128::from(ke)) >> 32;
        let dist = proxim_geo::isqrt((dn * dn + de * de) as u128) as i64;
        let speed = rng.range_i64(8_000, 25_000);
        let steps = (dist / speed).max(1);
        let course = bearing_ddeg(de as i64, dn as i64);
        t += rng.range_i64(2_000, 70_000); // a pause / GPS gap before the leg
        for k in 1..=steps.min(400) {
            t += 1000;
            let q = offset(p, de as i64 * k / steps, dn as i64 * k / steps);
            push(&mut out, rng, t, q, speed, course);
        }
        p = dest;
        if is_route {
            // drive the route towards its anchor and a little past it
            let verts: Vec<P> = sp["trigger"]["shape"]["vertices"]
                .as_array()
                .unwrap()
                .iter()
                .map(to_pos)
                .collect();
            let end = *verts.last().unwrap();
            let (kn, ke) = scale_at(p.0);
            let dn = ((i128::from(end.0) - i128::from(p.0)) * i128::from(kn)) >> 32;
            let de = ((i128::from(end.1) - i128::from(p.1)) * i128::from(ke)) >> 32;
            let dist = proxim_geo::isqrt((dn * dn + de * de) as u128) as i64;
            let v = rng.range_i64(6_000, 22_000);
            let steps = (dist / v).clamp(1, 500);
            let course = bearing_ddeg(de as i64, dn as i64);
            for k in 1..=steps + 8 {
                t += 1000;
                let q = offset(p, de as i64 * k / steps, dn as i64 * k / steps);
                push(&mut out, rng, t, q, v, course);
            }
            p = offset(
                p,
                de as i64 * (steps + 8) / steps,
                dn as i64 * (steps + 8) / steps,
            );
        } else {
            for _ in 0..rng.range_i64(5, 14) {
                t += 1000;
                push(&mut out, rng, t, p, 10_000, 0);
            }
            // leave: 40 m/s is too fast for the jump filter; walk away 50 m per second for 8 s
            for k in 1..=8 {
                t += 1000;
                let q = offset(p, 50_000 * k, 0);
                push(&mut out, rng, t, q, 12_000, 900);
            }
            p = offset(p, 400_000, 0);
        }
        // sometimes wait out the app interval so another fire is possible
        if rng.chance(2, 3) {
            t += 1_850_000;
        }
    }
    out
}
