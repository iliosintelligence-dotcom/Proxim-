//! Phone simulator: several partner apps on one phone, each running its own copy of the engine
//! (as the SDK does), fed the same location journey.
//!
//! What it models, and what it does not:
//! - Each app sees only the adverts the backend routed to it and decides alone, with its own
//!   policy, consent and state. Apps cannot see each other (OS sandboxing), so nothing here lets
//!   one app's decision depend on another's.
//! - The backend routes a campaign to partner apps by their published category policy
//!   ([`Routing`]); it never knows which apps share a phone.
//! - Time is the fix time, one fix per second, so a run is fully deterministic.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use proxim_decide::decide;
use proxim_types::defaults::{AFFINITY_MAX_MILLI, AFFINITY_NEUTRAL_MILLI};
use proxim_types::taxonomy::{matches, prefixes};
use proxim_types::{
    Clock, Consent, ConsentValue, EngineError, Fix, Outcome, Policy, ReasonCode, Signpost, State,
};

/// A partner app as the backend knows it: its published policy.
#[derive(Clone, Debug)]
pub struct PartnerApp {
    pub key: &'static str,
    pub name: &'static str,
    pub what: &'static str,
    pub policy: Policy,
}

/// An app on a phone, with what the user granted it.
#[derive(Clone, Debug)]
pub struct Installed {
    pub app: usize,
    pub location: bool,
    pub health: bool,
}

#[derive(Clone, Debug)]
pub struct Phone {
    pub name: &'static str,
    pub persona: &'static str,
    pub apps: Vec<Installed>,
    /// Indices into [`World::journeys`], in time order.
    pub journeys: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Advert {
    pub sender_name: &'static str,
    pub headline: &'static str,
    pub place: &'static str,
    pub signpost: Signpost,
}

#[derive(Clone, Debug)]
pub struct Journey {
    pub key: &'static str,
    pub name: &'static str,
    pub fixes: Vec<Fix>,
}

#[derive(Clone, Debug)]
pub struct World {
    pub apps: Vec<PartnerApp>,
    pub adverts: Vec<Advert>,
    pub phones: Vec<Phone>,
    pub journeys: Vec<Journey>,
}

/// How the backend picks the partner apps that receive a campaign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Routing {
    /// Every partner app whose policy accepts the category.
    Broad,
    /// Only the accepting apps with the highest affinity for the category (ties all receive it).
    BestFit,
}

impl Routing {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Broad => "broad",
            Self::BestFit => "best fit",
        }
    }
}

/// The app's affinity for a category if its policy accepts it (not blocked, accept list passes),
/// else `None`. Mirrors the engine's F16 and spec 7.1 so the backend never sends what the app
/// would drop.
#[must_use]
pub fn fit(policy: &Policy, category: &str) -> Option<u16> {
    let blocked = policy
        .categories_blocked
        .iter()
        .any(|b| matches(b, category));
    let accepted = policy.categories_accepted.is_empty()
        || policy
            .categories_accepted
            .iter()
            .any(|a| matches(a, category));
    if blocked || !accepted || policy.child_directed == Some(true) {
        return None;
    }
    let weight = prefixes(category)
        .find_map(|p| {
            policy
                .category_affinity
                .iter()
                .find(|a| a.category == p)
                .map(|a| a.weight_milli)
        })
        .unwrap_or(AFFINITY_NEUTRAL_MILLI);
    // The engine clamps weights to 3000 (spec 7.1); routing must rank them the same way.
    Some(weight.min(AFFINITY_MAX_MILLI))
}

/// For each app, the indices of the adverts the backend sends it.
#[must_use]
pub fn route(world: &World, routing: Routing) -> Vec<Vec<usize>> {
    let mut per_app: Vec<Vec<usize>> = world.apps.iter().map(|_| Vec::new()).collect();
    for (ai, ad) in world.adverts.iter().enumerate() {
        let fits: Vec<Option<u16>> = world
            .apps
            .iter()
            .map(|a| fit(&a.policy, &ad.signpost.category))
            .collect();
        let best = fits.iter().flatten().max().copied();
        for (pi, f) in fits.iter().enumerate() {
            let send = match (routing, f) {
                (_, None) => false,
                (Routing::Broad, Some(_)) => true,
                (Routing::BestFit, Some(w)) => Some(*w) == best,
            };
            if send && let Some(list) = per_app.get_mut(pi) {
                list.push(ai);
            }
        }
    }
    per_app
}

/// One advert shown to the user by one app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delivery {
    pub phone: usize,
    pub journey: usize,
    pub app: usize,
    pub advert: usize,
    pub t_ms: i64,
    pub path_tier: u8,
    pub conf_milli: u16,
}

/// Why an app that received an advert did not show it on a phone (all journeys together).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MissKind {
    /// A static filter: policy or consent (reason ids 200..=299).
    Policy(ReasonCode),
    /// An event happened but caps, pacing or the slot stopped it (400..=405).
    Paced(ReasonCode),
    /// The detector tracked the advert but a path gate held it (300..=399, the last one seen).
    Path(ReasonCode),
    /// The trigger never produced an event on these journeys.
    NotTriggered,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Miss {
    pub phone: usize,
    pub app: usize,
    pub advert: usize,
    pub kind: MissKind,
}

/// A phone where the app could not run the engine at all (F01, F02).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Silent {
    pub phone: usize,
    pub app: usize,
    pub reason: ReasonCode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunResult {
    pub routing: Routing,
    pub routed: Vec<Vec<usize>>,
    pub deliveries: Vec<Delivery>,
    pub misses: Vec<Miss>,
    pub silent: Vec<Silent>,
}

fn consent_for(inst: &Installed) -> Consent {
    let g = |b: bool| {
        if b {
            ConsentValue::Granted
        } else {
            ConsentValue::Denied
        }
    };
    Consent {
        location_matching: g(inst.location),
        nearby_health_events: g(inst.health),
        aggregate_reporting: ConsentValue::Denied,
        local_notifications: ConsentValue::Granted,
        health_receipt: inst.health.then(|| String::from("sim-health-receipt")),
    }
}

/// Run every phone's journeys through every installed app.
///
/// # Errors
/// An [`EngineError`] from `decide` (a malformed world); [`EngineError::Parse`] for an index
/// that points nowhere.
pub fn run(world: &World, routing: Routing) -> Result<RunResult, EngineError> {
    let routed = route(world, routing);
    let mut deliveries = Vec::new();
    let mut misses = Vec::new();
    let mut silent = Vec::new();
    for (phi, phone) in world.phones.iter().enumerate() {
        for inst in &phone.apps {
            let app = world.apps.get(inst.app).ok_or(EngineError::Parse)?;
            let ads = routed.get(inst.app).ok_or(EngineError::Parse)?;
            let signposts: Vec<Signpost> = ads
                .iter()
                .map(|&i| world.adverts.get(i).map(|a| a.signpost.clone()))
                .collect::<Option<_>>()
                .ok_or(EngineError::Parse)?;
            let index: BTreeMap<&str, usize> = ads
                .iter()
                .zip(&signposts)
                .map(|(&i, s)| (s.id.as_str(), i))
                .collect();
            let policy = Policy {
                explain: Some(true),
                ..app.policy.clone()
            };
            let consent = consent_for(inst);
            let mut state = State::empty();
            // Per advert: delivered, or the most telling reason it was not.
            let mut seen: BTreeMap<usize, MissKind> = BTreeMap::new();
            let mut delivered: BTreeMap<usize, bool> = BTreeMap::new();
            let mut stopped: Option<ReasonCode> = None;
            for &ji in &phone.journeys {
                let journey = world.journeys.get(ji).ok_or(EngineError::Parse)?;
                for fix in &journey.fixes {
                    let clock = Clock {
                        now_ms: fix.t_ms,
                        tz_offset_min: 0,
                    };
                    let (out, next) =
                        decide(&state, fix, &signposts, &policy, &consent, &clock, 0)?;
                    state = next;
                    if !out.fix.ok
                        && matches!(
                            out.fix.reason,
                            ReasonCode::ConsentDenied | ReasonCode::EngineDisabled
                        )
                    {
                        stopped = Some(out.fix.reason);
                        continue;
                    }
                    for n in out.notes.iter().flatten() {
                        let Some(&ai) = index.get(n.signpost.as_str()) else {
                            continue;
                        };
                        // Off the advert's road or out of its range is "not on this route".
                        if (300..400).contains(&n.reason.id())
                            && !matches!(n.reason, ReasonCode::OffCorridor | ReasonCode::OutOfRange)
                        {
                            let e = seen.entry(ai).or_insert(MissKind::Path(n.reason));
                            if matches!(e, MissKind::Path(_)) {
                                *e = MissKind::Path(n.reason);
                            }
                        }
                    }
                    for d in &out.decisions {
                        let Some(&ai) = index.get(d.signpost.as_str()) else {
                            continue;
                        };
                        if d.outcome == Outcome::Fire {
                            delivered.insert(ai, true);
                            deliveries.push(Delivery {
                                phone: phi,
                                journey: ji,
                                app: inst.app,
                                advert: ai,
                                t_ms: d.t_ms,
                                path_tier: d.path_tier,
                                conf_milli: d.path_conf_milli,
                            });
                        } else {
                            let id = d.reason.id();
                            let kind = if (200..300).contains(&id) {
                                MissKind::Policy(d.reason)
                            } else {
                                MissKind::Paced(d.reason)
                            };
                            let e = seen.entry(ai).or_insert(kind);
                            *e = (*e).min(kind);
                        }
                    }
                }
            }
            if let Some(reason) = stopped {
                silent.push(Silent {
                    phone: phi,
                    app: inst.app,
                    reason,
                });
                continue;
            }
            for &ai in ads {
                if delivered.contains_key(&ai) {
                    continue;
                }
                misses.push(Miss {
                    phone: phi,
                    app: inst.app,
                    advert: ai,
                    kind: seen.get(&ai).copied().unwrap_or(MissKind::NotTriggered),
                });
            }
        }
    }
    deliveries.sort_by_key(|d| (d.phone, d.t_ms, d.app, d.advert));
    Ok(RunResult {
        routing,
        routed,
        deliveries,
        misses,
        silent,
    })
}

/// Deliveries of the same advert by two or more apps on one phone: `(phone, advert, apps)`.
#[must_use]
pub fn duplicates(r: &RunResult) -> Vec<(usize, usize, Vec<usize>)> {
    let mut by: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for d in &r.deliveries {
        let apps = by.entry((d.phone, d.advert)).or_default();
        if !apps.contains(&d.app) {
            apps.push(d.app);
        }
    }
    by.into_iter()
        .filter(|(_, apps)| apps.len() > 1)
        .map(|((p, a), apps)| (p, a, apps))
        .collect()
}
