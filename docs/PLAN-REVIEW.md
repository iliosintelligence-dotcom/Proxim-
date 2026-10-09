# Proxim plan review

Companion to [`PLAN.md`](PLAN.md). Generated 2026-10-09.

## How the three designs scored

| Design | Judge total (3 lenses, max 30) |
|---|---|
| On-device-first (privacy by architecture) | 21 |
| Server-decisioning (ad-tech style) | 17 |
| Partner-DX-first (hybrid) - **chosen** | 23 |

## Open questions for the owner

1. Who are the 2 to 3 named design partners and what segment is first (navigation, transit, parking, EV/fuel, fitness, or a Health app)? The cold start and the pilot metro depend on this, and Health apps will mostly get foreground-only delivery.
2. Is it acceptable that the hospital-outreach flagship ships in v1 only as sandbox/PSA with approach signposts 1 to 8 km out (never at the hospital door), with live health traffic waiting on counsel sign-off, OHTTP, and the other gates? Who is the counsel and what is the budget and timeline?
3. Commercial terms: partner revenue share (planned 65 to 70 percent), rate card by place tier, flat-fee sponsorship prices for health, and whether PSA/civic senders run free or house-funded.
4. Legal entity, brand and domain for Proxim, and can you export the Lovable prototype (code or screenshots) so the console UX terminology can be reconciled at M7?
5. Do you have, or will you buy, at least one iPhone and one Android phone for the field-drive harness at M4, and do you prefer Android-first (the plan's default, because the geofence behavior is documented and emulators are easier) or iOS-first?

## Critic verdict

The plan is ready to start M0/M1. I found no blocker, and every "R" version I checked against crates.io and npm matches the registry as of today. Several important design contradictions should be fixed in the spec before M2 and M5 lock the pack format, pacing and billing.

## Critic findings (20)

No blockers. Items 1-10 are *important* and should be settled in the spec before the milestone that locks the affected format.

### 1. [IMPORTANT] Cross-platform byte-identical golden vectors are claimed, but the plan relies on std f64 math in several places

Cross-platform byte-identical golden vectors are claimed, but the plan relies on std f64 math in several places. `libm` is listed, yet `geo`, `h3o` and the Kalman and scoring code will call std `sin`, `cos`, `atan2`, `exp`, `powf` and `ln`. These resolve to glibc, Apple libm or compiler-builtins on Linux, iOS, Android and WASM, and they differ in the last ulp. M0, M1 and the end-to-end gate demand byte-identical traces across Rust, WASM, Android and iOS, so they would fail or flap.

**Fix:** Make 'no std transcendental math in the decision path' an enforced rule (clippy disallowed-methods) and route all trig, exp and pow through `libm`. Do not use `geo`'s geodesic or haversine in the kernel, or wrap it. Alternatively, define golden equality as exact decisions plus tolerance-bound intermediate values. Test the cross-platform float claim in the M0 spike, not only a trivial function.

### 2. [IMPORTANT] Pack size and device cache are internally inconsistent

Pack size and device cache are internally inconsistent. Tier 0 holds 500 signposts at 350 KB gzip excluding assets, but assets (logo up to 64 KB, image up to 40 KB) are prefetched with the pack. That is roughly 50 MB of assets against a 5 MB, 6-cell device cache cap. The size budget is deferred to M2 measurement, after the format is designed.

**Fix:** Set an explicit per-cell asset budget now. Rank-truncate to what fits (for example, assets only for the top 30 to 50 signposts), share assets across signposts by content hash, and decide the LRU eviction rules. Verify the budget in M2 against a synthetic dense metro.

### 3. [IMPORTANT] Pacing and billing control loop is broken by the privacy design

Pacing and billing control loop is broken by the privacy design. Events are delayed 1 to 24 h and carry only a UTC day, so the server learns spend up to about 25 h late. The `p_serve` overlay is refreshed every 6 h. Packs are identical for all devices and caps are client-trusted. A signpost with a small budget in a busy cell can overshoot massively, and 'overshoot absorbed by Proxim' is an uncapped liability. No exposure bound or overshoot metric is defined.

**Fix:** Model worst-case overshoot as a function of devices times per-device caps times lag. Cap it structurally, for example with a per-signpost max-devices-per-day proxy via tile-fetch counts, conservative `p_serve` initialised from fetch counts, and hard per-signpost lifetime impression ceilings baked into the pack. Add a simulator scenario with 10k simulated devices, report overshoot ratio, and set an acceptance threshold at M9.

### 4. [IMPORTANT] Confidential protected locations are shipped to every device through a public, keyless CDN

Confidential protected locations are shipped to every device through a public, keyless CDN. `sensitive_places` includes shelters (domestic-violence shelters keep addresses confidential) plus mental health, reproductive and HIV/STI clinics, and these perimeters are 'pack-shipped suppression zones'. Anyone can bulk-download the pack, and the plan treats this only as a targeting concern.

**Fix:** Exclude confidential-address categories from pack contents. Suppress by coarse over-approximation, or by ensuring no signpost may be anchored or routed within a large margin of those areas at authoring time instead of shipping them. Review each `kind` for publication safety and record the decision in an ADR.

### 5. [IMPORTANT] The sensitive-zone registry will make much of an urban area unservable and will have data-quality problems, with no feasibility measurement

The sensitive-zone registry will make much of an urban area unservable and will have data-quality problems, with no feasibility measurement. A 1,000 m standoff from every hospital, clinic and pharmacy (NPPES has hundreds of thousands of providers) can blanket downtown cores. R1 also applies to `health_adjacent` (pharmacies, gyms) while pharmacy is itself a sensitive-place kind, so a pharmacy sender is inside its own standoff. NPPES NPI-1 records include individual practitioners' addresses, some of which are home addresses.

**Fix:** Import only organisation NPIs (NPI-2) and practice-location addresses. Clarify which classes the standoff applies to and whether the sender's own facility is exempt for non-facility-adjacent classes. Compute a coverage map for the pilot metro in M5 showing the percentage of area and corridors where health classes are unservable, and use it to choose the pilot metro and standoff defaults.

### 6. [IMPORTANT] Measurement hard problem is under-specified

Measurement hard problem is under-specified. The value proposition promises promo-code and RSVP conversion counting, but the data model, event allowlist, API and milestones have no conversion entity, no redemption-reporting path and no fraud model for it. The DP/k-threshold scheme has no epsilon, no composition accounting across day, week and month releases, and no k for non-health classes. At pilot scale (10 to 20 local senders) most counts will fall under k or be noise-dominated, so small senders see nothing. Billing uses raw counts while senders see noised ones, which invites disputes.

**Fix:** Add a `conversions` mechanism (per-signpost redemption codes or sender-API redemption counts, aggregate only) and its tests. Specify epsilon per release and a total budget per signpost, with a single release schedule. Add general-class k values and a policy that billing disputes are resolved against raw aggregate counts shown to the sender as exact for non-health classes. Test with a small-sender pilot simulation to confirm the reports are usable.

### 7. [IMPORTANT] Post-approval sender abuse through landing pages

Post-approval sender abuse through landing pages. Landing URLs are scanned only on creation or edit of the signpost. The page content can change after approval (bait-and-switch, hospital impersonation, phishing, geo- or UA-cloaking). Taps open the page directly with no Proxim redirect hop, so there is no click-time check. The auto-pause only reacts to hide and report rates.

**Fix:** Add a scheduled re-crawl and re-scan of every live landing URL (rendered, from multiple user agents and regions, with redirect-chain and domain-age checks) that auto-revokes on a hit, with a tested revocation SLA. Add a domain-ownership binding so the landing domain must match the verified sender domain, and add cloaking and bait-and-switch cases to the rogue-sender suite.

### 8. [IMPORTANT] Battery, CPU and SDK-footprint budgets are absent

Battery, CPU and SDK-footprint budgets are absent. Battery is one of the eight hard problems, yet no milestone has a measurable energy budget, and M1 foreground mode runs continuous GPS while the app is open. There is no memory, startup or APK/IPA size budget beyond the '1.5 MB per ABI' note, and no energy profiling in the field harness.

**Fix:** Define budgets (for example mAh per hour in foreground and per day in background, p95 cold-start overhead, and APK/IPA delta). Collect battery in `tools/field-test` using Xcode Energy and Battery Historian or Perfetto. Make the budgets pass/fail at M8, M10 and M11.

### 9. [IMPORTANT] Approximate-location grant is not handled

Approximate-location grant is not handled. iOS 14+ and Android 12+ let users grant only approximate location (about 1 to 3 km). The quality gate rejects fixes with accuracy over 100 m, so for these users the engine silently never fires, and nothing in the UX, `doctor` or telemetry-free diagnostics explains why. This is a large share of real users.

**Fix:** Define behaviour explicitly: area signposts with a large radius only, or no-op with a 'why nothing showed' reason code. Include approximate-grant profiles in the OS models and scenarios, and have `ProximTestKit` and the copy ladder cover it.

### 10. [IMPORTANT] No age or minor signal

No age or minor signal. The plan has only a coarse `child_directed` kill switch. Mixed-audience apps with teens and 13-17 users are not addressed, although several state laws restrict targeted advertising to known minors and use of precise geolocation. The SDK has no input for the OS age signals (Play Age Signals API, Apple's declared age range) or partner age gates.

**Fix:** Add a closed-taxonomy `audience` input (adult-only, mixed, unknown), treat unknown conservatively for sensitive classes, and add it to the policy matrix, DPA and consent tests. Verify the current OS age-signal APIs and state minor-privacy laws in the M0 burn-down.

### 11. [MINOR] Web SDK breaks the 'tile requests carry no partner identifier' invariant (I8)

Web SDK breaks the 'tile requests carry no partner identifier' invariant (I8). A cross-origin CORS fetch from the partner page always sends an `Origin` header, which identifies the partner app. Setting `no-referrer` only removes Referer. The invariant is tested for native only, and the plan treats web as a safe place for non-health traffic only implicitly.

**Fix:** State that web tile requests expose partner origin, or proxy them through a same-origin partner path or a keyless edge function that strips it. Make I8 test the real `Origin` header in the Playwright HAR check.

### 12. [MINOR] Attestation conflicts with 'no persistent identifier'

Attestation conflicts with 'no persistent identifier'. App Attest requires the server to store a per-install key id and a counter to verify assertions, and Play Integrity verdicts can carry device-recall signals. A token 'issued after attestation' with 'per-token caps' implies the issuer can count or link per install. The plan says attestation defeats farms 'without a persistent device ID', which is not accurate for iOS.

**Fix:** State exactly what the issuer stores. Specify that attestation is exchanged only with the token issuer (not the ingest path), what is retained and for how long, and why this does not violate I5. Write this into the privacy report, the store-label answers and the DPA.

### 13. [MINOR] Wake-latency arithmetic for the background re-anchor ring is inconsistent with the approach-ring sizing

Wake-latency arithmetic for the background re-anchor ring is inconsistent with the approach-ring sizing. The re-anchor ring is 1.5 to 5 km by candidate density, not by speed. At 29 m/s and a 240 s p95 wake latency, the user is up to about 7 km beyond the ring when the app wakes, so any anchor within that stretch is missed before the planner re-plans. The M11 85% recall target would fail by construction on dense highway corridors.

**Fix:** Size the re-anchor ring with the same v x (T_wake + T_lead) formula. Add a Monte Carlo scenario that explicitly covers highway speed with the re-anchor wake, and acknowledge that high-speed background delivery will only be reliable on routes with sparse leapfrogging assumptions.

### 14. [MINOR] Pack compiler language is contradictory

Pack compiler language is contradictory. Section 2 says one Rust implementation runs in the pack compiler, but M2 builds a 'TS pack compiler' and `packages/geo-ts` re-implements corridor derivation, H3 covers and standoff checks. This duplicates geometry in two languages, the exact reason server-decisioning was rejected. 'Byte-identical pack hash' then also depends on TS float formatting and ordering. Running compile plus KMS signing plus Storage upload inside time-limited Vercel Functions is also a weak fit.

**Fix:** Compile packs through the WASM core in the worker. Limit `geo-ts` to console UX preview and parity-test it. Decide whether the worker runs on Vercel Functions or a longer-running host, and set function-duration limits for metro-scale rebuilds.

### 15. [MINOR] Revocation latency mismatch

Revocation latency mismatch. Revocation and kill-switch lists are polled 'at app start and each refresh', while the M5 gate and config TTL promise propagation within 5 minutes. A navigation app that stays foregrounded for hours may not poll for up to the 24 h soft TTL, and a device that is offline can render a revoked signpost for up to 72 h.

**Fix:** Specify a poll cadence while active (for example every 5 minutes on the non-location global endpoint) and a fail-closed rule for staleness of the revocation list. Record the worst-case takedown time per mode and define the 'stated SLA' used in the M12 takedown drill, which is currently never given a number.

### 16. [MINOR] Android 16 KB page-size requirement and non-GMS devices are not mentioned

Android 16 KB page-size requirement and non-GMS devices are not mentioned. Rust native libraries must be linked for 16 KB alignment to meet current Play requirements for newer target SDKs. The Android SDK depends on Play Services Location, so Huawei, Amazon and degoogled devices silently lose M1 to M3 behaviour with no fallback.

**Fix:** Add 16 KB alignment (NDK and linker flags) to the M0 AAR spike and CI check. Add an AOSP `LocationManager` fallback for foreground and partner-fed modes, or document unsupported devices and have `doctor` flag them.

### 17. [MINOR] Money-handling edge cases are missing

Money-handling edge cases are missing. There is no refund, chargeback or dispute handling in the ledger after prepaid credits have been consumed. Unused prepaid balances held by the platform and later partner payouts raise stored-value, tax-form (W-9, 1099) and sales-tax questions that are only partly mentioned ('Stripe Tax later').

**Fix:** Add chargeback and refund ledger entries and tests at M9. Add a partner tax-onboarding step before the first statement. Put prepaid balance terms (non-refundable, expiry) on the counsel checklist.

### 18. [MINOR] Pilot metro and jurisdiction are not selected, and jurisdiction rules cover only health geofences

Pilot metro and jurisdiction are not selected, and jurisdiction rules cover only health geofences. Some state privacy laws (for example Maryland's sensitive-data 'strictly necessary' rule, which covers precise geolocation) may restrict the partner's on-device processing of precise location for advertising regardless of consent. This plan lists Maryland only as a pre-GA re-verify item.

**Fix:** Add pilot-metro choice and a state-by-state precise-geolocation and ads matrix to the M0 legal burn-down. Feed it into `jurisdiction_rules` and the policy matrix, and pick a pilot state where on-device contextual location ads is clearly permitted.

### 19. [MINOR] Dangling and ambiguous plan references

Dangling and ambiguous plan references. Section 1 says design partners are an 'owner decision, see open questions', but the plan has no open-questions section. 'Closed beta' is used in scope and in the checklist, but no milestone says when external partners first receive SDKs. Production KMS and the pen test land at M12, while the text says KMS must be live before the first external partner. Sender/partner counts for the human-moderation workload and reviewer staffing are not given.

**Fix:** Add an open-questions list (design partners, pilot metro, relay vendor, reviewer staffing). Define which milestone opens the closed beta and gate it on the 'before first external partner' checklist.

### 20. [MINOR] Milestone ordering and verification gaps in the middle of the plan

Milestone ordering and verification gaps in the middle of the plan. Events, outbox and attestation are used by the Web and Android SDKs (M6, M8) but the ingest path and tokens are built in M9. The M13 gate 'partner vs Proxim viewable counts within 1%' compares two numbers that both come from the same SDK client report, so it is not an independent check. Native accessibility (VoiceOver, TalkBack, Dynamic Type) is verified only by axe on web.

**Fix:** Move the ingest endpoint, attestation and token issuance to M5, and cover event emission in the M6 and M8 gates. Define an independent reconciliation (partner server-side or ad-server logs versus SDK reports). Add native accessibility checks to M8 and M10.

## Assumptions the plan makes

- US-only launch; EU/UK deferred pending DPIA and counsel (GDPR Art. 9, ePrivacy 5(3), DSA 26(3)).
- Very small team (possibly one person with an AI coding agent); v1 scope is cut accordingly (Web + Android first, then iOS; React Native, live Connect payouts, OHTTP/blind tokens in production, learned routes, auctions and the 2-of-3 root ceremony are deferred).
- Verified by me this session: uniffi 0.32.2, h3o 0.11.0 (crates.io), next 16.4.0 (npm), and that Washington RCW 19.373.080 and the 2,000 ft geofence definition exist with commentary reporting no consent exception (secondary sources; no court or AG guidance found). All other versions, OS limits (iOS 20 conditions, Android 100 geofences and 2 to 6 minute latency, iOS 26 relaunch reports), store-policy text and other state laws are carried from upstream agents and are unverified; they are M0 burn-down items stored as data or simulator parameters, never hard-coded.
- Standoff default is 1,000 m (above WA 610 m and CT/NV 533 m plus GPS and OS cushion margin) and is configurable per jurisdiction via data; nothing here is a legal conclusion and counsel must confirm.
- The unreadable Lovable prototype is not assumed to constrain UI, copy or branding.
- Partner apps own location permission prompts, store declarations and CMP consent; the SDK never requests Always/background permission itself; the SDK provides its own consent sheet and signed receipt only for the health-events purpose.
- Pack fetches reveal only an IP and an H3 res-5 cell with no app key until OHTTP ships; Phase 1 reports are pseudonymous (not anonymous) and limited to non-sensitive classes.
- Fixed rate-card CPM pricing (no auctions) and 100 percent human approval of signposts in v1; on-device caps reset on reinstall and are client-trusted, with billing capped at budget and overshoot absorbed by Proxim.
- A dedicated new Supabase project is used and the existing 'ilios Healthcare' project (wjsjkemamttsuilhihip) is never reused; the Postgres major version is whatever the new project provides.
- AWS KMS (ECC_NIST_P256) reached via OIDC from Vercel is acceptable as the signing-key custodian; an OHTTP relay from a different organization is commercially obtainable before health traffic, otherwise the health pilot stays sandbox-only.

---

# Review of PLAN-ADDENDUM-01

Adversarial critic review of [`PLAN-ADDENDUM-01.md`](PLAN-ADDENDUM-01.md), 2026-10-09. **2 blockers, 12 important, 5 minor.** The addendum has **not yet been revised** to address these.

## Verdict

The addendum is a serious piece of work. Its main decisions hold up: no hosted ad notifications, local-only delivery, a relationship test for owner mode, an identity firewall, and honest background physics. I found no blocker in the direction. I did find three internal contradictions that break the flagship resort scenario as written. It also silently breaks invariants in four places, and v1 is much heavier than it claims.

The three contradictions:
- The owner-only app makes zero tile fetches, so it cannot hold the sensitive-zone data that I13 and I16 require at post time.
- The 24 h fail-closed revocation rule means the typical first-time guest, whose app was last opened at booking, never gets the welcome.
- The 12 h zone_state TTL kills departure for any multi-day stay.

Owner binding can also be used to run an unlabelled ad marketplace.

Store-policy claims are plausible. Compare fetched text with the addendum:
- Apple 2.5.18, 2.5.16, 4.5.3, 4.5.4, 4.10 and 5.1.5 read as the addendum says. I fetched the guidelines page, but it showed no "updated 2026-06-08" date, so that date is unconfirmed.
- Android geofencing limits (100 per app per user, 100-150 m radius, latency text, FLAG_MUTABLE, re-register rules) match the Android docs.
- Google's ads and system-notification rule is confirmed only through secondary summaries: support.google.com was unreachable from this sandbox, the same limit the addendum reports.
- Apple's text says "Push Notifications", so whether 4.5.4 binds SDK-posted local notifications is genuinely open, as the addendum says.
- Apple DTS (forum snippet) supports UNLocationNotificationTrigger needing only When-In-Use. A tutorial and a forum post report suspended-app and missed-trigger problems. Path S stays correctly flagged.
- Android's docs say background-location approval is not guaranteed and is limited to core functionality. The dry-run treatment (gap 6) understates that.

Recommended fixes before merge:
1. Resolve gaps 1-5 and 8.
2. Re-tier the milestones as in the scope assessment.
3. Add the Play-approval gate before M16.
4. Remove the promised 2-3 minute delivery figure until the p50/p95 and iOS inconsistencies are fixed.

## Scope assessment

v1 is not yet small enough for a very small team, and it only partly respects 'advertising first'. The addendum correctly keeps owner mode behind M11 and a signed design partner. But it moves a lot of owner-driven machinery into the hosted v1 path:
- the full event set in proxim-detect (cross, pass, dwell, sequence, exit);
- the eight new invariants I13-I20 with named CI tests;
- notification and region OS models at M4;
- the claim ladder with postcard proof, at M5;
- G1/G2 reporting rules, at M9;
- a notification benefit-rule engine, at M3.
It also adds a 25+ class picker backed by a new Overture/PostGIS/PMTiles/search pipeline, a noise-profile Drive-it preview, a WASM geometry port and ODbL lineage tooling. 'Days of work' for Stage 0 is not credible next to that list.

Cut or defer:
- Keep the event enum, field reservations and signed envelope v2.
- Implement ENTER, APPROACH and internal hysteresis only; the other events arrive with Stage 1.1 or 2.
- v1 picks: POI, area/neighbourhood, road corridor and custom shapes. Building, junction, car park, transit, brand and import come once they compile to those shapes.
- Move the claim ladder, G1/G2 reporting, notification models and the benefit engine to Stage 1.1/2.
- Keep I13-I20 as design constraints and write each test with its feature. Only I17, I18, the I2 lint scope and I8' need v1 tests.
- Run the ODbL open-extract process only after counsel has advised.
- Make the design-partner letter and a Play/Apple review outcome an explicit entry gate to M15-M18.

Keep as is: the hosted-notification deny, local-only delivery, mode as policy data, and the not-building list.

## Silent invariant changes (9)

- I8 to I8' weakens a PLAN invariant: tile fetches carrying no app key becomes 'never both key and location in one request'. It is marked as a change, but it leaves owner-only apps with no tile packs, which silently breaks I13/I16/R6/R7 (no device-side sensitive-zone data). Not acknowledged anywhere.
- I14 (identity firewall) and the 'arm list never leaves the device' claim are not guaranteed unless the owner bundle fetch is independent of arming. Per-venue or per-signpost fetching leaks the arm list. The addendum does not specify which.
- I14 and the hostVeto design: the veto is called on every candidate decision with intent and distance bucket, so identity-bearing host code learns per-user proximity even when nothing is posted. The addendum says this is out of scope for Proxim, but the SDK is the channel.
- I16 (dropped, never deferred, inside sensitive buffers for all classes) and the 'exit/dwell/sequence refused for zones touching a buffer' rule are unenforceable on owner-only devices for the same reason as I8'. They are also unmeasured for dense or campus venues.
- I19 (zone_state TTL 12 h, hard 24 h) is incompatible with the stated departure and dwell behaviour for multi-day stays. Either the invariant is relaxed silently or the feature does not work.
- I10/I20 (revoked content never rendered; revocation purge) with the 'no network call between wake and post' rule and the 24 h fail-closed window: the first-arrival case cannot satisfy both freshness and delivery.
- R2/R6/I6 'nothing logged or queued in a buffer' versus verify-on-wake: the verify step takes a location fix (and on iOS Path S the OS monitors the region) before the buffer test runs. The addendum treats the fix itself as acceptable without saying so. Likely fine (no log), but the order should be stated and tested.
- PLAN Section 7's mandatory 'Sponsored' label is changed to 'Public information' for hosted verified civic content in Section 2.3 and 8.1, but the Section 10 conflicts table does not list it.
- PLAN Section 5.3 / R-tier wording: hosted background (M3/M11) with partners that already hold Always is left in place, although the addendum's own store-policy logic says background location may not serve advertising. The conflict with the addendum's M3 rewording is not called out.

## Findings (19)

### A1. [BLOCKER] Owner-only apps cannot enforce sensitive-zone rules

Owner-only apps cannot enforce sensitive-zone rules. I8' says an owner-only app makes zero tile fetches, but I13, I16, R6 and R7 require the device to know it is outside every sensitive buffer at post time. That data ships only in public tile packs, and the confidential kinds are deliberately excluded from them. An owner-only device has no registry, so either the gate silently passes (a health bypass) or it fails closed and nothing ever posts.

**Fix:** Compile sensitive-zone data per venue into the owner bundle: all non-confidential buffers within ring + buffer + margin of each venue ring and approach corridor. Reject at authoring any venue whose ring overlaps a confidential coarse-barred cell. Add a core test that an owner bundle with no sensitive data refuses to post. Add the venue-neighbourhood case to the M15 verification.

### A2. [BLOCKER] The 24 h fail-closed revocation rule contradicts the flagship scenario

The 24 h fail-closed revocation rule contradicts the flagship scenario. Service notifications fail closed when the revocation list is older than 24 h, and wake-time network calls are banned (the M8 test asserts none). The list refreshes only 'whenever the app runs'. The typical guest opened the app at booking, days or weeks before arrival. The ring fires with a stale list and nothing posts. Bundle hard TTL is 7 d, so bookings made further ahead also expire. Worked-example step 11 assumes a list under 24 h old without saying how.

**Fix:** State the real freshness mechanism: periodic background refresh (WorkManager, BGAppRefreshTask) with measured hit rates. Alternatively, use a longer fail-closed window for service class with an explicit takedown-latency tradeoff. Alternatively, allow a bounded non-location revocation fetch at wake, but only after quantifying that its timing reveals arrival to Proxim, which would break I14's spirit. Add 'app unopened for 3, 8 and 21 days' scenarios to M16 and M18. Do not publish the 'within 2-3 minutes' promise before this is resolved.

### A3. [IMPORTANT] Departure and dwell break on any stay longer than 12-24 h

Departure and dwell break on any stay longer than 12-24 h. zone_state TTL is 12 h (24 h hard). EXIT is 'valid only after a valid ENTER or reconciled IN', and OS exit callbacks 'never override a fresh confident inside fix'. After the TTL the state is gone, so a guest leaving after a 3-night stay has no IN state and the exit is suppressed. Departure is one of the owner's two headline examples. Extending the TTL conflicts with I3/I19 retention.

**Fix:** Specify the exit rule for armed signposts: OS EXIT callback plus one confident outside fix is sufficient when the signpost is armed and the arm window is current. No stored visit state is then needed. Or set the I19 TTL from the arm window and say so. Add a 'multi-day stay then exit' golden vector.

### A4. [IMPORTANT] owner_binding is a loophole

owner_binding is a loophole. 'OWNER iff sender org == partner org, or an active binding both orgs signed' lets any partner app bind unrelated third parties. A mall app binding 200 tenants, or a partner selling bindings, yields third-party promotion with a 'From <tenant>' label instead of 'Sponsored', no rate card, no hosted deny, and only 5-10% post-hoc review after the first three signposts. The only barrier is the intent taxonomy enforced by review. Section 8 already lists 'Owner (+ hosted sponsor slots)' and malls with tenants, which contradicts 'third-party sponsored content inside owner mode is forbidden'.

**Fix:** Restrict bindings to same legal entity, parent/affiliate or managing agent with a documented contract and human verification. Tenants, sponsors and events with sponsors are hosted mode, which means no notifications. Strike the hybrid rows from the v2 table or mark them 'hosted, in-app only'. Require human review for every owner signpost whose sender is not the partner org itself. Add a mall-tenant promotion case to the advertiser-posing-as-owner corpus.

### A5. [IMPORTANT] Possible leak of the arm list (I14)

Possible leak of the arm list (I14). The arm list is meant never to leave the device. The addendum does not say whether the owner manifest/bundle fetch is all-venues-for-this-app or per-venue/per-signpost. A chain app that fetches only the bundle for the booked venue tells Proxim which venue (and roughly when) each IP/app install is going to. That is identity-adjacent and breaks the stated 'Proxim never sees which signposts are armed'.

**Fix:** Mandate fetch-all-for-app (or coarse fixed buckets) independent of arming. Add an egress-audit test that bundle requests are identical for different arm lists. Put a size cap on bundle count per app.

### A6. [IMPORTANT] Store-approval dependency is ordered too late and 'dry-run' is not a clearance

Store-approval dependency is ordered too late and 'dry-run' is not a clearance. The Android owner product depends on Google approving ACCESS_BACKGROUND_LOCATION for an arrival welcome as core functionality, which Android's docs say is not guaranteed and which the addendum marks [U]. A dry run cannot predict review. M12 'dry-runs' need a sample resort app with working notification delivery, which is not built until M16-M17. 'Store approvals in hand' appears only at M18, after M15-M17 engineering. There is no stated plan B if Play rejects (the OS-callback-only fallback addresses FGS, not the permission).

**Fix:** Make an actual Play declaration and App Store review outcome for the design partner's real app a gate before M16 build-out. State explicitly: if background location is denied, owner mode degrades to foreground/partner-fed in-app only and the notification product is dropped. Remove the dry-run claim from M12 or move it after M17.

### A7. [IMPORTANT] Hosted background mode (PLAN M3/M11) is not re-examined by the addendum's own logic

Hosted background mode (PLAN M3/M11) is not re-examined by the addendum's own logic. It says Google denies ads-only background location and limits M3 to 'never to obtain background location for advertising'. But navigation-type hosted partners that already hold Always would still feed hosted approach rings in background, using location granted for another declared purpose to serve ads. That raises the same Play/Apple 5.1.5 and data-use-disclosure question and is not flagged.

**Fix:** Add an explicit open item and counsel/store question. Default hosted background to off until a partner's declared purposes include it. Reflect this in the Section 10 row and M11 verification.

### A8. [IMPORTANT] The latency story is inconsistent

The latency story is inconsistent. '2-3 minutes after boundary' at 15 m/s is 1.8-2.7 km, but the example and table say about 1.7 km (about 1.9 min). The pilot gate allows p90 within 5 minutes, the iOS figure is 3-5 minutes, Android is up to 6 minutes, and the p95 is about 5 km, which exceeds the 3 km access road in the example (the guest reaches reception first). The 'honest promise copy' quotes only the optimistic Android figure and omits iOS. All of these numbers are [MODEL].

**Fix:** Reconcile p50/p95 in minutes and km. State separate Android and iOS promises. Replace contract copy with 'typically within a few minutes, sometimes later, not guaranteed'. Do not put model numbers in sender-facing UI as probabilities until M4/M11 field data exists.

### A9. [IMPORTANT] Sensitive-zone coverage is unmeasured for owner mode

Sensitive-zone coverage is unmeasured for owner mode. I16 drops all notifications for all classes inside any sensitive buffer (default 1,000 m, and the registry includes pharmacies and clinics). PLAN-REVIEW #5 already showed this blankets urban cores. The use-case map (campus, parking, events, malls, city hotels, estates) mostly sits in dense areas, and university campuses often contain a medical centre. The addendum lists these as v2 without a coverage analysis.

**Fix:** Run the M5 coverage map for owner mode before promising urban use cases. Mark urban and campus rows 'only where buffers allow'. Pick a rural or resort-type design partner.

### A10. [IMPORTANT] The generalization is real at the detector, envelope and schema level but is two products at the gate/selection/billing level

The generalization is real at the detector, envelope and schema level but is two products at the gate/selection/billing level. The Section 2.3 table differs on about 12 dimensions: verification, moderation, label, distribution, selection (V x Q x R x P versus priority/recency), billing, reporting, TTL, arm state, zone_state and veto. The claim that mode-as-data leaves 'the kernel unforked' is overstated: there are two gate pipelines. The second use case tested (events with sponsors, mall tenants) fails the model, as noted above.

**Fix:** Say plainly: shared detector, envelope, packs-format and renderer; two gate/selection pipelines. Test the abstraction against a third non-resort case (parking operator or museum) in the Stage 0 design review. Do not claim kernel unity.

### A11. [IMPORTANT] v1 scope is not small and breaches 'advertising first'

v1 scope is not small and breaches 'advertising first'. 'Days of schema work' for Stage 0 contradicts the M0-M7 changes. Build-now includes:
- proxim-detect with CROSS, PASS, DWELL, EXIT and SEQUENCE logic, none of which hosted v1 uses (enter and approach only);
- owner-mode policy rows and I13-I20 tests;
- a notification benefit-rule engine at M3;
- notification and region OS models plus a Path S device test at M4;
- the claim ladder with postcard proof, needed only for Stage 1.1 and owner mode;
- G1/G2 report policy at M9;
- a 25+ class pick catalogue with a PostGIS ref schema, pick-layer PMTiles, intersection index, search corpus and Photon fallback;
- a Drive-it simulator with noise profiles and 200 seeded runs per preview;
- an ODbL lineage and open-extract publishing process;
- a keyboard/coordinates-table editor;
- GeoJSON/GPX import fuzzing.
Individually reasonable; together they are a multi-month load before the first advertiser sees a console.

**Fix:** Cut v1 to the following:
- Detector: ENTER, EXIT hysteresis (internal only) and APPROACH, with the event enum and fields reserved.
- Pickers: POI, area/neighbourhood, road corridor, custom shape. Building, junction, car park, transit and import follow once they compile to the same shapes.
- Schema: reserve fields only. I13-I20 stay design rules, with tests landing alongside their features (M15+). Only I17, I18, the I2 lint scope and I8' need v1 tests.
- Defer the claim ladder, G1/G2 reporting and notification OS models to Stage 1.1 or 2.
Keep the per-venue ODbL extract until counsel has spoken.

### A12. [IMPORTANT] Private-content protection is thinner than its framing

Private-content protection is thinner than its framing. The owner bundle is fetched with a publishable key, so anyone with the APK can pull every guide and geometry (gate locations, 'emergency contacts', reception instructions). The semantic leak the linter cannot catch is, for example, a gate-procedure sentence. The planned `visibility` field and the word 'private' will mislead owners. Estate/HOA bundles expose perimeter and visitor-gate details, the same confidentiality issue as PLAN-REVIEW #4.

**Fix:** Console must say 'anyone with your app can read this'. Rename the field (`audience_scope`) and remove the word 'private'. Add a human-review checklist item for operational details (procedures, staff names, gate instructions). Estate bundles carry no perimeter detail beyond the ring.

### A13. [IMPORTANT] The surveillance path through host callbacks is not closed

The surveillance path through host callbacks is not closed. hostVeto receives a distance bucket and intent on every candidate decision, including ones it denies. The owner's app, which holds identity, therefore learns per-user presence near the venue even when nothing posts. That is an arrival oracle against I14's spirit. The addendum notes 'owner callbacks correlate' (risk 13) and offers a disable option, but veto is not covered by it. Workforce check-in is in the use-case map.

**Fix:** Make veto opt-in per signpost, declare it in the consent sheet and DPA, and drop it from v2.0. Put field-service/workforce rows in 'not building' until counsel and employee-monitoring rules are reviewed. Limit veto input to intent only, with no distance bucket.

### A14. [IMPORTANT] Moderation weakness in owner mode for lock-screen content: after the first three signposts, text is approved by rules plus an advisory LLM with 5-10% sampling, and the experience can be a `web` page on the owner's domain that Proxim does not control

Moderation weakness in owner mode for lock-screen content: after the first three signposts, text is approved by rules plus an advisory LLM with 5-10% sampling, and the experience can be a `web` page on the owner's domain that Proxim does not control. Re-crawl catches changes only on a schedule. A compromised or malicious owner account (credential theft) can push a phishing notification to every armed device that is still within its TTL.

**Fix:** Keep human review for every revision that changes notification title/body, geometry or link domain (not just first three). Allow auto-approval only for guide-content-only edits on already approved signposts. Add account-takeover/MFA requirements for owner-lane orgs and a signed-revision cool-off delay.

### A15. [MINOR] Silent invariant weakening

Silent invariant weakening. I8' ('never both app key and location-derived value in one request') is weaker than I8 and PLAN-REVIEW #11/#12 (Web Origin header, attestation key ids) already show the key-or-location separation is leaky. The new owner fetch cadence, IP and time make it more so. I2 is re-scoped to device-facing routes, but /v1/ref/at and the Drive-it preview are coordinate-bearing endpoints on the same API and need rate-limit and no-log guarantees. The addendum also silently changes PLAN Section 7's mandatory 'Sponsored' label to 'Public information' for hosted verified civic content, and the Section 10 conflict table omits it.

**Fix:** List the 'Public information' label as an explicit Section 10 conflict with a rule for who qualifies and a counsel flag. Add a no-log/no-retention requirement for authoring coordinate endpoints, and an egress-audit test for the owner fetch cadence.

### A16. [MINOR] Pack-budget accounting omits per-signpost signatures

Pack-budget accounting omits per-signpost signatures. M2 requires a signature on every signpost (no splicing). 500 signposts times an ES256 signature (64 bytes, about 86 base64url characters) is about 32-43 KB of incompressible data, roughly 10-12% of the 350 KB tier-0 budget before the reserved fields and local road context.

**Fix:** Add the signature overhead to the M2 size experiment. Consider one signed Merkle root per pack with per-entry inclusion proofs.

### A17. [MINOR] Driving behaviour rules conflict

Driving behaviour rules conflict. Section 5.3 and 5.4 say 'post silently while moving'. L3 says 'not at speed unless passive and title-only, or held until stationary with a 10 min expiry'. Arrival expiry is 30 min. Speed from a single wake fix is often absent, so 'unknown means quiet' is the common case. iOS has no 'silent while moving' that stops lock-screen exposure to a driver.

**Fix:** Pick one rule (post passive, title only) and make L3 and 5.3 agree. Remove the claim that this addresses distracted-driving exposure; keep it as a COUNSEL item.

### A18. [MINOR] The reasoning for denying hostVeto to hosted partners is inconsistent with PLAN Section 7

The reasoning for denying hostVeto to hosted partners is inconsistent with PLAN Section 7. The addendum says a hosted callback would reveal place category as a side channel. But PLAN already gives headless partners the `Proxim.signposts` SignpostDecision stream, which tells the partner which signposts matched. 'Declarative suppress flags with no return value' also leak, because the partner sees whether its slot filled.

**Fix:** Either accept that hosted partners can infer place category from decisions (a pre-existing PLAN decision) and say so, or require ProximRequiredChrome-style rendering for all hosted modes. Do not cite the side-channel as the reason.

### A19. [MINOR] Milestone ordering and verifiability issues:
- M3 verification asks counsel to review the policy matrix, but counsel funding is open question 6.
- M9 builds G1/G2 reporting policy before any G1 exists (claims arrive in Stage 1.1).
- The design-partner validation in risk 14 is not a milestone gate before M15.
- Provisional pilot gates sit beside promised copy that they contradict (see the latency gap).
- M12 already carries OHTTP, KMS drills, pen test and DPIA; adding owner dry-runs and a counsel memo there creates a bottleneck

Milestone ordering and verifiability issues:
- M3 verification asks counsel to review the policy matrix, but counsel funding is open question 6.
- M9 builds G1/G2 reporting policy before any G1 exists (claims arrive in Stage 1.1).
- The design-partner validation in risk 14 is not a milestone gate before M15.
- Provisional pilot gates sit beside promised copy that they contradict (see the latency gap).
- M12 already carries OHTTP, KMS drills, pen test and DPIA; adding owner dry-runs and a counsel memo there creates a bottleneck.

**Fix:** Move G1/G2 reporting to Stage 1.1. Add an explicit design-partner letter of intent and Play-declaration outcome as the M15 entry gate. Split the counsel memo into hosted (M12) and owner (before M15). Defer counsel review of the matrix to M12.

