# Phone simulator and category matching

How Proxim gets an advert from any industry to the right partner app on a user's phone, and how we test it end to end without real phones. The numbers from the latest run are in [`core/testdata/sim/phone-simulator-results.md`](../core/testdata/sim/phone-simulator-results.md); this page explains them.

## Who decides, and where

No app can look inside another app on the phone. iOS and Android sandbox every app, and Proxim never learns which other apps are installed. So "pick the right app on the phone" is done in two steps, neither of which needs that knowledge:

1. **Backend routing, per campaign.** Every partner app publishes a category policy: which industries it accepts, which it blocks, which it prefers. When a sender creates a campaign, the backend sends the advert only to the partner apps whose policy fits its category.
2. **On-device decision, per app.** Each partner app runs its own copy of the engine through the SDK. It checks the advert against the user's location and journey, the user's consents in that app, the app's caps and pacing, and its category preferences. Then it decides alone whether to show it.

Matching uses only the partner's published policy and the advert's category. No user profile is built and nothing crosses between apps.

## Categories (engine spec 7.1)

- **Shared list.** `proxim_types::taxonomy::TAXONOMY` has industries and their sub-categories: `health.hospital`, `food.cafe`, `auto.fuel`, `retail.grocery`, `finance.bank`, `gambling.casino`, and so on. A sender picks the most specific code.
- **`categories_blocked`.** Blocks a category and everything under it. Blocking `health` also blocks `health.hospital`. Reason code: `PARTNER_CATEGORY_BLOCKED`.
- **`categories_accepted`.** If set, only these industries are accepted. Anything else is dropped with `CATEGORY_NOT_ACCEPTED` (new reason 210).
- **`category_affinity`.** A relevance weight per category: 1000 is neutral, the maximum is 3000, and the most specific entry wins. When two adverts trigger at the same moment, the app shows the one its users care about most. A weight never adds deliveries: caps, pacing and every other filter stay the same.

## Routing modes (backend)

| Mode | Sends an advert to | Upside | Downside |
|---|---|---|---|
| Broad | every app that accepts its category | the most apps can carry it | generic apps (navigation, city guide) spend their one-advert-per-30-minutes slot on adverts a specialist app would carry better; more duplicates |
| Best fit | only the accepting apps with the highest weight for it | each app carries what fits it; more different adverts get through; fewer duplicates | a user who doesn't have, or switched off, the best-fit app doesn't get the advert |

In the current run:

- **Broad:** 9 of 18 adverts reached someone, with 5 duplicates.
- **Best fit:** 13 of 18 adverts reached someone, with 1 duplicate.
- **What best fit costs:** Finn refused location in RoadMate, the best-fit app for fuel, so under best fit he never sees the fuel advert. Under broad he gets it through CityGo.

## The simulated town

[`core/crates/proxim-sim/src/town.rs`](../core/crates/proxim-sim/src/town.rs) defines the town:

- **Apps (6):** CareWell (health), RoadMate (navigation), FitStride (running), ShopSmart (deals), CityGo (city guide) and KidsQuest (a child-directed game).
- **Adverts (18), from ten industries:**
  - drive-past adverts on the A1 expressway and Main Street;
  - walk-in adverts on the High Street and in Riverside Park;
  - two flu-jab adverts from the walk-in clinic: a walking signpost 1.5 km before it, and one at its door, which must be blocked.
- **Health facilities (2):** St Mary's Hospital and the Riverton Walk-in Clinic. Each has the 1,000 m standoff zone the SDK adds to every app's policy.
- **Phones (8),** each with its own apps and consents. In the evening Gus walks to the clinic, and Hana makes the same walk on poor GPS.
- **Journeys (8),** with one location fix per second:
  - two drives on the A1, one north and one south;
  - one drive along Main Street;
  - two walks on the High Street, and a walk to the clinic on good and on poor GPS;
  - one run in the park.

Every advert decision comes from the real engine (`proxim_decide::decide`), one call per app per fix.

## What the test proves (`cargo test -p proxim-sim --test phone_sim`)

These rules hold for every delivery, in both modes:

- the advert's category passes the delivering app's policy;
- the app has location consent;
- a health-class advert only comes through an app where the user granted nearby health events (with a receipt);
- a child-directed app never shows anything;
- an app shows at most one advert every 30 minutes, and the same advert at most twice a day.

Under best fit, an advert goes only to its highest-weight apps, and a duplicate can only come from apps tied on that weight.

The scenario outcomes pinned by the test:

- the hospital outreach reaches Ben through CareWell, never through RoadMate on the same phone;
- no partner shows the casino;
- nothing fires on the drive home (drive-past adverts only fire in the direction of travel);
- in the mall, the same moment gives CityGo's user the cinema and ShopSmart's user the sushi bar;
- on the A1, RoadMate shows fuel and the EV-charging advert is then held back by pacing;
- the walking flu-jab signpost reaches Gus through CareWell at confidence 700, 1.5 km before the clinic;
- the advert at the clinic door never fires (`TRIGGER_IN_STANDOFF`);
- Gus walks on into the clinic's standoff and to its door: no advert of any kind is shown within 1,000 m of the hospital or the clinic;
- Hana makes the same walk on 25 m GPS: not strong evidence, so the flu-jab signpost does not reach her (`PATH_CONFIDENCE_LOW`).

The run is deterministic, and the checked-in results file must match a fresh run. Regenerate it with `PROXIM_BLESS=1 cargo run -p proxim-sim --example phone_report`.

## What it shows us to fix next

- **Health adverts for people on foot (fixed in the engine, gated for release).** Before, no health advert could reach a pedestrian, because walking confidence was capped at 600 and health needs 700. A health advert at the facility door is not the fix: the plan keeps health adverts 1,000 m from any facility, and state laws restrict health messaging near facilities (Washington 2,000 ft, Connecticut and Nevada 1,750 ft; counsel to confirm). Now a walker heading towards a clinic reaches 700, but only on strong evidence: at least 5 fixes a minute, heading within 15 degrees, and accuracy of 20 m or better (errata E-P-FOOT). Health on ENTER still never fires.
  - **Simulator assumptions.** Walkers here have idealised GPS (8 m, steady course), and CareWell is assumed open for the whole 35-minute walk: health is in-app and foreground only. Real phones walking at 1.4 m/s report heading poorly, so real reach will be lower. E-P-FOOT is a model until field data (M4).
  - **Still open before any live health traffic (owner and counsel decisions).**
    - The M1 engine has two classes, general and health. The plan has four, including `health_facility`, which must produce no device events and no pacing row (I7, planned for M3). Today a delivered flu-jab advert still writes a local cap row and returns a decision to the host app. That row records that this person walked towards a clinic, so the cap store must be excluded from backups, and no host callback may expose a `health_facility` fire.
    - The plan conflicts with itself: PLAN 6.5 says `health_facility` has no pacing row, while Addendum 01 5.3 writes a ledger row when a notification posts. This needs an owner decision.
    - Standoffs here are measured from a facility's centre point. The laws measure from the perimeter, which needs the M5 facility registry.
    - Counsel should confirm whether a walking route that ends on the clinic's own street needs destination-style suppression.
    - The CityChem pharmacy advert is modelled as a general promotion. Under PLAN 6.5 a pharmacy is `health_adjacent`, which needs the receipt gate, and may itself be a sensitive place. M1 does not model this yet.
- **One advert per app per 30 minutes is tight.** On a 10-minute A1 drive, each app shows at most one advert, so the order of exits decides more than relevance does. Sender caps and dayparting (M3) are where this gets tuned.
- **Equal weights duplicate.** ShopSmart and CityGo both weight cafes at 1200, so under best fit Dev sees the cafe twice. The backend can break ties, for example by giving one app a weight one point higher, at the cost of reach for users who don't have that app.
- **Not modelled yet:**
  - OS notification limits;
  - background location and battery behaviour (real-device runs, M4);
  - sender caps across apps;
  - creative variants against ad fatigue (sender product, M3).
