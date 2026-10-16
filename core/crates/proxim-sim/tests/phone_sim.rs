//! Phone simulator: rules every delivery must obey, the scenario's key outcomes, and the
//! checked-in report (`core/testdata/sim/phone-simulator-results.md`).
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

use proxim_sim::phone::{MissKind, Routing, RunResult, World, duplicates, fit, run};
use proxim_sim::{report, town};
use proxim_types::{Class, ReasonCode};
use std::collections::BTreeMap;
use std::path::PathBuf;

const MIN_MS: i64 = 60_000;

fn world() -> World {
    town::world().unwrap()
}

fn runs(w: &World) -> [RunResult; 2] {
    [Routing::Broad, Routing::BestFit].map(|r| run(w, r).unwrap())
}

fn app(w: &World, key: &str) -> usize {
    w.apps.iter().position(|a| a.key == key).unwrap()
}

fn phone(w: &World, name: &str) -> usize {
    w.phones.iter().position(|p| p.name == name).unwrap()
}

fn advert(w: &World, id: &str) -> usize {
    w.adverts
        .iter()
        .position(|a| a.signpost.id.as_str() == id)
        .unwrap()
}

fn shown(r: &RunResult, w: &World, phone_name: &str) -> Vec<(String, String)> {
    let p = phone(w, phone_name);
    r.deliveries
        .iter()
        .filter(|d| d.phone == p)
        .map(|d| {
            (
                w.apps[d.app].key.to_string(),
                w.adverts[d.advert].signpost.id.as_str().to_string(),
            )
        })
        .collect()
}

#[test]
fn every_delivery_obeys_the_apps_policy_consent_and_pacing() {
    let w = world();
    for r in runs(&w) {
        assert!(!r.deliveries.is_empty());
        for d in &r.deliveries {
            let a = &w.apps[d.app];
            let ad = &w.adverts[d.advert];
            let inst = w.phones[d.phone]
                .apps
                .iter()
                .find(|i| i.app == d.app)
                .expect("delivered by an installed app");
            let what = format!(
                "{:?}: {} via {}",
                r.routing,
                ad.signpost.id.as_str(),
                a.name
            );
            assert!(r.routed[d.app].contains(&d.advert), "not routed: {what}");
            assert!(fit(&a.policy, &ad.signpost.category).is_some(), "{what}");
            assert!(inst.location, "no location consent: {what}");
            assert_ne!(a.policy.child_directed, Some(true), "{what}");
            if ad.signpost.class == Class::Health {
                assert!(inst.health, "health advert without health consent: {what}");
            }
        }
        // Pacing per app on a phone: 30 min between adverts, an advert at most twice a day.
        let mut last: BTreeMap<(usize, usize), i64> = BTreeMap::new();
        let mut per_ad: BTreeMap<(usize, usize, usize), u32> = BTreeMap::new();
        for d in &r.deliveries {
            if let Some(prev) = last.insert((d.phone, d.app), d.t_ms) {
                assert!(
                    d.t_ms - prev >= 30 * MIN_MS,
                    "{:?} paced too close",
                    r.routing
                );
            }
            let n = per_ad.entry((d.phone, d.app, d.advert)).or_default();
            *n += 1;
            assert!(*n <= 2);
        }
    }
}

#[test]
fn best_fit_sends_each_advert_only_to_its_highest_affinity_apps() {
    let w = world();
    let r = run(&w, Routing::BestFit).unwrap();
    for (ai, ad) in w.adverts.iter().enumerate() {
        let fits: Vec<Option<u16>> = w
            .apps
            .iter()
            .map(|a| fit(&a.policy, &ad.signpost.category))
            .collect();
        let best = fits.iter().flatten().max().copied();
        for (pi, f) in fits.iter().enumerate() {
            let routed = r.routed[pi].contains(&ai);
            assert_eq!(
                routed,
                f.is_some() && *f == best,
                "{}",
                ad.signpost.id.as_str()
            );
        }
    }
    // A duplicate under best fit can only come from apps tied on the advert's best weight.
    for (_, ai, apps) in duplicates(&r) {
        let cat = &w.adverts[ai].signpost.category;
        let weights: Vec<_> = apps.iter().map(|&a| fit(&w.apps[a].policy, cat)).collect();
        assert!(weights.windows(2).all(|x| x[0] == x[1]), "{cat}");
    }
}

#[test]
fn routing_trade_off_best_fit_reaches_more_adverts_with_fewer_duplicates() {
    let w = world();
    let [broad, best] = runs(&w);
    let reached = |r: &RunResult| {
        let mut v: Vec<usize> = r.deliveries.iter().map(|d| d.advert).collect();
        v.sort_unstable();
        v.dedup();
        v.len()
    };
    assert!(duplicates(&best).len() < duplicates(&broad).len());
    assert!(reached(&best) > reached(&broad));
    // ...but a phone without the best-fit app loses the advert: Finn refused location in
    // RoadMate, the best fit for fuel, so best fit shows him no fuel advert while broad does
    // (through CityGo).
    let finn_fuel = |r: &RunResult| shown(r, &w, "Finn").iter().any(|x| x.1 == "fuel-a1");
    assert!(finn_fuel(&broad) && !finn_fuel(&best));
}

#[test]
fn the_scenarios_key_outcomes() {
    let w = world();
    let [broad, best] = runs(&w);
    let pair = |a: &str, b: &str| (a.to_string(), b.to_string());
    for r in [&broad, &best] {
        // Hospital outreach reaches the driver through the health app he shared health with,
        // never through the navigation app on the same phone.
        let ben = shown(r, &w, "Ben");
        assert!(ben.contains(&pair("carewell", "outreach-a1")));
        assert!(!ben.contains(&pair("roadmate", "outreach-a1")));
        // No partner accepts gambling; a child-directed app never runs the engine.
        let casino = advert(&w, "casino-a1");
        assert!(r.deliveries.iter().all(|d| d.advert != casino));
        assert!(shown(r, &w, "Eve").is_empty());
        let kids = app(&w, "kidsquest");
        assert!(r.deliveries.iter().all(|d| d.app != kids));
        for p in ["Dev", "Eve"] {
            assert!(r.silent.iter().any(|s| s.phone == phone(&w, p)
                && s.app == kids
                && s.reason == ReasonCode::EngineDisabled));
        }
        // Location consent refused: RoadMate stays silent on Finn's phone.
        assert!(r.silent.iter().any(|s| s.phone == phone(&w, "Finn")
            && s.app == app(&w, "roadmate")
            && s.reason == ReasonCode::ConsentDenied));
        // Drive-past adverts fire in the direction of travel only: nothing on the way home.
        let south = w.journeys.iter().position(|j| j.key == "a1-south").unwrap();
        assert!(r.deliveries.iter().all(|d| d.journey != south));
        // Health on foot (E-P-FOOT): the lawful walking signpost 1.5 km before the clinic reaches
        // Gus through CareWell, where he shares health interests, at the health floor of 700.
        let walk = advert(&w, "flu-clinic-walk");
        let gus = r
            .deliveries
            .iter()
            .find(|d| d.advert == walk)
            .expect("the walking flu-jab signpost is delivered");
        assert_eq!(
            (gus.phone, gus.app, gus.conf_milli),
            (phone(&w, "Gus"), app(&w, "carewell"), 700)
        );
        assert_eq!(r.deliveries.iter().filter(|d| d.advert == walk).count(), 1);
        // The same walk on 25 m GPS is not strong evidence: capped at 600, no health advert.
        assert!(r.misses.iter().any(|m| m.phone == phone(&w, "Hana")
            && m.advert == walk
            && m.kind == MissKind::Path(ReasonCode::PathConfidenceLow)));
        // The advert at the clinic door is what the law and PLAN 6.5 forbid: dropped by F19.
        let door = advert(&w, "flu-clinic-door");
        assert!(r.deliveries.iter().all(|d| d.advert != door));
        assert!(r.misses.iter().any(|m| m.phone == phone(&w, "Gus")
            && m.advert == door
            && m.kind == MissKind::Policy(ReasonCode::TriggerInStandoff)));
    }
    // Same mall, same moment, each app picks the advert its users care about most.
    let dev = shown(&best, &w, "Dev");
    assert!(dev.contains(&pair("citygo", "mall-cinema")));
    assert!(dev.contains(&pair("shopsmart", "mall-sushi")));
    let t = |ad: &str| {
        best.deliveries
            .iter()
            .find(|d| d.phone == phone(&w, "Dev") && w.adverts[d.advert].signpost.id.as_str() == ad)
            .unwrap()
            .t_ms
    };
    assert_eq!(t("mall-cinema"), t("mall-sushi"));
    // One app delivers one advert per 30 minutes: on the A1, RoadMate shows the fuel advert
    // and the EV-charging advert just after it is paced out.
    assert!(best.misses.iter().any(|m| m.phone == phone(&w, "Ada")
        && m.advert == advert(&w, "ev-a1")
        && m.kind == MissKind::Paced(ReasonCode::CapAppInterval)));
}

/// No advert of any class is shown within a health facility's 1,000 m standoff (F10), and every
/// health advert's trigger lies outside it (F19).
#[test]
fn nothing_fires_within_a_facility_standoff() {
    let w = world();
    let zones = town::FACILITIES;
    let dist_m = |a: (i32, i32), b: (i64, i64)| {
        let (lat, lon) = town::at(b.0, b.1).unwrap();
        // Planar metres near the town (latitude 45): 1e-7 deg is 1.11 cm north, 0.79 cm east.
        let dn = i64::from(a.0 - lat) * 111_195 / 10_000_000;
        let de = i64::from(a.1 - lon) * 78_626 / 10_000_000;
        (dn * dn + de * de).isqrt()
    };
    for r in runs(&w) {
        for d in &r.deliveries {
            let fix = w.journeys[d.journey]
                .fixes
                .iter()
                .find(|f| f.t_ms == d.t_ms)
                .unwrap();
            for (id, _, centre) in zones {
                let m = dist_m((fix.lat_e7, fix.lon_e7), *centre);
                assert!(m > 1000, "{id}: delivery {m} m away");
            }
        }
    }
    // Every policy carries the facility zones (the SDK adds them).
    for a in &w.apps {
        assert_eq!(a.policy.standoff_zones.len(), zones.len(), "{}", a.name);
    }
}

#[test]
fn routing_clamps_weights_like_the_engine() {
    use proxim_types::{CategoryAffinity, Policy};
    let policy = |w| Policy {
        category_affinity: vec![CategoryAffinity {
            category: "food".into(),
            weight_milli: w,
        }],
        ..Policy::default()
    };
    assert_eq!(fit(&policy(60_000), "food.cafe"), Some(3000));
    assert_eq!(
        fit(&policy(60_000), "food.cafe"),
        fit(&policy(3000), "food.cafe")
    );
    assert_eq!(fit(&policy(1500), "food.cafe"), Some(1500));
}

#[test]
fn runs_are_deterministic() {
    let w = world();
    assert_eq!(runs(&w), runs(&w));
}

/// The `core/` directory. `PROXIM_CORE_DIR` overrides the build machine's path when the tests
/// run on a device (Android emulator) with the tree copied over.
fn core_dir() -> PathBuf {
    std::env::var_os("PROXIM_CORE_DIR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

#[test]
fn checked_in_report_is_current() {
    let w = world();
    let text = report::markdown(&w, &runs(&w));
    let path = core_dir().join("testdata/sim/phone-simulator-results.md");
    let old = std::fs::read_to_string(&path).unwrap();
    assert!(
        old == text,
        "{} is stale: cargo run -p proxim-sim --example phone_report with PROXIM_BLESS=1",
        path.display()
    );
}
