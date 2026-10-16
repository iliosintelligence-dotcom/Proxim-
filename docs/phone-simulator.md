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

- **Broad:** 8 of 17 adverts reached someone, with 4 duplicates.
- **Best fit:** 12 of 17 adverts reached someone, with 1 duplicate.
- **What best fit costs:** Finn refused location in RoadMate, the best-fit app for fuel, so under best fit he never sees the fuel advert. Under broad he gets it through CityGo.

## The simulated town

[`core/crates/proxim-sim/src/town.rs`](../core/crates/proxim-sim/src/town.rs) defines the town:

- **Apps (6):** CareWell (health), RoadMate (navigation), FitStride (running), ShopSmart (deals), CityGo (city guide) and KidsQuest (a child-directed game).
- **Adverts (17), from ten industries:**
  - drive-past adverts on the A1 expressway and Main Street;
  - walk-in adverts on the High Street and in Riverside Park.
- **Phones (7),** each with its own apps and consents.
- **Journeys (6),** with one location fix per second:
  - two drives on the A1, one north and one south;
  - one drive along Main Street;
  - two walks on the High Street;
  - one run in the park.

Every advert decision comes from the real engine (`proxim_decide::decide`), one call per app per fix.

## What the test proves (`cargo test -p proxim-sim --test phone_sim`)

These rules hold for every delivery, in both modes:

- the advert's category passes the delivering app's policy;
- the app has location consent;
- a health advert only comes through an app with health consent;
- a child-directed app never shows anything;
- an app shows at most one advert every 30 minutes, and the same advert at most twice a day.

Under best fit, an advert goes only to its highest-weight apps, and a duplicate can only come from apps tied on that weight.

The scenario outcomes pinned by the test:

- the hospital outreach reaches Ben through CareWell, never through RoadMate on the same phone;
- no partner shows the casino;
- nothing fires on the drive home (drive-past adverts only fire in the direction of travel);
- in the mall, the same moment gives CityGo's user the cinema and ShopSmart's user the sushi bar;
- on the A1, RoadMate shows fuel and the EV-charging advert is then held back by pacing.

The run is deterministic, and the checked-in results file must match a fresh run. Regenerate it with `PROXIM_BLESS=1 cargo run -p proxim-sim --example phone_report`.

## What it shows us to fix next

- **Health walk-in adverts never fire.** In M1 a health advert needs drive-past confidence (errata E-D-HEALTH), so the flu clinic reaches nobody, even with consent (Gus). Opening health ENTER needs a privacy decision first.
- **One advert per app per 30 minutes is tight.** On a 10-minute A1 drive, each app shows at most one advert, so the order of exits decides more than relevance does. Sender caps and dayparting (M3) are where this gets tuned.
- **Equal weights duplicate.** ShopSmart and CityGo both weight cafes at 1200, so under best fit Dev sees the cafe twice. The backend can break ties, for example by giving one app a weight one point higher, at the cost of reach for users who don't have that app.
- **Not modelled yet:**
  - OS notification limits;
  - background location and battery behaviour (real-device runs, M4);
  - sender caps across apps;
  - creative variants against ad fatigue (sender product, M3).
