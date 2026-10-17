//! Builders and a trace driver shared by the proxim-detect tests.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::print_stderr
)]

use proxim_detect::{Detected, Event, Prepared};
use proxim_geo::{Frame, frame_for};
use proxim_path::{PathSummary, path_update};
use proxim_types::{
    Class, DetState, Enu, EventKind, Fix, Id, Mem, Mode, Position, Reserved, Shape, Signpost,
    Trigger,
};

/// Synthetic origin at latitude 45, longitude 7 (invariant I2: not a real place of a person).
pub const ORIGIN: Position = Position {
    lat_e7: 450_000_000,
    lon_e7: 70_000_000,
};

/// Route length used by the approach fixtures: 6 km due north of the origin.
pub const ROUTE_M: i64 = 6000;

pub fn frame() -> Frame {
    frame_for(ORIGIN).unwrap()
}

pub fn pos(e_mm: i32, n_mm: i32) -> Position {
    frame().unproject(Enu { e_mm, n_mm }).unwrap()
}

pub fn fix_at(
    t_ms: i64,
    e_mm: i32,
    n_mm: i32,
    acc_mm: u32,
    speed: Option<u32>,
    course: Option<u16>,
) -> Fix {
    let p = pos(e_mm, n_mm);
    Fix {
        t_ms,
        lat_e7: p.lat_e7,
        lon_e7: p.lon_e7,
        acc_mm,
        speed_mmps: speed,
        speed_acc_mmps: None,
        course_ddeg: course,
        course_acc_ddeg: None,
        is_mock: false,
    }
}

pub fn trigger(shape: Shape) -> Trigger {
    Trigger {
        origin: ORIGIN,
        reach_m: None,
        shape,
        max_event_age_s: None,
        max_event_dist_m: None,
        lead_s: None,
        d_lo_m: None,
        d_hi_m: None,
        mode: None,
        speed_min_mmps: None,
        speed_max_mmps: None,
        bearing_tol_ddeg: None,
    }
}

pub fn signpost(event: EventKind, trigger: Trigger) -> Signpost {
    Signpost {
        id: Id::new("sp-1").unwrap(),
        rev: 1,
        class: Class::General,
        category: "cafe".into(),
        sender: "sender-1".into(),
        sensitivity: 0,
        event,
        valid_from_ms: 0,
        valid_until_ms: i64::MAX,
        min_path_tier: None,
        min_path_conf_milli: None,
        rank_milli: None,
        trigger,
        intent: Reserved::default(),
        content_mode: Reserved::default(),
        precision_class: Reserved::default(),
        min_mode: Reserved::default(),
        max_acc_m: Reserved::default(),
        arm_scope: Reserved::default(),
        relation: Reserved::default(),
        selector: Reserved::default(),
        live_at_ms: Reserved::default(),
        fresh_required_s: Reserved::default(),
        cooldown_s: Reserved::default(),
        creatives: Reserved::default(),
        rot_days: Reserved::default(),
        pack_kind: Reserved::default(),
    }
}

/// Circle of `radius_m` centred on the origin (ENTER).
pub fn circle(radius_m: u32) -> Signpost {
    signpost(
        EventKind::Enter,
        trigger(Shape::Circle {
            center: ORIGIN,
            radius_m,
        }),
    )
}

/// Axis-aligned square polygon centred on the origin, half side `half_m` (ENTER).
pub fn square(half_m: i32, inradius_m: Option<u32>) -> Signpost {
    let h = half_m * 1000;
    let vertices = vec![pos(-h, -h), pos(h, -h), pos(h, h), pos(-h, h)];
    signpost(
        EventKind::Enter,
        trigger(Shape::Polygon {
            vertices,
            inradius_m,
        }),
    )
}

/// A route running 6 km due north from the origin; the anchor is its end (APPROACH).
pub fn route(buffer_m: u32) -> Signpost {
    let vertices = vec![ORIGIN, pos(0, (ROUTE_M * 1000) as i32)];
    signpost(
        EventKind::Approach,
        trigger(Shape::Route {
            vertices,
            buffer_m,
            anchor_along_m: None,
        }),
    )
}

pub fn with_mode(mut sp: Signpost, mode: Mode) -> Signpost {
    sp.trigger.mode = Some(mode);
    sp
}

/// One recorded detector step.
#[derive(Clone, Debug)]
pub struct Step {
    pub fix: Fix,
    pub summary: PathSummary,
    pub out: Detected,
}

/// Runs the path filter and one detector over a trace, threading both states explicitly.
pub struct Driver {
    pub prep: Prepared,
    pub mem: Mem,
    pub st: Option<DetState>,
    pub steps: Vec<Step>,
    pub explain: bool,
}

impl Driver {
    pub fn new(sp: &Signpost) -> Self {
        Self {
            prep: Prepared::new(sp).unwrap(),
            mem: Mem::default(),
            st: None,
            steps: Vec::new(),
            explain: true,
        }
    }

    /// Feed a fix with `now = fix.t_ms + latency_ms`.
    pub fn feed(&mut self, fix: Fix, latency_ms: i64) -> Detected {
        let (mem, summary) = path_update(&self.mem, &fix).unwrap();
        self.mem = mem;
        let out = self
            .prep
            .step(self.st, &summary, &fix, fix.t_ms + latency_ms, self.explain)
            .unwrap();
        self.st = out.state;
        self.steps.push(Step { fix, summary, out });
        out
    }

    pub fn feed_all(&mut self, fixes: &[Fix]) -> Vec<Detected> {
        fixes.iter().map(|f| self.feed(*f, 0)).collect()
    }

    pub fn events(&self) -> Vec<(usize, Event)> {
        self.steps
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.out.event.map(|e| (i, e)))
            .collect()
    }
}

/// Along-route position (mm) of a fix on the due-north test route: its northing in the origin frame.
pub fn along_of(f: &Fix) -> i64 {
    i64::from(frame().project(f.position()).unwrap().n_mm)
}

/// The `Armed` fields `(confirm_t, a_prev_mm, last_t)` of a detector state, if it is armed.
pub fn armed_fields(st: Option<DetState>) -> Option<(Option<i64>, Option<i64>, i64)> {
    match st {
        Some(DetState::Armed {
            confirm_t,
            a_prev_mm,
            last_t,
        }) => Some((confirm_t, a_prev_mm, last_t)),
        _ => None,
    }
}

/// A vehicle driving due north along the route axis at `v_mmps`, 1 Hz fixes, accuracy 5 m,
/// starting at `n0_mm` at `t0_ms`, `count` fixes. Doppler speed and course are reported.
pub fn drive_north(t0_ms: i64, n0_mm: i64, v_mmps: u32, count: usize, e_mm: i32) -> Vec<Fix> {
    (0..count)
        .map(|i| {
            let n = n0_mm + i64::from(v_mmps) * i as i64;
            fix_at(
                t0_ms + 1000 * i as i64,
                e_mm,
                i32::try_from(n).unwrap(),
                5000,
                Some(v_mmps),
                Some(0),
            )
        })
        .collect()
}
