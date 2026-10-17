//! Engine spec 7.6 (engine_spec 2, M3 slice 5): the driving-safe format, dayparts and quiet hours.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_types::{Driving, Format, Outcome, Policy, ReasonCode, Signpost, Window};

const HOUR: i64 = 3_600_000;
/// `T0` is Sunday 2026-10-11 08:00 UTC; this is 00:00 UTC that day.
const SUN_MIDNIGHT: i64 = T0 - 8 * HOUR;

fn drive_fires(policy: Policy) -> (Vec<Option<Format>>, Vec<ReasonCode>) {
    let mut sim = Sim::new(vec![route("r", 2500, 30)]);
    sim.policy = policy;
    let outs = sim.run(&drive_north(T0, 1_400_000, 15_000, 40, 0));
    let fires = outs
        .iter()
        .flat_map(|o| o.decisions.iter())
        .filter(|d| d.outcome == Outcome::Fire)
        .map(|d| d.format)
        .collect();
    let drops = outs.iter().flat_map(drop_reasons).map(|(_, r)| r).collect();
    (fires, drops)
}

#[test]
fn driving_shows_the_glanceable_only_and_a_walk_shows_the_full_card() {
    assert_eq!(drive_fires(explain()).0, [Some(Format::Glanceable)]);
    let mut sim = Sim::new(vec![circle("c", 200)]);
    let outs = sim.run(&enter_cycle(T0, false));
    let fire = outs
        .iter()
        .flat_map(|o| o.decisions.iter())
        .find(|d| d.outcome == Outcome::Fire)
        .unwrap();
    assert_eq!(fire.format, None, "not moving: the full card");
}

#[test]
fn a_partner_can_only_make_driving_stricter() {
    let with = |d| Policy {
        driving: Some(d),
        ..explain()
    };
    assert_eq!(drive_fires(with(Driving::Inbox)).0, [Some(Format::Inbox)]);
    let (fires, drops) = drive_fires(with(Driving::Off));
    assert!(fires.is_empty());
    assert!(
        drops.contains(&ReasonCode::DrivingUnsafeFormat),
        "{drops:?}"
    );
}

fn at(sp: Signpost, policy: Policy, t0: i64, tz: i16) -> (bool, Vec<ReasonCode>) {
    let mut sim = Sim::new(vec![sp]);
    sim.policy = policy;
    sim.tz = tz;
    let outs = sim.run(&enter_cycle(t0, false));
    let fired = outs.iter().any(|o| !fires(o).is_empty());
    (
        fired,
        outs.iter().flat_map(drop_reasons).map(|(_, r)| r).collect(),
    )
}

fn lunch(days: u8) -> Signpost {
    let mut sp = circle("c", 200);
    sp.dayparts = Some(vec![Window {
        days,
        start_min: 690,
        end_min: 840,
    }]);
    sp
}

#[test]
fn dayparts_follow_local_time_and_weekday() {
    // 11:30-14:00 every day: 08:00 is outside, 12:00 inside.
    let (fired, drops) = at(lunch(0), explain(), T0, 0);
    assert!(
        !fired && drops.contains(&ReasonCode::OutsideDaypart),
        "{drops:?}"
    );
    assert!(at(lunch(0), explain(), SUN_MIDNIGHT + 12 * HOUR, 0).0);
    // The same instant in UTC+4 is 12:00 local: inside.
    assert!(at(lunch(0), explain(), T0, 240).0);
    // Weekdays only: Sunday noon is outside, Monday noon inside.
    assert!(!at(lunch(0b001_1111), explain(), SUN_MIDNIGHT + 12 * HOUR, 0).0);
    assert!(at(lunch(0b001_1111), explain(), SUN_MIDNIGHT + 36 * HOUR, 0).0);
}

#[test]
fn quiet_hours_hold_every_advert() {
    let quiet = Policy {
        quiet_hours: vec![Window {
            days: 0,
            start_min: 1260,
            end_min: 480,
        }],
        ..explain()
    };
    let (fired, drops) = at(circle("c", 200), quiet.clone(), SUN_MIDNIGHT + 22 * HOUR, 0);
    assert!(
        !fired && drops.contains(&ReasonCode::QuietHours),
        "{drops:?}"
    );
    assert!(
        !at(circle("c", 200), quiet.clone(), SUN_MIDNIGHT + 7 * HOUR, 0).0,
        "07:00 is still quiet"
    );
    assert!(at(circle("c", 200), quiet, SUN_MIDNIGHT + 9 * HOUR, 0).0);
}

#[test]
fn malformed_windows_fail_closed() {
    let mut empty = circle("c", 200);
    empty.dayparts = Some(vec![]);
    let (fired, drops) = at(empty, explain(), T0, 0);
    assert!(!fired && drops.contains(&ReasonCode::TriggerInvalid));
    let mut bad = circle("c", 200);
    bad.dayparts = Some(vec![Window {
        days: 0,
        start_min: 0,
        end_min: 1500,
    }]);
    assert!(
        at(bad, explain(), T0, 0)
            .1
            .contains(&ReasonCode::TriggerInvalid)
    );
    // Bad quiet hours are a policy error: no decision at all.
    let policy = Policy {
        quiet_hours: vec![Window {
            days: 0x80,
            start_min: 0,
            end_min: 1,
        }],
        ..explain()
    };
    let st = proxim_types::State::empty();
    assert!(
        proxim_decide::decide(
            &st,
            &still(T0, 0, 0),
            &[],
            &policy,
            &granted(),
            &clock(T0),
            0
        )
        .is_err()
    );
}
