//! System-level properties over random traces: totality, caps, pacing, privacy of the output and
//! state invariants. Every property is checked on traces that actually fire.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proptest::prelude::*;
use proxim_decide::decide;
use proxim_types::canon::to_canonical_json;
use proxim_types::{
    Caps, Clock, Consent, DetEntry, DetState, EngineError, Fix, Id, Mem, Outcome, Output, Policy,
    SignpostCaps, State, TraceFix,
};
use std::collections::BTreeMap;

/// A multi-day schedule of ENTER cycles at random offsets plus random walks in between.
fn schedule(seed: u64, days: i64) -> Vec<Fix> {
    let mut rng = Splitmix(seed);
    let mut out: Vec<Fix> = Vec::new();
    let mut t = T0 + rng.range(0, 6 * 3_600_000);
    let end = T0 + days * DAY;
    let mut first = true;
    while t < end {
        match rng.range(0, 9) {
            0..=5 => {
                out.extend(enter_cycle(t, !first));
                t = out.last().unwrap().t_ms;
            }
            6 => {
                out.extend(drive_north(t, 1_400_000, 15_000, 60, 0));
                t = out.last().unwrap().t_ms;
            }
            _ => {
                let w = random_walk(rng.next_u64(), 25, t);
                t = w.last().unwrap().t_ms;
                out.extend(w);
            }
        }
        first = false;
        // Gap: from nothing to a few hours, sometimes a whole day.
        let hi = if rng.chance(10) {
            30 * 3_600_000
        } else {
            2 * 3_600_000
        };
        t += rng.range(2_000, hi);
    }
    out
}

struct Run {
    outs: Vec<Output>,
    states: Vec<State>,
    fixes: Vec<Fix>,
    nows: Vec<i64>,
}

fn run_with(
    signposts: &[proxim_types::Signpost],
    policy: &Policy,
    consent: &Consent,
    tz: i16,
    fixes: Vec<Fix>,
    clock_skew: &dyn Fn(usize, i64) -> i64,
) -> Run {
    let mut state = State::empty();
    let mut r = Run {
        outs: vec![],
        states: vec![],
        fixes: vec![],
        nows: vec![],
    };
    for (i, f) in fixes.into_iter().enumerate() {
        let now = clock_skew(i, f.t_ms).max(0);
        let (o, s) = decide(
            &state,
            &f,
            signposts,
            policy,
            consent,
            &Clock {
                now_ms: now,
                tz_offset_min: tz,
            },
            0,
        )
        .unwrap();
        r.outs.push(o);
        r.states.push(s.clone());
        r.fixes.push(f);
        r.nows.push(now);
        state = s;
    }
    r
}

fn check_state_invariants(r: &Run) {
    let mut prev_hwm = 0;
    for s in &r.states {
        assert_eq!(s.v, 1);
        assert!(s.mem.trace.len() <= 16);
        assert!(s.mem.trace.windows(2).all(|w| w[0].t_ms < w[1].t_ms));
        assert!(
            s.mem.det.windows(2).all(|w| w[0].id < w[1].id),
            "det sorted, unique"
        );
        assert!(
            s.caps.signposts.windows(2).all(|w| w[0].id < w[1].id),
            "caps sorted, unique"
        );
        assert!(s.mem.jump_rejects < 3);
        assert!(s.caps.hwm_ms >= prev_hwm, "hwm is monotone");
        prev_hwm = s.caps.hwm_ms;
        if let Some(l) = s.caps.last_fire_ms {
            assert!(l <= s.caps.hwm_ms);
        }
        for e in &s.caps.signposts {
            assert!(e.day_n <= 2 && e.week_n <= 5, "{e:?}");
            assert!(e.week_n >= e.day_n.min(1));
        }
        // The state is accepted back by decide (round trip through canonical text).
        let text = to_canonical_json(s).unwrap();
        let _: State = proxim_types::strict::from_canonical_json(text.as_bytes()).unwrap();
    }
}

/// Local day and week of an instant (the independent restatement of spec 7).
fn day_week(now: i64, tz: i16) -> (i64, i64) {
    let local = now + i64::from(tz) * 60_000;
    let day = local.div_euclid(DAY);
    (day, (day + 3).div_euclid(7))
}

fn check_caps_and_pacing(r: &Run, tz: i16) {
    let mut per_day: BTreeMap<(String, i64), u32> = BTreeMap::new();
    let mut per_week: BTreeMap<(String, i64), u32> = BTreeMap::new();
    let mut last_fire: Option<i64> = None;
    for (o, now) in r.outs.iter().zip(&r.nows) {
        let f: Vec<_> = o
            .decisions
            .iter()
            .filter(|d| d.outcome == Outcome::Fire)
            .collect();
        assert!(f.len() <= 1, "one slot");
        if let Some(d) = f.first() {
            assert!(o.fix.ok);
            let (day, week) = day_week(o.t_ms, tz);
            let id = d.signpost.to_string();
            let a = per_day.entry((id.clone(), day)).or_default();
            *a += 1;
            assert!(*a <= 2, "more than 2 fires of {id} on local day {day}");
            let b = per_week.entry((id.clone(), week)).or_default();
            *b += 1;
            assert!(*b <= 5, "more than 5 fires of {id} in local week {week}");
            if let Some(l) = last_fire {
                assert!(o.t_ms - l >= 1_800_000, "two fires {} ms apart", o.t_ms - l);
            }
            last_fire = Some(o.t_ms);
            assert!(*now <= o.t_ms);
        }
    }
}

fn fire_total(r: &Run) -> usize {
    r.outs.iter().map(|o| fires(o).len()).sum()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 24, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn prop_caps_pacing_and_state_invariants_hold_on_multi_day_traces(
        seed in any::<u64>(),
        tz in -720i16..=840,
    ) {
        let fixes = schedule(seed, 9);
        let r = run_with(&world(), &explain(), &granted_health(), tz, fixes, &|_, t| t);
        check_state_invariants(&r);
        check_caps_and_pacing(&r, tz);
    }

    #[test]
    fn prop_the_clock_may_lag_or_jump_without_breaking_any_invariant(
        seed in any::<u64>(),
        skew_seed in any::<u64>(),
    ) {
        // `now` wanders +-2 days around the fix time; fixes keep honest times. Nothing may panic,
        // the high-water mark never decreases, caps never exceed their ceilings, and no fire
        // happens within 30 minutes of another measured on the clamped `now`.
        let fixes = schedule(seed, 6);
        let skew = move |i: usize, t: i64| {
            let mut g = Splitmix(skew_seed ^ (i as u64).wrapping_mul(0x9E37_79B9));
            if g.chance(30) { t + g.range(-2 * DAY, 2 * DAY) } else { t }
        };
        let r = run_with(&world(), &explain(), &granted_health(), 60, fixes, &skew);
        check_state_invariants(&r);
        check_caps_and_pacing(&r, 60);
    }

    #[test]
    fn prop_no_consent_no_fire_no_retained_memory(seed in any::<u64>(), which in 0u8..4) {
        let consent = match which {
            0 => Consent::default(),
            1 => Consent { location_matching: proxim_types::ConsentValue::Denied, ..granted_health() },
            2 => Consent { location_matching: proxim_types::ConsentValue::Unknown, ..granted_health() },
            _ => granted_health(),
        };
        let fixes = schedule(seed, 2);
        let r = run_with(&world(), &explain(), &consent, 0, fixes, &|_, t| t);
        if which < 3 {
            prop_assert_eq!(fire_total(&r), 0);
            for (o, s) in r.outs.iter().zip(&r.states) {
                prop_assert!(!o.fix.ok);
                prop_assert_eq!(&s.mem, &Mem::default());
                prop_assert!(s.caps.signposts.is_empty() && s.caps.last_fire_ms.is_none());
            }
        } else {
            prop_assert!(fire_total(&r) > 0 || r.fixes.len() < 30);
        }
    }

    #[test]
    fn prop_output_never_contains_a_position(seed in any::<u64>()) {
        let fixes = schedule(seed, 2);
        let r = run_with(&world(), &explain(), &granted_health(), 0, fixes, &|_, t| t);
        for (o, f) in r.outs.iter().zip(&r.fixes) {
            let text = to_canonical_json(o).unwrap();
            prop_assert!(!text.contains("lat") && !text.contains("lon"));
            for v in [f.lat_e7, f.lon_e7] {
                if v.unsigned_abs() >= 1_000_000 {
                    prop_assert!(!text.contains(&v.to_string()), "{text}");
                }
            }
        }
    }

    #[test]
    fn prop_nothing_fires_inside_a_standoff_and_nothing_is_kept_there(seed in any::<u64>()) {
        use proxim_types::StandoffZone;
        // A zone over the circle and the square of `world()`.
        let policy = Policy {
            standoff_zones: vec![StandoffZone { id: Id::new("z").unwrap(), center: pos(500_000, 0), standoff_m: Some(1500) }],
            ..explain()
        };
        let fixes = schedule(seed, 3);
        let mut state = State::empty();
        for f in fixes {
            let before = to_canonical_json(&state.caps).unwrap();
            let (o, s) = decide(&state, &f, &world(), &policy, &granted_health(), &clock(f.t_ms), 0).unwrap();
            if o.fix.reason == proxim_types::ReasonCode::SensitiveStandoff {
                prop_assert!(o.decisions.is_empty() && o.notes.is_none());
                prop_assert_eq!(&s.mem, &Mem::default());
                prop_assert_eq!(to_canonical_json(&s.caps).unwrap(), before);
            }
            state = s;
        }
    }
}

// ---- totality on hostile (but well-formed) inputs ----

fn hostile_state(rng: &mut Splitmix) -> State {
    let wild = |rng: &mut Splitmix| -> i64 {
        match rng.range(0, 5) {
            0 => i64::MAX,
            1 => i64::MAX - rng.range(0, 1000),
            2 => 0,
            3 => rng.range(0, T0 * 2),
            _ => rng.range(0, i64::MAX),
        }
    };
    let n_trace = rng.range(0, 16) as usize;
    let mut t = wild(rng).min(i64::MAX - 100_000);
    let mut trace = Vec::new();
    for _ in 0..n_trace {
        t += rng.range(1, 5000);
        trace.push(TraceFix {
            t_ms: t,
            lat_e7: rng.range(-900_000_000, 900_000_000) as i32,
            lon_e7: rng.range(-1_800_000_000, 1_800_000_000) as i32,
            acc_mm: rng.range(0, i64::from(u32::MAX)) as u32,
            v_mmps: rng
                .chance(50)
                .then(|| rng.range(0, i64::from(u32::MAX)) as u32),
        });
    }
    let mut ids: Vec<String> = (0..rng.range(0, 6))
        .map(|i| format!("{}-{i}", ["c", "r", "s"][rng.range(0, 2) as usize]))
        .collect();
    ids.sort();
    ids.dedup();
    let st = |rng: &mut Splitmix| -> DetState {
        match rng.range(0, 4) {
            0 => DetState::Cand {
                n: rng.range(0, 99) as u32,
                first_t: wild(rng),
                last_t: wild(rng),
            },
            1 => DetState::In {
                xn: rng.range(0, 99) as u32,
                x_first_t: rng.chance(50).then(|| wild(rng)),
                last_t: wild(rng),
            },
            2 => DetState::Cool {
                until_ms: wild(rng),
            },
            3 => DetState::Armed {
                confirm_t: rng.chance(50).then(|| wild(rng)),
                a_prev_mm: rng
                    .chance(50)
                    .then(|| rng.range(i64::MIN / 2, i64::MAX / 2)),
                last_t: wild(rng),
            },
            _ => DetState::Latched {
                why: proxim_types::LatchWhy::Fired,
                last_t: wild(rng),
            },
        }
    };
    let det: Vec<DetEntry> = ids
        .iter()
        .map(|id| DetEntry {
            id: Id::new(id).unwrap(),
            rev: rng.range(0, 3) as u32,
            st: st(rng),
        })
        .collect();
    let caps: Vec<SignpostCaps> = ids
        .iter()
        .map(|id| SignpostCaps {
            id: Id::new(id).unwrap(),
            day_idx: wild(rng),
            day_n: rng.range(0, 5) as u32,
            week_idx: wild(rng),
            week_n: rng.range(0, 9) as u32,
            tz_offset_min: None,
        })
        .collect();
    State {
        v: 1,
        caps: Caps {
            hwm_ms: wild(rng),
            last_fire_ms: rng.chance(50).then(|| wild(rng)),
            signposts: caps,
        },
        mem: Mem {
            last_t_ms: trace
                .last()
                .map(|t| t.t_ms)
                .or_else(|| rng.chance(30).then(|| wild(rng))),
            jump_rejects: rng.range(0, 2) as u8,
            trace,
            vf_mmps: rng
                .chance(50)
                .then(|| rng.range(0, i64::from(u32::MAX)) as u32),
            h: rng.chance(50).then(|| {
                [
                    rng.range(-(1 << 30), 1 << 30) as i32,
                    rng.range(-(1 << 30), 1 << 30) as i32,
                ]
            }),
            h_t_ms: rng.chance(50).then(|| wild(rng)),
            det,
        },
    }
}

fn hostile_fix(rng: &mut Splitmix) -> Fix {
    let big = |rng: &mut Splitmix| match rng.range(0, 3) {
        0 => i64::MAX,
        1 => rng.range(0, i64::MAX),
        _ => rng.range(0, T0 * 2),
    };
    Fix {
        t_ms: big(rng),
        lat_e7: match rng.range(0, 3) {
            0 => i32::MAX,
            1 => i32::MIN,
            _ => rng.range(-900_000_000, 900_000_000) as i32,
        },
        lon_e7: match rng.range(0, 3) {
            0 => i32::MAX,
            1 => i32::MIN,
            _ => rng.range(-1_800_000_000, 1_800_000_000) as i32,
        },
        acc_mm: rng.range(0, i64::from(u32::MAX)) as u32,
        speed_mmps: rng
            .chance(50)
            .then(|| rng.range(0, i64::from(u32::MAX)) as u32),
        speed_acc_mmps: rng
            .chance(30)
            .then(|| rng.range(0, i64::from(u32::MAX)) as u32),
        course_ddeg: rng.chance(50).then(|| rng.range(0, 4000) as u16),
        course_acc_ddeg: rng.chance(30).then(|| rng.range(0, 5000) as u16),
        is_mock: rng.chance(10),
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 600, failure_persistence: None, ..ProptestConfig::default() })]

    /// Hostile but structurally valid state, fix and clock: never a panic, only Ok or a typed
    /// error, and an Ok state is itself acceptable input again.
    #[test]
    fn prop_decide_is_total_on_hostile_inputs(seed in any::<u64>()) {
        let mut rng = Splitmix(seed);
        let state = hostile_state(&mut rng);
        let fix = hostile_fix(&mut rng);
        let clock = Clock {
            now_ms: match rng.range(0, 3) { 0 => i64::MAX, 1 => 0, _ => rng.range(0, i64::MAX) },
            tz_offset_min: rng.range(-1000, 1000) as i16,
        };
        let policy = Policy {
            explain: Some(true),
            simulator_mode: Some(rng.chance(50)),
            max_acc_mm: rng.chance(50).then(|| rng.range(0, i64::from(u32::MAX)) as u32),
            lead_s_min: rng.chance(30).then(|| rng.range(0, 500) as u32),
            lead_s_max: rng.chance(30).then(|| rng.range(0, 500) as u32),
            slots: rng.chance(30).then(|| rng.range(0, 255) as u8),
            ..Policy::default()
        };
        match decide(&state, &fix, &world(), &policy, &granted_health(), &clock, rng.next_u64()) {
            Ok((o, s)) => {
                prop_assert!(o.v == 1 && s.v == 1);
                let text = to_canonical_json(&s).unwrap();
                let back: Result<State, EngineError> = proxim_types::strict::from_canonical_json(text.as_bytes());
                prop_assert!(back.is_ok());
            }
            Err(e) => prop_assert!(matches!(e, EngineError::Parse | EngineError::Limit | EngineError::Overflow
                | EngineError::OutOfFrame | EngineError::UnsupportedLatitude | EngineError::DegenerateGeometry)),
        }
    }
}
