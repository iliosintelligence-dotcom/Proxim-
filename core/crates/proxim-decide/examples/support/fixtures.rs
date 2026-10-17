//! Synthetic fixtures shared by the golden generator and the integration tests (invariant I2:
//! nothing here is a real place or a real person's trace). Included with `#[path]`, so it is not a
//! crate of its own. Origin: latitude 45, longitude 7, as in the proxim-detect tests.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use proxim_geo::{Frame, frame_for};
use proxim_types::{
    Class, Clock, Consent, ConsentValue, Enu, EventKind, Fix, Id, Mode, Policy, Position, Reserved,
    Shape, Signpost, Trigger,
};

/// Synthetic origin at latitude 45, longitude 7.
pub const ORIGIN: Position = Position {
    lat_e7: 450_000_000,
    lon_e7: 70_000_000,
};

/// 2026-10-11 08:00:00 UTC in ms.
pub const T0: i64 = 1_791_705_600_000;
/// One day in ms.
pub const DAY: i64 = 86_400_000;

pub fn frame() -> Frame {
    frame_for(ORIGIN).unwrap()
}

pub fn pos(e_mm: i32, n_mm: i32) -> Position {
    frame().unproject(Enu { e_mm, n_mm }).unwrap()
}

/// A fix at an ENU offset (mm) from the origin.
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

pub fn clock(now_ms: i64) -> Clock {
    Clock {
        now_ms,
        tz_offset_min: 0,
    }
}

pub fn clock_tz(now_ms: i64, tz_offset_min: i16) -> Clock {
    Clock {
        now_ms,
        tz_offset_min,
    }
}

pub fn granted() -> Consent {
    Consent {
        location_matching: ConsentValue::Granted,
        ..Consent::default()
    }
}

/// Granted, plus the health consent and a receipt.
pub fn granted_health() -> Consent {
    Consent {
        location_matching: ConsentValue::Granted,
        nearby_health_events: ConsentValue::Granted,
        health_receipt: Some("receipt-test-1".into()),
        ..Consent::default()
    }
}

/// A policy with `explain` on (drops and notes are listed), everything else default.
pub fn explain() -> Policy {
    Policy {
        explain: Some(true),
        ..Policy::default()
    }
}

pub fn trigger(origin: Position, shape: Shape) -> Trigger {
    Trigger {
        origin,
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

pub fn signpost(id: &str, event: EventKind, trigger: Trigger) -> Signpost {
    Signpost {
        id: Id::new(id).unwrap(),
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
        rot_days: None,
        wordings: None,
        pack_kind: Reserved::default(),
    }
}

/// ENTER circle of `radius_m` centred on the origin.
pub fn circle(id: &str, radius_m: u32) -> Signpost {
    circle_at(id, 0, 0, radius_m)
}

/// ENTER circle centred at an ENU offset (mm); its trigger origin is its centre.
pub fn circle_at(id: &str, e_mm: i32, n_mm: i32, radius_m: u32) -> Signpost {
    let c = pos(e_mm, n_mm);
    signpost(
        id,
        EventKind::Enter,
        trigger(
            c,
            Shape::Circle {
                center: c,
                radius_m,
            },
        ),
    )
}

/// APPROACH route running `len_m` due north from the origin; the anchor is its end.
pub fn route(id: &str, len_m: u32, buffer_m: u32) -> Signpost {
    let vertices = vec![ORIGIN, pos(0, (i64::from(len_m) * 1000) as i32)];
    signpost(
        id,
        EventKind::Approach,
        trigger(
            ORIGIN,
            Shape::Route {
                vertices,
                buffer_m,
                anchor_along_m: None,
            },
        ),
    )
}

pub fn with_mode(mut sp: Signpost, mode: Mode) -> Signpost {
    sp.trigger.mode = Some(mode);
    sp
}

/// `count` fixes of a vehicle driving due north at `v_mmps`, one per second, accuracy 5 m, from
/// northing `n0_mm` at `t0_ms`, at easting `e_mm`. Doppler speed and course 0 are reported.
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

/// As [`drive_north`] but heading due south (course 180 degrees) from `n0_mm`.
pub fn drive_south(t0_ms: i64, n0_mm: i64, v_mmps: u32, count: usize, e_mm: i32) -> Vec<Fix> {
    (0..count)
        .map(|i| {
            let n = n0_mm - i64::from(v_mmps) * i as i64;
            fix_at(
                t0_ms + 1000 * i as i64,
                e_mm,
                i32::try_from(n).unwrap(),
                5000,
                Some(v_mmps),
                Some(1800),
            )
        })
        .collect()
}

/// A stationary fix (speed 0, no course) at an ENU offset.
pub fn still(t_ms: i64, e_mm: i32, n_mm: i32) -> Fix {
    fix_at(t_ms, e_mm, n_mm, 5000, Some(0), None)
}

/// A tiny deterministic generator (splitmix64) so property tests and the golden generator need no
/// randomness crate and every trace is reproducible from a seed.
#[derive(Clone, Debug)]
pub struct Splitmix(pub u64);

impl Splitmix {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (i128::from(hi) - i128::from(lo) + 1) as u128;
        let r = u128::from(self.next_u64()) % span;
        (i128::from(lo) + r as i128) as i64
    }

    pub fn chance(&mut self, percent: i64) -> bool {
        self.range(0, 99) < percent
    }
}

/// A square polygon ENTER signpost with half side `half_m`, centred at an ENU offset (mm).
pub fn square_at(id: &str, e_mm: i32, n_mm: i32, half_m: i32) -> Signpost {
    let h = half_m * 1000;
    let c = pos(e_mm, n_mm);
    let vertices = vec![
        pos(e_mm - h, n_mm - h),
        pos(e_mm + h, n_mm - h),
        pos(e_mm + h, n_mm + h),
        pos(e_mm - h, n_mm + h),
    ];
    signpost(
        id,
        EventKind::Enter,
        trigger(
            c,
            Shape::Polygon {
                vertices,
                inradius_m: None,
            },
        ),
    )
}

/// A varied signpost set around the origin: two circles, a square, two routes (vehicle, foot) and
/// a health circle. Used by the property tests; ids are distinct and sort in a fixed order.
pub fn world() -> Vec<Signpost> {
    let mut health = circle_at("h-1", 400_000, -300_000, 250);
    health.class = Class::Health;
    let mut r2 = with_mode(route("r-2", 2000, 60), Mode::Foot);
    r2.trigger.origin = pos(40_000, 0);
    r2.rank_milli = Some(1500);
    let mut c2 = circle_at("c-2", 0, 500_000, 300);
    c2.rank_milli = Some(700);
    vec![
        circle("c-1", 200),
        c2,
        square_at("s-1", 1_000_000, 0, 300),
        route("r-1", 2500, 30),
        r2,
        health,
    ]
}

/// A random walk of `n` fixes starting near the origin: speed 0..25 m/s, slowly turning,
/// sometimes without Doppler/course, accuracy 3..90 m, sampling 0.5..8 s, with the odd outlier.
pub fn random_walk(seed: u64, n: usize, t0_ms: i64) -> Vec<Fix> {
    let mut rng = Splitmix(seed);
    let mut e = rng.range(-1_500_000, 1_500_000);
    let mut north = rng.range(-1_500_000, 1_500_000);
    let mut heading = rng.range(0, 3599); // 0.1 degree
    let mut t = t0_ms;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let dt = rng.range(500, 8000);
        t += dt;
        heading = (heading + rng.range(-150, 150)).rem_euclid(3600);
        let v = rng.range(0, 25_000);
        // Move: 1 ddeg = 0.1 degree; use the engine's own table for direction.
        let u = proxim_geo::unit_from_course(heading as u16).unwrap();
        let step = v * dt / 1000;
        e += step * i64::from(u[0]) / (1 << 30);
        north += step * i64::from(u[1]) / (1 << 30);
        let mut fe = e;
        let mut fnn = north;
        if rng.chance(2) {
            fe += rng.range(-400_000, 400_000); // outlier
            fnn += rng.range(-400_000, 400_000);
        }
        let acc = rng.range(3_000, 90_000) as u32;
        let doppler = rng.chance(80);
        let mut f = fix_at(
            t,
            fe.clamp(-50_000_000, 50_000_000) as i32,
            fnn.clamp(-50_000_000, 50_000_000) as i32,
            acc,
            doppler.then_some(v as u32),
            (doppler && rng.chance(85)).then_some(heading as u16),
        );
        f.is_mock = rng.chance(1);
        out.push(f);
    }
    out
}

/// Fixes that visit a place: `n_inside` stationary fixes 4 s apart at the offset, then
/// `n_outside` stationary fixes 5 s apart 600 m east of it, the first one 65 s after the last
/// inside fix (a gap above 60 s, so that the move is not a `FIX_JUMP`). Three outside fixes span
/// 10 s and confirm the exit.
pub fn visit(t0_ms: i64, e_mm: i32, n_mm: i32, n_inside: usize, n_outside: usize) -> Vec<Fix> {
    let mut v = Vec::new();
    for i in 0..n_inside {
        v.push(still(t0_ms + 4000 * i as i64, e_mm, n_mm));
    }
    let t1 = t0_ms + 4000 * n_inside.saturating_sub(1) as i64 + 65_000;
    for i in 0..n_outside {
        v.push(still(t1 + 5000 * i as i64, e_mm + 600_000, n_mm));
    }
    v
}

/// A deterministic tour of [`world`]: drives the vehicle route, then visits the circle, the
/// square and the health circle, with two hours between legs. It produces fires with the
/// `granted_health` consent and exercises ENTER, EXIT and APPROACH.
pub fn tour(t0_ms: i64) -> Vec<Fix> {
    let hour = 3_600_000;
    let mut v = drive_north(t0_ms, 1_400_000, 15_000, 60, 0);
    v.extend(visit(t0_ms + 2 * hour, 0, 0, 5, 6));
    v.extend(visit(t0_ms + 4 * hour, 1_000_000, 0, 5, 6));
    v.extend(visit(t0_ms + 6 * hour, 400_000, -300_000, 5, 6));
    v
}

/// One ENTER cycle at the origin: with `release`, an extra fix at the centre first (it releases a
/// previous cool-down and is not counted), then 4 still fixes 4 s apart (the ENTER event is the
/// 4th), then, 65 s after the last inside fix (no `FIX_JUMP`), two fixes 600 m east 10 s apart
/// that confirm the exit (cool-down 30 min, so cycles must be at least about 1900 s apart).
pub fn enter_cycle(t0_ms: i64, release: bool) -> Vec<Fix> {
    let mut v = Vec::new();
    let mut t = t0_ms;
    if release {
        v.push(still(t, 0, 0));
        t += 4000;
    }
    for i in 0..4 {
        v.push(still(t + 4000 * i, 0, 0));
    }
    t += 12_000 + 65_000;
    v.push(still(t, 600_000, 0));
    v.push(still(t + 10_000, 600_000, 0));
    v
}
