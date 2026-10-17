//! "Riverton", the simulator's fictional town: six partner apps, 18 adverts from ten industries,
//! two health facilities, eight phones and eight journeys. Coordinates are metres east and north of the town centre
//! (latitude 45.0, longitude 7.0); every road is straight and runs north-south or east-west.
//!
//! ```text
//!   N
//!   8000 |  A1 expressway (x = -3000), northbound adverts: outreach, fuel, hotel, EV, casino
//!   2000 |  Riverside Park path (y = 2000): gym, sportswear, smoothie bar
//!      0 |  Main Street (y = 0), eastbound adverts: drive-through, supermarket
//!        |  High Street (x = 500), on foot: cafe, bar, bank, mall, pharmacy, flu-jab signpost
//!  -1500 |  St Mary's Hospital (x = -1500); -3000: Riverton Walk-in Clinic (x = 500)
//!  -8000 |
//!
//! Both facilities carry the 1,000 m standoff zone the SDK hands every app (engine spec F10 and
//! F19): no health advert may be placed or fire within it (PLAN 1 and 6.5; state laws restrict
//! health messaging within 1,750 to 2,000 ft of a facility).
//! ```

use crate::phone::{Advert, Installed, Journey, PartnerApp, Phone, World};
use alloc::string::String;
use alloc::vec::Vec;
use proxim_geo::trig::scale_for_lat_q;
use proxim_types::{CategoryAffinity, EngineError, Fix, Mode, Policy, Signpost, StandoffZone};
use serde::Deserialize;
use serde_json::{Value, json};

/// Latitude 45.0, longitude 7.0.
pub const CENTRE: (i32, i32) = (450_000_000, 70_000_000);
/// Monday 2026-10-12 00:00:00 UTC, the simulated day.
pub const DAY0_MS: i64 = 1_791_763_200_000;
const HOUR_MS: i64 = 3_600_000;
const MIN_MS: i64 = 60_000;

fn round_div(a: i128, b: i128) -> i128 {
    (2 * a + b).div_euclid(2 * b)
}

/// Position `e_m` east and `n_m` north of the centre (spec 1.2 inverse via the SCALE table).
///
/// # Errors
/// From the SCALE table, or a coordinate outside `i32`.
pub fn at(e_m: i64, n_m: i64) -> Result<(i32, i32), EngineError> {
    at_mm(e_m.saturating_mul(1000), n_m.saturating_mul(1000))
}

fn pos(e_m: i64, n_m: i64) -> Result<Value, EngineError> {
    let (lat, lon) = at(e_m, n_m)?;
    Ok(json!({"lat_e7": lat, "lon_e7": lon}))
}

fn signpost(v: &Value) -> Result<Signpost, EngineError> {
    Signpost::deserialize(v).map_err(|_| EngineError::Parse)
}

struct Ad {
    id: &'static str,
    sender: &'static str,
    category: &'static str,
    health: bool,
    headline: &'static str,
    place: &'static str,
}

fn base(ad: &Ad, event: &str, trigger: Value) -> Value {
    json!({
        "id": ad.id, "rev": 1,
        "class": if ad.health { "health" } else { "general" },
        "category": ad.category, "sender": ad.sender,
        "sensitivity": i32::from(ad.health), "event": event,
        "valid_from_ms": 0, "valid_until_ms": DAY0_MS + 7 * 24 * HOUR_MS,
        "trigger": trigger
    })
}

/// A drive-past advert: an APPROACH route along a road, firing about `lead_s` before the anchor
/// (the last vertex), in the direction of travel only.
fn approach(ad: &Ad, from: (i64, i64), anchor: (i64, i64)) -> Result<Advert, EngineError> {
    approach_by(ad, from, anchor, Mode::Vehicle)
}

/// [`approach`] for a given travel mode (a walking signpost fires about 100 m before its anchor).
fn approach_by(
    ad: &Ad,
    from: (i64, i64),
    anchor: (i64, i64),
    mode: Mode,
) -> Result<Advert, EngineError> {
    let trigger = json!({
        "origin": pos(anchor.0, anchor.1)?,
        "shape": {"kind": "route", "buffer_m": 30,
                  "vertices": [pos(from.0, from.1)?, pos(anchor.0, anchor.1)?]},
        "lead_s": 40, "mode": mode
    });
    advert(ad, &base(ad, "approach", trigger))
}

/// The health facilities: `(id, name, position)`. Each gets the default 1,000 m standoff zone.
pub const FACILITIES: &[(&str, &str, (i64, i64))] = &[
    ("st-marys", "St Mary's Hospital", (-1500, -1500)),
    ("walk-in-clinic", "Riverton Walk-in Clinic", (500, -3000)),
];

/// The standoff zones of [`FACILITIES`], as the SDK supplies them to every app's policy.
///
/// # Errors
/// From the geometry helpers (never for the built-in town).
pub fn standoff_zones() -> Result<Vec<StandoffZone>, EngineError> {
    FACILITIES
        .iter()
        .map(|(id, _, (e, n))| {
            StandoffZone::deserialize(&json!({"id": id, "center": pos(*e, *n)?}))
                .map_err(|_| EngineError::Parse)
        })
        .collect()
}

/// A walk-in advert: an ENTER circle of `radius_m` around a place.
fn enter(ad: &Ad, centre: (i64, i64), radius_m: u32) -> Result<Advert, EngineError> {
    let c = pos(centre.0, centre.1)?;
    let trigger = json!({
        "origin": c, "shape": {"kind": "circle", "center": c, "radius_m": radius_m}
    });
    advert(ad, &base(ad, "enter", trigger))
}

fn advert(ad: &Ad, v: &Value) -> Result<Advert, EngineError> {
    Ok(Advert {
        sender_name: ad.sender,
        headline: ad.headline,
        place: ad.place,
        signpost: signpost(v)?,
    })
}

const fn ad(
    id: &'static str,
    sender: &'static str,
    category: &'static str,
    headline: &'static str,
    place: &'static str,
) -> Ad {
    Ad {
        id,
        sender,
        category,
        health: false,
        headline,
        place,
    }
}

const fn health_ad(
    id: &'static str,
    sender: &'static str,
    category: &'static str,
    headline: &'static str,
    place: &'static str,
) -> Ad {
    Ad {
        id,
        sender,
        category,
        health: true,
        headline,
        place,
    }
}

/// The adverts, one or two per industry.
///
/// # Errors
/// From the geometry helpers (never for the built-in town).
pub fn adverts() -> Result<Vec<Advert>, EngineError> {
    const A1: i64 = -3000;
    const MAIN: i64 = 0;
    const HIGH: i64 = 500;
    const PARK: i64 = 2000;
    Ok(Vec::from([
        approach(
            &health_ad(
                "outreach-a1",
                "St Mary's Hospital",
                "health.outreach",
                "Free blood-pressure checks today, next exit",
                "A1, exit 3",
            ),
            (A1, -7000),
            (A1, -1000),
        )?,
        approach(
            &ad(
                "fuel-a1",
                "FuelCo",
                "auto.fuel",
                "Fuel 10c off at the next exit",
                "A1, exit 4",
            ),
            (A1, -3000),
            (A1, 2000),
        )?,
        approach(
            &ad(
                "hotel-a1",
                "Riverside Hotel",
                "travel.hotel",
                "Rooms tonight from 79",
                "A1, exit 5",
            ),
            (A1, 1000),
            (A1, 6000),
        )?,
        approach(
            &ad(
                "ev-a1",
                "VoltUp",
                "auto.ev_charging",
                "Fast charging, 20 minutes to 80%",
                "A1, exit 5",
            ),
            (A1, 2000),
            (A1, 6500),
        )?,
        approach(
            &ad(
                "casino-a1",
                "LuckyStar Casino",
                "gambling.casino",
                "Free spins tonight",
                "A1, exit 6",
            ),
            (A1, 3000),
            (A1, 7500),
        )?,
        approach(
            &ad(
                "drive-thru",
                "BurgerBarn",
                "food.fast_food",
                "Drive-through breakfast deal",
                "Main Street",
            ),
            (-3000, MAIN),
            (-1000, MAIN),
        )?,
        approach(
            &ad(
                "supermarket",
                "FreshMart",
                "retail.grocery",
                "Weekly shop: 15% off fresh food",
                "Main Street",
            ),
            (-1000, MAIN),
            (1500, MAIN),
        )?,
        enter(
            &ad(
                "cafe",
                "BeanThere",
                "food.cafe",
                "Flat white and a pastry for 4",
                "High Street south",
            ),
            (HIGH, -900),
            150,
        )?,
        enter(
            &ad(
                "bar",
                "The Tap Room",
                "alcohol.bar",
                "Happy hour 5 to 7",
                "High Street south",
            ),
            (HIGH, -400),
            150,
        )?,
        enter(
            &ad(
                "bank",
                "Northbank",
                "finance.bank",
                "Open a savings account in 10 minutes",
                "High Street",
            ),
            (HIGH, 100),
            150,
        )?,
        enter(
            &ad(
                "mall-cinema",
                "Riverside Cinema",
                "entertainment.cinema",
                "Matinee tickets half price",
                "Riverside Mall",
            ),
            (HIGH, 700),
            150,
        )?,
        enter(
            &ad(
                "mall-sushi",
                "Sushi Go",
                "food.restaurant",
                "Lunch box 9.90",
                "Riverside Mall",
            ),
            (HIGH, 700),
            150,
        )?,
        enter(
            &ad(
                "pharmacy",
                "CityChem",
                "health.pharmacy",
                "Vitamins: buy one, get one free",
                "High Street north",
            ),
            (HIGH, 1200),
            150,
        )?,
        // The version PLAN 1 and 6.5 permit (pending counsel): a walking signpost whose route
        // ends 1.5 km before the clinic, outside its 1,000 m standoff.
        approach_by(
            &health_ad(
                "flu-clinic-walk",
                "Riverton Walk-in Clinic",
                "health.clinic",
                "Walk-in flu jabs, 1.5 km ahead on the High Street",
                "High Street south, walking",
            ),
            (HIGH, 0),
            (HIGH, -1500),
            Mode::Foot,
        )?,
        // The version PLAN 1 and 6.5 rule out (and state health-geofence laws restrict, per
        // counsel): an advert at the clinic door. The sender backend rejects it; the engine
        // drops it too (F19 TRIGGER_IN_STANDOFF).
        enter(
            &health_ad(
                "flu-clinic-door",
                "Riverton Walk-in Clinic",
                "health.clinic",
                "Walk-in flu jabs today",
                "At the clinic door",
            ),
            (HIGH, -3000),
            150,
        )?,
        enter(
            &ad(
                "gym",
                "IronWorks Gym",
                "fitness.gym",
                "First week free",
                "Riverside Park",
            ),
            (800, PARK),
            200,
        )?,
        enter(
            &ad(
                "sportswear",
                "StrideShop",
                "retail.sportswear",
                "Running shoes 30% off",
                "Riverside Park",
            ),
            (1800, PARK),
            150,
        )?,
        enter(
            &ad(
                "smoothie",
                "GreenCup",
                "food.healthy",
                "Protein smoothie 3.50",
                "Riverside Park",
            ),
            (2600, PARK),
            150,
        )?,
    ]))
}

fn aff(category: &str, weight_milli: u16) -> CategoryAffinity {
    CategoryAffinity {
        category: category.into(),
        weight_milli,
    }
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| String::from(*s)).collect()
}

/// The partner apps and the category policy each publishes, plus the facility standoff zones
/// the SDK adds to every app's policy.
///
/// # Errors
/// From the geometry helpers (never for the built-in town).
pub fn apps() -> Result<Vec<PartnerApp>, EngineError> {
    let restricted = strings(&["alcohol", "gambling"]);
    let zones = standoff_zones()?;
    let mut apps = Vec::from([
        PartnerApp {
            key: "carewell",
            name: "CareWell",
            what: "health and wellbeing",
            policy: Policy {
                categories_accepted: strings(&["health", "fitness", "food.healthy"]),
                categories_blocked: restricted.clone(),
                category_affinity: Vec::from([aff("health", 2000), aff("fitness", 1200)]),
                // The only app here that may receive health adverts (PLAN 6.5; spec 7.7).
                profile: Some(proxim_types::Profile::Sealed),
                health_tier: Some(true),
                ..Policy::default()
            },
        },
        PartnerApp {
            key: "roadmate",
            name: "RoadMate",
            what: "navigation and driving",
            policy: Policy {
                categories_blocked: restricted.clone(),
                category_affinity: Vec::from([
                    aff("auto", 2000),
                    aff("food.fast_food", 1500),
                    aff("travel", 1300),
                ]),
                ..Policy::default()
            },
        },
        PartnerApp {
            key: "fitstride",
            name: "FitStride",
            what: "running and fitness tracking",
            policy: Policy {
                categories_accepted: strings(&[
                    "fitness",
                    "food.healthy",
                    "retail.sportswear",
                    "health.wellness",
                ]),
                category_affinity: Vec::from([
                    aff("fitness", 2000),
                    aff("food.healthy", 1500),
                    aff("retail.sportswear", 1500),
                ]),
                ..Policy::default()
            },
        },
        PartnerApp {
            key: "shopsmart",
            name: "ShopSmart",
            what: "shopping deals",
            policy: Policy {
                categories_accepted: strings(&["retail", "food", "beauty", "entertainment"]),
                categories_blocked: restricted,
                category_affinity: Vec::from([aff("retail", 2000), aff("food", 1200)]),
                ..Policy::default()
            },
        },
        PartnerApp {
            key: "citygo",
            name: "CityGo",
            what: "city guide and transit",
            policy: Policy {
                categories_blocked: strings(&["gambling"]),
                category_affinity: Vec::from([
                    aff("entertainment", 1800),
                    aff("travel", 1500),
                    aff("food.cafe", 1200),
                    aff("alcohol", 1100),
                ]),
                ..Policy::default()
            },
        },
        PartnerApp {
            key: "kidsquest",
            name: "KidsQuest",
            what: "a children's game (child-directed)",
            policy: Policy {
                child_directed: Some(true),
                ..Policy::default()
            },
        },
    ]);
    for a in &mut apps {
        a.policy.standoff_zones.clone_from(&zones);
    }
    Ok(apps)
}

/// Fixes one second apart along an axis-aligned polyline, `speed_mmps` on every leg.
///
/// # Errors
/// [`EngineError::Parse`] for a diagonal leg; geometry errors.
fn walk(
    start_ms: i64,
    points: &[(i64, i64)],
    speed_mmps: u32,
    acc_mm: u32,
) -> Result<Vec<Fix>, EngineError> {
    let mut fixes = Vec::new();
    let mut t = start_ms;
    let step_mm = i64::from(speed_mmps);
    for leg in points.windows(2) {
        let [(e0, n0), (e1, n1)] = leg else {
            continue;
        };
        let (de, dn) = ((e1 - e0) * 1000, (n1 - n0) * 1000);
        let course: u16 = match (de.signum(), dn.signum()) {
            (0, 1) => 0,
            (1, 0) => 900,
            (0, -1) => 1800,
            (-1, 0) => 2700,
            _ => return Err(EngineError::Parse),
        };
        let len_mm = de.abs() + dn.abs();
        let mut s = 0;
        while s < len_mm {
            let (e_mm, n_mm) = (e0 * 1000 + de.signum() * s, n0 * 1000 + dn.signum() * s);
            let (lat, lon) = at_mm(e_mm, n_mm)?;
            fixes.push(Fix {
                t_ms: t,
                lat_e7: lat,
                lon_e7: lon,
                acc_mm,
                speed_mmps: Some(speed_mmps),
                speed_acc_mmps: Some(500),
                course_ddeg: Some(course),
                course_acc_ddeg: Some(50),
                is_mock: false,
            });
            t += 1000;
            s += step_mm;
        }
    }
    Ok(fixes)
}

fn at_mm(e_mm: i64, n_mm: i64) -> Result<(i32, i32), EngineError> {
    let lat_q =
        i32::try_from(round_div(i128::from(CENTRE.0), 100_000)).map_err(|_| EngineError::Parse)?;
    let (kn, ke) = scale_for_lat_q(lat_q)?;
    let dlat = round_div(i128::from(n_mm) << 32, i128::from(kn));
    let dlon = round_div(i128::from(e_mm) << 32, i128::from(ke));
    let lat = i32::try_from(i128::from(CENTRE.0) + dlat).map_err(|_| EngineError::Parse)?;
    let lon = i32::try_from(i128::from(CENTRE.1) + dlon).map_err(|_| EngineError::Parse)?;
    Ok((lat, lon))
}

const CAR_MMPS: u32 = 29_000; // 104 km/h
const TOWN_CAR_MMPS: u32 = 13_000; // 47 km/h
const WALK_MMPS: u32 = 1_400;
const RUN_MMPS: u32 = 2_400;

/// The journeys, by start time on the simulated day.
///
/// # Errors
/// Geometry errors (never for the built-in town).
pub fn journeys() -> Result<Vec<Journey>, EngineError> {
    let at_time = |h: i64, m: i64| DAY0_MS + h * HOUR_MS + m * MIN_MS;
    Ok(Vec::from([
        Journey {
            key: "park-run",
            name: "07:00 run along Riverside Park",
            fixes: walk(at_time(7, 0), &[(0, 2000), (3000, 2000)], RUN_MMPS, 6000)?,
        },
        Journey {
            key: "a1-north",
            name: "08:00 drive north on the A1 expressway",
            fixes: walk(
                at_time(8, 0),
                &[(-3000, -8000), (-3000, 8000)],
                CAR_MMPS,
                5000,
            )?,
        },
        Journey {
            key: "main-east",
            name: "12:00 drive east along Main Street",
            fixes: walk(
                at_time(12, 0),
                &[(-3000, 0), (3000, 0)],
                TOWN_CAR_MMPS,
                5000,
            )?,
        },
        Journey {
            key: "high-street",
            name: "13:00 walk up the High Street",
            fixes: walk(
                at_time(13, 0),
                &[(500, -1500), (500, 1500)],
                WALK_MMPS,
                8000,
            )?,
        },
        Journey {
            key: "a1-south",
            name: "17:30 drive home south on the A1",
            fixes: walk(
                at_time(17, 30),
                &[(-3000, 8000), (-3000, -8000)],
                CAR_MMPS,
                5000,
            )?,
        },
        Journey {
            key: "evening-walk",
            name: "19:30 walk down the High Street",
            fixes: walk(
                at_time(19, 30),
                &[(500, 1500), (500, -1500)],
                WALK_MMPS,
                8000,
            )?,
        },
        // Continues into the clinic's standoff and to its door: nothing may fire inside it.
        Journey {
            key: "clinic-walk",
            name: "19:30 walk down the High Street to the walk-in clinic",
            fixes: walk(
                at_time(19, 30),
                &[(500, 1500), (500, -2950)],
                WALK_MMPS,
                8000,
            )?,
        },
        Journey {
            key: "clinic-walk-poor-gps",
            name: "19:30 the same walk with 25 m GPS accuracy",
            fixes: walk(
                at_time(19, 30),
                &[(500, 1500), (500, -2950)],
                WALK_MMPS,
                25_000,
            )?,
        },
    ]))
}

/// The whole built-in world.
///
/// # Errors
/// Geometry or parse errors (never for the built-in town).
pub fn world() -> Result<World, EngineError> {
    let journeys = journeys()?;
    let j = |key: &str| journeys.iter().position(|x| x.key == key);
    let js = |keys: &[&str]| -> Result<Vec<usize>, EngineError> {
        keys.iter()
            .map(|k| j(k).ok_or(EngineError::Parse))
            .collect()
    };
    let apps = apps()?;
    let a = |key: &str| apps.iter().position(|x| x.key == key);
    let inst = |key: &str, location: bool, health: bool| -> Result<Installed, EngineError> {
        Ok(Installed {
            app: a(key).ok_or(EngineError::Parse)?,
            location,
            health,
        })
    };
    let phones = Vec::from([
        Phone {
            name: "Ada",
            persona: "commuter who drives everywhere",
            apps: Vec::from([
                inst("roadmate", true, false)?,
                inst("shopsmart", true, false)?,
                inst("citygo", true, false)?,
            ]),
            journeys: js(&["a1-north", "main-east", "a1-south"])?,
        },
        Phone {
            name: "Ben",
            persona: "drives past the hospital; granted nearby health events in CareWell",
            apps: Vec::from([
                inst("carewell", true, true)?,
                inst("roadmate", true, false)?,
            ]),
            journeys: js(&["a1-north", "a1-south"])?,
        },
        Phone {
            name: "Cara",
            persona: "runner; did not grant nearby health events",
            apps: Vec::from([
                inst("fitstride", true, false)?,
                inst("carewell", true, false)?,
                inst("shopsmart", true, false)?,
            ]),
            journeys: js(&["park-run", "high-street"])?,
        },
        Phone {
            name: "Dev",
            persona: "shopper in town, with a child's game installed",
            apps: Vec::from([
                inst("shopsmart", true, false)?,
                inst("citygo", true, false)?,
                inst("kidsquest", true, false)?,
            ]),
            journeys: js(&["high-street", "evening-walk"])?,
        },
        Phone {
            name: "Eve",
            persona: "child's tablet: only the game",
            apps: Vec::from([inst("kidsquest", true, false)?]),
            journeys: js(&["high-street"])?,
        },
        Phone {
            name: "Gus",
            persona: "walks all the way to the walk-in clinic; granted nearby health events in CareWell",
            apps: Vec::from([inst("carewell", true, true)?, inst("citygo", true, false)?]),
            journeys: js(&["high-street", "clinic-walk"])?,
        },
        Phone {
            name: "Hana",
            persona: "same walk to the clinic on poor GPS (25 m); granted nearby health events in CareWell",
            apps: Vec::from([inst("carewell", true, true)?]),
            journeys: js(&["clinic-walk-poor-gps"])?,
        },
        Phone {
            name: "Finn",
            persona: "refused location matching in RoadMate",
            apps: Vec::from([
                inst("roadmate", false, false)?,
                inst("citygo", true, false)?,
            ]),
            journeys: js(&["a1-north", "main-east"])?,
        },
    ]);
    Ok(World {
        apps,
        adverts: adverts()?,
        phones,
        journeys,
    })
}
