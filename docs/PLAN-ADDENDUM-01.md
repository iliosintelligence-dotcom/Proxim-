> **Status: DRAFT - critic review complete, revisions pending (2026-10-09).** Addendum to [`PLAN.md`](PLAN.md) covering the owner's
> expanded requirements: general location-trigger platform (advertising first), selectable map features,
> enter/exit/cross events, notification delivery, and the estate/resort use case.
> The critic found **2 blockers, 12 important and 5 minor issues** (see the end of [`PLAN-REVIEW.md`](PLAN-REVIEW.md)); the text below does **not** yet incorporate them. In particular the resort walkthrough in section 7 and the v1 scope in sections 8 and 11 are known to need changes. Nothing here is legal advice.

# PLAN ADDENDUM 01: From ad network to location-trigger platform

Status: proposed amendment to docs/PLAN.md v1.0 (2026-10-09). Not yet merged into PLAN.md. Nothing here is a legal conclusion; items marked COUNSEL need qualified legal review.

Evidence tags used throughout: [V] verified against a primary document on 2026-10-09; [V-sec] verified only through secondary summaries (Google Play pages were unreachable); [M] measured on real Overture Maps data (release 2026-09-23.1); [MODEL] output of a seeded simulation with assumed GNSS and OS-latency distributions anchored to documented ranges (not field data); [U] unverified. Section 14 collects what is still open.

Where the specialist reports disagreed, this addendum decides and gives the reason inline ("Decision:").

---

## 1. What changes (one page)

**The product.** Proxim stays a device-side decision core that never receives coordinates. It is now described as a location-trigger and contextual-experience platform with four stages: Trigger (a map feature or drawn shape plus an event) -> Gate (consent, caps, safety, benefit) -> Delivery (in-app slot, inbox, or local notification) -> Experience (what opens on tap). Advertising is the first vertical and ships first, exactly as PLAN.md scopes it. A venue talking to its own customers (resort, estate, campus, car park) is the second vertical ("owner mode"), built after the advertising pilot.

**What is the same.** The unit is still the "signpost". One Rust core runs on phones, in the simulator and in the console preview. Packs stay public, keyless and identical for all partners. The server never sees a coordinate. M0 and M1 location modes are unchanged. The SDK still never asks for Always or background location. Health signposts are still in-app only.

**What is new, in plain language**
1. **Pick things on the map.** Senders will be able to pick a business, building, road, junction, car park, bus stop, station, campus, park or neighbourhood, or draw a shape. The console turns each pick into a trigger shape. Gates, entrances and single points are pickable, but the console tells the sender honestly what will actually fire (section 3 and 4).
2. **Events, not just presence.** A signpost can fire on enter, exit, cross (a line, from any direction or one way), dwell and approach. In v1 advertising uses enter and approach only. Cross arrives right after v1 (foreground and partner-fed only, claimed places only). Exit and dwell are for owner mode only until counsel has looked at them.
3. **Notifications, first-party only.** Proxim will post only LOCAL notifications, and only for owner mode (the sender is the partner app's own organisation), service content, general-sensitivity places. Third-party adverts will not be delivered as OS notifications: Apple 2.5.18 and 4.10 and Google's rules are against it [V for Apple; V-sec for Google]. Adverts keep the in-app slot and the inbox.
4. **Private information is handed off, never stored.** A notification and the public guide are generic. Anything about a specific guest (room, door code, booking) is fetched after tap by the owner's own app from the owner's own backend. Proxim never sees identity, location or arrival events.
5. **"Beneficial" is a layered gate, not a model that watches the user.** Declared intent classes, review-time AI on the sender's content, deterministic on-device rules, user controls, and a deny-only veto in the owner's own app. No per-user AI inside Proxim.

**What is not possible (said once, plainly)**
- A single gate, door or point cannot be detected point-precisely in the background. Background means a ring of at least 150 m (Android) or 150-200 m (iOS) that wakes the app minutes late. At 15 m/s the OS typically reports "entered" about 1.6-1.8 km after the boundary [MODEL anchored to V docs]. Gate-level (3-5 s) detection exists only in foreground or partner-fed modes.
- Indoors, floor-level and "inside this building" are not decidable.
- In dense urban canyons, boundaries are uncertain by about 50 m and "which of two close roads" is only 58-68% right [MODEL]. The console labels this instead of promising it.
- Nothing is promised after force-quit or before first unlock.
- A notification already posted cannot be recalled until the app next runs.
- Proxim cannot stop an owner's own app logging arrivals against a customer identity. It can only refuse to provide that channel.

**Sequencing.** Stage 0 (during M0-M2, days of schema and test work): reserve the fields and the detector so nothing breaks later. Stage 1: advertising v1 as planned plus the map pickers that compile to existing shapes. Stage 1.1 (after M13): cross for claimed places. Stage 2 (after M11 field data and a signed design partner): owner mode, local notifications and guides (M15-M18). Later: everything else in section 8.

---

## 2. The platform model: Trigger -> Gate -> Delivery -> Experience

### 2.1 Naming (ADR-001)
- Keep **signpost** as the unit. Capitalised sub-objects: Trigger, Gate, Delivery, Experience. "Sponsored", rate card, CPM and quality score belong to `content_mode = hosted`, not to the noun.
- `content_mode` enum: `hosted` (third-party marketplace content) and `owner` (first-party). Column is `content_mode` (not `role`) to avoid clashing with `org_members.role = owner`. Console labels: "Promote" and "Venue".
- `event` is the geometric transition (present, enter, exit, cross, dwell, approach). `intent` is the benefit class (wayfinding, arrival_service, departure_service, event_info, transactional, safety_info, promotion, advertisement).
- Vocabulary fix: crate `proxim-events` keeps its meaning (telemetry "report events"). Geometric transitions are "trigger events" and live in a new pure crate `proxim-detect` (decision: new crate, not a module, because the simulator and console reuse it and it must stay replayable and dependency-light).

### 2.2 The four stages
1. **Trigger** = shape + event. Shapes: `zone` (circle or polygon), `gate` (oriented transect with inbound side), `point_pass` (closest approach), `road` (today's corridor), `junction` (node plus arm gates, later), `approach` (existing), `sequence` (<= 2 steps; reserved in the format, not built until tests need it). Events: `present` (today's behaviour, equals confirmed enter), `enter`, `exit`, `cross{any|in|out}`, `dwell{min_s}`, `approach`. Each trigger carries `precision_class` (BG_RING >= 150 m, FINE, EXACT), `min_mode` (M1, M2, M3), `max_acc_m`, hysteresis parameters, `cooldown_s`, `max_event_age_s`, `max_event_dist_m`. The console reads allowed combinations from a core function `capabilities(shape, event, mode)`; it never hard-codes reliability tables.
2. **Gate** = ordered pure filters with reason codes. Universal: signature, expiry, seq, revocation, consent receipts, kill switch, child-directed, quality gate, sensitive-zone rules, fidelity. Mode-specific: hosted (partner category and sensitivity policy, caps, V x Q x R x P score) or owner (binding valid, armed, priority then recency). Plus schedule, caps by intent, and the benefit rules of section 6.
3. **Delivery** = ordered channels with fallback, always to a quieter channel: `in_app_slot`, `inbox`, `local_notification`, later `host_callback`. Negotiated against partner policy, OS permission and consent.
4. **Experience** = `card` | `guide` | `deeplink` | `web`, inline or content-addressed. A guide is at most 12 pages of structured blocks (heading, text, image, map pin, link, button, deeplink); no HTML or JS.

`decide(state, fix, packs, policy, consent, clock, seed)` returns `(decisions, state')`; trigger state is explicit input and output so golden vectors cover enter->exit sequences.

### 2.3 Hosted versus owner
Discriminator is a relationship test, not a content test: **OWNER iff sender org == partner org, or an active `owner_binding` exists that both orgs signed and that is bound to the partner app's signing identity** (bundle id or package plus certificate SHA-256, already in `api_keys`). Everything else is HOSTED. Without this test, owner mode becomes a loophole for advertisers.

| Dimension | HOSTED (advertising, v1) | OWNER (first party, v2) |
|---|---|---|
| Sender and receiver | Third-party sender, any partner app | Sender org owns the receiving app |
| Verification | Tiers 0-2 as in PLAN section 4 | Tier >= 1 plus venue place_claim (physical-address proof) plus app signing-identity binding |
| Moderation | 100% human in v1 | Human for the first 3 signposts, any new domain, phone or payment link, any promotion intent; rules plus Claude advisory plus domain-bound links plus secret lint; 5-10% sampled post-hoc; instant revoke |
| Label | SDK-rendered "Sponsored" (or "Public information" for verified civic) | Signed "From <verified owner>"; notifications carry the same text in the OS subtitle; third-party sponsored content inside owner mode is forbidden |
| Distribution | Public, cell-keyed, keyless tile packs | Owner bundle: signed, bound (`aud`) to the partner app, fetched with the publishable key and NO location. Treated as extractable by anyone who has the key |
| Content | Card (headline, body, cta, landing_url) | Card, guide, deeplink, web (owner-domain-bound) |
| Delivery | in_app_slot and inbox only; no OS notification on the roadmap | in_app_slot, inbox, local_notification (service intents first) |
| Events | enter, approach (v1); cross (v1.1, claimed places, foreground/M2, in-app) | adds exit, dwell, directional cross; sequence if needed |
| Selection | S = V x Q x R x P, pacing, diversity | Owner priority, then recency; cooldown and re-arm; no V, Q or pacing |
| Billing | Prepaid credits, CPM, partner rev share | Subscription per venue per month; manual invoices first; no rev share; no per-user or MAU metering |
| Reporting to Proxim | PLAN section 6.2 events, k-threshold, stored-once noise | Zero events by default (the owner's app counts its own taps). Optional aggregate `experience_opened` later |
| TTL | Soft 24 h, hard 72 h | Bundle soft 24 h, hard 7 d; notification posting additionally needs a revocation list younger than 24 h |
| Health classes | Per PLAN section 6.5 gates | Not allowed (R6 and R1-R4 apply unchanged) |

Decisions recorded: owner bundle hard TTL is 7 d, not the 30 d one specialist proposed, because revocation polling happens whenever the app runs and a longer window weakens takedown; the owner may ask to extend after the pilot. Owner-mode reporting is zero events by default (the platform specialist's "exact daily counts" and the abuse reviewer's flight-level aggregates both dropped) because the owner's app already sees its own taps, the subscription price needs no counts, and "posted" is not "seen" (iOS Path S cannot observe delivery at all).

### 2.4 How the privacy invariants and the billboard principle apply

**Billboard principle, hosted (clarified, not changed).** A signpost may depend on where the device is and is heading, the time, the partner app's declared closed-taxonomy context, topics the user chose in the on-device Preference Center (closed non-sensitive taxonomy, visible, resettable, never transmitted), and the user's own suppress actions. A partner may suppress, never select or boost, through declarative flags evaluated inside the core with no value returned to the partner. A signpost never depends on attributes inferred about the user, on health or other sensitive inputs, or on any identifier.

**Billboard principle, owner (new wording, with justification).** A first-party signpost may depend on where the device is, time, and which of the owner's own signposts the owner's app has armed on this device. Proxim never receives, stores, derives or logs who the user is, which signposts are armed, or that a trigger fired. Justification: the principle exists to stop third-party targeting and Proxim-side profiling. In owner mode the sender and the party that knows the customer are the same organisation, and identity never crosses to Proxim. Arming is a list of signpost ids chosen by the owner's app from its own booking state; it is not a user attribute and is never transmitted.

| Invariant | Hosted | Owner | Change |
|---|---|---|---|
| I1 core has no network | applies | applies | none |
| I2 no coordinate or finer-than-r5 cell in any request, log, fixture | applies | applies; notification payloads and deep links carry only signpost_id, rev, event type | **Scope clarified:** privacy-lint covers device-facing routes (/t, /m, /v1/config, /o, /v1/events, /v1/reports, SDK fixtures). Sender authoring endpoints must carry coordinates by definition and are an explicit reviewed allowlist |
| I3 history only on device; trail ring memory-only | applies | applies | **Amended narrowly:** a device-side `zone_state` table is allowed for owner-mode exit/dwell only (see I19) |
| I4 consent fails closed | applies | applies | new purposes (section 5.5) |
| I5 no ad ID, no cross-app identifier | applies | applies; Proxim never holds push tokens | none |
| I6 nothing logged or queued in a sensitive buffer | applies | applies | **Extended** to every notification surface and all classes (I16) |
| I7 health_facility produces no device events | applies | applies | extended: no exit/dwell/sequence trigger on health_facility or prohibited anchors |
| I8 tile fetches carry no app key | applies | applies | **Restated as I8':** a network request never carries both an app key and a location-derived value (tiles: location, no key; config and owner bundle: key, no location). An owner-only app makes zero tile fetches |
| I9 child-directed inert | applies | applies | none |
| I10 unsigned, expired, revoked never rendered | applies | applies at post time, schedule time (Path S) and tap time | extended |
| I11 base artifacts have no background permission | applies | applies | **Extended:** base artifacts also contain no notification-posting code and no POST_NOTIFICATIONS |
| I12 SDK-owned receipt for health | applies | applies | **Extended** to `local_notifications` and `background_location_for_triggers` |

**New invariants (each needs a named test; `check-invariants` fails CI otherwise)**
- **I13 Notification gate.** An OS notification is posted or scheduled only if: mode is owner; class is service (promo reserved, off); sensitivity is general; the signpost is bound to this app; its signature is verified at post (or schedule) time; the SDK consent receipt for the purpose exists; OS permission is fully granted; the device is outside every sensitive buffer; caps and quiet hours pass. Third-party content is never posted by OS-scheduled triggers; only the kernel posts, at wake time.
- **I14 Identity firewall.** Proxim never receives, stores or forwards push tokens, user or guest identifiers, entitlements, arming lists, or per-device enter, exit, cross, dwell or arrival events. No presence-attestation API, no webhook, SMS or email on trigger, and no notification to anyone but the device user.
- **I15 Private content.** No packs or owner bundles contain private content. Bundle bytes are treated as public-by-extraction and linted for secrets.
- **I16 Sensitive buffers.** No notification surface is evaluated, queued or deferred (dropped, not deferred) inside a sensitive buffer. Enter/exit/dwell triggers are refused for zones whose geometry plus hysteresis margin touches a buffer. No `zone_state` is written there.
- **I17 Signed envelope.** Each signpost's signature covers geometry, event types, delivery, text, label key, purpose, audience (`aud`), sensitivity class and expiry.
- **I18 Micro-geometry.** Triggers below the 150 m floor exist only under a verified, feature-bound place claim over a public-access, non-residential feature.
- **I19 Visit state.** `zone_state` is persisted only for owner-mode exit/dwell; 15-minute buckets; default TTL 12 h, hard max 24 h; encrypted, excluded from backup; wiped on consent withdrawal and `eraseAllLocalData`; never uploaded; bounded in count, size and age.
- **I20 Revocation.** Withdrawal, kill switch and revocation remove pending and delivered Proxim notifications on the next run; the Path S exposure window is bounded by its arm horizon.

---

## 3. Selectable map features, data stack, authoring UX

### 3.1 Catalogue (what a sender can pick)
Default trigger names the shape the console produces. "Foreground" means M1/M2 only. Precision is stated for the best mode and for background.

| Element | Data source | Default trigger | Precision caveat | When |
|---|---|---|---|---|
| Business / POI (shop, cafe, gym, hotel, museum) | Overture places (CDLA-P/Apache/CC0 per source); OSM amenity, shop, tourism | zone circle, enter; approach for drive-up | POI points are often frontage or centroid, tens of metres off; closed or moved places (use confidence and operating_status); duplicates | v1 |
| Brand / chain (all sites) | Overture brand, OSM brand tags | group of POI zones, one revision, <= 25 per campaign | Matching imperfect; conquesting a competitor is an owner decision | later |
| Building footprint | Overture buildings (ODbL), OSM building | zone: footprint inflated to the decidable minimum, labelled "at or near" | Indoors undecidable; footprint offsets of metres; house/detached/dwelling classes locked | v1 |
| Building entrance / door | Overture infrastructure entrance, OSM entrance | not standalone; anchors approach and orients the zone | Sparse; entrances closer than GPS error | later |
| Gate / barrier / lift gate / toll booth (incl. estate and resort gates) | Overture infrastructure barrier classes; OSM barrier with access tags | gate (transect across nearest drivable segment, inbound side); in background a ring of at least 150 m | 16-44% of gates in dense cities are not on a drivable road within 5 m and most vehicle gates are service roads [M]; private gates often unmapped; fires 3-5 s after crossing in foreground only | draw-first: v1 (as zone); true cross: v1.1 |
| Address point | Overture addresses (per-source licence) | centres the map and names a place; residential locked | Rooftop versus parcel centroid varies; NAD licence terms unchecked | v1 (search only) |
| Road / street | Overture segments (ODbL), OSM highway | road (corridor, 50 m buffer within 30-500 m, direction locked on one-way) | Parallel, frontage, opposite carriageways: 23-39% of arterial points have another unconnected road within 15 m [M]; urban canyon correct-road 55-65% even with a matcher [MODEL] | v1 |
| Road section between junctions | segments plus connectors | corridor limited to the section; approach to the next junction | Sections under 200 m are too short for heading gating; widened or refused | v1 |
| Junction / roundabout / signals | derived from connectors and OSM nodes, clustered within 30-50 m | approach anchor with sender-chosen lead, or zone of max(150 m, cluster extent + 50 m) | Which arm: 88-100% open sky but 58-68% urban canyon [MODEL]; arms under 25 degrees apart at 3-18% of junctions need 60-140 m lead [M]. Console warns per junction | v1 (anchor); arm/movement gates later |
| Highway exit / ramp | OSM motorway_junction, link ways | approach along the mainline, 0.5-8 km | Ramp versus mainline needs bearing and persistence; late in background at highway speed | v1 |
| Bridge / tunnel / underpass | OSM flags; Overture segment flags | corridor on deck, or approach to the portal; never inside a tunnel | Overpass ambiguity: Chicago 14.7% of drivable length is bridge-flagged [M]; GPS drops in tunnels | later |
| Car park / garage and entrances | OSM parking, parking_entrance; Overture parking | zone (surface); gate at a ramp for garages | Underground has no GPS; use the entrance. First partner segment (parking apps) | v1 |
| Fuel / EV / service area | Overture places, charging_station; OSM | zone or approach | Forecourt centroid; adjacent roads cause false fires | v1 |
| Bus / tram stop, platform | OSM, Overture bus_stop, platform | zone at the effective minimum | Stops on both sides overlap at 150 m; "near", not "at" | v1 |
| Station / terminal / ferry | OSM, Overture station classes | zone or area; entrances as anchors | Underground stations have no GPS | v1 |
| Airport and terminals | OSM aeroway, Overture airport | area (aerodrome) or approach on access roads | Tens of km2; secure zones; terminal-level and indoor not supported | later |
| Campus / mall / stadium / industrial park | OSM landuse, Overture land_use | area simplified to <= 200 vertices | Loose polygons; schools and hospitals locked | v1 |
| Resort / hotel property / theme park / golf / campground | Overture land_use resort, OSM leisure=resort (tourism=resort discouraged) | area where mapped, else POI buffered; enter plus (owner) exit | Many resorts are mapped as a point only; gates often unmapped; estates have no accepted tag (residential=gated was abandoned) | v1 (draw-first) |
| Estate / gated community | mostly sender-drawn; OSM access=private, barrier=gate | owner mode only (claimed) | Boundaries often missing; third-party advertising on residential estates barred | owner mode (v2) |
| Park / trail / playground | OSM, Overture | area; narrow corridor for trails (>= 30 m) | National parks exceed 100 km2 (pick an entrance); playgrounds restricted | v1 |
| Beach, water, marina, rail, level crossing, toll, border, pedestrian crossing | OSM, Overture | beach buffered inland; others later; pedestrian crossing is visual reference only | Crossings are metres wide, below GPS error | later / never standalone |
| Neighbourhood / district | Overture divisions (ODbL) | area, <= 100 km2 | Informal boundaries; copy says "people in this area" | v1 |
| City / county / state | divisions | refused above 100 km2 with "pick a smaller part" | Exceeds pack budget | later |
| Hospital, clinic, pharmacy | OSM, Overture, NPPES | locked for general senders; verified tier-2 only, approach 1-8 km, in-app, sandbox | R1-R6; registry incomplete | gated (PLAN 6.5) |
| School, childcare, worship, shelter, immigration | OSM, Overture | visible but locked; shelters never in the pick layer or packs | R4; confidential-address kinds use coarse over-approximated barred cells (about 5 km2, H3 res 7, [U] confirm with h3o) | locked |
| Custom circle / polygon / line / route | terra-draw; Valhalla snap for "snap my rough line" | zone / area / road / gate | Below the effective minimum is widened and the sender is told | v1 |
| Imported shape (GeoJSON, GPX, KML) | client-side parse, server revalidation | by geometry type, reviewed on the map | <= 2 MB, <= 5,000 vertices, no KMZ or shapefile, NetworkLink ignored; no address CSV | v1 |

### 3.2 Data stack (decision)
- **Reference layer:** Overture Maps, pinned release, loaded per metro bbox through DuckDB into a PostGIS `ref` schema (`ref_features`, `ref_junctions`, `ref_search`). OSM PBF is used only for Valhalla. Every pick stores a `feature_ref {source, id, version, release}` plus a geometry snapshot, never a bare coordinate. Why Overture-first: one schema, GERS ids that persisted for 99.70-99.96% of drivable segments over five weeks [M], a per-feature version for drift checks, and OSM tags preserved in `source_tags` (access=private survives).
- **Basemap:** Protomaps PMTiles cut to the pilot region, styled with protomaps-themes-base, loaded through the pmtiles protocol in MapLibre. Basemap tiles are display only: they omit tags and MapLibre returns clipped, duplicated tile fragments [V].
- **Pick layer:** a separate ID-bearing PMTiles (gid, class, name, flags) for hover and hit-testing. On click the console calls `GET /v1/ref/features/{gid}` for the authoritative unclipped geometry. `POST /v1/ref/at` returns the ranked stack of features under a point for disambiguation and for the keyboard "Nearby" list. Geometry read from a tile is never persisted.
- **Search:** Postgres `ref_search` (pg_trgm, prefix, unaccent) over places, addresses, divisions, road names and a generated intersection index ("Main St & 5th Ave"). Results carry a feature_ref. Gate: a 200-query labelled corpus with top-3 >= 95%; if it fails, add Photon behind the same API. No Google, Mapbox or Apple geocoding or maps (their terms conflict with storing derived geometry and shipping it in public packs [V-sec]). No public Nominatim or Photon demo in production.
- **Snapping:** Valhalla (MIT) only for "snap my rough line" and "route A to B". Roads are picked from reference features directly, not through the router. Record the extract date and way ids.
- **Geometry code:** buffering, simplification, corridor, tripline derivation, junction clustering and H3 covers run in the Rust/WASM core in console, API and pack compiler (decision: not geo-ts, which keeps rendering helpers only), with golden vectors. This removes critic finding 14's dual-implementation risk.
- **Pipeline:** `tools/mapdata` is deterministic for a pinned release and bbox and emits ref tables, pick-layer PMTiles, search table, intersection index, a per-class coverage report and a licence manifest. Run a coverage audit (gates, entrances, parking, transit, resorts) per candidate pilot metro before promising any class.
- **Stable references:** the signed authored geometry is the authority. A `map_ref` block (GERS ids plus release, OSM way ids plus changeset, 2-4 OpenLR-style location reference points, geometry SHA-256) drives a re-validation job per Overture release. Drift (> 15 m, class or direction change, missing id, closed place) produces a proposed new revision that re-enters review. Live geometry is never edited silently. Packs never depend on map ids at runtime.

### 3.3 Licensing posture
- Overture base, buildings, divisions and transportation are ODbL; places are not [V]. OSM is ODbL. Attribution: "(c) OpenStreetMap contributors, Overture Maps Foundation", plus per-source CC BY duties (Esri, Google Open Buildings, geoBoundaries) [V].
- Whether public packs of derived trigger shapes, and especially any road-graph context, are a "Derivative Database" (share-alike) or a Produced Work is **COUNSEL**. Conservative posture that costs little because packs are already public: record lineage (`feature | drawn | imported | snapped`) and `licence_class` per shape; `attributions[]` in the pack manifest, the console map, a Data sources page and the SDK "Why this?" sheet; keep the ref database internal; publish on a schedule an open extract of trigger geometries whose lineage includes ODbL features; CI fails if a data source is not registered with a licence class.
- Tracing a rendered ODbL map may still be derivative (COUNSEL).

### 3.4 Authoring UX and campaign builder
- **Picker.** First screen "What do you want to pick?": a business or place, a building, a road, a junction, a gate or entrance, a car park, a bus or train stop, an area (campus, park, resort), a neighbourhood, or draw it. The chip filters the pick layer. Hover highlights; click opens a popover (name, type, address, "Use this"). Overlaps become a stack ordered entrance/gate, building, place, road, junction, area, neighbourhood, with "select parent". Targets >= 24 x 24 CSS px (WCAG 2.5.8).
- **Anchor, Shape, Event are separate.** Anchor = what was picked (with provenance). Shape = what fires (from a versioned default recipe in the Rust core, editable; hand edits mark it "detached from map feature" but keep the anchor for drift checks). Event = enter, leave, pass through (either way, in only, out only), stay, approaching.
- **Recipes (core functions).** Building -> footprint inflated by max(0, k x acc_p90 - inradius). Gate -> transect across the nearest drivable segment, width = road width + margin, inbound side from the road graph or a property polygon; refuse or warn if no drivable road within 10 m (pedestrian gate). Point -> point_pass with radius >= 25 m walking, >= 50 m driving or urban, >= 100 m for any urban promise. Junction -> approach anchor, or arm gates at lead d >= max(30 m, 3 sigma / (2 sin(theta/2))). Road -> corridor with direction arrows every about 150 m; dual carriageways are two corridors.
- **Honesty built in.** Every trigger shows its precision label (Gate-level / Block-level / Area-level) and per-mode detectability before submit: M1/M2 yes or no; M3 ring size and expected fire distance. Unsupported event and mode combinations are disabled with the reason, not hidden. "Drive it" runs the real WASM core in a Web Worker with noise profiles (open sky, suburban, urban canyon) and OS profiles (foreground, Android background, iOS background), 200 seeded runs, and shows detection probability, false-fire list from auto-generated decoy routes, latency and fire distance, with the text "simulated; real devices vary" and the engine build hash. Same code as the SDK, so M7's "Drive-it equals CLI" gate extends to it.
- **Guardrails as data (`packages/policy`, enforced identically in UI, API validator and DB constraints).** Minimum effective size by tier; corridor length >= 200 m and width 30-500 m; <= 500 drawn and <= 200 compiled vertices; valid, non-self-intersecting polygons; area <= 100 km2 (over-limit gets "pick a smaller part"); <= 25 triggers per campaign; <= 20 geometries per signpost; <= 200 active signposts per verified org per r5 cell (20 for tier 0); <= 25 micro-geometries per org per cell. Private-home lock: building classes house, detached, dwelling_house, bungalow, semidetached, address points, and service-road subclasses driveway, parking_aisle, alley are not targetable by third parties. "Auto-fix" buttons instead of bare errors.
- **Multi-trigger campaigns.** Reusable named `zones` with immutable `zone_revisions`; `rules` = zone_revision x event x experience x delivery x schedule, wrapped by the existing `campaigns` row. Wizard: Where, When, What, Test (Drive it), Review. Recipes: "Welcome and thank you" (enter plus leave, owner), "On the way" (approach then arrive), "Gate pass" (cross inbound), "Event day" (area plus schedule). A cap-budget view warns that PLAN section 5.6 caps can silently suppress a second message; overlapping rules warn. Reserve `group_id` and `priority` in the pack format at M2; v1 rules are independent.
- **Accessibility (console rules).** terra-draw has no keyboard vertex placement [V], so we supply: search to result list to "Use this place" then numeric radius; a "Nearby features" listbox; a coordinates-table editor; MapLibre KeyboardHandler enabled; drag alternatives (WCAG 2.5.7); aria-live summaries of geometry and Drive-it results. Wrap terra-draw behind our own `GeometryEditor` interface and pin the version. Phones: pick plus slider first-class; polygon editing desktop-first with a handoff link.

---

## 4. Events

### 4.1 Definitions and state machine
All events are produced by `proxim-detect`, a pure function `detect(state, fix, triggers, clock) -> (state', events[])`. Fix = position, accuracy_m, speed, course, speed_acc, timestamp, flags. Output TriggerEvent = trigger_id, rev, event, direction, confidence (CONFIRMED, INFERRED, OS_ONLY), ts_bucket. Geometry is stored as fixed-point integers in a per-cell local plane; side-of-line, segment intersection and point-segment distance use exact integer arithmetic; only the lat/lon projection uses libm, once per cell. The matcher, if built, uses log-space costs with +, -, *, / and sqrt only, ties broken by (cost, edge id). This answers critic finding 1 for the detector.

Starting defaults (tune in the simulator; all are data):
- `acc_eff = max(reported accuracy, 10 m) x kappa`; kappa 1.0, inflated adaptively when fix-to-fix residuals versus Doppler speed exceed 2x reported accuracy.
- **ENTER** (OUT -> IN): signed depth `d >= k_in x acc_eff` (k_in 1.0; 1.5 for FINE zones) for N_in = 3 consecutive accepted fixes spanning >= 3 s (>= 10 s if speed < 1.5 m/s). Never on a fix whose accuracy circle straddles the boundary. A zone whose inradius < k_in x acc_eff is `undecidable` (reason ACC_TOO_COARSE) and never fires. Re-arm only after confirmed EXIT plus cooldown (default 30 min).
- **EXIT** (IN -> OUT): `d <= -k_out x acc_eff` (k_out 1.0, sticky 1.5 for FINE) for N_out = 5 fixes or >= 10 s. Hold state through fix gaps up to 120 s; no fix for 15 min resets silently to OUT without firing. EXIT is valid only after a valid ENTER or a reconciled IN. OS exit callbacks reconcile but never override a fresh confident inside fix.
- **CROSS** (gate, any direction): oriented segment with half-width = road half-width + margin 4-15 m. Fires when a confident fix on one side (|s| >= k x acc_eff) is followed within T_gap (30 s; 10 s above 10 m/s) by a confident fix on the other side and the interpolated intersection lies within half-width + k x acc_eff. Emits direction in or out. Each direction has its own cooldown (default 10 min). Directions are "any", "in only", "out only"; "in" means toward the authored inbound side.
- **PASS** ("cross that point from any direction"): the interpolated track comes within `r_pass` of the point (segment-to-point distance so sampling gaps do not matter).
- **DWELL**: continuous IN for >= T_dwell (>= 2 min, <= 6 h), once per visit; evaluated conservatively from the bucket end after a restart.
- **APPROACH**: the existing plan semantic (along-route distance enters [d_min, d_max], monotonically decreasing over >= 3 fixes, speed and bearing gates), re-armed after the distance exceeds d_max x 1.2 or the anchor is passed. Anchor may be a zone, gate or junction.
- **ARM_ENTER / MOVEMENT** (junction): directed crossing of an arm gate at lead d_arm confirmed by a bearing-gated corridor (v1) or a local matcher (v1.1); MOVEMENT (a -> b) is ARM_ENTER(a) then a crossing of outbound arm b within 60 s. Foreground only.
- **SEQUENCE**: at most 2 steps, same sender, window <= 30 min. Mainly a precision tool (ring, then gate within 120 s separates drive-in from drive-by). Cross-sender or long-window sequences are visit-history profiling and are not allowed. Reserved in the format; built only if tests show a need; owner mode only.

Persisted visit state (owner exit/dwell only): table `zone_state(trigger_id, rev, state, since_bucket_15m, last_fire_bucket, seq_step, ttl_expires)`, written only on change, keyed by trigger_id so H3 cell swaps do not duplicate enters. On every OS wake or cold start run `reconcile()`: ask the OS (iOS requestState, Android initial trigger disabled for firing), take one accurate fix, apply the detector, and drop any late event older than 2x the p95 latency of the OS profile. Hosted enter/cross need only in-memory counters (seconds to minutes), so no persisted state is added for advertising.

### 4.2 Reliability by location mode

| Event | M1 foreground / M2 partner-fed (1 Hz fixes) | M3 background (OS tripwire as wake hint, then verify) |
|---|---|---|
| ENTER zone | Declared about 3-5 s (about 50 m at 15 m/s) after the boundary. Tangential drive-by false enter with the rule: 2.8% open, 5.2% suburban, 19.4% urban (naive 53-60%) at 0 m clearance; <= 1.4% at 50 m clearance [MODEL]. Stationary 25 m outside 10 min: 0% open, 2% suburban, 34-69% urban canyon [MODEL] | Android: reported usually < 2 min, 2-3 min on average, up to 6 min stationary [V]. iOS: boundary + about 200 m cushion + 20 s, 3-5 min average, about 10 s wake (up to about 30 s with a background task) [V]. At 15 m/s the OS reports about 1.6-1.8 km past the boundary (p95 about 5 km); walking median 170-290 m [MODEL]. Ring on a public road also fires for drive-bys, so the geometry check on wake decides |
| EXIT | Sticky; 5-10 s later than the true exit. Flapping on an edge cut from 44-59 to 1.2-3.2 transitions per 10 min [MODEL] | Reported about 1.8 km after leaving at 15 m/s (p95 about 5 km) [MODEL]; weakest event. "Goodbye at the gate" is unachievable; "thanks for visiting" within minutes is |
| CROSS gate | Detection 100% open, 99.6-99.7% suburban, 89.9-95.4% urban at 1 Hz; 82-84% urban at 5 s fixes. False cross from a public road 20-40 m away <= 0.2% open and suburban, 0.7-5.2% urban [MODEL] | Not available point-precisely. Becomes ring wake then bounded fix then check against the real geometry, which yields "arrived at the grounds", not "crossed the gate" |
| PASS point | r = 25 m: 100% if the track passes within 12 m (open sky); urban canyon driving 79% even through the point and 14-34% false at 2r; r = 50 m urban 98% with 2-10% false; r = 100 m clean [MODEL] | Ring >= 150 m then verify |
| DWELL | Good for T >= 2 min outdoors at accuracy <= 20 m; indoors drift causes false exits | Android best effort (loitering delay, extra 2-6 min); iOS has no native dwell, only if a later wake occurs. Not promised |
| APPROACH | Per PLAN tiers P1-P3 | Ring radius R = cushion + v x (T_wake_p95 + T_ready + T_lead): for a 90% chance of waking and locking before the anchor: about 0.4-0.5 km walking, 2.2 km at 8 m/s, 4.1-4.2 km at 15 m/s, 7 km at 25 m/s [MODEL]. Highway speed in the background only works for far anchors |
| Junction arm / movement | 88-100% open/suburban, 58-68% urban canyon (exact edge at 50 m before and after) [MODEL on real graphs] | Not available |

### 4.3 Honest minimum radii and latency (what the console shows)
- **Foreground and partner-fed:** zone inradius >= k_in x acc_eff (practically >= 25-50 m); point_pass >= 25 m walking, >= 50 m driving or urban, >= 100 m for any urban promise; gate setback >= 30-40 m from any other public road; detection 3-5 s after the true crossing.
- **Background:** radius >= 150 m on Android (docs say 100-150 m) and >= 200 m recommended on iOS (Apple staff: 50 m is too small, try 150-200 m; rural may need 1 km) [V]; honest promise is "welcome within about 2-3 minutes of entering"; before-the-gate only with a 4-5 km approach ring that spends a scarce slot and extra wakes.
- **Always undecidable:** indoors, floor level, approximate-location grants, iOS Precise Location off (region monitoring is documented unavailable under reduced accuracy [V]). Reason codes REDUCED_ACCURACY, ACC_TOO_COARSE, BACKGROUND_PERMISSION_MISSING appear in `status` and the TestKit inspector.
- **Region budgets:** iOS 20 regions and Android 100 geofences per app, shared with the host app [V]. One ring per trigger; hysteresis lives in the engine, not in extra rings. Owner-mode rings are reserved first (a handful per venue); hosted approach rings take the remainder. Per-day wake cap. Ring sizing uses the formula above for every ring including the re-anchor ring (this replaces PLAN section 5.7 step 1's density-based 1.5-5 km rule and answers critic finding 13).
- All OS numbers are data tagged per OS release and replaced by field measurements at M4/M11.

### 4.4 Roads and junctions: how precise, and what it costs
- **v1 (decision):** authored geometry only: road chain plus buffer, bearing and speed band; direction gating (the plan's P1) already reaches 94-98% correct-road fixes in open sky [MODEL on Overture graphs]. The console computes at authoring time (from the open graph) the per-junction lead requirement and the parallel-road exposure and shows them.
- **v1.1, pending counsel:** optional local road context per EXACT signpost (<= 30 candidate edges within 300 m, about 0.6-1.5 KB gzip each, cap about 100 per cell, about 100 KB gzip per cell) enabling a small online HMM matcher. It adds 2-4 points in open sky plus overpass and decoy handling; it does not rescue urban canyons (62-64% correct-road even with the matcher) [MODEL].
- **Not doing:** a global road skeleton in packs. A whole compact graph costs 0.27-1.28 MB gzip per dense r5 cell [M] against the 350 KB tier-0 budget. PLAN section 8's deferral stands.
- **Targets replaced:** PLAN section 5.5's single "dense-urban false-fire <= 15%" becomes per-profile: open sky correct-road >= 94%, suburban >= 85%, urban canyon "not promised" (reported, with route intent P2 or a confirming sequence required for exact-road claims).
- **Pack cost accounting:** road context counts inside the tier budget, is a separate ODbL-attributed layer, and CI fails above about 1.5 KB gzip per signpost or about 100 KB per cell on a dense-metro fixture.
- **Parallel and multi-level roads are common** [M]: opposite carriageways, ramps, frontage roads. Direction gating, persistence and route intent are what remove them; the console shows which decoys the preview found.

---

## 5. Notification delivery

### 5.1 Local versus remote (decision)
**Local only.** Proxim never runs APNs or FCM and never holds a device token. Remote push for location needs either coordinates at the server (breaks PLAN section 5.1 and I2/I3) or a per-install token (a device identifier, breaks I5), adds latency, and fails offline in resort dead zones. Revocation is pull-based. An owner that already knows its customers' locations may run its own push outside Proxim; a later optional hook `Proxim.notifications.present(signpostId)` would let that push use the same signed shell, caps and label. Deferred until after the pilot.

**Hosted adverts as OS notifications: not built.** Apple 2.5.18 (display advertising not in notifications), 2.5.16 (notifications relate to the app's content), 4.10 (no monetising push), 4.5.3 and 4.5.4 (no unsolicited promotion; explicit in-app opt-in plus opt-out) [V, guidelines dated 2026-06-08]. Google Play: system notifications only for an app's integral features; ads in system notifications are a listed violation; ads display only inside the serving app [V-sec; primary pages unreachable]. Google ended Nearby Notifications in 2018 as spam [V-sec]. The policy matrix row hosted x any OS notification is `deny`, enforced by a build flag and by signed partner config. It is reopened only with written pre-clearance from both stores plus counsel; no engineering is spent before that.

### 5.2 Per-OS mechanics

| | Android | iOS |
|---|---|---|
| Wake source | `GeofencingClient` transition -> PendingIntent -> non-exported manifest receiver (PendingIntent FLAG_MUTABLE from Android 12 so Play services can add extras) -> Rust core -> post. ENTER/EXIT; `INITIAL_TRIGGER_ENTER` disabled so arming while inside does not fire | **Path D (default):** region monitoring via `RegionBackend` (legacy CLCircularRegion now; CLMonitor behind the same interface, because Apple marks the legacy API with a deprecation path [V]) -> app relaunched with about 10 s (up to about 30 s with `beginBackgroundTask`) -> core -> post. Requires Always. **Path S (optional, flagged):** `UNLocationNotificationTrigger` needs only When-In-Use plus notification permission because the system monitors the region [V], but content is frozen at scheduling time and no fire-time check runs |
| Limits | 100 geofences per app; radius 100-150 m minimum; re-register on boot, app replace, data clear, `GEOFENCE_NOT_AVAILABLE` [V] | 20 regions per app (shared with the host) [V]; 64 pending local notifications [V-sec]; no events under reduced accuracy; no wake if Background App Refresh is off [V]; monitoring resumes only after first unlock [V] |
| Latency | Usually < 2 min; 2-3 min average under background limits; up to 6 min stationary [V] | 3-5 min average [V] |
| Verify on wake | Use the transition's triggering location plus one bounded `getCurrentLocation`; a foreground-service burst is optional. A geofence event is an exemption from the background-FGS-start ban [V], but Play approvability of an FGS for geofencing is [U] (policy page dates conflict: 2026-08-26 versus 2027-01-27). Design the fallback as OS-callback-only coarse events (`os_only`) | One `requestLocation` fix within about 8 s; require accuracy <= 100 m and distance to shape <= radius + 100 m; if no fix arrives, post only if the signpost allows "trust the OS event" and its radius >= 300 m, else drop |
| Permissions | `POST_NOTIFICATIONS` (API 33+), `ACCESS_FINE_LOCATION` and `ACCESS_BACKGROUND_LOCATION` (users grant it on a Settings page after education UI), Play background-location declaration with video. No lower-permission path for app-closed delivery. A foreground-only grant means no app-closed delivery. Approximate-only grant means geofencing unavailable | Path D: Always (requested once, after When-In-Use). Path S: When-In-Use. Notification permission in both. Provisional authorization delivers quietly, no prompt: allowed for service class only, never for promotion |
| Posting | Direct Activity PendingIntent via `setContentIntent` (no trampoline, Android 12+ [V]); two channels: "Arrival notices" (IMPORTANCE_DEFAULT) and "Arrival notices (quiet)" (IMPORTANCE_LOW); never IMPORTANCE_HIGH; `setTimeoutAfter(expiry)` | Interruption level passive (default) or active (stationary, Path D only); never timeSensitive or critical in v1; thread id per venue |
| After tap | Verified `SignpostTap` handed to the host | Same |

Nothing is promised for force-quit or force-stop, reboot before first unlock, or devices without Play services. An unconfirmed single report says iOS 26 stopped relaunching force-quit apps from region events [U]; Apple DTS says region monitoring is an exception [V-sec]. Default to the legacy path; device matrix on every OS beta.

**Path S is not in the pilot's first cut.** It bypasses the Gate at fire time (no fresh consent, cap, revocation or sensitive-zone check), cannot observe delivery, and is reported to fire immediately after Wi-Fi location errors. When added (after device tests, behind a flag): owner mode only, `repeats=false`, radius >= 200 m, generic evergreen passive text, at most 8 requests, armed at each app run with a 24 h default horizon (72 h hard maximum), pre-gated at schedule time, cancelled on withdrawal or revocation poll, verified again on tap. Path D is the default wherever the partner holds Always.

### 5.3 Wake procedure (identical Rust for both OSes, zero network)
1. Map the request id to a signpost in the local verified store; ignore Intent extras and userInfo.
2. Re-verify JWS, kid, rev, expiry, seq, revocation list freshness (service class fails closed past 24 h; promotion, if ever enabled, past 6 h), partner kill flag.
3. Consent receipts for `location_matching` and `local_notifications`, OS notification status, class and sensitivity rules, sensitive buffers (drop, never defer).
4. Confirm presence with the verify-on-wake step in 5.2; apply the detector against the real geometry.
5. Caps from the local ledger and `zone_state`.
6. Speed gate: speed above about 2.5 m/s or unknown means quiet channel or passive, no sound, no screen wake, no action needing input. Speed on iOS Path S is unknowable, so it is always passive.
7. Build content only from signed fields; call the host veto if configured; post.
8. Write a local ledger row only. No network call.
Wake budgets: iOS about 10-30 s, Android receiver well under 10 s (hand longer work to WorkManager; do not start an FGS just to post a notification).

### 5.4 Caps, quiet hours, labelling, health
- **Caps (decision, merged from four proposals):** arrival and departure intents: one per trigger per 6 h re-arm (and one per visit), at most 2 per day per app; at most 3 notifications per day per app across intents; per-place cooldown 4 h to absorb flapping; exit hysteresis >= max(100 m, 2x accuracy); partners may only lower. Safety information is exempt from quiet hours but not from caps. Quiet hours: service class is never suppressed but is forced to quiet/passive between 22:00 and 07:00 (guests arrive at night); a promotion class, if ever enabled, is suppressed 21:00-08:00. Notifications auto-expire (arrival 30 min; departure dropped if > 20 min late or > 5 km away). Caps remain client-trusted and reset on reinstall; server-side partner anomaly checks (notification taps per active install) back them.
- **Labelling.** The OS shows the partner app's icon and name, so the notification cannot honestly be "from" anyone else. Owner notifications put "From <verified owner>" in a fixed subtitle/subText slot rendered by the SDK from the signed sender identity (`NotificationChrome`, non-editable). That is truthful only because sender org == partner org is enforced at authoring, signing, compile and in the kernel. Notification text carries no URL, no digit run of 4 or more, no rich text, no remote image; NFKC-normalised with bidi and zero-width controls stripped; alert-style emoji and a security/verify/OTP/emergency lexicon blocked; the title format is renderer-built.
- **Lock-screen safety.** Text is generic and location-neutral by construction ("Welcome to Pine Ridge. Tap for your arrival guide."): no name, room, code, health or condition content. iOS cannot force preview hiding; it is a system setting [V], hence the design rule.
- **Health and sensitive (R6 made absolute).** health_adjacent, health_facility and prohibited_anchor signposts never produce an OS notification, lock-screen exposure, App Clip or ephemeral notification, in any mode, including a facility messaging its own patients. Sealed or health-category partner apps emit no location-triggered notifications of any class (the app icon plus timing reveals context). Inside any sensitive buffer all notification surfaces are dropped for all classes. Hospital, clinic, mental/reproductive/substance health, shelters, worship, schools, childcare, immigration, correctional, union and military venues are excluded from owner-mode notifications; they may use user-initiated flows (QR, "I have arrived") outside the geofence machinery. COUNSEL: Washington's statute names notifications explicitly [V-sec].
- **Driving.** Distracted-driving and product-liability exposure is COUNSEL. PLAN section 5.5's "taps disabled while moving" cannot apply to an OS notification; "post silently while moving" replaces it. No CarPlay or Android Auto surfaces.

### 5.5 Consent model
- Purposes (separate, default-off): `location_matching`, `aggregate_reporting`, `nearby_health_events` (unchanged); `local_notifications` (id kept; redefined as the user's agreement, in the partner's own UI, to location-triggered OS notifications from this app, with qualifier class=service; distinct from `location_matching` and from the OS notification permission, and all three are required); `background_location_for_triggers` (the user's opt-in to arrival assistance in the partner's UI, recorded with the SDK receipt; the OS prompt and Google's prominent disclosure remain the partner's); `promotional_notifications` reserved, off, not built (Apple 4.5.4 needs explicit opt-in language plus an in-app opt-out).
- The SDK-owned `ProximConsentSheet` and device-bound signed receipt become mandatory for `local_notifications` and `background_location_for_triggers`, not just health (FTC InMarket, Gravy and Kochava orders point at SDK vendors' consent verification [V-sec]). The sheet states what location is used, that it stays on the device, what triggers (enter, exit, dwell), the caps, and how to withdraw; decline is as prominent as accept; withdrawal is one tap and clears pending and delivered notifications, outbox and unregisters OS regions.
- Apple 5.1.2(i) and 4.5.4: notifications and location must never be required to use the app; the partner keeps an always-available in-app "Arrival guide".
- The SDK never calls `requestAlwaysAuthorization` or requests `ACCESS_BACKGROUND_LOCATION` (PLAN sections 5.3, 5.7, 7 stay literally true). Features are tiered by what the partner chooses to hold; `proxim doctor` states the tier.

### 5.6 SDK API (experimental in v1; every symbol is a promise)
Seven symbols, no coordinates, no identifiers, no user attributes:
`Proxim.notifications.configure(presenter, hostVeto: nil)`; `.status` (stream: needsOsPermission, needsLocationPermission(level), reducedAccuracy, paused(consent), armed(count), degraded); `.requestOsAuthorization(provisional: false)` (partner-called only); `.arm(signpostIds:until:)`; `.disarmAll()`; `.taps` (stream of verified `SignpostTap`); and `Proxim.consent.set(.localNotifications(.service), granted:)`. Kotlin mirrors it (`ProximNotifications.configure/status/requestOsAuthorization/arm/disarmAll/handleTap`). Notification code ships only in the optional artifacts (`sdk-background` / `ProximBackground`, or a small `ProximNotifications` product); base artifacts declare neither POST_NOTIFICATIONS nor background location (extends I11, manifest-merge CI test). The only telemetry for notifications is nothing: owner mode emits no events (section 2.3); the host counts its own taps.

---

## 6. The "beneficial" gate (layered)

Owner requirement: a notification should only be shown if the app or the AI has decided it is beneficial to the user. Resolution: five layers; none builds a user profile; there is no runtime AI on user data inside Proxim.

| Layer | What it decides | May use | May NOT use |
|---|---|---|---|
| L1 Intent whitelist (signed, deterministic) | Which channels are even eligible. Notification-eligible: wayfinding, arrival_service, departure_service, event_info, transactional, safety_info. Promotion only in owner mode with `promotional_notifications` consent (later). Advertisement: never | The signpost's own declared intent, mode, class, place-derived sensitivity | Sender-declared sensitivity (class comes from the place registry) |
| L2 Authoring-time review (human plus Claude advisory) | Is the copy specific, timely, useful at that place, free of urgency tricks and condition implications, lock-screen safe, not promotion in disguise | Sender content only, as data (structured output, no tools, content delimited, fail closed on outage) | User data of any kind. Model output never edits content or adds signposts |
| L3 Deterministic on-device situational rules | Place-bound (radius <= 2 km), time-bounded or per-visit; non-redundant (suppress if the owner's app is foreground, already delivered this visit, or stale per `max_event_age_s` and `max_event_dist_m`); not at speed unless passive and title-only (<= 7 words) or held until stationary with a 10 min expiry; quiet hours; caps | Location, heading, speed, time, partner-declared context, the owner's arm list on device | Any inferred user trait, history beyond `zone_state`, ad IDs |
| L4 User control (on device) | Per-intent toggles in a drop-in Notification Preferences view, a "Turn off these alerts" notification action, OS channel mute, dismiss/hide/report | The user's explicit actions, stored locally | Positive interest learning (P4 stays off; negative-only suppression is allowed) |
| L5 Owner-side veto (owner mode, v2.x) | Deny-only `hostVeto(decision)` inside the owner's own app, 200 ms budget, default-deny on timeout, receives signpost id, intent, distance bucket, never coordinates; cannot raise caps or bypass the Gate | The owner's own data about its own customer, held by the owner | Not available in hosted mode (a callback would tell the partner which category of place the user is near: a side channel). Hosted partners get declarative suppression-only flags evaluated in the core with no return value, disabled for sealed or health partners |

Aggregate feedback (mute and opt-out rate) drives server-side auto-pause and owner warnings. Marketing and "Why this?" copy states the real logic (place, time, app context, topics you chose) and never says "AI chose this for you". Per-guest AI personalisation, if the owner wants it, happens inside the owner's app on the owner's data.

---

## 7. Worked example: a resort, drive-in, notification, guide, private information

Setting: "Pine Ridge Resort" has its own app, Android and iOS. A guest with a booking drives in along a 3 km private access road to a main gate, then to reception.

**Console setup (resort staff)**
1. The resort org completes verification tier >= 1 (domain DNS, business), claims the venue as a place claim with a physical-address proof (postcard or PIN letter), binds its app (bundle id or package plus certificate SHA-256), and is checked against the venue exclusion list (not a covered-entity provider, treatment centre, senior-care campus or R4 category). First venue reviewed manually.
2. Venue wizard: search "Pine Ridge Resort". If Overture has a resort land-use polygon it is offered; otherwise the staff draw the grounds. They pick the main gate from the map (Overture barrier class gate). The console resolves it to a point, nearest drivable segment, direction and inbound side, and warns if no drivable road is within 10 m.
3. Rules: Arrival = enter(grounds polygon, inflated to the background minimum), intent arrival_service, local_notification with inbox fallback; re-arm 6 h; expire 30 min. Optional Approaching = ring of about 4 km, passive, off by default (costs a slot). Departure = exit(grounds), owner-only, passive, dropped if > 20 min late or > 5 km away. Gate welcome = cross(in) at the gate, in-app card, foreground or partner-fed only.
4. Copy: title "Welcome to Pine Ridge" (<= 40), body "Tap for your arrival guide." (<= 90). No names, room numbers or codes; the secret linter blocks digits, "password", "wifi", "code", "PIN", tokens in URLs.
5. Experience: a guide (Welcome, Reception and parking pin, Wi-Fi network name only, Dining hours, Activities, Emergency contacts) plus a button "Open my reservation" that is a deep link into the resort's app.
6. "Drive it" shows: M2 or foreground: gate-level, fires 3-5 s after the gate; background: grounds-level, expected report median about 1.7 km inside the grounds at 15 m/s, p95 about 5 km, 2-3 min after crossing the boundary; iOS cushion about 200 m; urban noise profile not relevant at a rural site. The preview states "not promised after force-quit". A lock-screen mock shows the text.
7. Submit. Automated checks (domain-bound links, Web Risk, secret/PII lint, AI benefit rubric); the first three signposts get human review. The worker compiles an **owner bundle** (signed ES256, `aud` = the resort app, geometry inline, indexed internally; text <= 1 MB gzip, assets <= 4 MB [U, targets]). The rings are NOT in public tile packs.

**Device**
8. Booking created in the resort's backend. The resort app (not Proxim) decides: `Proxim.notifications.arm([arrival, departure], until: checkout + 1 day)`. The arm list is stored encrypted on the device and never uploaded.
9. The app fetches the owner manifest with its publishable key and no location, verifies the JWS, caches bundle and assets. The guest opted in to "Arrival assistance" in the resort's own UI, which also triggered the SDK consent sheet receipt, the OS notification prompt and the Android background-location flow (resort-owned prompts, Play declaration and video).
10. The tripwire planner registers the grounds ring from reserved owner slots.
11. The guest drives in. At 15 m/s the OS wakes the app about 2-3 minutes after the boundary, about 1.7 km along the access road (Android; iOS 3-5 min). The core verifies the signature and revocation list (< 24 h old), reads consent receipts, takes the triggering location plus one fix (accuracy <= 100 m), confirms inside the polygon, and runs the Gate. Speed is above 2.5 m/s, so it posts to the quiet channel (passive on iOS) with no sound and no screen wake. Subtitle: "From Pine Ridge Resort". If notifications are off, an inbox banner shows on next open.
12. Tap: a direct PendingIntent (Android) opens the host with signpost_id, rev and event type only. The SDK re-verifies signature and revocation, then the guide renders from cache (works offline). "Open my reservation" navigates inside the resort app.
13. The resort app authenticates the guest with its own session and loads room, folio and door instructions from the resort's backend. This is the first moment anything guest-specific exists, and it is entirely outside Proxim.
14. Leaving: an exit fires minutes late (about 1.8 km at 15 m/s), passive, and is dropped if more than 20 min late.

**What Proxim sees**
- Sees: the resort's console content (geometry, copy, bundle); that app X fetched manifest/bundle/config (IP, time, publishable key; no cell, no location); revocation polls; billing at flat subscription.
- Does not see: any coordinate or route; that a trigger fired, a notification was posted or tapped; who the guest is; the booking; the arm list; any push token. No events in owner mode.
- The resort sees what its own app collects. If the resort logs trigger callbacks against an identity, that is the resort's first-party processing and must be in its privacy notice (SDK option to disable host callbacks; COUNSEL on roles).
- The OS vendors: lock-screen text is visible to bystanders; Android geofencing runs through Google Play services, so geofence definitions may be visible to Google [U]; the egress audit cannot see it.

**Honest promise copy for contract and docs:** "Arrival welcome within about 2-3 minutes of entering the grounds, when the guest has granted notification and Always/all-the-time location and has not force-quit the app. Gate-level in-app welcome when the app is open. Nothing is promised otherwise."

**Cheaper alternatives to offer the resort** (no phone location in Proxim): the gate or check-in system fires the resort's own push, with Proxim supplying only the guide experience; an Apple Wallet pass with up to 10 relevant locations (secondary [V-sec]); QR or NFC at the gate to an App Clip or web guide for first-time visitors. A first-time visitor cannot receive a notification from an app they have not installed; App Clips are not a notification target for installed users, and Google Play Instant has ended [V-sec].

---

## 8. Use-case map and staged roadmap

### 8.1 Use-case map

| Use case | Mode | Trigger and delivery | Experience on tap | Identity lives in | Phase |
|---|---|---|---|---|---|
| Local business advertising | Hosted | place, area, corridor, approach; enter/approach; in-app slot, inbox; Sponsored | card -> landing page | none | v1 |
| Civic PSAs, event promotion | Hosted ("Public information") | approach, corridor, area; in-app | card with official link | none | v1 |
| Claimed public-access gate/entrance (parking garage, venue gate) advertising | Hosted | cross in, foreground/M2, in-app card | card | none | v1.1 |
| Resort / hotel arrival and departure | Owner | enter(grounds) ring, exit; local notification, inbox | guide + deeplink | resort app and backend | v2 |
| Gated estate / HOA (guests, visitors, contractors; residents' exit off by default) | Owner | ring around community, visitor-gate approach; service notices | notices guide + deeplinks | estate app | v2, careful |
| Campus (adult users) | Owner | enter campus, shuttle stops; in-app, service notices | wayfinding guide + SSO deeplink | institution SSO | v2 |
| Events and festivals | Owner (+ hosted sponsor slots with their own labels) | time-boxed geofences; event_info | schedule guide, ticket deeplink | organiser backend | v2 |
| Parking and EV | Owner (also v1 hosted segment) | approach, enter lot; transactional | session/payment deeplink | operator app | v2 |
| Museums, tourism, heritage trails | Owner | POI zones, approach; offline guides | multi-page guide | none | v2 |
| Parks, trails, outdoors | Owner | trailhead, corridor, exit protected area | rules and conditions guide | none | v2 |
| Public-safety / hazard alerts | Civic | approach, corridor; in-app first; informational only, never an emergency authority (Apple 5.1.5) | official card | none | later, counsel |
| Malls (operator + tenants), airports, delivery/curbside | Hybrid / Owner | tenant sub-senders, access-road approach | directory, order deeplink | operator apps | later |
| Field service / workforce check-in | Owner | enter site, exit | checklist | employer | later, counsel (employee monitoring) |
| Hospitals, clinics, K-12 schools, childcare | none | R1-R6, I7, I9 apply in every mode | n/a | n/a | blocked pending counsel |

### 8.2 Staged roadmap
- **Stage 0, "generality tax" (during M0-M2, schema, docs and tests only):** ADR-001; reserve `content_mode` (CHECK = hosted), `event` (default present), `intent`, `delivery`, `experience` (card), `visibility`, `pack_kind`, `event_params`, `group_id`, `priority`, `map_ref`, `precision_class`, `min_mode` in schema and pack header; `proxim-detect` with golden vectors for enter->exit sequences; `decide()` takes and returns trigger state; policy matrix gains mode and surface dimensions (hosted x notification = deny; owner rows present but disabled); sim OS models gain notification and region parameters; rename report vs trigger events.
- **Stage 1, advertising v1 (M1-M14 as planned):** hosted, in-app only; pickers for POI, building (zone), road (corridor), junction (approach anchor), parking, transit, area, neighbourhood, custom shapes and import; honest preview and badge; guardrails and claims. Events: enter and approach.
- **Stage 1.1 (after M13):** cross and point_pass for claimed public-access places, foreground/M2, in-app only; junction arm gates; local road context if counsel clears it; brand multi-select; KML.
- **Stage 2, owner pilot (after M11 field data, working Android SDK, and a signed design partner):** M15 owner server and core, M16 local notifications, M17 guides and Venue flow, M18 one-resort pilot. Android first.
- **Stage 3:** exit/dwell beyond the pilot, arming scopes and host veto, sealed pages, iOS Path S GA, self-serve owner onboarding, owner-hosted bundle origins, bring-your-own-trigger hook.
- **Later:** civic safety notifications, delivery/arrival, hybrid malls, BLE/NFC/Wallet/App Clip front doors, restricted populations only after counsel and gates.

---

## 9. Abuse controls

### 9.1 Narrow geometry (stalking a home, workplace or clinic door)
Real risk: a tier-0 sender draws 10-30 m on a victim's driveway and reads counts, per-day billing lines or landing-page hits as a presence and routine oracle. Controls:
- **Geometry classes.** G0 (any sender): place radius >= 150 m, unchanged. G1 micro (building, gate, entrance, point_pass): allowed only with a verified, feature-bound place claim (level 2) over a public-access feature (retail, lodging, venue, parking, transit, campus), not residential, private-access or driveway, outside sensitive buffers, and with effective footprint at or above the accuracy gate. G2 road and junction: only road classes with through-traffic; cul-de-sacs, service roads and driveways barred for third parties. G1/G2 get flight-level reporting only. Floors live in `policy_rules` data and are enforced three times (DB constraint, pack compiler, kernel) like the health standoff.
- **Claim ladder.** Level 0 no claim (G0 only). Level 1 domain DNS plus KYB plus phone callback to an independently listed number. Level 2 (required for G1 and for owner mode): Level 1 plus physical-address proof (postcard/PIN letter), a two-person reviewer rule for residential-adjacent features, per-feature binding, annual expiry, non-transferable, logged in the audit chain, and an objection channel (resident, neighbour, HOA, owner) that pauses the signpost immediately.
- **Closing the "trigger edited after approval" hole (PLAN section 4).** `signpost_triggers` becomes part of the immutable, signed `signpost_revision` (geometry, events, delivery, purpose, audience, expiry). Any change is a new revision that re-enters review. pgTAP proves trigger rows are immutable after submit.
- **Reporting as an oracle.** For G1/G2: flight-level only; threshold applied to the NOISY value; no `partner_app` dimension in sender-facing releases; no per-day counts; a per-sender privacy budget across overlapping geometries (reject a new geometry whose symmetric difference with an existing one is small); flat-fee billing; reach estimates from static priors, bucketed. PLAN-REVIEW #6's "exact counts for non-health" is amended to exclude G1/G2. Differencing and padding attacks are our own analysis [U]; demonstrate them in the simulator before thresholds are fixed. Residual that cannot be removed: a tap reveals IP and time to the verified owner of a claimed place. No URL macros, ever.
- **Authoring oracle.** Generic message "geometry not permitted here" for any cause involving confidential kinds, equalised timing, per-account and per-IP rate limits, alerts on near-identical rejected geometries, no confidential perimeters in packs (coarse barred cells instead), place search excludes residential and private records for non-claimants.
- **Limits.** <= 500 vertices, <= 20 geometries per signpost, NaN/antimeridian/self-intersection rejected, per-org quotas inside tier-0 rank truncation, parser fuzzing.
- **Bulk address upload is not offered.** A CSV of addresses is a customer list expressed as geometry.
- **Events.** Exit, dwell and sequence reveal a visit: owner mode only, never on health_facility or prohibited anchors, suppressed inside any standoff with nothing persisted (R7).

### 9.2 Notifications (spam, phishing, spoofing, residue)
- Kernel-only posting at wake time after every hard filter; a lint fails the build on `UNLocationNotificationTrigger`, `UNCalendarNotificationTrigger` or alarm-scheduled content in any third-party path.
- Structured template only; renderer-built title and label; Unicode and lexicon rules (section 5.4); phishing corpus in the review suite (security alert, verify account, package held, emergency, bank, hospital results, "Sponsored" faked, bidi, homoglyphs). Zero false-approves.
- Caps, cooldowns and hysteresis (section 5.4); dedicated channel; server-side anomaly detection of notification taps per active install with suspension and clawback.
- Residue: location-neutral text, auto-removal at expiry, barred sensitive classes, no sound for non-urgent.
- Revocation: online worst case 10 min (5-minute poll), offline new-notification worst case 24 h for service class; purge delivered notifications whose signpost is revoked at each wake.

### 9.3 The owner-mode loophole (advertisers posing as owners, health bypass)
- Same-org plus cert-bound `aud` DB constraint; a hosted sender key cannot set or flip `content_mode`.
- Closed `purpose` taxonomy and label key inside the signed revision; the SDK prints the label; a promotion cannot be downgraded to service after approval; mixed content is promotion.
- All URLs https on the verified owner domain; no custom schemes, macros, or off-domain redirects; scheduled re-crawl with auto-revoke; owner app routes limited to an allowlist in the signed partner config; SSRF-safe fetcher with rebinding tests.
- Sensitivity class derives from the place registry, never from the sender; R1-R6 apply unchanged; venue exclusion list checked at onboarding with a human step; residential estates: guests, visitors and contractors only, resident exit notifications off by default and requiring explicit opt-in.
- No per-impression billing and no rev share in owner mode removes the incentive to disguise ads; a linked-org check blocks one company acting as sender and partner to push unlabelled third-party content; partner onboarding screens out apps whose main function is monitoring another person.
- First three signposts, any new domain/phone/payment link and any promotion get human review; 5-10% sampled afterward.

### 9.4 Private-content leakage
- Packs and bundles are public-by-extraction: the publishable key is not a secret, tile packs are public, and content is content-addressed and immutable forever.
- Console secret linter on submit (digit runs, "password", "wifi", "code", "PIN", key-like tokens, secrets in query strings, QR-like payloads); CI byte-grep of every compiled pack and bundle for fixture secrets; zero false-approve target on a 100-item corpus.
- Pattern A (default): teaser (<= 90 chars) plus opaque `ref`; the owner's backend authorises by session, never by knowing the ref. Pattern B (later): private geometry in an owner pack served by the owner with a Proxim-signed manifest. Sealed (encrypted) pages keyed by the owner app are a later tier that protects against outsiders only, not against Proxim.
- Proxim does not open gates or hold access credentials (Apple 5.1.5).

---

## 10. Conflicts with PLAN.md and resolutions

| Section / rule / invariant | Conflict | Resolution |
|---|---|---|
| Section 1 billboard principle | Owner mode: a resort knows its guests; hosted "AI beneficial to the user" idea | Two-mode wording (section 2.4). Hosted unchanged; owner restated with arming list that never leaves the device. No runtime user AI |
| Section 4 `signpost_triggers` DB constraint and section 5.4 "place radius >= 150 m" | Owner wants gate, building, point, junction arm, cross | Keep 150 m for BG_RING and unclaimed senders; G1/G2 under claims (I18); FINE/EXACT with `min_mode` M1/M2 and a precision label. Rename `gate_from/gate_to` to `window_from/window_to` so "gate" has one meaning |
| Section 5.4 primitives | No event dimension, no line shape | Add `event`, `gate`, `point_pass`, `junction`, `sequence` as compile targets; reserve in pack format at M2; v1 hosted enables enter and approach |
| Section 4 `signpost_triggers` vs revision immutability | Triggers not bound to the revision: post-approval retargeting | Fold geometry, events, delivery, purpose, audience, expiry into the signed immutable revision; pgTAP test (I17) |
| Sections 2, 3, 4, 7 map tooling | No reference-feature tables, pick data, search; protomaps lacks tags | Add `tools/mapdata`, `ref` schema, pick layer, `/v1/ref/*`, ADR on data stack and release pinning |
| Section 3 `packages/geo-ts` | Console, API and compiler must produce identical geometry | All derivation in Rust/WASM; geo-ts reduced to rendering helpers with parity tests |
| Section 5.2 and I8 (public, keyless, identical packs) | Owner private content and private triggers | Owner bundle channel keyed by app, no location (I8'), content treated as extractable, private data by handoff (I15) |
| Section 5.2 pack budgets (350 KB, 5 MB / 6 cells) | Road context, many shapes; critic #2 inconsistent | Road context counted inside the tier budget with caps; separate owner cache (text <= 1 MB gzip, assets <= 4 MB, never LRU-evicted while active) [U targets]; shape and campaign budgets as policy data |
| Section 5.5 P1 "dense-urban false-fire <= 15%" | Not credible for exact road claims | Per-profile targets; urban canyon reported, not promised; route intent or confirming sequence required for exact-road claims |
| Section 5.6 step 1 quality gate | 100 m gate accepts fixes straddling a 25-50 m geometry | Per-trigger `max_acc_m` (50 place, 25 FINE, 15 gate), straddle rule, ACC_TOO_COARSE, approximate-location reason code (critic #9) |
| Section 5.6 step 5 caps; section 5.5 driving-safe ("taps disabled while moving") | Written for in-app cards; once-per-visit semantics for returning guests | Notification-specific caps by intent; re-arm cycles; "post silently while moving"; owner dismiss cooldown per visit not per week; hide/mute stay permanent |
| Section 5.7 tripwire planner | Re-anchor ring density rule; slot economics with exit; FGS question | Ring formula R = cushion + v x (T_wake_p95 + T_ready + T_lead); one ring per trigger; engine hysteresis; per-day wake cap; reconcile() on wake; OS-callback-only coarse fallback if Play rejects FGS use |
| Section 5.3 M3 text "Health app requesting Always for ads will likely be rejected" | Too narrow: Google denies ads-only background location for any partner; Apple 5.1.5 needs direct relevance | M3 is for owner-mode partners whose arrival feature is documented core function and for partners whose own function needs background location; never to obtain background location for advertising |
| Section 5.8 R2, R6, I6, I7 | Health-scoped; events and notifications create presence records and lock-screen exposure | R6 absolute for notifications in any mode; add R7 (no exit/dwell/sequence on health or prohibited anchors; suppress, persist nothing), R8 (private residence, driveway, service road locked picks), R9 (claimed private areas: only the claimant, in owner mode, sets triggers inside; owner decision on excluding third-party ads there); I6/I7 extended (I16) |
| Section 6.1 I3 and section 4 device entities | Stateful events need persisted state | `zone_state` for owner exit/dwell only (I19); hosted uses in-memory counters |
| Section 6.1 I2 and M3 lint ("lat field in fixture fails") | Sender authoring must carry coordinates | Scope lint to device-facing routes; explicit allowlist for authoring |
| Section 6.1 I8 | Owner bundle needs a key | I8' (never both key and location in one request) |
| Section 6.1 I11 | Notification code and permission in base artifacts | Notification code only in background artifacts; manifest-merge test |
| Section 6.2 data-flow inventory | New flows: owner bundle GET, owner manifest | Add them (key, no location); owner mode emits no events; add egress-audit tests |
| Section 6.4 consent (`local_notifications` single flag; SDK sheet only for health) | Apple 4.5.4, Google disclosure, FTC SDK-consent line | Purposes of section 5.5; SDK-owned sheet and receipt for notifications and background trigger use |
| Section 6.5 and 6.6 | Owner = hospital using its own geofence; roles posture does not cover owner mode | No owner exemption; health owners get user-initiated flows only; write an owner DPA and roles table (controller, processor, independent controller); no BAA in v1 (COUNSEL) |
| Section 7 rendering: mandatory "Sponsored" label, ProximRequiredChrome, V x Q x R x P, CPM, k-threshold | Written as universal | Scope to `content_mode = hosted` via `mode_policies` data; owner label "From <owner>"; owner billing subscription |
| Section 8 deferred "local notifications (non-health only, later)" | Framed as merely later; hosted ad notifications are blocked by store policy | Replace with: "Owner-mode service-class local notifications: beta after M11, M8, M10 and a design partner. Hosted and promo notifications: not planned (store policy)" |
| Section 8 / PLAN-REVIEW #6 "exact counts for non-health" | Micro-geometry makes exact counts a presence oracle | Exclude G1/G2 from exact counts; flight-level, noisy-thresholded, flat fee |
| Section 6.4 / 5.5 P4 learned habits | Owner "AI beneficial" idea would reverse the decision | P4 stays off; negative-only suppression allowed; any revisit needs DPIA and counsel |
| Section 1 item 2 (CT, NV "similar rules at 1,750 ft") | Secondary sources: CT covers mental and reproductive/sexual health facilities only; NV any medical facility; WA names "notifications" | Correct text; keep standoff as data (COUNSEL to confirm statutory wording) |
| Section 3 crate `proxim-events` | Name collides with trigger events | New crate `proxim-detect`; glossary: "report events" vs "trigger events" |
| Section 2 stack and critic #1 (libm) | Matcher and crossing code | Allowed in detector: + - * / and sqrt only plus integer geometry; trig/exp/pow remain disallowed by clippy |
| PLAN-REVIEW #15 revocation latency | Notifications raise harm of stale content | Numbers: poll every 5 min while active; service notifications fail closed past a 24 h-old list; purge on wake; stated SLA online <= 10 min |
| Section 5.7 item 7 (FGS from geofence unverified) | Partly resolved | Exemption exists [V]; Play approvability [U] (dates conflict); device-test at M4/M11 |

---

## 11. Milestone changes (summary; full list with verification in the structured output)

- **M0:** ADR-001 (naming, modes); burn-down adds Google Play primary text, FGS-geofencing policy date, iOS Path S/D behaviours, state precise-geolocation matrix, ODbL question, counsel list.
- **M1:** `proxim-detect`, accuracy-aware state machine, AR(1) plus jump noise model, scenarios S01-S06, S11 and golden vectors.
- **M2:** reserved pack and schema fields; signed envelope v2; `attributions[]`; road-context size experiment on a dense-metro fixture.
- **M3:** policy matrix gains mode, surface, intent; R7-R9; I13-I20 tests; privacy-lint rescoped.
- **M4:** notification and region OS models; ring formula; device harness measures latencies and Path S behaviour.
- **M5:** revision folding, claim ladder, `ref` schema, owner tables reserved (CHECK content_mode = hosted).
- **M7:** pickers, pipeline, search corpus, guardrails, reliability badge, keyboard path, import.
- **M8/M10/M11:** notification code in background artifacts only; RegionBackend; ring formula verified in the field; Play FGS question settled.
- **M9:** per-mode report allowlist; flight-level for G1/G2.
- **M12:** store pre-clearance dry runs and counsel memo extended to owner mode.
- **New M15-M18:** owner server and core; local-notification delivery; guides and Venue flow; one-resort pilot.

---

## 12. New risks

1. **Scope explosion for a very small team.** Mitigation: Stage 0 is schema and tests only; owner features start after M11 and a signed design partner; mode-as-data keeps the kernel unforked; section 8 "not building" list is binding.
2. **Senders read "select a gate" as exact.** Mitigation: precision label, per-mode preview, auto-widen, disabled combinations, contract language ("about", never "exactly").
3. **Micro-geometry used for stalking.** Mitigation: section 9.1.
4. **Store rejection or removal** (Apple 4.5.4/5.1.5, Google core-functionality and system-functionality rules), including a reviewer classing a welcome as promotion. Mitigation: first-party binding, service-only copy, base artifacts free of notification and background code, always-available non-notification path, dry-run submissions before the pilot, written pre-clearance for anything else.
5. **Google Play may not approve a foreground-service burst or geofencing FGS** (policy dates conflict). Mitigation: OS-callback-only coarse mode designed first.
6. **iOS wake reliability** (single unconfirmed iOS 26 force-quit report; Path S unobservable). Mitigation: legacy API default, never promise after force-quit, device matrix per OS beta.
7. **Owner-mode loophole** for advertisers. Mitigation: section 9.3.
8. **Persisted `zone_state` becomes a visit history.** Mitigation: owner exit/dwell only, 15-minute buckets, 12 h TTL, encrypted, never in buffers, invariant tests.
9. **ODbL share-alike exposure** from shapes (and road context) in public packs. Mitigation: lineage, attribution, open extract, counsel before the first external partner, fallback to authored geometry only.
10. **Pack bloat and wake/battery cost** from many triggers and kilometre-scale rings. Mitigation: caps, per-cell quotas, one ring per trigger, wake cap, battery measurement in the field harness (critic #8).
11. **Urban-canyon false enters** (34-69% for a stationary user 25 m outside in the pessimistic model). Mitigation: adaptive accuracy inflation, larger radii, urban profile in the preview, calibrate from real traces at M4.
12. **Distracted driving and product liability** from prompting drivers. Mitigation: silent landing while moving, counsel.
13. **Owner callbacks correlate with identity** (the owner's own first-party processing). Mitigation: disclosure duties in DPA, option to disable callbacks, COUNSEL.
14. **Commodity pressure** (a Wallet pass plus the owner's own push is a cheap alternative). Mitigation: validate with one design partner before M15; differentiate on one trigger model, offline guides, and no identity or location at Proxim.
15. **Overture schema churn and monthly releases.** Mitigation: pin the release, contract tests, own taxonomy mapping table, drift job.

---

## 13. Open owner decisions

1. **Notifications for advertising.** Accept that third-party adverts will never be OS notifications in v1 (they stay in-app and inbox), and that notifications become a first-party owner-mode product? If you want ad notifications anyway, someone must obtain written guidance from Apple and Google first, and no engineering starts before that.
2. **First non-ad pilot.** Name the design partner (it must be the venue's own app, with arrival assistance as a documented core feature and an explicit opt-in), and accept Android first with "Allow all the time" plus a Play declaration, iOS Always as default (When-In-Use Path S only after device tests), a one-partner private beta, and an owner DPA with no PHI.
3. **Precision policy.** Accept that anything smaller than 150 m exists only for verified, feature-bound claims (postcard proof, days of latency, a named reviewer with a two-person rule) over public-access places; that homes, driveways and private roads are never targetable by third parties; and that estates are served only through owner mode by the managing entity.
4. **Exit and dwell.** Allow them for owner mode only (with the I3 amendment, 12 h bucketed state, never near sensitive places) and keep them off for advertising until counsel has advised?
5. **"The AI decides it is beneficial."** Accept the five-layer meaning (declared intent, review-time AI, deterministic on-device rules, user controls, owner-side veto) with no runtime AI on user data in Proxim? If you want per-user AI, it lives in the owner's app on the owner's data.
6. **Money and time for evidence.** Fund (a) qualified counsel on: store policy pre-clearance, WA/NV/CT/MD/OR/VA/CA/NY precise-geolocation and geofence rules, HIPAA and covered-entity owners, distracted driving, TCPA/CIPA for push and SDK calls, ODbL derivative-database question, owner DPA and roles; (b) one iPhone, one Android and one real gated venue for the M4/M11 drives, to replace the modelled latencies; (c) a pilot-metro choice after the coverage audit.

(Decided by the team, not the owner: Overture-first data stack, Postgres search with Photon fallback, owner pricing as per-venue subscription with manual invoices, bundle TTL 7 d, zero Proxim events in owner mode.)

---

## 14. Facts still unverified

**Verified on 2026-10-09 (primary unless noted)**
- Apple App Review Guidelines (updated 2026-06-08): 2.5.16, 2.5.18, 4.5.3, 4.5.4, 4.10, 5.1.1, 5.1.2(i), 5.1.5. developer.apple.com/app-store/review/guidelines.
- Apple: `UNLocationNotificationTrigger` needs only When-In-Use; system limits triggers; accuracy reduced disables region monitoring; region monitoring 20 limit, ~200 m cushion, 20 s hold, 3-5 min, ~10 s wake, no wake with Background App Refresh off; CLMonitor 20 conditions, post-unlock only; legacy region API carries a CLMonitor-successor notice; interruption levels; `hiddenPreviewsBodyPlaceholder` is a system setting.
- Android: geofencing (100 per app, 100-150 m, latency <2 min / 2-3 min / up to 6 min, FINE + BACKGROUND, re-register rules); background limits; FGS background-start exemption for geofence events (page updated 2026-10-01) and Android 14 while-in-use limits; POST_NOTIFICATIONS (API 33+); notification channel immutability; Android 12 trampoline ban.
- MapLibre 6.13.0 tile-clipping docs; terra-draw 1.37.0 modes and absent keyboard coordinate entry; Protomaps tag omissions and licensing; Overture release list (2026-08-19.0, 2026-09-23.0, 2026-09-23.1), schema classes, per-theme licences and attribution text.
- [M] measurements on Overture 2026-09-23.1: road graph sizes per r5 cell, junction arm angles, parallel-road exposure, gate positions, GERS id persistence (99.70-99.96%).

**Verified only through secondary sources [V-sec]**
- Google Play: ads in system notifications as a violation; background-location core-functionality rule; ads-inside-the-app rule; prominent disclosure; the January 2027 minimum-scope policy. support.google.com was unreachable.
- Washington RCW 19.373.080 wording (notifications named); Connecticut and Nevada scope; Maryland, Oregon, Virginia, California rules; FTC InMarket, Gravy, Mobilewalla, Kochava order details; Google Play Instant shutdown; Nearby Notifications shutdown; 64 pending notification cap; Wallet pass 10 locations.

**Unverified [U]**
- Real OS wake latency, miss rates and iOS cushion by device (all numbers marked [MODEL] come from assumed lognormals).
- Real GNSS error and accuracy under-reporting in urban canyons; dual-frequency phones are likely better.
- Whether `UNLocationNotificationTrigger` regions share the 20-region pool, persist across reboot and force-quit, or work as documented under When-In-Use after suspension; Driving Focus and CarPlay behaviour for third-party notifications.
- Whether an Android foreground-service burst after a geofence wake is Play-approvable; the true effective date of the related Play policy item (2026-08-26 versus 2027-01-27).
- iOS 26/27 relaunch after force-quit and CLMonitor reliability; Android 17 and iOS 26/27 behaviour changes (pages only partially read).
- Whether Google Play accepts background location for a hospitality or estate app as core functionality; whether App Review applies 4.5.4 to local notifications posted by an SDK; whether either store treats a first-party welcome as promotion.
- Whether Android geofence definitions are sent to Google; non-GMS fallback.
- ODbL consequences for derived trigger shapes and road context (Derivative Database versus Produced Work); TomTom-sourced records inside the Overture feed; National Address Database redistribution terms.
- Extrapolation of five-week GERS id attrition to a year; Overture release cadence as a dependency.
- Battery cost of 1 Hz bursts and of the matcher in Rust (unmeasured); Supabase Storage HTTP Range support for PMTiles; Valhalla RAM for a US graph; Photon sizing; H3 res-7 area for coarse barred cells; Overture bbox pushdown in DuckDB.
- Provisional numbers invented here and to be calibrated: owner bundle budgets, per-org quotas, vertex caps, tripline length (60 m), reliability thresholds, residential-address density threshold, pilot acceptance gates (p90 notification within 5 minutes of ring entry in Android Always drives, zero notifications beyond ring + 300 m in 20 drive-bys, mute rate under 2%).
- Legal conclusions of every kind: this addendum draws none.
