> **Status: DRAFT v1.0 for owner review (2026-10-09).** Produced by a multi-agent design process
> (3 independent proposals, a 3-lens judge panel, synthesis, then a gap critic).
> Nothing here is legal advice. Open decisions and the critic's findings are in
> [`PLAN-REVIEW.md`](PLAN-REVIEW.md). Version numbers marked "R" are unverified until M0.

# Proxim: Final Build Plan (v1.0, 2026-10-09)

Base: proposal #2 (partner-DX-first, on-device kernel, signed tile packs). Grafted from #0: OS-adapter simulator with oracle metrics, tripwire planner, deterministic math and golden traces, invariant-to-test mapping, mock-location filters, glanceable driving format, flat-fee/no-telemetry health, OHTTP and unlinkable tokens as a hard gate before health traffic, no app key on tile fetches. Grafted from #1: dedicated Supabase project, data-driven policy matrix, place-claims, rogue-sender test suite, S2 "never served" class. All judge must_fix items are addressed; the mapping is in section 12.

**Verified in this session (2026-10-09):** crates.io: uniffi 0.32.2 and h3o 0.11.0 are current stable. npm: next 16.4.0 is latest. Web search of the Washington statute text and commentary: RCW 19.373.080 and the RCW 19.373.010 geofence definition (virtual boundary of 2,000 ft or less from the perimeter of an in-person health-care facility) are real. Commentary says there is no consent exception. That means opt-in does not cure a geofence inside the buffer, so the standoff rule is a hard requirement. This is a secondary reading, not legal advice, and no court or AG guidance on measurement was found.
**Carried from the upstream design agents and NOT re-verified by me:** all other crate and npm versions, the Android geofencing facts (100 per app, 100 to 150 m radius, 2 to 6 min latency, re-register after reboot), iOS 20 monitored-condition cap and the iOS 26 relaunch reports (single-developer forum posts), Apple guideline text, Google Play background-location policy and its conflicting effective dates, other state laws and the FTC orders. Every one of these is an M0 burn-down item recorded in `docs/adr/000-verified-facts.md` with a date and source. None is hard-coded. OS and legal numbers live in the simulator OS-model profiles and in `jurisdiction_rules` data.

---

## 1. Product definition and explicit assumptions

**What Proxim is.** Infrastructure for location-triggered "signposts" delivered inside other companies' apps. It is a delivery and decisioning layer, not a consumer app. It has no end-user-facing app, no account system for end users, and no profile of anyone.

**Parties**
- **Senders** (advertisers/organizations): local businesses first, then events, civic bodies and verified public-health/healthcare organizations. They draw a place, area, corridor or approach geometry, write a structured message, set budget and schedule, and submit for review. They never hold signing keys.
- **Partner apps** (publishers): embed the Proxim SDK (Web, Android, iOS; React Native later). They supply consent, context (a closed taxonomy of app category and screen topics), optionally location and route intent, a UI slot, and policy (category allow/block, sensitivity ceiling, caps, quiet hours).
- **End users** of partner apps: see a labelled, relevant signpost when they are at a place, inside a corridor or approaching an anchor ahead along their path. They control topics, consent and "why am I seeing this".

**Positioning (the "billboard principle").** A signpost may depend on where the device is, where it is heading, the time and the app's declared context. It may never depend on who the user is. The SDK has no field for user attributes, health data or advertising IDs. "Smart" means contextual and situational relevance plus quality, pacing and safety. It does not mean profiling. This is also the position that Apple, Google, the FTC and state health-privacy laws punish least.

**Explicit challenges to the owner's reading**
1. **"In the user's path" differs by partner type.** The literal expressway sign ("exit 14 ahead in 1.2 km") is achievable only when the app has high-quality foreground or partner-fed location (navigation, ride-hail, transit, parking, EV/fuel, fitness). In the background the honest promise is "event ahead in the next 1 to 8 km". For a Health app with When-In-Use permission, "in your path" realistically means "in your area, or on your route if the app feeds route intent, shown when the app is open". User-facing copy is tied to a path-confidence ladder and never claims more than the tier supports (section 5).
2. **The flagship example (Health app, hospital outreach, in the user's path) cannot be built as "an ad at the hospital door".** Washington bans geofences of 2,000 ft or less around in-person health-care facilities for health-related messaging, with a private right of action and apparently no consent exception. Connecticut and Nevada have similar rules at 1,750 ft. The deliverable version is an **approach signpost 1 to 8 km out** (like a real highway sign), from a verified hospital or health department, to an opted-in Health-app user, never inside a 1,000 m standoff of any facility, in-app only, with no per-impression telemetry. It is built and demonstrated end to end in sandbox/PSA mode in v1. **Live health traffic is gated on counsel sign-off and the technical gates in section 6.**
3. **Proxim must not drift into a consumer app.** Everything user-facing is a slot component, a consent sheet or a privacy center that lives inside partner apps.

**Why each side participates**
- **Partners:** a brand-safe local/civic revenue stream (default 65 to 70% share, commercial decision), no ATT prompt, no ad ID, no backend to run, and a store-readiness kit. Honest disclosure: this is contextual inventory, so eCPMs will be lower than behavioral networks. We measure the gap in the pilot.
- **Senders:** a signpost placed where their audience physically passes, a "Drive it" preview showing exactly where and when it fires, prepaid budgets, promo-code and RSVP conversion counting, and reach estimates from tile-fetch counts. No user-level data is claimed or sold.
- **End users:** relevant local information with a mandatory sponsor label, a "why am I seeing this" sheet that states nothing about them left the phone, and hide/mute/report controls.

**First partner segment (cold start).** Navigation, transit, parking, EV/fuel, ride/delivery and fitness apps that already hold location. Health apps are a gated, foreground-first second segment. Single-metro pilot, 2 to 3 named design partners (owner decision, see open questions). Inventory seeding: sandbox synthetic signposts so integration never waits for senders, plus civic/PSA house campaigns capped at 30% of slots and still quality-gated.

**Assumptions**
- US-only at launch. EU/UK deferred pending DPIA and counsel (GDPR Art. 9 inference, ePrivacy 5(3), DSA 26(3)).
- Platforms are Web and mobile on one backend. Mobile v1 is Android first, iOS next. React Native after the pilot.
- The Lovable prototype was unreadable. The console is net-new and nothing is assumed about its UI or branding.
- A very small team (possibly one person with an AI coding agent). Every choice below optimizes for what such a team can build, test and operate.
- Counsel review is a launch gate for health, not an engineering deliverable. Nothing here is a legal conclusion.

---

## 2. Stack (committed)

"V" means verified this session. "R" means reported current by upstream agents on 2026-10-09 and to be re-verified by lockfile at M0. Pin only what installs cleanly.

| Layer | Choice | Version | Why |
|---|---|---|---|
| Decision core | Rust, Cargo workspace, 2024 edition. `proxim-core` has no network crates (cargo-deny enforced). Crates: h3o, geo, rstar, p256 + sha2 + base64 (hand-rolled ES256 JWS verify, about 150 lines, fuzzed), serde_json, libm, proptest, cargo-fuzz | h3o 0.11.0 (V); geo 0.33.1, rstar 0.13.0, p256 0.14.0, libm 0.2.16, proptest 1.11.0 (R); Rust toolchain pinned in `rust-toolchain.toml` at M0 | One implementation of matching, path, caps, consent and verification runs on phones, web (WASM), the simulator, the console preview and the pack compiler. libm gives bit-stable trig for golden traces. "No network in core" makes "raw location never leaves the phone" a compile-time property. |
| FFI | UniFFI (Swift, Kotlin), wasm-bindgen (Web/Node) | uniffi 0.32.2 (V); wasm-bindgen 0.2.129 (R) | Generated bindings keep native shells thin. |
| Android SDK | Kotlin, Compose + View UI, Play Services Location (GeofencingClient, FusedLocationProviderClient with PendingIntent), WorkManager, Play Integrity. Two artifacts: `sdk` (no background permission) and `sdk-background` (adds ACCESS_BACKGROUND_LOCATION, geofence and boot receivers). minSdk 26 | Kotlin 2.x, AGP and play-services-location 21.x: pin at M0 | Base artifact must never merge the background permission (manifest-merge CI test). Geofences are lost on reboot/replace so boot re-registration is mandatory in the background artifact. |
| iOS SDK | Swift (strict concurrency), SwiftUI + UIKit views, SwiftPM XCFramework binary target with checksum. Products `Proxim` and `ProximBackground`. Legacy `CLCircularRegion` + significant-change as the background foundation, `CLMonitor` behind a flag. `PrivacyInfo.xcprivacy`. min iOS 16 | Xcode/Swift pinned at M0 | Forum reports of CLMonitor and relaunch regressions are unconfirmed, so the conservative API is default and nothing depends on terminated-app relaunch. |
| Web SDK | TypeScript + WASM core, `<proxim-signpost>` web component, thin React wrapper. Foreground Geolocation only | TypeScript 7.0.2 (R, pin whatever Next 16.4 and Vitest accept) | Browsers have no background geolocation. Fastest demo and quickstart surface. |
| RN/Expo | Deferred to after the pilot. Plan: Expo Modules wrapper over native SDKs | expo 57.0.27, react-native 0.87.1 (R) | Logic stays in the Rust core, so the wrapper is thin. |
| Console and docs | Next.js (App Router), React 19, Tailwind, shadcn/ui on Vercel. MapLibre GL JS + terra-draw over a self-hosted Protomaps PMTiles basemap. MDX docs with Scalar-rendered API reference. Zod for schemas | next 16.4.0 (V); maplibre-gl 6.13.0, zod 4.6.5 (R) | Web is required and Vercel is connected. Self-hosted tiles avoid leaking draft geometry to a third-party map vendor. |
| API | Hono + zod-openapi on Vercel Functions (Node runtime), OpenAPI 3.1 contract, RFC 9457 problem+json, idempotency keys | hono 4.13.13 (R) | One TypeScript runtime and deploy pipeline. Public surface is only the audited Proxim API. PostgREST and the Supabase anon key are never exposed to partner apps. |
| Database | **New dedicated Supabase project** (staging and prod), Postgres + PostGIS, Auth, RLS on every table, Storage, pg_cron, pgTAP. H3 sets computed in the app tier and stored as `bigint[]` with GIN | Postgres major version is whatever the new project provides; run `list_extensions` at M0 and record. supabase-js 2.117.3 (R) | Connected tool, strong fit (PostGIS buffers, RLS tenancy). **Never reuse the existing "ilios Healthcare" project** (wjsjkemamttsuilhihip): ad-tech data must not commingle with healthcare data. CI fails if the configured project ref is that one. |
| Jobs | Postgres `jobs` table drained with `FOR UPDATE SKIP LOCKED` by the worker, triggered by Vercel Cron (every minute) and pg_cron | n/a | No queue vendor, no Redis. Deterministic and testable. |
| Signing | JWS ES256 (ECDSA P-256) with the online key in **AWS KMS** (ECC_NIST_P256), reached from Vercel by OIDC federation (no long-lived secret in env). One offline root key on a hardware token signs the key set (upgrade to 2-of-3 before GA). Clients pin root public keys. Note: KMS returns DER, JWS needs raw r\|\|s, so convert and test | jose 6.2.12 (R) for test-side verification | P-256 is supported by every KMS/HSM and every platform. Ed25519 availability was a gamble. **KMS is live before the first external partner sees a signed signpost, not "before GA".** Tests use local software keys. |
| Pack hosting | Supabase Storage public bucket behind CDN (immutable, content-addressed objects plus tiny short-TTL manifests) | n/a | Static files, cheap, almost stateless to operate. |
| Payments | Stripe Checkout for prepaid sender credits, Stripe Tax later; internal double-entry ledger is the source of truth. **Partner payouts are manual statements in v1; Connect Express later** | stripe-node pinned at M0 | Cuts scope. Test mode through the closed beta. |
| Moderation | Rules engine + Claude via the Anthropic API (structured output, pinned model ID, advisory only) + Google Web Risk for URLs + NPPES NPI lookup for healthcare senders + perceptual hash for logo reuse + human queue | model ID pinned at build | The LLM recommends. Deterministic rules and humans decide. In v1 **every signpost gets human approval.** |
| Attestation | Apple App Attest, Google Play Integrity (token issuance only). Web: origin allowlist + Vercel Firewall rate limits + reduced billing weight | n/a | Defeats emulator farms without a persistent device ID. |
| Network privacy (phase 2) | Oblivious HTTP (RFC 9458) via a third-party relay with Proxim as gateway (relay vendor to be confirmed commercially; Fastly is the candidate), plus Privacy Pass-style blind tokens. RFC numbers to be re-checked at M0 | n/a | Hard gate before any health-category traffic (section 6). |
| Simulation | `proxim-sim` (Rust, also WASM), Valhalla or OSRM on an OSM extract for synthetic routes, `xcrun simctl location`, adb emulator geo, mitmproxy, k6, Playwright `setGeolocation` | Playwright 1.64.0, Vitest 5.0.3 (R) | Section 5 and 11. |
| Monorepo/CI | pnpm + Turborepo + Cargo workspace; GitHub Actions (ubuntu + macOS); Vercel previews; Supabase branching | pnpm 12.10.1, turbo 2.11.7 (R) | Core, wire format, SDKs and services version together. |
| Observability | Vercel observability + Supabase logs + OpenTelemetry with a mandatory scrubber that drops IPs and lat/lng-shaped values. Sentry in the console only. **No third-party analytics or crash SDK inside the Proxim SDKs** | n/a | Privacy invariants apply to our own logs. |

---

## 3. Monorepo layout

```
README.md                      thesis, quickstart links (replaces the placeholder line)
docs/
  architecture.md              trust boundaries and the data-flow inventory (the ONLY things that may leave a device)
  privacy-invariants.md        numbered invariants I1..In, each mapped to a named automated test
  threat-model.md              STRIDE per component, key-compromise drills, abuse cases
  legal-gates.md               health/jurisdiction launch gates and counsel sign-offs
  adr/000-verified-facts.md    dated verified/unverified list; adr/NNN-*.md decisions
core/                          Cargo workspace (shared by every client)
  crates/proxim-geo            ENU projection, polygon/polyline/corridor tests, along-route distance, H3 helpers
  crates/proxim-path           Kalman heading/speed, path_confidence, route-intent intersection, destination truncation
  crates/proxim-pack           JWS parse/verify, pack/manifest/revocation/keyset/partner-config formats, rollback checks
  crates/proxim-decide         pure decide(state, fix, packs, policy, consent, clock, seed) -> decisions + reason codes
  crates/proxim-sched          tripwire planner + OsAdapter trait (real shells and SimulatedOS implement it)
  crates/proxim-events         event allowlist types, outbox, jitter, caps
  crates/proxim-ffi            UniFFI definitions; crates/proxim-wasm  wasm-bindgen build
  crates/proxim-sim            replayer, OS models, oracle, scenario runner, Monte Carlo, CLI
  testdata/                    golden vectors, scenario YAML, GPX/GeoJSON routes, test keys, fuzz corpora
sdks/{android,ios,web,react-native(placeholder),flutter(placeholder)}
apps/
  console                      Next.js: sender, partner, admin/moderation, Route Lab and "Drive it"
  api                          Hono OpenAPI app (config, events, reports, attest, management routes)
  worker                       pack compiler, signer client, moderation pipeline, rollups, ledger, pacing overlay
  docs                         MDX docs, quickstarts compiled in CI, API reference
samples/{android,ios,web}      runnable partner apps with sandbox key and bundled GPX (incl. HealthDemo in Sealed profile)
packages/
  schema                       Zod + OpenAPI + event allowlist (single source of truth)
  policy                       taxonomy, sensitivity classes, jurisdiction rules, policy matrix as data
  geo-ts                       corridor derivation, H3 covers, standoff checks (parity-tested vs Rust)
  signer                       KMS signing client (imported only by apps/worker)
  cli                          `proxim doctor`, `proxim sim`, `proxim pack inspect|verify`
supabase/{config,migrations,seed,tests}   pgTAP tests live here
tools/{privacy-lint,egress-audit,field-test,check-invariants}
fixtures/{routes,packs,scenarios}
.github/workflows/             ci-core, ci-android, ci-ios, ci-web, ci-db, ci-egress, nightly-montecarlo
```

---

## 4. Data model and row-level security

**Server entities** (all in the new Supabase project; ids are UUIDv7):

| Entity | Key fields | Notes |
|---|---|---|
| `orgs`, `org_members` | org kind sender/partner/platform, `verification_tier` 0 email, 1 business/domain, 2 regulated healthcare/public health; member role owner/admin/editor/finance/viewer | Tenant root. A company may hold both a sender and a partner org. |
| `sender_verifications`, `place_claims` | method (domain_dns, phone, stripe_kyb, npi_lookup, manual), evidence in a private bucket, reviewer, expiry (annual re-check); place_claims(org, place, evidence, status) | Only a verified owner may anchor or name a healthcare/sensitive place (anti-impersonation). Fuzzy name matching flags lookalikes. |
| `api_keys` | kind publishable/partner_secret/sender_secret, prefix, SHA-256 of a 32-byte random secret (shown once), scopes, bound bundle id / Android package + cert SHA-256 / web origins, revoked_at | Publishable keys are identifiers, not secrets. They gate config and attestation only. Separate from Supabase keys. |
| `partner_apps`, `partner_policies`, `partner_surfaces` | category (server-trusted), `child_directed` (SDK inert), `profile` standard/sealed, location_mode, health_tier_enabled (needs signed addendum), allow/block categories, blocked sender ids, sensitivity ceiling, caps, quiet hours, region budgets, surfaces with closed-taxonomy context tags and `driving_safe` | Policy compiles to a signed partner config (TTL 5 min). It doubles as the remote kill switch. |
| `campaigns` | budgets, flight, tz, dayparts, pacing mode, status | Money lives here so creative edits never touch billing history. |
| `signposts`, `signpost_revisions` | stable id; immutable revision: headline <=40, body <=90, glanceable <=7 words, cta, landing_url, `sensitivity_class` general / health_adjacent / health_facility / prohibited_anchor, content sha256, review id, `jws`, `signed_kid` | Revisions are immutable once submitted. Any edit makes a new revision that re-enters review. Only an approved revision is signed. Senders can be escalated in class, never downgraded. |
| `signpost_triggers` | kind place/area/corridor/approach, `geom geography` (GiST), buffer/radius, bearing + tolerance, direction mode, min speed, gate_from/gate_to along-route metres, `tiles_r5 bigint[]` (GIN), `cover_r8`, standoff_checked, standoff_min_m | DB constraints: radius >= 150 m, area <= 100 km2, health classes >= standoff from any facility. |
| `targeting_rules` | partner categories, include/exclude partners, context tags, motion modes, dayparts, locales, os, min sdk | No user-attribute columns exist, by design. |
| `sensitive_places` | kind (hospital, clinic, pharmacy, mental_health, reproductive_sexual, substance_treatment, hiv_sti, school, shelter, worship, immigration), perimeter geom, source (nppes/osm/manual/sender_declared), jurisdiction, active | Drives authoring rejection, pack-shipped suppression zones, destination suppression. Changes go through an audit log. |
| `jurisdiction_rules`, `policy_rules` | region, rule_key, params, effective/expiry; versioned immutable policy matrix: sensitivity x partner context x place class x jurisdiction x consent -> allow / deny / require_in_app_only | Data, not code. Defaults: US-WA health standoff 1,000 m; EU/UK served=false. Policy version recorded on every pack build. |
| `moderation_reviews` | stage auto/human, verdict, reasons, model output, reviewer | Auto stage fails closed to human. Health-class and a sender's first three campaigns always need a human. In v1 all do. |
| `packs`, `pack_heads`, `pace_overlays`, `revocations`, `keysets` | `(cell_r5, seq, sha256, storage_path, built_at, expires_at)`; head pointer is the only mutable item; keyset versions and revoked kids are global | Packs are deterministic: the same DB snapshot gives byte-identical output and hash. |
| `events_raw` (partitioned daily) | event_id random, partner_app_id, signpost_id, rev, event_type, slot_key, **event_day only (no hour)**, sdk_ver, os, attested, simulated, token, ivt_flags | **No cell, no hour, no IP, no device or session ID, no coordinates.** Raw purged after 14 days. Duplicate event_id rejected. Not written at all for `health_facility`. |
| `event_aggregates_daily`, `report_releases` | counts per (day, partner_app, signpost, event_type); releases with k-threshold and stored-once Laplace noise | Sender-facing reads use only `report_releases`. |
| `ledger_accounts`, `ledger_entries`, `invoices`, `statements` | double-entry, per-txn sum zero (deferred constraint trigger), settlement idempotent per (day, signpost, partner) | Charge = min(measured, remaining budget). Overshoot is absorbed by Proxim. |
| `delivery_budgets` | signpost, day, spent, p_serve | Pace controller state. |
| `audit_log` | actor, action, entity, before/after hashes, prev_hash, hash (hash chain), at | Append-only. Covers approvals, key ops, policy changes, health-tier enablement, overrides. |
| `jobs`, `webhook_endpoints`, `webhook_deliveries` | standard | Webhooks are HMAC-signed with a timestamp and replay window. |

**Device-side entities** (SQLite, excluded from backups; iOS `NSFileProtectionCompleteUntilFirstUserAuthentication`, Android Keystore-wrapped key): `pack_cache` (<= 6 cells LRU, <= 5 MB, rollback guard storing max seq per cell); `impression_ledger` (caps, timestamps coarsened to 15 min, never written inside a sensitive buffer); `consent_state` plus **consent receipts** (purposes, jurisdiction, timestamp, signed by a device-bound key); `preferences` (topic chips); `report_outbox` (one event per request, random 1 to 24 h send delay, dropped on consent withdrawal); `trail_ring` (memory only, last 15 minutes of fixes, never persisted); `place_graph` (learned routes, **deferred and off**).

**RLS approach**
1. RLS is enabled with `FORCE` on every table in an exposed schema. The `anon` role has zero grants.
2. Tenant policies key on `org_id` through `private.is_org_member(org_id, roles[])`, a `SECURITY DEFINER` helper evaluated as `(select auth.uid())` so the planner caches it.
3. Authorization uses `app_metadata` only, never `user_metadata`. Staff, payout and any health-tier org require MFA (`aal2`).
4. Separate Postgres roles: console users (RLS), `worker` (service-side, can read approved data and write packs and ledger), `ingest` (insert-only on `events_raw`), `serving` (read-only on serving tables). The service-role key never leaves server runtimes.
5. Triggers enforce revision immutability, the standoff constraint, the S2/prohibited_anchor rejection and ledger balance.
6. pgTAP attack suite (cross-tenant read/write for every table and role, anon denial, MFA claim required, revision immutability) plus Supabase security and performance advisors are required CI checks.
7. Partner and sender secret keys are stored only as hashes. The public API (Hono) is the only surface partner apps ever touch.

---

## 5. Trigger, path-awareness and decision-engine design

### 5.1 Principle: what runs where

| Concern | Server (no user data) | Device (everything user-specific) |
|---|---|---|
| Content | Verify sender, moderate, sign (KMS), compile deterministic per-cell packs, rank and truncate, revoke | Verify signature, expiry, seq, kid, revocation before any use |
| Location | Never receives a coordinate, route or cell finer than H3 res 5 | All matching, path inference, trails, route intent, learned places (deferred) |
| Eligibility | Flight dates, budget remaining, jurisdiction tile rules, sender tier, brand-safety blocks, health-class gating by tile jurisdiction | Consent, partner policy (signed config), sensitivity ceiling, sensitive-zone suppression, child-directed kill |
| Selection | Rate card, quality prior (shrunk CTR from aggregates), diversity quota (30% reserved for small local senders), pacing overlay `p_serve` | Score, caps, pacing draw, slot allocation, TTL, render decision |
| Money | Ledger, billing, budget cap | Nothing |
| Measurement | Aggregates with thresholds | Viewability measurement, outbox |

Decision: **no live ad-decision API in v1.** Proposal #1's server-decisioning was rejected because every wake would send a cell, heading and partner identity to the server (health inference), because it adds Redis, a GSP auction and reservation machinery nobody needs for a first cohort, and because it duplicates geometry in two languages. Proposal #0's blind-token and TUF ceremony was deferred. The server never learns that a particular device was near a particular signpost. An optional live-confirm voucher call for non-sensitive signposts is a post-pilot option for exact budget control.

### 5.2 Pack distribution
- The device derives its H3 res-5 cell (about 253 km2) locally and fetches `GET /t/v1/{r5}/manifest` (small signed manifest with seq, expiry, tier hashes) and then immutable `GET /t/v1/{r5}/{sha256}.pack`. **Tile requests carry no publishable key, no cookie, no device ID and no Referer.** Packs are identical for all partners. Partner policy is applied on the device from the signed config, so the URL identifies a coarse area and never a partner app or health profile. Neighbour cells are prefetched only when within 3 km of an edge. This is an accepted, declared "coarse location" disclosure, and phase 2 (OHTTP) removes the IP.
- Pack tiers: tier 0 (top 500 by rank, target <= 350 KB gzip excluding assets) downloads immediately. Tiers 1 and 2 come on unmetered networks. Assets (logo <= 64 KB, image <= 40 KB, content-addressed) are fetched with the pack, never lazily at fire time, because a lazy fetch would leak the match event. Soft TTL 24 h, hard TTL 72 h. Health classes in a tile carry a shorter TTL (6 h). Pack size budgets are provisional and are measured at M2.
- Global metadata (timestamp, keyset, revocation list) is small and not location-specific. Emergency kill rides this path and is polled at app start and each refresh.

### 5.3 Location modes (honest ranking)
- **M0 single-shot:** user taps "nearby" and one fix gives a local match.
- **M1 foreground:** SDK-owned When-In-Use updates only while the app is open. No background permission, no Play declaration. Default for most apps and the only mode for health partners at launch.
- **M2 partner-fed (headline mode):** the host already has location or navigation and calls `Proxim.location.update(fix)` and optionally `Proxim.route.set(polyline6, etas)`. Zero added permission or battery, full fidelity. This is the expressway-signpost experience.
- **M3 managed background (beta, separate artifact, Android first):** OS tripwires. The SDK never requests Always/background permission itself. The partner owns the prompt, the store declaration and the demo video. Only for partners whose core function is location. A Health app requesting Always for ads will likely be rejected (Apple 5.1.5, Google core-functionality rule).

Partner-facing fidelity tiers (docs and sales framing): **A continuous** (partner-fed or active session), **B background geofence** (approach radius >= 2 km walking/transit, >= 5 km driving, arrives late at highway speed), **C foreground/on-open** (area signposts and "arm on open"). Contracts never promise terminated-app or force-quit delivery.

### 5.4 Trigger primitives (sender-facing)
- **place** (point + radius >= 150 m), **area** (polygon <= 100 km2), **corridor** (road polyline + 30 to 500 m buffer + direction-of-travel bearing + speed band), **approach** (corridor + "fire when 0.5 to 8 km before the anchor, measured along the polyline").
- The editor snaps to roads (Valhalla), shows the live firing window, and warns about parallel roads, overpasses and slow frontage roads.

### 5.5 Path awareness: tiers, accuracy and the copy ladder
| Tier | How | Confidence and honest accuracy | Allowed copy |
|---|---|---|---|
| P0 presence | Inside the trigger shape | Limited by GPS (5 to 10 m open sky, 20 to 50+ m urban) | "near you" |
| P1 heading/corridor | Accuracy <= 65 m, >= 3 fixes within 60 s, speed >= 2.5 m/s for vehicle modes, smoothed course, cross-track <= buffer, heading within +/-45 degrees (corridor bearing +/-30), along-distance monotonically increasing over >= 3 fixes, along-distance within [d_min, d_max]. Pure geometry, no map matching. | Expect false fires on parallel roads, overpasses and dense grids. Direction, persistence and speed band are what remove the opposite carriageway. Dense-urban false-fire <= 15% is a tracked metric, not a hidden one. | "ahead" |
| P2 route-confirmed | Partner supplies the polyline and ETAs. The engine intersects the route AHEAD of progress with the geometry and fires inside the ETA window (for example 45 to 180 s ahead). | Highest (about 0.9 while cross-track < 50 m) | "on your route" |
| P3 trajectory swath | Kalman forward swath, length clamp(speed x lookahead, 300 m, 2 km), half-angle clamp(2 x course_accuracy + 8 degrees, 8, 40) | Usable at 0.5 to 2 km lead only. Unreliable beyond about 3 km and on curves/interchanges. Used in v1 only as a candidate widener, never alone for health classes. | "ahead" if corroborated by P1 |
| P4 learned habits | On-device Markov model over H3 res-9 | **Deferred, off.** Needs persistent history, creates a health-inference surface, adds store burden. | n/a |

Each signpost declares the minimum tier and `min_path_confidence` it accepts (0.5 general, 0.7 health classes). Road-skeleton snapping is deferred but the format reserves a field. GPS reality: heading is unreliable below about 2 m/s, so heading tiers require speed above 2 m/s.

### 5.6 Device pipeline (`proxim-decide`, deterministic and seedable)
1. **Quality gate:** accuracy <= 100 m (<= 50 m for place), age <= 30 s, reject mock locations (Android `isMock`, iOS `isSimulatedBySoftware`) outside simulator mode, reject impossible jumps.
2. **Path update:** smoothed heading/speed, activity, optional route progress.
3. **Candidate query:** R-tree of activation zones intersected with the corridor, route or forward swath.
4. **Hard filters (each yields a reason code):** signature, expiry, seq, kid not revoked, not revoked by list; consent purposes; partner category allow/block, sender block, sensitivity ceiling, min verified tier; flight/daypart/quiet hours; path tier and confidence; bearing/speed; child-directed kill; **sensitive-zone rules**; surface ready.
5. **Caps (hard):** per signpost 2/day and 5/week, per sender 3/day, per app interruptive 1 per 30 min and 4/day (partners may only lower, except nav/transit partners may raise to 8), dismiss gives a 7-day cooldown, hide is permanent for that signpost, report is permanent for that sender on that device. Local per install and reset on reinstall (documented).
6. **Score:** `S = V x Q x R x P`. V = min(rate_card_cpm / category_floor, 3) so money cannot buy unlimited relevance. Q = 0.5 + 0.25 x sender_trust + 0.25 x shrunk_ctr (computed server-side into the pack). R = partner-category match (1.0 exact, 0.6 adjacent, otherwise hard fail) + 0.2 per matching surface tag (cap 1.3) + motion/daypart fit + 0.3 if the topic is in the on-device Preference Center. P = path_confidence x g(ETA to anchor), where g peaks at the sender's ideal lead (30 to 120 s driving, 2 to 5 min walking). Weights are shipped in the pack so "why am I seeing this" lists true contributions.
7. **Pacing:** pass only if `hash(install_salt, jti, 15-min bucket) < p_serve`. The salt is local and never sent, and the draw is deterministic so reopening the app cannot reroll. A pace overlay older than 6 h halves p. Repeat-exposure decay 0.5^seen_count rotates creatives.
8. **Slot allocation:** top-1 per slot, ties by jti. If the surface is not ready, hold until TTL then discard silently without counting.

**Pricing in v1:** fixed rate card by place tier and format (no bidding), which keeps billing deterministic. Auctions are a post-pilot option.

**Driving-safe format:** when activity is automotive and speed > 5 m/s, only the glanceable variant is emitted (<= 7 words plus icon, no required interaction, >= 3 s display, CTA saved to a "Saved signposts" list). Non-driving-safe surfaces queue to an inbox with taps disabled while moving.

### 5.7 OS-limit-aware background scheduler (M3 beta, `proxim-sched`)
OS regions are scarce wake-up tripwires, not the matching engine. `plan_tripwires(position, heading, candidates, slot_budget)` is a pure, unit-tested function.
1. Slot 1 is always a **re-anchor ring** around the current position (radius 1.5 to 5 km by candidate density). Exiting it wakes the app to re-plan.
2. Candidate activation zones are clustered (greedy set cover) into approach rings. Ring radius R = clamp(v_class x (T_wake_p95 + T_lead), 150 m, 15 km). Example from the Android docs' latency range: highway 29 m/s x 240 s = about 7 km; walking 1.5 m/s x 240 s = about 360 m.
3. On wake, run a bounded high-accuracy burst (<= 60 to 90 s) and match exactly in the core. Drop any decision whose anchor is behind the user or older than 30 s.
4. **Leapfrog:** place the next 1 to 3 rings ahead along the predicted path.
5. **Host-shared budget:** defaults are iOS 1 + 8 of 20 and Android 1 + 40 of 100, exposed in the partner console and by API so the SDK never starves the host's own geofences. A partner that already uses many regions gets a visible "degraded" state.
6. Android re-registers on BOOT_COMPLETED, MY_PACKAGE_REPLACED and a WorkManager self-check. iOS re-registers on relaunch and after first unlock. Force-quit and permission downgrade degrade to M1/M2 and evaluate on next foreground. Dwell triggers in the background are best-effort only.
7. Whether Android 12+ allows the foreground-service start from a geofence wake, iOS legacy hysteresis and significant-change granularity, and iOS 26 relaunch behavior are all **unverified**. They are measured on devices at M4 and M11 and encoded as OS-model parameters tagged per OS release.

### 5.8 Sensitive-zone rules (enforced at authoring, signing, pack compile and in the kernel)
- **R1 standoff:** a health-class signpost's trigger geometry may not come within `standoff_m` (default **1,000 m**, raised by `jurisdiction_rules`; this exceeds WA's 610 m and CT/NV's 533 m with GPS and OS-cushion margin) of any registry facility, **and always of its own sender-declared facility even if the registry misses it**.
- **R2 inner ring:** while the device is within the standoff of any registry facility, health-class signposts are suppressed. Nothing is written to the impression ledger, no event is queued, and no learning happens (for all classes).
- **R3 destination suppression:** if the partner's route ends at a registry facility, health-class signposts are suppressed for the whole trip and the last 1.5 km of the route is truncated before matching (default `redactTerminal=true`).
- **R4 prohibited anchors:** reproductive/sexual health, mental health, substance-use treatment, HIV/STI, fertility, shelters, places of worship, immigration services, and any copy implying a condition ("Do you have ...") are never proximity-targeted in v1.
- **R5 billboard API rule:** closed-taxonomy context keys only. The SDK rejects unknown keys and the privacy lint fails the build if a user-attribute field is added.
- **R6 notifications:** health-class signposts are in-app surfaces only. No local notifications or lock-screen exposure.

---

## 6. Privacy architecture and sensitive-category (health) policy

### 6.1 Invariants (each mapped to a named test; `check-invariants` fails CI if one has none)
- **I1** `proxim-core` has no network dependency (cargo-deny).
- **I2** No request, header, log or fixture contains a coordinate pair, route, or an H3 index finer than res 5.
- **I3** Matching, path, caps, preferences and any history live only on the device, encrypted and excluded from backups. The trail ring is memory-only.
- **I4** Consent fails closed. Unknown or missing means denied. Withdrawal wipes the outbox and affected cached data.
- **I5** No IDFA/AAID, no ATT prompt, no cross-app identifier.
- **I6** Inside any sensitive buffer nothing is logged or queued.
- **I7** `health_facility` signposts produce no device events at all.
- **I8** Tile fetches carry no app key, cookie or device ID.
- **I9** Child-directed apps hard-disable the engine.
- **I10** Unsigned, expired, rolled-back or revoked content is never rendered.
- **I11** The base Android artifact never contains `ACCESS_BACKGROUND_LOCATION`.
- **I12** Health-class signposts require an SDK-issued consent receipt.

### 6.2 Data-flow inventory (the only things that may leave a device)
1. Tile manifest and pack GETs (res-5 cell, IP, time; no key). Global metadata GETs. Config GET (publishable key and SDK version, no location).
2. One-time attestation exchange (no location).
3. Events: one event per request, fixed padded size, `{event_id, signpost_id, rev, event_type, slot, UTC day, sdk/os version, attested flag, simulated flag, token}`. No hour, cell, session or device ID. Random 1 to 24 h delay, per-install daily cap, dropped on consent withdrawal.
4. "Report this signpost" carries only the signpost id and reason.

### 6.3 Phasing of network privacy (honest)
- **Phase 1 (closed beta, non-sensitive classes only):** reports are pseudonymous, not anonymous. Platform infrastructure logs (Vercel, Supabase, CDN) still hold IPs for their retention windows. We minimize log retention, disable request-metadata log drains and document it. The ingest token is a stateless HMAC issued after attestation. We do not store the mapping, but that is policy, not cryptography.
- **Phase 2 (hard gate before ANY health-category traffic, and before GA):** OHTTP through a relay operated by a different organization than the gateway, plus Privacy Pass-style blind tokens so the issuer cannot link a token to a report. External cryptographic review. The vendor and RFC numbers are confirmed at M0, and if no relay is commercially available the health pilot stays sandbox-only.
- **Phase 3 (deferred):** secret-shared aggregation (DAP).

### 6.4 Consent (verifiable, not just a flag)
- Purposes are separate and default-off: `location_matching`, `aggregate_reporting`, `nearby_health_events`, `local_notifications` (blocked for health). `route_learning` stays off in v1.
- General purposes come from the partner's CMP through `Proxim.consent.set` (TCF/GPP/GPC accepted).
- **`nearby_health_events` can be granted only through the SDK-owned `ProximConsentSheet`**, which writes a device-bound signed consent receipt (purpose, copy version hash, jurisdiction, timestamp) to the local store. The kernel refuses health classes without a valid receipt. Headless integrations cannot enable health classes. Receipts are exportable for the partner's audit, and the consent copy is versioned.
- `Proxim.privacyReport()`, `exportLocalData()` and `eraseAllLocalData()` exist. The partner-drop-in Privacy Center and Preference Center store state on the device only.

### 6.5 Health policy
- **Classes:** `general`; `health_adjacent` (gyms, pharmacies, wellness events); `health_facility` (hospital, clinic, public-health outreach); `prohibited_anchor` (never served); condition-specific content is rejected at moderation.
- **Who may send:** tier-2 verified senders only (NPPES NPI match, domain proof, phone callback, human review, MFA). Every health-class creative and every edit gets human review.
- **Who may receive:** partner `profile = sealed`, `health_tier_enabled` by signed addendum, user receipt, jurisdiction allows, background disabled, no notifications, assets prefetched, web SDK excluded.
- **Telemetry:** `health_facility` is **flat-fee sponsorship** with partner-measured reach, no device events, no pacing row (p fixed at 1), and no visit conversion ever. `health_adjacent` events are thresholded (k >= 100) and noised, with no cell and no hour.
- **No health inputs:** the SDK has no API for HealthKit, Health Connect or user health traits, and the partner contract forbids passing health-derived values. The integration scanner flags suspicious tag keys.
- **Live health traffic requires ALL of:** written counsel sign-off recorded in `docs/legal-gates.md` (WA, NV, CT, FTC order terms, Apple 5.1.3/2.5.18/5.1.5, Google Play declarations); DPIA; partner DPA; OHTTP + blind tokens live and reviewed; prod KMS signing keys with a key-compromise drill passed; registry governance in place; SDK-owned consent receipts shipped; external privacy review.
- **Registry governance:** NPPES + OSM + manual additions with audit log, a false-negative reporting channel, a quarterly completeness assessment (mirroring the FTC Kochava order's structure), and the "own declared facility" rule above.

### 6.6 Roles and legal posture (to be confirmed by counsel)
- Proxim intends to act as processor/service provider that receives no precise geolocation and does not sell or share personal information. A DPA template defines who obtains consent (partner via CMP plus the SDK receipt for health), who "collects" under on-device processing (treated as the partner's processing and possibly still "collection" under broad definitions), and what the App Privacy label and Google Data safety form must declare for the SDK (approximate location for advertising, not linked to identity). The console generates draft answers. We do not rely on "on-device means no data" as a legal conclusion.
- US-only. EU/UK deferred. COPPA: inert when child-directed.
- Retention: raw events 14 days; request logs 7 days; aggregates 25 months; billing records 7 years; audit log 7 years (holds no end-user data).

---

## 7. Partner SDK and API surface

**Principles:** headless engine first plus optional prebuilt UI. The SDK never requests location permission by default and holds no persistent identifier. Android and iOS base artifacts contain no background permission or mode.

**Surface (same shape on every platform; Swift shown)**
```
Proxim.configure(publishableKey, environment: .sandbox|.live, locationMode: .foreground|.partnerFed|.background)
Proxim.consent.set(...)            // from the partner CMP; health purpose only via ProximConsentSheet
Proxim.context.set(topics:, surface:)   // closed taxonomy ids only
Proxim.location.update(fix)        // partner-fed mode
Proxim.route.set(polyline6, etas) / clear()   // redactTerminal defaults to true
Proxim.signposts: AsyncStream<SignpostDecision>   // decisions already made
ProximSlot(slot: "home_banner")    // SwiftUI / Compose / <proxim-signpost slot="...">
signpost.markShown(visibleMs:), .markTapped(), .markDismissed(), Proxim.report(signpost, reason)
Proxim.setSurfaceReady(Bool), Proxim.privacyReport(), Proxim.eraseAllLocalData()
ProximTestKit: GPX/GeoJSON feed, fixed clock, "why nothing showed" inspector
```
**Rendering:** a standardized text-first card (verified sender chip with signed name/domain/tier, headline <= 40, body <= 90, glanceable variant, distance/direction chip, CTA, mandatory "Sponsored" label, "Why this?" and hide/report menu). Partners control theme tokens, not content or chrome. Headless integrations must use `ProximRequiredChrome` and are billed as "unverified display" at a reduced share until reviewed. A billable impression requires >= 50% of pixels visible for >= 1 s. Landing pages open in SFSafariViewController/Custom Tabs with no redirect hop through Proxim.

**Server API (OpenAPI 3.1, `/v1`, generated clients, Schemathesis-fuzzed)**
- Public to devices: `GET /t/v1/{r5}/manifest` and pack objects (no key); `GET /m/v1/timestamp|keyset|revocations`; `GET /v1/config` (publishable key, signed partner config); `POST /v1/attest`; `POST /v1/events` (one event; OHTTP target in phase 2); `POST /v1/reports`.
- Partner (secret key or console): apps, keys, policies, surfaces, revenue reports, statements, webhooks `signpost.revoked`, `policy.updated`.
- Sender: campaigns, signposts (draft then `/submit`), `POST /v1/triggers:preview` (geometry + route to firing timeline using the same WASM core), places search, delivery reports, top-ups. Sender keys can create drafts only and never publish without review.

**Partner controls:** category allow/block (Proxim taxonomy mapped to IAB Content Taxonomy 3.x), sender blocklist, sensitivity ceiling, min verified tier, Sealed profile, caps (stricter only), quiet hours, slots, competitive separation, driving-safe override (stricter only), region budgets, background on/off, remote kill switch (signed flag in the 5-minute config), "Report" wired to moderation.

**Integrate in about a day (design target, measured in M13)**
- Hour 0 to 1: dependency, `configure`, run `npx proxim doctor` (Info.plist strings, manifest, privacy manifest, ProGuard rules, PendingIntent flags, stray health-looking keys).
- Hour 1 to 3: wire consent (drop-in sheet or host API).
- Hour 3 to 5: place the slot, set context.
- Hour 5 to 6: call `location.update` or `route.set` if the app has them.
- Hour 6 to 8: run the shipped GPX scenarios via TestKit, confirm the console flips to "SDK detected", download the generated store-label answers.

---

## 8. v1 scope versus deferred

**v1 (closed beta + single-metro pilot; sized for a very small team)**
- `proxim-core`, `proxim-pack`, `proxim-sim` with iOS/Android OS models, oracle metrics, scenario corpus and CI gates (the proof that triggers fire correctly).
- Triggers: place, area, corridor with bearing/speed band, approach, partner route-intent matching, P1/P2/P3 path tiers.
- Modes: foreground and partner-fed on **Web and Android**, then **iOS**. Managed-background is a flagged **Android-first beta** in a separate artifact.
- Signed per-cell packs, pace overlay, revocation, signed partner config, KMS-backed signing.
- Sender console: onboarding with tiers 0 to 2, MapLibre editor, live phone preview, "Drive it" panel, prepaid Stripe credits (test mode, then live for non-health), delivery reports. Fixed rate-card CPM on viewable impressions.
- Moderation: structured creatives only (no HTML/JS/tags), rules + Claude pre-screen, Web Risk, human approval of every signpost, report/hide auto-pause (>= 0.5% after 2,000 viewable impressions).
- Partner console: apps, keys, policy, handshake detection, kill switch, store-label generator, revenue dashboard with manual statements.
- Events: single-event unlinkable-by-design reports, attestation-gated ingest tokens, k-thresholds and stored-once noise, ledger and reconciliation.
- Health: full pipeline built and demonstrated in sandbox/PSA with HealthDemo; live traffic gated (section 6.5).
- Sandbox synthetic inventory and PSA house campaigns for cold start.

**Deferred**
- React Native/Expo wrapper, Flutter, live Stripe Connect payouts (manual first), cloud-KMS 2-of-3 root ceremony (single hardware root first).
- OHTTP + blind tokens in production (scheduled as the hard gate before health traffic and GA), DAP aggregation.
- Learned habit routes (P4), road-skeleton snapping, CarPlay/Android Auto, local notifications (non-health only, later).
- Auctions, live-confirm vouchers, CPA, OpenRTB.
- Visit/footfall conversion (non-sensitive only, local-DP, later), geo-lift holdouts.
- EU/UK, reproductive/mental-health/substance/HIV categories (not planned), self-serve healthcare senders, auto-approval tiers and trust-score-driven SLAs.
- Delta packs, multilingual moderation, cross-app capping (impossible without an identifier).
- Partners needing server-side decisions (offer `proxim-core` in their own backend).

---

## 9. Milestones (each with deliverable and concrete verification)

| # | Deliverable | Verification |
|---|---|---|
| **M0** Foundations and walking skeleton | Monorepo, CI (ubuntu + macOS), new Supabase project (not "ilios Healthcare"), `docs/` set including `privacy-invariants.md` and ADR-000. **Toolchain spike:** one Rust function through UniFFI to an XCFramework, an AAR and WASM, with a golden vector passing in CI on macOS and Linux. Burn-down of unverified facts: `list_extensions`, Supabase/Vercel limits, iOS and Android region/relaunch docs, Google Play effective date, OHTTP relay commercial availability, RFC numbers. | CI green on an empty-but-wired workspace; golden vector identical across Rust, WASM, Android and iOS builds; ADR-000 closed with dates; CI check fails when the configured project ref is the ilios Healthcare ref; `check-invariants` fails the build for an invariant with no test; cargo-deny rejects a deliberate network crate in core. **Go/no-go:** if the spike cannot be made to work inside a fixed time-box (2 weeks), fall back to native reimplementation (Kotlin, Swift, TS) guided by the shared golden fixtures. |
| **M1** Core geometry and **simulator v1** | `proxim-geo`, `proxim-path`, `proxim-decide` v0, `proxim-sim` (GPX/GeoJSON/synthetic generators with noise, tunnel dropouts, scenario YAML, omniscient oracle, JUnit, CLI), first 10 scenarios (expressway inbound/outbound/opposite carriageway, overpass, parallel frontage road, walking, tunnel, health-standoff). | proptest vs a turf.js reference on 1M random cases within tolerance; scenarios pass in CI; negative scenarios produce no fire; mutation check (deliberately broken matcher turns the suite red); byte-identical JSON report on rerun and across OSes. |
| **M2** Signed packs | `proxim-pack` (JWS ES256, manifest, seq/expiry/rollback, revocation, keyset), TS pack compiler, local test keys, golden vectors. | Flip any byte, wrong kid, revoked kid, lower seq, expired manifest, replayed pack: all rejected; Rust and TS verify each other's vectors; same DB snapshot gives byte-identical pack hash; tier-0 size budget measured and set; fuzz run on the parser. |
| **M3** Decision kernel and privacy lint | Consent fail-closed, partner policy overlay, caps, scoring, pacing draw, slots, sensitive-zone R1 to R6, driving-safe format, child kill, policy matrix as data with exhaustive tests; `privacy-lint` and egress-audit harness. | Consent fuzz (no processing without purposes); cap invariants for random fix sequences; scenarios prove health-class signposts never fire, log or queue inside the standoff and are suppressed on destination; policy matrix tested exhaustively; lint fails when a `lat` field is injected into a fixture; egress audit passes for every scenario. |
| **M4** OS model, tripwire planner and **early real-device harness** | `OsAdapter` trait + SimulatedOS (iOS and Android profiles, failure injections: reboot, force-quit, permission downgrade, Low Power, clock skew, expired/revoked pack), `plan_tripwires`, `tools/field-test` logging app (one iPhone + one Android), first scripted walks/drives. | Monte Carlo (1,000 runs/scenario nightly): slot use never exceeds budget, re-anchor ring always present, recall vs oracle reported per speed class; latency pushed past ring sizing fails as expected; first field logs compared with simulator predictions and OS-model parameters updated with OS-release tags. Docs state that the simulator validates logic, not real wake behavior. |
| **M5** Schema, RLS, API, signer | Migrations, RLS, constraints, facility registry import, Hono API, worker pack builder/publisher, KMS signing in staging, jobs table. | pgTAP: 40+ cross-tenant/anon attack cases pass; advisors clean; standoff and size constraints reject bad geometry; Schemathesis clean; tamper/rollback/rotated-kid tests; k6: cached pack GET 500 rps p95 <= 150 ms at edge; revocation reaches a simulated device within the 5-minute TTL. |
| **M6** Web SDK and sample | WASM core, `<proxim-signpost>`, sample, TestKit, quickstart compiled in CI, sandbox synthetic inventory. | Playwright `setGeolocation` walks a route and the card appears in the expected window and not before/after; HAR proves no coordinate in any request; axe checks pass; a developer who did not write the SDK completes the quickstart (timed). |
| **M7** Sender console, moderation, "Drive it" | Auth/orgs, tiers 0 to 2, map editor, snap-to-road, standoff warnings, phone preview, WASM Drive-it, human queue, rules + Claude + Web Risk, approve-sign-publish. | Playwright E2E from signup to approved to pack head advances to the web sample fires it; Drive-it output equals CLI output on 20 shared scenarios; **rogue-sender suite** (edit after approval, claim a hospital place, read another org, submit prohibited anchor, medical-claim copy, lookalike name) all blocked or queued; 60+ labelled creatives with zero false-approves on impersonation and miracle-claim sets; model-API outage fails closed. |
| **M8** Android SDK and sample | UniFFI Kotlin, `sdk` + `sdk-background` AARs, Compose/View UI, Play Integrity, boot/replace re-registration, mock-location filters. | Instrumented tests (test provider, emulator route replay) match the Rust runner's fires; **manifest-merge test fails if the base AAR has ACCESS_BACKGROUND_LOCATION**; OkHttp egress test; reboot re-registration on a device; physical-device drive logged. |
| **M9** Partner console, events, billing | Partner portal, signed config + kill switch, store-label generator, attested ingest tokens, single-event outbox, daily close, k-thresholds with stored-once noise, ledger, Stripe test-mode top-ups, manual statements. | Property tests (ledger sums to zero, charge <= budget, idempotent settlement); replayed events rejected; fraud sims (no attestation, replay, spoofed GPX, canary hit) rejected or flagged; re-querying a noised report returns the identical value; `health_facility` produces zero device events; reconciliation job matches ledger and aggregates; config change propagates within one TTL. |
| **M10** iOS SDK and sample | UniFFI Swift, SwiftPM (`Proxim`, `ProximBackground`), SwiftUI/UIKit, privacy manifest, ConsentSheet with receipts. | XCTest conformance reproduces all golden vectors; XCUITest with `simctl location` playback matches Rust fires; URLProtocol egress test; TestFlight run on a physical device; App Store privacy-label dry run. |
| **M11** Background beta (Android first) | `sdk-background` scheduler with leapfrogging, region-budget settings, iOS legacy-region implementation after Android. | Simulator target: >= 85% of approach signposts fire at least 1 km before the anchor at 65 mph under the p95 240 s latency profile (provisional until measured); regions never exceed budget; 3 drives + 3 walks per OS compared with prediction, **field recall within 10 points of simulator or OS parameters are updated and scenarios rerun**; results documented as known limits. |
| **M12** Hardening and gates | Prod KMS key and rotation/compromise drill, hardware root, OHTTP + blind tokens, external pen test and privacy review, store dry-runs, DPIA, DPA, incident and takedown runbooks, registry governance. | High pentest findings closed and retested; key-compromise drill passes; mitmproxy on production builds shows no IP-bearing report path once OHTTP is on; takedown drill stops a live signpost within the stated SLA; counsel memo on file. |
| **M13** Single-metro pilot (non-health) | 2 to 3 named location-core partners, 10 to 20 local senders, PSA fill. | Pre-registered thresholds: observed fire rate and lead time within the modelled band; hide-or-report < 0.5%; partner vs Proxim viewable counts within 1%; zero privacy-lint/egress violations; billing reconciled for four consecutive weeks; three independent integrators each finish in one working day; geo-holdout/promo-code evidence collected for the CTR gap vs behavioral baselines. |
| **M14** Health pilot (gated) | One Sealed-profile partner, one tier-2 sender, approach signposts 1 to 8 km out, flat fee, in-app only. | All section 6.5 gates documented before any traffic; scenario proof that no signpost fires or logs inside the standoff and none fires for a destination at a facility; zero device events; kill-switch drill. |

---

## 10. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Background reliability is far weaker than the pitch implies (iOS cap 20, no restore after reboot until unlock, force-quit reports; Android 2 to 6 min latency, loss on reboot/replace/data clear) | Foreground and partner-fed are the primary modes. Ring sizing from p95 latency, leapfrogging, re-registration, early device harness (M4), parameters tagged per OS release, nothing promised for terminated apps. |
| Store rejection (Apple 5.1.3/5.1.5/2.5.18, Google background-location core-function rule) | Base artifacts without background; partners own prompts and declarations; no health inputs; store-readiness kit and dry runs; target partners with genuine location use. |
| Health legal exposure (WA geofence ban with private right of action and apparently no consent exception; CT/NV; FTC; GDPR Art. 9) | 1,000 m standoff, approach-only, in-app only, flat fee, no telemetry, receipts, counsel gate, US-only, sandbox until gates pass. Residual: regulators may still view an offsite health message to a Health-app user as location-based. |
| Cold start: no inventory, narrow partner set | Sandbox inventory, PSA fill, named design partners, single metro. |
| Lower eCPM and weaker proof for local businesses than behavioral networks | Position as billboard economics; promo codes/QR, RSVP, holdouts; measure gap in pilot; partners may later run the core server-side. |
| Path accuracy (parallel roads, overpasses, grids) | Route intent preferred, bearing/speed/persistence gates, editor warnings, copy ladder, tracked-weak scenarios. |
| Client-trusted billing and fraud | Attestation, mock filters, canary signposts, per-token caps, anomaly checks vs tile-fetch counts, holds, billing capped at budget. |
| Rust+UniFFI toolchain and size (about 1.5 MB per ABI, to be measured) | M0 spike with a fixed time-box and a native-reimplementation fallback; golden vectors across every binding. |
| Key compromise or signing bug | KMS from the first external partner, offline root, kid revocation, short TTLs, compromise drill, byte-flip fuzz in CI. |
| Bundle-fetch correlation (IP to cell) | No key on tile requests, coarse cell, neighbour prefetch only near edges, minimal logging, OHTTP gate. |
| Consent integrity (partner can lie) | SDK-owned receipt for health, contract terms, audit export, spot audits. |
| Hospital impersonation or misleading health claims | Tiers, place-claims, NPI and domain checks, signed sender identity rendered by the SDK, human approval, immutable revisions, auto-pause, takedown SLA. |
| Registry incompleteness | Own-declared-facility rule, manual additions, report channel, quarterly assessment. |
| Over-scoped for a small team | Section 8 cuts; serial milestones with go/no-go points; no Redis, no queue vendor, no auction, no custom crypto until M12. |
| Unverified facts (Swift/Kotlin versions, Postgres/PostGIS availability, Google effective date, OHTTP relay, Android 12+ FGS exemption) | ADR-000 burn-down at M0 and before each release; physical unknowns become measured simulator parameters. |
| Distracted-driving exposure | Glanceable-only while driving, inbox queueing, disabled taps, partner `driving_safe` flag, no audio. |

---

## 11. Testing and CI strategy

**The simulator is a first-class deliverable and arrives at M1, before any server, console or native SDK.** It runs the real `proxim-core` against a scenario and asserts what fires, when and how far ahead. It can only validate our logic. Real OS wake behavior is validated only by device drives, so every doc and CI gate says so.

1. **Unit, property, fuzz:** proptest for geometry vs turf.js, planner invariants, cap invariants, consent fuzz over all purpose combinations; cargo-fuzz on JWS/pack parsing; h3o vs h3-js parity on 100k points.
2. **Golden vectors:** the same decision traces must be byte-identical in Rust, WASM, Android and iOS (libm for determinism).
3. **Scenario corpus (YAML):** route, speed profile, noise model, OS profile, pack fixture, partner policy and context, consent, seed, expectations ("fires between 1.0 and 3.0 km before the anchor", "never inside standoff", "max 2/hour", "no event contains coordinates", "regions <= budget"). Monte Carlo nightly (500 to 1,000 seeds). Initial gates (provisional, tuned after field data): route-confirmed recall >= 95%, corridor >= 90%, swath >= 80% at 1 to 2 km lead with false-fire <= 10%, overall recall vs oracle >= 90% (foreground/partner-fed). Known-weak scenarios (parallel road, overpass, dense grid) are tracked metrics, not hidden.
4. **Egress audit and privacy lint:** static lint over OpenAPI, SQL and SDK event types for forbidden fields; runtime capture (Playwright HAR, URLProtocol, OkHttp interceptor, mitmproxy) asserting no coordinate pair near the route, no H3 finer than res 5, no signpost id without reporting consent, no request outside the allowlist.
5. **Database:** pgTAP RLS attack suite, constraint triggers, ledger invariants, Supabase advisors.
6. **API and backend:** Schemathesis, k6 (cached pack GET and event POST), snapshot tests (byte-identical packs, size budgets), billing property tests, replay/fraud simulations, moderation corpus (zero false-approve on impersonation and health claims, fail closed on outage).
7. **Native harnesses:** XCUITest with `simctl location`, Android instrumentation with test providers, manifest-merge test for the background permission. These validate shell code, not wake behavior.
8. **Field drives:** `tools/field-test` on 2+ devices (iOS and Android), 3 drives and 3 walks per OS per mode, with force-quit, reboot and Low Power variants; rerun on every OS beta; results recalibrate the OS model.
9. **End-to-end:** sender creates, moderation approves, builder publishes, simulated SDK fetches and fires, report is made, aggregates, release thresholds and invoice reconcile; revoke mid-trip and confirm the device stops after the next poll.
10. **CI:** GitHub Actions (ubuntu: Rust, TS, pgTAP, Android emulator; macOS: iOS builds and simulator), Vercel previews, Supabase branch per PR, gitleaks, CodeQL, Dependabot, cargo-deny, `check-invariants`, nightly Monte Carlo and fuzz. Required checks on main: lint, unit, golden vectors, privacy-lint, egress audit, pgTAP, manifest-merge.

---

## 12. Security and compliance checklist

**Judge must-fix mapping:** MVP cut to Web + Android first (section 8); toolchain spike and fallback in week one (M0); early device harness (M4); kickoff verification of OS, policy and platform facts (M0 and ADR-000); host-shared region budget (5.7); no default Always requirement and named first segment (5.3, section 1); flagship reframed (section 1); simple human moderation (section 8); simulator claims separated from reality (section 11); minimal ops and KMS before external partners (section 2, M5, M12); verifiable consent (6.4); OHTTP before health traffic (6.3); no health per-impression telemetry (6.5); registry governance (6.5); controller/processor/DPA (6.6); no notifications for health (R6); primary-source verification (ADR-000).

**Checklist (all must be green before the stated gate)**

Before first external partner (closed beta):
- [ ] New Supabase project; CI blocks the ilios Healthcare project ref.
- [ ] RLS forced on every exposed table; anon has zero grants; pgTAP attack suite required; advisors clean.
- [ ] Authorization via `app_metadata` only; MFA (`aal2`) for staff, payout and tier-2/health orgs.
- [ ] API and sender keys stored as hashes, shown once, scoped, bound to bundle id/package + cert/origin; rotation and revocation work.
- [ ] Prod signing key in KMS via OIDC (no static cloud credentials in Vercel/CI env); hardware root key; clients pin roots and reject unknown/expired/revoked kids; rollback and replay checks.
- [ ] Only approved immutable revisions are signed; senders cannot hold keys; structured creatives only; landing-URL scans with redirect resolution; SSRF-safe fetching; re-scan on edit.
- [ ] Audit log hash chain verified; manual ledger adjustments logged.
- [ ] Privacy invariants I1 to I12 each have a named passing test; egress audit and privacy lint required on main; log scrubber test; Sentry only in console.
- [ ] Base Android AAR has no background permission; iOS background lives in a separate SwiftPM product; privacy manifests ship; store-readiness kit generated.
- [ ] Moderation: human approval of every signpost; fail-closed on classifier outage; report/hide auto-pause; takedown runbook with SLA tested.
- [ ] Rate limits, request size limits and attestation on public endpoints; webhook HMAC with replay window; Stripe webhook idempotency.
- [ ] Partner DPA and sender terms (no health-derived inputs, no resale, clawback on abuse).

Before any health-category traffic (all of):
- [ ] Counsel written sign-off for WA, NV, CT, FTC order terms, Apple and Google policy position; recorded in `docs/legal-gates.md`.
- [ ] DPIA complete; primary-source verification of statutes and store policies recorded with dates.
- [ ] OHTTP with an independent relay and blind tokens live and externally reviewed.
- [ ] SDK-owned consent receipt enforced; `nearby_health_events` default-off and separate.
- [ ] Standoff, inner-ring, destination suppression and prohibited anchors enforced at authoring, signing, pack compile and kernel; scenarios prove it.
- [ ] Flat fee; zero device events for `health_facility`; thresholded and noised aggregates (k >= 100) for `health_adjacent`; no hour or cell anywhere.
- [ ] Registry governance live; own-declared-facility rule enforced.
- [ ] Key-compromise and kill-switch drills passed; tier-2 sender MFA; human review of every health edit.
- [ ] In-app only; no notifications; web SDK excluded.

Before GA / any non-US launch:
- [ ] 2-of-3 offline root ceremony; external pen test closed; EU/UK requires separate DPIA, EU representative, ePrivacy 5(3) and DSA 26(3) review; re-check Google Play background-location effective date and Android 17 behaviors; re-verify state laws (CT, NV, MD, OR, VA, NY).
