# Democratic advertising: what Proxim can and cannot do

This page reviews the owner's advertising brief (2026-10-16), from "Advertising & Marketing" through to "Eliminating Value Erosion in the Ad Supply Chain". It sets the brief against the engine, the plan (`PLAN.md`, both addenda) and the privacy invariants. It records:

- what is possible;
- what is not, and why;
- what is easiest to build;
- what changed in phases 1 and 2 because of it.

## The thesis

The brief describes an industry in which the advertiser's dollar passes through 4 to 6 intermediaries before it reaches anyone:

- a DSP;
- an exchange;
- an SSP;
- data brokers;
- verification.

On the brief's numbers:

- **Fees:** 28 to 40% of every dollar is lost to these intermediaries.
- **Fraud:** 10 to 25% more is lost to fraud in unprotected buys.
- **Data:** targeting rests on user data that people increasingly distrust.

The brief also gives the ways out, in order of effort:

- **Level 1:** optimise the existing supply path.
- **Level 2:** better deal types (private marketplaces, programmatic guaranteed).
- **Level 3:** direct advertiser–publisher deals.
- **Level 4:** own the infrastructure.

"Democracy in advertising", read against that brief, means three things:

1. **Short, visible money path.** The advertiser's money reaches the publisher through one visible fee, not a hidden chain.
2. **A small local business can buy on the same terms as a large one,** and cannot simply be outbid.
3. **Nobody's attention is bought with their personal data.**

## Where Proxim already stands

Proxim is a Level 4 build: it owns the whole stack, with no DSP, exchange, SSP or data broker. Its deals work like Level 3 direct deals, sold at a fixed price. Most of the brief's democratic levers are already in the plan or the code.

| Brief's lever | Proxim today | Where |
|---|---|---|
| Cut intermediaries (levels 3 and 4) | Sender → Proxim → partner app. No auction, exchange or data broker in the path. | PLAN 5.1 "no live ad-decision API" |
| Programmatic guaranteed, the brief's "sweet spot": fixed price, no auction | Fixed rate card per place tier and format; no bidding in v1 | PLAN 5.6 "Pricing in v1" |
| Money cannot buy all the attention | Paid value factor capped at 3x the category floor. **New: the device now enforces the cap too** (E-D-RANK). | PLAN 5.6 score `V`; engine spec 7 |
| Small businesses get a fair share | 30% of delivery reserved for small local senders | PLAN 5.1 selection row |
| Publishers keep most of the money | Partner apps get a 65 to 70% share by default (commercial decision) | PLAN 1 |
| Fraud protection without a verification middleman | Attested ingest tokens, replay rejection, spoofed-GPS and canary checks | PLAN M9 |
| Privacy-first contextual targeting, not user profiles | "Billboard principle": an advert depends on where you are and the app's context, never on who you are. No advertising ID. | PLAN 1, invariant BP, I5 |
| Context matching, the "one master creative, localised" idea | Industry taxonomy; each partner app accepts, blocks and prefers categories on the device | engine spec 7.1, phone simulator |
| Edge decisioning (Frost & Sullivan #1) | The whole decision runs on the phone in one shared Rust engine (iOS, Android, web) | `core/` |
| AI creative with human control | AI-drafted wordings, human review of every wording, signed and immutable | Addendum 02 |
| Transparency | "Why am I seeing this" lists the real score contributions. Senders see exactly where and when an advert fires ("Drive it"). | PLAN 5.6, 7 |

One honest point: a 65 to 70% partner share means Proxim keeps 30 to 35%, about the same as the "tax" the brief criticises. The democratic difference has to come from three things:

- **One fee:** a single published fee instead of five hidden ones.
- **No fraud loss:** advertisers do not lose a further 10 to 25% to fraud.
- **Fair access:** the price cap and the small-sender quota.

The fee level itself is an owner decision (below).

## Possible, ranked by effort (the path of least resistance)

**Easy: data or rules on what already exists.**

1. **Publish the fee waterfall.** Every statement shows advertiser paid, partner share and Proxim fee, as one line each. This needs only the planned ledger (M9) and a fixed published take rate.
2. **Small budgets for small senders.** Prepaid credits with a low minimum (PLAN M9 Stripe top-ups). The 3x price cap and the 30% quota stop them being crowded out.
3. **Category relevance per publisher.** Done (spec 7.1).
4. **Civic and community messages** on the same rails, at a lower or zero rate card (PLAN 1 already lists civic senders).

**Medium: small new features on the planned M2 to M9 work.**

5. **A sender API that AI agents can drive.** The brief's "agentic" trend, e.g. an MCP server or something compatible with the Ad Context Protocol. Once the console API exists, putting a typed, documented interface over it is little extra work. Agents get the same fixed prices and limits as people.
6. **Outcome reporting without surveillance.** Promo codes and RSVPs (planned), and aggregate reach from pack downloads. This is the privacy-safe version of "closed-loop attribution".
7. **Live facts in creatives** (price, stock, opening hours) through signed revisions. This is "commerce-aware creative" without per-user data.

**Hard: later stages or partners.**

8. **Digital billboards and transit screens (DOOH) as partner surfaces.** The same engine could run on a screen, which is a fixed location. Needs a partner and a screen SDK.
9. **Self-serve partner onboarding for publishers at scale**, i.e. a marketplace of partner apps.

## Not possible, and why

| Brief's item | Why not | What we do instead |
|---|---|---|
| Behavioural or predictive personalisation; DMPs, CDPs; third-party segments; retargeting | Breaks the billboard principle and I5 (no cross-app identifier); app store rules and state privacy and health laws penalise it; it is the surveillance the thesis rejects | Context and place only; topics the user picks on the device (Preference Center) |
| Real-time bidding, header bidding, OpenRTB auctions | Every auction would send the device's area, heading and app to a server (I2, I3); auctions reward the deepest pockets | Fixed rate card plus price cap; auctions stay a post-pilot option (PLAN 5.6) |
| "Geolocation analytics: who they are and where they came from" | Forbidden by I2 and I3: no coordinates or routes leave the phone | Reach counts at coarse area level, k-thresholded |
| Visit conversion and ROAS in the retail-media style | Would reveal individual visits; never for health classes (PLAN 6.5) | Promo codes, RSVPs, taps, all aggregate |
| Data clean rooms on user data | No user-level data exists to match, by design | Not needed |
| Cross-app frequency control | Apps cannot see each other on a phone; no shared ID by design (I5) | Per-app caps; backend best-fit routing reduces duplicates (phone simulator) |
| CTV, in-game, influencer marketplaces | Outside Proxim's location-triggered product | Not planned |

## Recommended path

1. **M2 signed packs.** Nothing can be delivered to a real phone without signed, revocable content. It is the next milestone and blocks everything after it.
2. **M3 decision kernel extras.** Pacing draw, small-sender quota, driving-safe format, privacy lint.
3. **A thin sender console and ledger, run as one pilot in one town.**
   - Fixed published price and published fee waterfall.
   - A handful of local businesses and civic senders, and 1 to 2 partner apps.
   - Promo-code measurement.
   - The web SDK can go first, because the WASM build already works. Android follows, then iOS.
4. **Agent-ready sender API**, once the console API is stable.

This order uses what already exists (the engine, the simulators, category matching) and puts real money through the shortest path early.

## Changes to phases 1 and 2 made in response

- **Done:** E-D-RANK. The engine caps paid rank at 3x, so the "money cannot buy unlimited relevance" rule holds on the device as well as on the server.
- **Done earlier:** category matching (7.1) and best-fit routing. These are the "contextual targeting" the brief recommends.
- **Unchanged and confirmed:** no auctions, no user data, on-device decisioning, fixed pricing.

## Owner decisions this brief raises

1. **Take rate.** Keep a 65 to 70% partner share, or offer a lower Proxim fee, or a fee cap, for small senders or civic messages?
2. **Publishing the fee waterfall** to senders and partners. Recommended.
3. **Small-sender quota.** Keep 30%? And how is "small" defined?
4. **Whether and when to offer an agent API.** Recommended after the console API.
