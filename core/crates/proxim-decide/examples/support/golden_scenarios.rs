//! The golden decide scenarios (engine-spec 9). Inputs only: expectations are produced by running
//! `decide`, so a vector changes bytes only when behaviour changes. Synthetic data (invariant I2).
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use super::fixtures::*;
use proxim_decide::golden::{Scenario, StepInput};
use proxim_types::{
    Class, Clock, Consent, ConsentValue, EventKind, Fix, Id, Policy, Position, Shape, StandoffZone,
};

fn scenario(
    id: &str,
    signposts: Vec<proxim_types::Signpost>,
    policy: Policy,
    consent: Consent,
    steps: Vec<StepInput>,
) -> Scenario {
    Scenario {
        id: id.into(),
        seed: 0,
        policy,
        consent,
        signposts,
        steps,
    }
}

/// Steps whose clock equals the fix time and whose timezone is `tz`.
fn steps(fixes: &[Fix], tz: i16) -> Vec<StepInput> {
    fixes
        .iter()
        .map(|f| StepInput {
            clock: clock_tz(f.t_ms, tz),
            fix: *f,
        })
        .collect()
}

fn with_clocks(fixes: &[Fix], clocks: &[i64]) -> Vec<StepInput> {
    fixes
        .iter()
        .zip(clocks)
        .map(|(f, c)| StepInput {
            clock: Clock {
                now_ms: *c,
                tz_offset_min: 0,
            },
            fix: *f,
        })
        .collect()
}

fn quiet() -> Policy {
    Policy::default()
}

pub fn scenarios() -> Vec<Scenario> {
    let mut v = Vec::new();

    // 1. Drive into a circle: ENTER fires on the 4th inside fix, exit hysteresis, cool-down.
    let fixes: Vec<Fix> = (0..60)
        .map(|i| {
            fix_at(
                T0 + 1000 * i,
                0,
                -500_000 + 20_000 * i as i32,
                5000,
                Some(20_000),
                Some(0),
            )
        })
        .collect();
    v.push(scenario(
        "drive_in_enter",
        vec![circle("c-1", 200)],
        explain(),
        granted(),
        steps(&fixes, 0),
    ));

    // 2. Approach a route anchor at 15 m/s: one fire (time to anchor), none after the anchor.
    let fixes = drive_north(T0, 1_400_000, 15_000, 85, 0);
    v.push(scenario(
        "approach_fire_on_route",
        vec![route("r-1", 2500, 30)],
        explain(),
        granted(),
        steps(&fixes, 0),
    ));

    // 3. Drive past: a road 600 m east of both a circle and a route axis. No event.
    let fixes = drive_north(T0, -400_000, 20_000, 50, 600_000);
    v.push(scenario(
        "drive_past_parallel_road",
        vec![circle("c-1", 200), route("r-1", 2500, 30)],
        explain(),
        granted(),
        steps(&fixes, 0),
    ));

    // 4. Opposite carriageway: 8 m east of the axis, heading south. Never fires.
    let fixes = drive_south(T0, 1_500_000, 15_000, 55, 8_000);
    v.push(scenario(
        "opposite_carriageway",
        vec![route("r-1", 2500, 30)],
        explain(),
        granted(),
        steps(&fixes, 0),
    ));

    // 5. U-turn at 1000 m from the anchor, outside D_fire (645 m): no fire.
    let mut fixes = drive_north(T0, 700_000, 15_000, 21, 0); // up to n = 1000 m
    fixes.extend(drive_south(T0 + 21_000, 1_000_000 - 15_000, 15_000, 30, 0));
    v.push(scenario(
        "u_turn_before_anchor",
        vec![route("r-1", 2500, 30)],
        explain(),
        granted(),
        steps(&fixes, 0),
    ));

    // 6. Standoff: a zone 1300 m west of a circle. Fixes inside the zone read SUPPRESSED (no
    // explain), memory is wiped; once clear the circle works normally.
    let zone = StandoffZone {
        id: Id::new("z-1").unwrap(),
        center: pos(-1_300_000, 0),
        standoff_m: None,
    };
    let fixes: Vec<Fix> = (0..70)
        .map(|i| {
            fix_at(
                T0 + 1000 * i,
                -1_700_000 + 30_000 * i as i32,
                0,
                5000,
                Some(30_000),
                Some(900),
            )
        })
        .collect();
    v.push(scenario(
        "standoff_suppressed",
        vec![circle("c-1", 200)],
        Policy {
            standoff_zones: vec![zone.clone()],
            ..quiet()
        },
        granted(),
        steps(&fixes, 0),
    ));

    // 6b. The same drive with explain: the reason is SENSITIVE_STANDOFF, still no decisions.
    v.push(scenario(
        "standoff_explained",
        vec![circle("c-1", 200)],
        Policy {
            explain: Some(true),
            standoff_zones: vec![zone],
            ..quiet()
        },
        granted(),
        steps(&fixes[..20], 0),
    ));

    // 7. Consent denied / unknown / child-directed.
    let fixes = visit(T0, 0, 0, 6, 0);
    v.push(scenario(
        "consent_denied",
        vec![circle("c-1", 200)],
        explain(),
        Consent {
            location_matching: ConsentValue::Denied,
            nearby_health_events: ConsentValue::Granted,
            aggregate_reporting: ConsentValue::Granted,
            local_notifications: ConsentValue::Granted,
            health_receipt: Some("receipt-test-1".into()),
        },
        steps(&fixes, 0),
    ));
    v.push(scenario(
        "consent_unknown",
        vec![circle("c-1", 200)],
        explain(),
        Consent::default(),
        steps(&fixes, 0),
    ));
    v.push(scenario(
        "child_directed_disabled",
        vec![circle("c-1", 200)],
        Policy {
            child_directed: Some(true),
            ..explain()
        },
        granted(),
        steps(&fixes[..4], 0),
    ));

    // 8. Cap exhaustion: two fires on a local day (UTC+2), the third is CAP_SIGNPOST_DAY, the
    // next local day fires again. Visits 1950 s apart (cool-down and the 30 min app interval).
    let spacing = 1_950_000;
    let mut fixes = Vec::new();
    for k in 0..3 {
        fixes.extend(enter_cycle(T0 + spacing * k, k > 0));
    }
    fixes.extend(enter_cycle(T0 + DAY, true));
    v.push(scenario(
        "cap_exhaustion",
        vec![circle("c-1", 200)],
        explain(),
        granted(),
        steps(&fixes, 120),
    ));

    // 9. A clock that goes backwards: the persisted high-water mark clamps `now`. Fixes keep their
    // true times; while the clock lags, `now` stays at the mark, fixes more than 5 s ahead of it
    // are FIX_FROM_FUTURE, and nothing reopens. Then the clock recovers.
    let fixes = enter_cycle(T0, false);
    let mut all = fixes.clone();
    let mut clocks: Vec<i64> = fixes.iter().map(|f| f.t_ms).collect();
    // A second cycle 1950 s later with the clock stuck one hour and then one day behind.
    let second = enter_cycle(T0 + 1_950_000, true);
    for (i, f) in second.iter().enumerate() {
        all.push(*f);
        clocks.push(match i {
            0 | 1 => f.t_ms - 3_600_000,
            2 | 3 => f.t_ms - DAY,
            _ => f.t_ms,
        });
    }
    v.push(scenario(
        "clock_goes_backwards",
        vec![circle("c-1", 200)],
        explain(),
        granted(),
        with_clocks(&all, &clocks),
    ));

    // 10. Malformed and implausible fixes, one reason each, then a jump that resets.
    let good = |i: i64| still(T0 + 4000 * i, 0, 0);
    let mut bad: Vec<Fix> = vec![good(0), good(1)];
    bad.push(Fix {
        lat_e7: 900_000_001,
        ..good(2)
    });
    bad.push(Fix {
        lon_e7: -1_800_000_001,
        ..good(2)
    });
    bad.push(Fix {
        course_ddeg: Some(3600),
        ..good(2)
    });
    bad.push(Fix {
        is_mock: true,
        ..good(2)
    });
    bad.push(Fix {
        acc_mm: 100_001,
        ..good(2)
    });
    bad.push(Fix {
        t_ms: T0 + 4000 * 2 + 60_000,
        ..good(2)
    }); // future (clock lags by 60 s)
    bad.push(good(0)); // out of order
    bad.push(Fix {
        t_ms: T0 - 100_000,
        ..good(2)
    }); // stale
    bad.push(good(3));
    // Three implausible jumps in a row: reject, reject, reset-and-accept.
    bad.push(still(T0 + 4000 * 4, 3_000_000, 0));
    bad.push(still(T0 + 4000 * 5, 3_000_000, 0));
    bad.push(still(T0 + 4000 * 6, 3_000_000, 0));
    bad.push(still(T0 + 4000 * 7, 3_000_000, 0));
    let clocks: Vec<i64> = bad
        .iter()
        .enumerate()
        .map(|(i, f)| {
            if i == 7 {
                T0 + 8000
            } else {
                f.t_ms.max(T0 + 4000 * 2)
            }
        })
        .collect();
    v.push(scenario(
        "malformed_fixes",
        vec![circle("c-1", 200)],
        explain(),
        granted(),
        with_clocks(&bad, &clocks),
    ));

    // 11. Two ENTER signposts complete on the same fix: the higher score wins, the other is
    // LOST_SLOT, reverts, and is then held back by the app interval.
    let mut a = circle("a-1", 200);
    a.rank_milli = Some(1000);
    let mut b = circle("b-1", 200);
    b.rank_milli = Some(2000);
    let fixes = visit(T0, 0, 0, 7, 0);
    v.push(scenario(
        "two_signposts_one_slot",
        vec![a, b],
        explain(),
        granted(),
        steps(&fixes, 0),
    ));

    // 12. Reserved events fail closed.
    let mk = |id: &str, ev: EventKind| {
        let mut sp = circle(id, 200);
        sp.event = ev;
        sp
    };
    let fixes = visit(T0, 0, 0, 5, 0);
    v.push(scenario(
        "unsupported_events",
        vec![
            circle("ok-1", 200),
            mk("u-cross", EventKind::Cross),
            mk("u-dwell", EventKind::Dwell),
            mk("u-exit", EventKind::Exit),
            mk("u-pass", EventKind::Pass),
            mk("u-seq", EventKind::Sequence),
        ],
        explain(),
        granted(),
        steps(&fixes, 0),
    ));

    // 13. First armed fix too close to give the lead (e.g. after a GPS drop): TOO_LATE_FOR_LEAD.
    let fixes = drive_north(T0, 2_240_000, 15_000, 20, 0);
    v.push(scenario(
        "late_join_too_late",
        vec![route("r-1", 2500, 30)],
        explain(),
        granted(),
        steps(&fixes, 0),
    ));

    // 14. Health class: no consent receipt -> never; and health geometry inside a standoff.
    let mut h = circle("h-1", 200);
    h.class = Class::Health;
    let mut h2 = circle_at("h-2", 3_000_000, 0, 200);
    h2.class = Class::Health;
    let near_zone = StandoffZone {
        id: Id::new("z-2").unwrap(),
        center: Position {
            ..pos(3_500_000, 0)
        },
        standoff_m: None,
    };
    let fixes = visit(T0, 0, 0, 5, 0);
    v.push(scenario(
        "health_gates",
        vec![h.clone(), h2.clone()],
        Policy {
            standoff_zones: vec![near_zone.clone()],
            ..health_partner()
        },
        granted(),
        steps(&fixes[..3], 0),
    ));
    v.push(scenario(
        "health_with_receipt",
        vec![h, h2],
        Policy {
            standoff_zones: vec![near_zone],
            ..health_partner()
        },
        granted_health(),
        steps(&fixes, 0),
    ));

    // 15. Boundary jitter: a stationary device 5 m inside and outside the edge of a polygon never
    // ENTERs (its accuracy circle always straddles the boundary).
    let sq = square_at("s-1", 0, 0, 300);
    let fixes: Vec<Fix> = (0..24)
        .map(|i| {
            let n = if i % 2 == 0 { 295_000 } else { 305_000 }; // 5 m inside / outside the north edge
            still(T0 + 4000 * i, 0, n)
        })
        .collect();
    v.push(scenario(
        "boundary_jitter_never_enters",
        vec![sq, polygon_wide_check()],
        explain(),
        granted(),
        steps(&fixes, 0),
    ));

    // 15b. An event read late: the clock lags the fix by 5 s at the 4th fix and the trigger allows
    // 2 s of event age, so the event is consumed as EVENT_TOO_OLD and the detector is `in`.
    let mut aged = circle("c-1", 200);
    aged.trigger.max_event_age_s = Some(2);
    let fixes = visit(T0, 0, 0, 7, 0);
    let clocks: Vec<i64> = fixes
        .iter()
        .enumerate()
        .map(|(i, f)| if i == 3 { f.t_ms + 5000 } else { f.t_ms })
        .collect();
    v.push(scenario(
        "aged_clock_event_too_old",
        vec![aged],
        explain(),
        granted(),
        with_clocks(&fixes, &clocks),
    ));

    // 15c. Revisions: the highest revision of an id is processed, lower ones are superseded and an
    // id with two copies of its top revision is ambiguous (neither is processed).
    let mut r1 = circle("a-1", 200);
    r1.rev = 1;
    let mut r2 = circle("a-1", 200);
    r2.rev = 2;
    let d1 = circle("d-1", 200);
    let d2 = circle("d-1", 250);
    let fixes = visit(T0, 0, 0, 5, 0);
    v.push(scenario(
        "superseded_and_ambiguous_revisions",
        vec![d2, r2, d1, r1],
        explain(),
        granted(),
        steps(&fixes, 0),
    ));

    // 16. A tour of the whole synthetic world (ENTER, EXIT, APPROACH, polygon, health).
    let fixes = tour(T0);
    v.push(scenario(
        "world_tour",
        world(),
        health_partner(),
        granted_health(),
        steps(&fixes, 0),
    ));

    v
}

/// A second, far-away polygon so that the jitter scenario also exercises a signpost out of reach.
fn polygon_wide_check() -> proxim_types::Signpost {
    let mut sp = square_at("s-2", 40_000_000, 0, 300);
    if let Shape::Polygon { inradius_m, .. } = &mut sp.trigger.shape {
        *inradius_m = Some(300);
    }
    sp
}
