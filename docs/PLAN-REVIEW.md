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


---

# Verification of PLAN-ADDENDUM-01 revision 2

Three independent verifiers reviewed the **integrated** revision (before repair). The repair step then applied **83 targeted edits (0 failed to apply)** and deliberately left 7 issues unfixed. **The repaired text has not been re-verified.**

## Verifier verdicts

**closure**: Both blockers (A1, A2) and 14 of the other 17 findings are closed. A5, A8 and A10 are partially closed because the revision contradicts itself in one place each. All 9 silent-invariant items have a row in 10.1 or a fix. Item 2 (I14, arm list) has a residual timing leak, and item 7 (R2/R6/I6 order) is contradicted by the inbox fallback. The one new [V] I could check, the Play FGS-removal date of 2027-01-27, is confirmed. I fetched developer.android.com/distribute/play-policies and its timeline says "January 27, 2027 ... removing geofencing as an approved foreground services use case ... use the Geofence API". The ten internal contradictions below should be fixed before merge.

**consistency**: Not yet internally consistent. I found no blocker, but there are about 12 important contradictions and about 14 minor ones. Several of the important ones sit in the flagship path (the 5.3 wake order against the section 7 inbox fallback, the 4.1 detector against the single-fix wake, the 8.4 gate against M15 scope), and several break the "single source of truth" promise of 2.6 and the "every change is in 10.1" promise of the header. The arithmetic checks out: N2 and N3 are exact products or lognormal consequences of N1, and the tier-0 budget figure is right (500 x 65 B = 32.5 KB = 9.3%, leaving about 317 KB and about 635 B per signpost). All section cross-references I traced resolve, including 2.3a, 5.2a, 9.3a, 9.5, 9.6, 11.8, 15.x, "decision N" and "risk 14". No stale "visibility" term survives. The Stage 2 effort sum (32-58 pw) is correct. The main problems are semantic contradictions, ordering dependencies and a few undefined items (R7-R9, the wake cap, S01-S11).

**scope-feasibility**: PROCEED WITH FIXES. I found no blocker. The revision closes the earlier holes: arm-list leak, the 'veto' oracle, the stolen-credential path, the I8 loosening and the FGS fallback. The fetched Apple and Android primary texts match the addendum's quotes and its handling of the open questions (4.5.4 says 'Push Notifications', FGS removal on 2027-01-27, geofence latency figures).

1. Scope. The advertising-first v1 is credible as a delta of 18-32 pw. It is not yet honestly 'small' as a total. State the total on top of the 85-150 pw base, and strip the owner-only work that still sits in M4, M5 and M7. Reorder the ODbL pickers behind the M12b memo.

2. Gates. G-O0 and G-O1 need named owners, independent approvers, evidence paths and a code-level binding, and G-O0 must fix its 10-versus-20 sample contradiction. G-O1 must also cover the Play notification-policy position and an SDK-integrated build pass before M18. Apple 4.10 and the Play integral-feature rule belong in the owner counsel scope (COUNSEL, no conclusion drawn). Add an Android wake-fix availability measurement before M16.

3. Privacy. Fix three things before Stage 2 design freezes: the I14(b) refresh() timing correlation, the host-observability overstatement in I14(c) and the consent sheet line, and the shelter-location oracle in the venue site test and registry re-test. None blocks the hosted v1.

Everything about owner mode stays correctly gated behind the hosted pilot, G-O0 and G-O1. Legal questions remain with counsel, with no conclusions drawn here.

## Closure of the critic's findings (A1-A19)

| Finding | Verdict | Evidence |
|---|---|---|
| A1 | closed | 5.7 gives the signed per-venue sidecar, with NO_DATA failing closed, confidential kinds never shipped, and a compile-time rejection of any venue whose C(V) meets a barred H3 res-7 cell. I13 requires 'positively CLEAR per the signed venue sidecar'. T-SZ-01..12 and the M15 verify ('bundle without sensitive data refuses to post') cover it. Side issue: 5.7's 'too dense for arrival notices' message conflicts with the byte-identical refusal in 9.6 (see issues). |
| A2 | closed | N4 sets W_service at 72 h, with a 6 h young-revision rule. 5.2a states the mechanism, lists the rejected alternatives and bans any wake-time fetch. 7 step 11 spells out the stale-guest case. M16 tests unopened 1/3/8/21 d with 71h59/72h01 boundary vectors, and G-O2 plans field scenarios. The '2-3 minute' promise is withdrawn in 11.1(1). Decision 4 correctly stays open for the owner. |
| A3 | closed | 4.1 defines armed OS exit with no stored visit state, using the arm record (from/until, 'fired' mark) plus the OS region state. 5.6 gives arm(from:until:). Dwell moves to Stage 3. I19 keeps TTL 12 h / 24 h hard max. GV-EXIT-01 is the multi-day-stay vector. Minor: the arm record table is not in the I3 snapshot allow-list (see issues). |
| A4 | closed | 2.3 ships O1 only, with a DB constraint, signed `relation` in I17, and O2-O4 reserved and off. Tenants and sponsors are hosted in 2.3, 8.1 and 9.3. 8.3 P3 and RS-OWN-01/02 cover the mall-tenant and agency cases. Every new signpost goes to the human lane. |
| A5 | partially_closed | 2.3a mandates fetch-all, lints the /o routes, caps bundles at N10, and EG-01..09 check arm-list invariance. But partner-called refresh() (5.2a, 7 step 8b, a contract term) lets the partner time requests from booking state. I14(b) says the schedule is a function of time and cache state 'and nothing else', and 2.3a says Proxim 'cannot tell an armed install from an unarmed one'. Neither is true once refresh() is partner-timed. 2.3a also says fetch 'at app start' while 5.2a says a wake launch never refreshes; that is not reconciled in 2.3a. |
| A6 | closed | 8.4 and G-O1 make a real Play declaration outcome and App Review outcome for the partner's production build the gate for M16. It states plan B (foreground / partner-fed in-app only, no retry by re-framing) and deletes the FGS option. 10.1 relabels the 'dry run' as a 'label-accuracy rehearsal (not a clearance)'. |
| A7 | closed | 5.1 sets hosted_background = off by default, with five enablement conditions and an open store/COUNSEL question. The 10.1 row and the M3, M11 and M13 edits are in place, and HOSTED_BACKGROUND_OFF is a reason code. Wording residue: 4.3 and 8.8 still say hosted approach rings need the OS model (see issues). |
| A8 | partially_closed | 2.6 N1-N3 is one reconciled parameter set. I re-derived N2, the lognormal fits and N3 (63% / 32% / 9% / 33%) and they check out. Per-OS copy is in 7. But 3.4 and 7 step 6 show the sender a 'modelled report distance as a range (N2)' from a [MODEL] figure. That contradicts 2.6 ('a [MODEL] figure renders as not promised') and 11.1(1) (no sender-, partner- or owner-facing distance figure outside promises.yaml). 7's 'no ... distance ... number in ... sender UI' says the same. The partner dev-docs copy (numbers plus '0.9 km') relies on a vendor-quote exception that exists only in 8.7. |
| A9 | closed | 9.6 specifies the coverage tool with A/B/C variants, the per-venue enter-footprint test, exit_dispersion / p_drop_exit, declared-facility intake, the rural design-partner profile and fixtures. The metro-level run is scheduled at M5 and G-O0 item 6. 8.1 marks urban and campus rows 'only where buffers allow', I16 is not narrowed, and the lack of registry data is stated honestly. |
| A10 | partially_closed | 2.5 deletes 'unforked' and lists what is shared versus the two pipelines. 8.3 tests the parking operator. But the 8.3 'Stage 0 gate' (P1-P3 run through both gate pipelines in the simulator) cannot be run at Stage 0. The owner gate pipeline is built at M15 (11.3 M3 moves owner tests to M15/M16; 11.5 M15 lists 'parking scenarios in both pipelines'; 8.2 Stage 0 only writes YAML). |
| A11 | closed | The detector is cut to enter, approach and internal exit (4.1). There are four pickers (3.1, 3.4). The claim ladder, G1/G2 reporting, cross/pass, import and Drive-it v2 move to 1.1 (11.4). Notification models and benefit rules move to M16. I13-I16, I19 and I20 become design-rule with check-invariants (8.7). Effort figures are given (8.6) and 'days of work' is deleted. I checked the arithmetic (53-94 vs 18-32 pw; 60% / one fifth). |
| A12 | closed | `audience_scope` replaces `visibility`. The 9.4 console warning and checkbox are given verbatim. There is a Class A/B split, and Class B is one ring only (N10, I15, byte-grep). The nine-item checklist, linters, leak-purge drill and word-ban lint are all present. |
| A13 | closed | hostVeto is removed with no Stage 3 variant (6, 8.2, 13.5). I14(c) bans host output on enter/exit/cross/dwell/approach and on Gate decisions. 5.6 has no callback. Workforce check-in moves to not-building (8.5). RS-VETO-01 covers it. |
| A14 | closed | 9.3a sets human-lane and auto-lane rules by structured diff, a 24 h hold, signed `live_at`, ATO controls (aal2, passkey, recovery freeze) and the young-revision rule (N4). Notification taps open only a guide or an allowlisted deeplink; `web` is reachable only from the inbox or an in-app card. Revoke scopes and the RS-ATO-01 bound on a stolen credential are stated. |
| A15 | closed | I8 is restated in three parts, with (a) covering /t/* with no key, cookie or ID, and I8' withdrawn. I2 is scoped and I2a (9.5) adds no-log, no-store, POST-only and rate limits for authoring routes. 'Public information' is a 10.1 row, OFF by default, with eligibility rules and a COUNSEL flag. EG tests cover owner fetch cadence. |
| A16 | closed | N10 includes signatures (500 x 65 B = 32.5 KB = 9.3%, leaving about 317 KB). Merkle proofs are rejected in 10.1 and 15.1. M2 verify covers splice, drop, swapped signatures, rollback and size. The S1-S5 'no splicing' labels appear only in the 15.1 log and are not defined in the body (minor). |
| A17 | closed | One rule applies in 5.4, L3 and N6: every location-triggered OS notification is passive and title-only, with no speed logic. The distracted-driving benefit claim is removed and left to COUNSEL, with a doc-lint ban. PLAN 5.5 is relabelled in 10.1. |
| A18 | closed | 6 accepts that hosted partners can infer place category from the pre-existing SignpostDecision stream. The stated reason for denying a hosted callback is now the billboard principle (identity-bearing partner code in the decision path). A DPA clause on SignpostDecision use is added (M9, COUNSEL). |
| A19 | closed | Counsel review moves from M3 to M12b, with a signed self-review at M3. G1/G2 reporting moves to M13.1. G-O0 requires an LOI, and the hosted and owner counsel memos are split (11.6). M12 splits into a/b/c, and pilot thresholds are pre-registered in promises.yaml (N12). Minor: the level-2 claim is built in both M13.1 and M15. |

## Important issues raised (28; 26 minor not listed)

1. **[closure] 2.2, 5.3, 7 steps 3 and 11, I16, 10.1 (I6 row)**: Inbox fallback contradicts the 'write-iff-post' wake order. 2.2 (inbox as a delivery channel), 7 step 3 ('local notification with inbox fallback') and 7 step 11 ('if notifications are off, an inbox banner shows on next open') need a persistent inbox write at wake. 5.3 Phase A(3) says an OS-notification-status failure means 'return; nothing written, logged, queued', Phase D writes only the ledger row, and 5.7 and I16 treat an inbox write as a forbidden notification surface. This is silent-invariant item 7 (R2/R6/I6 order), claimed fixed in 10.1 but contradicted in the worked example.  
   *Suggested fix:* Remove the inbox fallback from owner-mode background wakes. Make the inbox an in-app, foreground-only channel for owner mode, or state the exception explicitly (write only after CLEAR and Gate pass, never in a buffer) and amend 'persistent writes iff a notification is posted' to cover it. Then add a WO test for 'notifications off'.
2. **[closure] 2.3a, 2.4 I14(b), 5.2a, 7 step 8b and 'What Proxim sees', 11.8 EG**: Arm-independence is overclaimed. Partner-called refresh() (a contract term, 7 step 8b; 5.2a) lets the partner time /o and /v1/config requests from booking state, so Proxim sees IP and time that correlate with bookings. I14(b) says request schedule is a function of time and cache state 'and nothing else', 2.3a says Proxim 'cannot tell an armed install from an unarmed one', and the inventory line says 'identical for every install'. 2.3a also lists a fetch 'at app start' while 5.2a says a wake launch never starts a refresh. EG-xx tests only arm()/disarmAll().  
   *Suggested fix:* Either declare partner-timed refresh() as a residual timing channel in I14(b) and the data-flow inventory, or constrain refresh() (coalesce to the install-seeded schedule or rate-limit it). Reword 2.3a's 'app start' to 'foreground start, not a location relaunch'. Add an EG test that a wake launch with a stale cache sends nothing.
3. **[closure] 2.6, 3.4, 7 (step 6 and promise copy), 11.1(1), 8.7**: Sender-facing distance figure contradicts the numbers rule. 3.4 (Drive-it) and 7 step 6 show 'the modelled report distance as a range (N2)' to senders. 2.6 says a [MODEL] figure renders as 'not promised', and 11.1(1) bars any sender-, partner- or owner-facing distance figure outside promises.yaml. The partner dev-docs copy in 7 carries Google quotes and '0.9 km per minute', and the vendor-quote exception exists only in 8.7.  
   *Suggested fix:* Show a qualitative warning only ('kilometres at driving speed') until field data exist. Alternatively, state one explicit exception: vendor quotes and the labelled N2 range are allowed in docs and Drive-it, with check-promises allowlisting them. Make 2.6, 11.1(1) and 7 agree.
4. **[closure] 8.3, 8.2, 11.3 M3, 11.5 M15**: The 8.3 'Stage 0 gate' (P1-P3 run through both gate pipelines in the simulator, P2 and P3 yield zero OS notifications) cannot be run at Stage 0. The owner gate pipeline and its tests are built at M15 (11.3 M3, 11.5 M15), and 8.2 Stage 0 only authors YAML.  
   *Suggested fix:* Re-word the Stage 0 gate as a design review of P1-P3 YAML against the policy matrix (hosted x notification = deny). Move the simulator run through both pipelines to M15 verification.
5. **[closure] 5.7 (Limits), 9.1 authoring oracle, 9.6 fixtures, 7 step 1**: Refusal-message inconsistency creates an oracle for confidential places. 5.7 gives a distinct message for a dense venue ('too dense for arrival notices; use in-app') and a generic one for a confidential-cell hit ('arrival notices can't be used at this location'). 7 uses 'not available at this location', and 9.6 requires a byte-identical refusal across causes. A generic refusal in a low-density area then reveals a nearby barred cell. The residual of probing the wizard's 'Available / Not available here' answer across points is not declared.  
   *Suggested fix:* Use one message and one timing for all causes (density included). Declare the probing residual in 9.1/9.6 with the N11 rate limits as mitigation. Add a fixture asserting the density and confidential refusals are byte-identical.
6. **[consistency] 5.3, 2.2(3), 7 steps 3 and 11, 2.3 table**: The inbox fallback contradicts the write-iff-post rule and the wake order. 5.3 says: "persistent writes happen if and only if a notification is posted", and in Phase A "Any failure: return; nothing written, logged, queued, scheduled or fetched" (that covers OS notification status). Section 7 step 3 says "local notification with inbox fallback", and step 11 says "If notifications are off, an inbox banner shows on next open." 2.2(3) says channels "always fall back to a quieter one". Step 11 also says a stale list gives "no notification, no inbox row". An inbox write for "notifications off" is a persistent write without a post, and the inbox is itself an I16 "notification surface".  
   *Suggested fix:* Pick one. Either owner location wakes never write to the inbox (delete the fallback from step 3 and step 11, and from 2.2(3) for the wake path), or add an explicit exception to the write-iff-post property and to I16 and say which conditions may write an inbox row. State that the in-app arrival guide is the only fallback.
7. **[consistency] 4.1 ENTER vs 5.3 Phase B/C and N7 vs 4.2 approach**: The detector the wake runs is undefined, and the two specifications are incompatible. 4.1: "ENTER: signed depth d >= k_in x acc_eff ... for N_in = 3 consecutive fixes spanning >= 3 s", "never on a fix whose accuracy circle straddles the boundary". APPROACH needs "decreasing over >= 3 fixes, speed/bearing gates". 5.3 says Phase B takes "One fix", Phase C runs the "Detector against the real geometry (enter ...)", and N7 says "distance to shape <= radius + 100 m". 5.4 says "Speed, heading and activity are not inputs". A single fix cannot satisfy N_in = 3 or the 3-fix approach rule. 7 step 3 still offers an owner "Approaching ring". Both PLAN 5.7 step 3 (burst up to 60-90 s) and the addendum's FGS-burst removal leave only one fix.  
   *Suggested fix:* Define a separate OS_ENTER_CONFIRM check for wakes (single fix, accuracy <= 100 m, inside or within the N7 tolerance) and say it is not the 4.1 ENTER rule. Declare that owner approach notifications are not possible with one fix (remove the optional Approaching ring from step 3), or allow a bounded multi-fix wake and reconcile it with 'one fix' and the 8 s iOS budget.
8. **[consistency] N7 and 5.7 P(V) vs N2/N3 and 3.1 estate row**: The arrival distance limit conflicts with the modelled report distance for small shapes. N7: "distance to shape <= radius + 100 m (arrival, approach)"; 5.7 sizes the posting envelope as "shape + the 100 m verify tolerance". N2 and section 1 say that at 15 m/s the OS reports entry "about 2.5 km (Android) to 3.9 km (iOS) past the boundary at the median". For a Class B estate (3.1: "one ring", never below 150 m) or any 150 m place, the device is 2.5-3.9 km past the ring when the wake runs, so arrival is dropped in most drive-in cases. Only a large grounds polygon containing the road passes. N3's "63% / 32% get the welcome" silently assumes the 3 km road is inside the shape. Departure, by contrast, is allowed up to 5 km (N6).  
   *Suggested fix:* State the assumption in N3 and 7 (the access road lies inside the signed shape). Add a console rule: for shapes smaller than the modelled report distance, drive-in arrival is mostly dropped and the sender is told. Or allow an arrival distance bound tied to max_event_dist_m, and widen the 5.7 envelope to match.
9. **[consistency] 8.3 (ParkCo, 30 garages) vs N10, 8.5, 13.2, 11.7**: The third-use-case test violates the addendum's own caps. 8.3 and 4.3 use "ParkCo (30 garages, own app)" and "always-armed venues (30 garages)" as owner mode (P1). N10: "Owner bundle set per app: at most 20 venues". 8.5 Not building: "apps with more than 20 venues". 13.2: "chains over 20 venues are unsupported in v2.0". 11.7: `partner_apps.max_owner_venues (<= 20)`.  
   *Suggested fix:* Either re-size the test case to 20 venues or fewer (it still exceeds the iOS region budget of 20 only with approach rings), or raise the cap with a stated reason. Otherwise say explicitly that ParkCo P1 is an out-of-cap stress case for the planner only.
10. **[consistency] 8.4 Rule vs 11.5 M15 and 15.2(8), M16**: G-O1 scope and milestone scope disagree. 8.4: "No engineering that exists only for OS notifications, background rings, exit or dwell in owner mode (the M16 scope) starts until G-O1 passes ... M15 (owner server, bundles, in-app guides, sensitive-data compile) may proceed." 11.5 M15 includes "armed OS exit (no dwell)", "planner accepts owner candidates", "freshness mechanism and `refresh()` (5.2a)" and the owner gate pipeline. 15.2(8) puts "armed-exit vectors" in M15. These are background-ring, exit and notification-only work, but M15 is gated by G-O0 only.  
   *Suggested fix:* Move armed OS exit, owner planner support and the notification-only parts of the freshness mechanism into M16 (or into a G-O1-gated sub-milestone), or narrow the 8.4 rule to say which of them are exempt and why.
11. **[consistency] 8.3 Stage 0 gate vs 8.2, 11.1(5), 11.5 M15, 2.4 design-rule**: A Stage 0 gate needs something that is built later. 8.3: "Stage 0 gate: P1-P3 run through both gate pipelines in the simulator; P2 and P3 yield zero OS notifications", and 8.2 Stage 0: "parking scenarios P1-P3 (8.3) as YAML". But the owner gate pipeline, selector and arm_scope are listed in M15 ("owner gate pipeline, `selector`, `arm_scope` ... parking scenarios in both pipelines"). 11.1(5) says there is no owner engineering before the hosted pilot, and design-rule invariants forbid guarded code (the owner-mode Cargo feature) until Stage 2.  
   *Suggested fix:* Make the Stage 0 gate YAML-only plus a hosted-pipeline run. Move 'run through both pipelines' to M15, which already says it, and delete it from 8.3.
12. **[consistency] 3.4 Drive-it, 7 step 6 vs 2.6, 11.1(1), 7 'Honest promise copy'**: Sender UI shows a model distance. 2.6: "A [MODEL] figure renders in sender-facing copy as 'not promised'". 11.1(1): "no sender-, partner- or owner-facing latency, distance or probability figure outside `policy/promises.yaml`". The 7 heading says "no latency, distance or hit-rate number in contracts or sender UI". But 3.4 says "For background it shows the modelled report distance as a range", and 7 step 6 says "the modelled report distance as a range (N2)". N2 is entirely [MODEL].  
   *Suggested fix:* Replace the range with a qualitative statement ('kilometres at driving speed'), or add a promises.yaml entry flagged 'not promised' that explicitly permits the range. Make 3.4, 7 step 6 and 11.1(1) say the same thing.
13. **[consistency] 2.4 I13 vs 5.2 (Path S, iOS permissions)**: I13 says an OS notification is posted or scheduled only if "OS permission is fully granted" and "Only the kernel posts, at wake time". 5.2 allows "provisional authorization for service class only" on iOS. 5.2 also describes Path S, where the OS posts at fire time and the content is "frozen at scheduling". 5.2 admits Path S "bypasses the Gate at fire time", so the I13 condition 'device positively CLEAR per sidecar' cannot be evaluated at post time. 9.2 bans UNLocationNotificationTrigger only "in any third-party path", so owner Path S is permitted.  
   *Suggested fix:* Add an explicit Path S clause to I13 (schedule-time conditions only, plus the ring-clearance pre-gate) and either define 'fully granted' to include provisional or remove the provisional-authorization sentence.
14. **[consistency] 5.3 step 9 and 4.3 vs N6**: The per-day wake cap is undefined and conflicts with write-iff-post. 4.3: "The per-day wake cap counts only wakes that reach Phase C (5.3)." 5.3 Phase D step 9 increments "the per-day wake counter" in the same step as the ledger row, i.e. only after a post. A counter that counts Phase C wakes that end in no post requires a persistent write without a post, which breaks "persistent writes happen if and only if a notification is posted". N6 (which should hold every cap, per 2.6) has no wake-cap value at all.  
   *Suggested fix:* Either drop the wake cap, or define it as counting posts only (then it duplicates N6's 3 per day), or allow a memory-only counter. Put the value in N6.
15. **[consistency] N6 vs 4.1 ENTER, 7 step 3, 9.3a**: The cooldown and re-arm numbers disagree, and 'once per arm window' defeats the re-arm. N6: "Arrival: one per trigger per 6 h re-arm and one per arm window"; "per-place cooldown 4 h". 4.1: "Re-arm after confirmed EXIT plus cooldown (30 min)". 7 step 3: "re-arm 6 h". 9.3a: "once per trigger per re-arm and at most 2 per day per app". If arrival is limited to one per arm window (step 8: a 2-day window), a 6 h re-arm and '2 per day' cannot occur for the same signpost. Also, PLAN 5.6 step 5 caps (per signpost 2/day and 5/week, per sender 3/day, per app interruptive 4/day) are not reconciled in 10.1/10.2 with N6's '3 per day per app'.  
   *Suggested fix:* Define the arrival constraint once (for example: one per arm window, with the 6 h re-arm applying only when arm_scope = always). Say which cooldown is the detector hysteresis (30 min) and which is the cap cooldown (4 h). List the PLAN 5.6 cap differences in 10.2.
16. **[consistency] 2.4 I17 vs 2.1, 6 L1, 3.2/2.2 fields**: The signed envelope list omits fields the kernel enforces. I17 signs "geometry, event types, delivery, text, label key, purpose, aud, audience_scope, sensitivity class, relation, arm_scope, selector, sz_sha256, live_at, fresh_required_s, pack_kind and expiry". It does not name `content_mode` (2.1 reserves it, and the gate pipeline is chosen by it), `intent` (L1 'Intent whitelist (signed)'), `experience` (guide vs deeplink, which is the tap target), `venue_exposure_class`, `min_mode`, `precision_class`, `max_event_age_s` and `max_event_dist_m`, `cooldown_s`. 'purpose' appears in I17 and 9.3 but is not in the 2.1 reserved-field list, whereas 'intent' is. The same list is copied into 10.1.  
   *Suggested fix:* Add the missing fields to I17, or state that `purpose` means `intent` and that fields not listed are covered by 'geometry/event parameters'. Pick one term (intent or purpose) and use it everywhere.
17. **[consistency] 11.5 M15 vs 11.4 M13.1, 9.1, 11.2 G-O0, 2.3 table**: Ordering and duplication of the level-2 claim. 9.1 and 11.4 put the claim ladder (levels 0-2) in M13.1 (Stage 1.1, behind G-1.1). 2.3 requires owner mode to have a "venue place claim (physical address, level 2)". 11.5 M15 lists "level-2 claim" again. G-O0 does not require G-1.1 or M13.1. So it is unclear whether Stage 2 builds the ladder itself or depends on Stage 1.1, which may not exist.  
   *Suggested fix:* Make M13.1 a G-O0 prerequisite, or move the level-2 claim wholly to M15 and have M13.1 reuse it. Say which.
18. **[consistency] 10.1 header and table vs 2.4, 2.5, 8.7, M3**: 10.1 promises "every changed invariant, rule or label ... with old wording, new wording and reason", but there are gaps. I7 is marked 'extended to exit/dwell/sequence' in the 2.4 table, but has no 10.1 row. R7, R8 and R9 are 'added' in 10.1's R6 row and used elsewhere (9.1 'R7', 2.5 'R1-R9', M3 'R8', 'R7, R9'), but no document gives their wording. I14(b) overlaps I8(b): both govern `/v1/config` and `/o/*` request invariance, yet I8(b) is 'enforced in v1' and I14 is 'design-rule until Stage 2'.  
   *Suggested fix:* Add I7 and R7-R9 rows with full wording to 10.1 (or 2.4). Make the I8(b) / I14(b) split explicit: which clause owns `/v1/config`, and which status applies.
19. **[consistency] 4.3 'Foreground/partner-fed' bullet vs 9.1, I18, N10**: The foreground minimum radii conflict with the v1 150 m floor. 4.3 (what the console shows): "zone inradius >= k_in x acc_eff (practically 25-50 m)", "gate setback >= 30-40 m". 9.1 and I18: "In v1 no sub-150 m geometry exists for any sender"; "Triggers below the 150 m floor exist only under a verified, feature-bound place claim" (1.1). 3.4 recipes use 25 m / 50 m / 100 m radii, flagged 1.1 only in a parenthesis.  
   *Suggested fix:* Mark the 25-50 m values in 4.3 as 'Stage 1.1, claimed places only', or reword them as 'the physical limit, not what the console accepts'.
20. **[scope-feasibility] 1, 8.6, 12 risk 1, 13 decision 3**: The 'v1 is small' claim covers only the addendum delta. The re-tier is 18-32 pw [U], but it sits on PLAN.md M0-M13, which the addendum itself judges at about 85-150 pw. That is about 103-182 pw before the first advertiser. At the stated 4 pw per calendar month for one engineer, that is roughly 26-45 months; a team of 2-3 would still need about 9-20 months. Decision 3 asks the owner to accept the delta, not the total, and the total is never stated.  
   *Suggested fix:* State total effort to the first live advertiser (base plus addendum Stage 0/1) and a headcount assumption in 8.6 and in decision 3. Ask the owner to accept that total, not 'one fifth'. Keep the existing 're-estimate at M1/M5' rule.
21. **[scope-feasibility] 3.1, 4.1, 5.1, 8.2, 11.3 (M4, M5, M7, M11), 11.4**: Hidden heavy or off-path items remain inside v1. (a) M5 still includes the metro-level A/B/C owner-buffer coverage run, the registry import with confidential flags, and the 4-scope revocation and W_service plumbing. These are owner-mode items, and 'coverage map' in the 11.5-20 pw picker block is ambiguous about whether they are counted. (b) M7 builds the owner auto-lane, the change-class diff classifier with a 40+ diff table, and the phishing corpus, then ships the lane disabled. Hosted v1 is 100% human review, so this is dead code, which the addendum's own design-rule principle forbids. (c) M4 and M11 build the ring formula, OS model, tripwire planner, 21-day test and drives, while hosted rings are OFF in the pilot ('zero hosted rings'). Only G-O0 needs those data. (d) Three of the four v1 pickers (area, neighbourhood, road) are ODbL-lineage, sandbox-only until the M12b memo, and are built at M7 before that memo exists. If counsel says no, they fall back to 'authored geometry only' and the work is wasted. (e) v1 advertises 'enter and approach', but with hosted background off, approach (PLAN tier B, multi-km) is only deliverable for foreground or partner-fed navigation-type partners. The addendum never says which partner types v1 actually serves.  
   *Suggested fix:* Cut or defer. Move the A/B/C coverage run and owner-lane diff/auto-lane to Stage 2 (M15). Reduce M4/M11 to the minimum field data needed for G-O0 and label it so. Ship POI plus draw-your-own first, and add the ODbL pickers after the M12b memo. Add one sentence naming the v1 partner archetype for approach events and which location mode it uses.
22. **[scope-feasibility] 11.2 (G-O0, G-O1), 8.4, 8.7**: The G-O0 entry gates are only partly real. Nothing names a gate owner (person or role) or an independent approver. Items 7 (design ADR), 8 (alternatives note) and 9 ('burn-down recorded') are self-attested by the team that wants to proceed. Nothing ties code to the gate: check-invariants blocks guarded code paths, but flipping owner-mode CHECKs or the Cargo feature does not require a gate record. G-O1 has no timebox and no named person who judges 'approved with conditions the product cannot meet'.  
   *Suggested fix:* Add 'gate owner' and 'evidence artefact path' columns. Name an approver who is not the author of the work being gated (the owner, or outside counsel for the memo items). Add a CI rule that merging the owner-mode feature, the content_mode CHECK widening, or notification-posting symbols requires a matching G-O0/G-O1 entry id in docs/legal-gates.md. Add a decision rule for 'approved with conditions' and a date-based review point if Google's outcome is pending.
23. **[scope-feasibility] 2.6, 11.2 G-O0 (5), N12, 11.3 M4/M11**: G-O0 item 5 and N12 contradict the addendum's own sample-size rule. G-O0 accepts '>= 10 background drives per OS and speed class' and N12 pre-registers p50/p90 thresholds from them. 2.6 says no percentile is published from fewer than 20 samples per OS and speed class (report min, median, max and N). With 2 OS x 3 speed classes, 10 drives per cell cannot support a p90 gate.  
   *Suggested fix:* Either raise the gate to 20 per cell (or reduce the cells to Android-only drive and walk, since iOS is measure-only) or pre-register N12 as min/median/max ranges with N, not p50/p90.
24. **[scope-feasibility] 8.4, 5.1, 11.2 G-O1, 11.5 M18, 11.6**: G-O1 clears less than it appears to. It records a Google or Apple outcome for the partner's own plain-geofence feature, not for the SDK-driven product. The addendum marks 'does re-review happen after adding the SDK' as [U], yet M18 go-live is not conditioned on a pass of the SDK-integrated build. Two further policy questions are not in the gate evidence. (1) Google's system-notification rule, [V-sec] only, via a search snippet of 'Unauthorized Use or Imitation of System Functionality': system-level notifications only for an app's integral features, with a system notification used to serve an ad as a listed violation. This is separate from the background-location declaration. (2) Apple 4.10, [V] fetched today: 'You may not monetize built-in capabilities ... such as Push Notifications'. The addendum cites 4.10 only against hosted adverts, but owner mode is a per-venue subscription for a notification-delivery product. Apple 4.5.4 itself is worded for 'Push Notifications' only [V fetched], so the addendum's 'open [U]' on SDK local notifications is correctly stated.  
   *Suggested fix:* Add to G-O1: the Play notification-policy position stated in the declaration and reviewer notes. Add an M18 go-live condition: Play and App Review pass on the exact SDK-integrated build. Add Apple 4.10 and Google's integral-feature rule on a paid notification product to the owner COUNSEL scope (no conclusion drawn here). Consider pricing the subscription for hosting bundles and guides, not 'notifications'.
25. **[scope-feasibility] 5.2, 5.3 (step 4), N7, 8.4, 11.2 G-O0 (5)**: Removing the foreground-service fallback makes the Android verify fix a single point of failure, and nothing before M16 measures it. The Android docs I fetched confirm a geofence-event FGS exemption still exists and that Play removes geofencing as an approved FGS use case on 2027-01-27. The design now requires a triggering location (age <= 30 s) or a bounded getCurrentLocation from a receiver (N7); 'triggering-location availability' is [U] (section 14). If the fix is often absent, 5.3 drops every post silently. G-O1's partner-built plain-notification build does not exercise this path, so the gate can pass while the SDK path fails at M16.  
   *Suggested fix:* Add to G-O0 item 5 and M4: log how often a usable fix (accuracy <= 100 m, age <= 30 s) is obtained at geofence wake, per Android version and standby bucket. Pre-register a minimum rate. If it is below that, Android notification delivery is not a viable product and plan B applies earlier.
26. **[scope-feasibility] 2.3a, 2.4 I14(b), 5.2a, 5.6, 7 step 8b**: I14(b) is violated by the recommended integration. It says the schedule of /o/* and /v1/config requests is a function of app, SDK version, content versions, cache, install-seeded schedule and clock 'and nothing else'. But 2.3a and 5.2a add a partner-called refresh(), and the contract term and step 8b tell partners to call it from their pre-arrival message, which is per-guest and booking-driven. Proxim would see IP and time of a request clustered around the booking, so arm-correlated timing reaches Proxim. 2.3a only says the requests are the 'same', not that the timing is independent.  
   *Suggested fix:* Narrowest honest fix: add 'partner refresh() calls' to the I14(b) input list and declare the timing correlation as a residual under I8(c). Recommend that partners call refresh() for all installs on a schedule rather than per guest, and keep the pre-arrival message's app-open path as the primary mechanism. Add an egress test showing refresh() with an unchanged cache produces the same requests as a timer refresh.
27. **[scope-feasibility] 2.4 I14(c), 5.3 step 4, 5.5 owner line, 6 (No host veto), 9.3 RS-VETO-01**: The 'no arrival oracle for the host' claim overstates what the architecture can deliver, and the consent text inherits the overstatement. The SDK runs in the venue app's process. The OS relaunches that app at the ring crossing, and iOS passes a location launch reason to the host's launch code (UIApplication.LaunchOptionsKey.location, [U], from general knowledge and not fetched today). The host can read its own delivered notifications and its own sandbox, which holds the SDK ledger row and the arm record's 'fired' mark. I14(c) and RS-VETO-01 cover only SDK-registered outputs. The consent line 'Proxim does not tell this venue's app when you arrive' is literally true but is likely to mislead a user. 5.3 step 4 mentions only 'host code in a relaunch' in passing.  
   *Suggested fix:* Restate I14(c) as 'no SDK-provided callback, stream or return value ...; OS-visible facts in the host process (launch reason, the app's own delivered notifications, its own sandbox) are a declared residual'. Change the sheet line to say the phone may start the venue's app in the background when the user enters the area and the venue's app may be able to tell. Extend the DPA clause to launch reason as well as taps, status and delivered-notification state. Add a launch-reason probe to RS-VETO-01 as a documented, not prevented, channel. COUNSEL: deception risk (FTC) if the sheet is left as is.
28. **[scope-feasibility] 5.7 (confidential kinds), 9.1 authoring oracle, 9.6, 7 step 1, N11**: The venue site-test and registry re-test create a shelter-location oracle. 'Not available at this location' is byte-identical across causes, but in a rural area with no hospital, clinic or pharmacy, a refusal can only plausibly come from a confidential barred H3 res-7 cell (DV shelters). An attacker can probe a grid of candidate venues. Separately, re-testing live venues on each registry update and suspending their notifications with a generic message reveals when a new confidential site appears nearby, by timing. Equalised timing and rate limits (N11) slow this but do not remove it. It is unclear whether the 'Available / Not available here' check runs before the Level-2 address-proof claim, which would make probing cheap. Harm: locating DV shelters, which is the highest-harm confidential kind.  
   *Suggested fix:* Run the site test only after a Level-2 physical-address claim, and cap distinct-location tests per org (N11 already limits unreviewed orgs to 3 venues; apply it to tests too). Make refusals for confidential reasons a superset of the real cells: add keyed-hash decoy cells so a refusal in a rural area is not diagnostic. Apply registry-update suspensions on a fixed, delayed batch cycle with jitter, with a message that names only 'registry update'. Counsel and the registry-governance ADR to approve before Stage 2.

## Deliberately not fixed by the repair step

- R8 and R9 wording (consistency 10.1 gap): no document defines them and I would be inventing rules; the author of those rules must write them. I added I7 and R7 rows (R7 wording taken from 9.1) and flagged R8/R9 as unwritten in the R7 row.
- Keyed-hash decoy cells so a confidential-cell refusal in a rural area is not diagnostic (scope shelter-location oracle): a design trade-off that needs the registry-governance ADR and counsel; not decided here. The edits instead declare the probing residual, use one message and timing for all causes, run the site test only after the level-2 claim, cap tests per org, and batch registry-update suspensions.
- Pricing the owner subscription for hosting bundles and guides rather than 'notifications' (scope G-O1 point): an owner/commercial decision plus COUNSEL; only the Apple 4.10 and Google integral-feature questions were added to the owner memo scope.
- Alternative for small shapes: allow an arrival distance bound tied to max_event_dist_m and widen the 5.7 envelope (consistency N7 vs N2). This changes sidecar sizing and I16 exposure, so it needs an owner decision; the edits take the other option (state the access-road-inside-shape assumption, tell the sender, drop arrival beyond the 100 m tolerance). Estate (Class B 150 m ring) drive-in arrival is consequently mostly unavailable; the owner should confirm that is acceptable.
- Named gate owners and approver identities (scope G-O0/G-O1): roles are specified (product owner, outside counsel) but actual names and the 14-day / 90-day decision timeboxes are provisional and need the owner to confirm.
- Actual headcount and funding for the 20-drives-per-cell Android field programme and the total-effort figure (103-182 pw, 26-46 months for one engineer) rest on judgement [U]; the owner must state headcount. The edits state the total and ask for acceptance of it but cannot supply field data or confirm that the ODbL-picker deferral is acceptable for the first demo.
- Whether the metro-level A/B/C coverage run can wait until just before G-O0: I deferred it from M5 to a G-O0 preparation task, but the registry import stays at M5 because the G-O0 site test needs it. If the owner wants the 'urban cores are unservable' claim measured earlier, that item should be restored to M5.

## Still unresolved (integrator)

- All latency and distance figures (N1-N3) are [MODEL] from two Google sentences and an assumed tail ratio; the iOS delay in minutes has no primary source. Only field data (M4/M11, 21-day unattended device, G-O0 drives) can close this.
- Share of guests stale at arrival, WorkManager/BGAppRefreshTask refresh reliability for a rarely opened app, the meaning of 'Network: Disabled' for Rare/Restricted buckets and geofence-woken receivers: unmeasured.
- Facility density per km2 and real serviceable shares: no registry data were loaded, so the sidecar size model and the 'urban cores/campuses largely unservable' claim are assumptions; margin 250 m, cushion 500 m, p_drop_exit 20% and 3 km clearance are provisional.
- Store outcomes (Play background location for a hospitality arrival feature, App Review on SDK local notifications, re-review after adding the SDK, whether a minimal partner-built feature is judged like the SDK-driven one): only an actual submission (G-O1) answers them.
- COUNSEL items with no conclusion drawn: store policy position, WA and other geofence/precise-geolocation statutes (including the transient in-memory fix), the Public information label, ODbL derivative-database question, O3/O4 relations, hosted-background store and law question, employee monitoring, distracted driving and product liability, owner DPA and roles.
- Effort figures are judgement (draft v1 53-94 pw vs re-tiered 18-32 pw [U]); counsel, store review, pen test and postcard durations are not estimated.
- Revoke latency, CDN purge time, reviewer capacity at pilot volume, Supabase WebAuthn MFA support, Vercel OIDC role separation and platform log retention are unmeasured or unverified.
- The six owner decisions are open; W_service=72h, O1-only, label default OFF, the re-tiered v1 and plan B are recommendations until accepted.
- Scenarios S01-S06 and S11 named in the M1 milestone are not defined in any document, and PLAN 5.3 tier B radii disagree with 4.2; both need a decision after M4/M11.
- PLAN-REVIEW #19 and #20 (closed-beta milestone definition, ingest/attestation order, native accessibility) are outside this addendum and not applied.

---

# Review of PLAN-ADDENDUM-02 (sender product)

Adversarial critic review, 2026-10-10. The addendum has **not yet been revised** to address it.

## Verdict

Not ready as written. Rotation reproduces and the honesty of the commuter walkthrough on two apps, reinstall and unopened days is good. Three defects block it: the billing waiver at T=20 on noisy values leaks 33 to 67% of small signposts and is exploitable by splitting into many signposts; partial family shipping is not verifiable under the single signature and a 4-byte `vid`; and the AI extraction stage contradicts the new I2a/I25 invariants. The invariant accounting is also inaccurate: the billboard principle and report auto-pause are loosened, `var` is history-derived despite I24 saying none is, and the token-unlinkability story is internally inconsistent. The epsilon story understates device-level cost by about 6x, and v1 pays for per-version analytics (`var`, C3, per-version codes) that 80% or more of pilot senders cannot use. I verified three AI-policy claims on primary pages [V]: structured outputs do not support min/maxLength and refusal or max_tokens can return non-conforming output; Sonnet 5.5 is $2/$10 and Haiku 5.5 is $0.10/$0.50 per MTok; and API content is not retained by default except for the Covered Models (Fable 5/5.1, Mythos 5/5.1, which keep 30 days), with flagged content kept up to 2 years even under ZDR. So the builder's 'up to 30 days' is wrong for the pinned Sonnet and Haiku models and the retention item can be closed. Fix the three blockers, cut `var`/C3/per-version codes from v1, relabel the loosenings, and test the date-keyed rotation alternative before M3. Simulation scripts and my extras are in /tmp/claude-0/-home-user-Proxim-/29850a78-8535-5334-9f6d-ac96b0135424/scratchpad/critic/ (extra.py, extra2.py, extra3.py, rerun_out.txt).

## Statistics check

I re-computed the sample-size table with the standard two-proportion formula (alpha 0.05 two-sided, 80% power) and the relative-CI formula, and it reproduces: 2,315 per arm for 1% vs 2%; 7,747 for 1% vs 1.5%; 4,670 for 0.5% vs 1%; 26,124 for 0.09% vs 0.18%; about 42,700 for 1.0% vs 1.2%; and 380 / 1,521 / 6,085 views for +/-100 / 50 / 25% on a 1% CTR [MODEL]. Noise sd values reproduce too (7.06 at b=5, 3.51 at b=2.5, 1.36 at b=1). What does not survive: (1) the sample sizes ignore release noise; with 2-week cells the 1% to 2% test needs about 3.0k (one cell) to 4.8k (six cells) views per arm [MODEL, mine]. (2) The '1.9% of pilot senders can finish a 2x test' and '48 views per version' come from pilot.py with 3 variants (and N=5), not the recommended 6; at N=6 it is 0.35% and about 40 views per version per 2 weeks, and only about 20% of pilot senders ever reach the >=300-view CTR display rule [MODEL, my recompute on the lognormal]. (3) The small-sender display claims check out for the family weekly cell (P(display) 92.6% at 29 views, about 100% at 119 and 5.2% of senders under 20 true views a week) and for per-version cells (99% at the median, 6 to 9% at p10), but they are mostly irrelevant, because an even rotation makes the per-version view table information-free. (4) The 'waiver bias of order one view a week' is wrong: billed/true is 0.33 at 15 views, 0.67 at 20, 0.90 at 25; 40 signposts of 15 views are billed at 33% (see gap 1). (5) The epsilon/composition paragraph undercounts by about 6x under its own device-level framing (6 per cube-week, 12 per week across C1 and C3, 72 over a flight), and C3's table row mislabels the effective epsilon (Delta 10 for a 2-week cell). (6) The pilot inputs (20,000 WAD, q_open 0.30 and so on) are all [U], as the addendum says.

## Simulation check

I copied rotation_sim.py and variant_select.py to scratchpad/critic and re-ran with 2,000 commuters per cell. All 22 sanity checks pass and results.json is byte-identical to the original (sha256 e65453dbbbf7e9d7b1911c1f5e506c1c6109e7047bf3b80e33b4f49525c6a795) [M]. The cap-coverage (61% vs 100% weekday coverage), decay-as-gate (5.3 / 4.8 / 4.0 exposures in 60 days), reinstall (16.6% at N=6) and two-app (12.5% / 41.3%) figures reproduce. The recommended-N logic follows from the table (min N is 4 at p=1.0 and 0.7, and 3 at 0.4), but with caveats: (a) `lrs_decay` equals round-robin in every cell, so the simulation does not exercise the ledger, decay, purge, dismiss or new-revision logic; (b) 'within 3 calendar days' is trivially 0 for N>=4 at one exposure/day, so it is a cadence check, not a fatigue measure, and N=6 is a cap chosen for byte and review cost rather than derived from the simulation; (c) opens are independent per day, with no holidays or illness; (d) the 'previous 7 weekdays' 45% figure at N=6 comes from the builder's separate commuter.py (different model); (e) golden vectors are Python checked against itself. I also tested one alternative the addendum dismissed, a family-global date-keyed order with no per-install offset [MODEL, mine]: it removes cross-day repeats for a two-app daily commuter (0% against 29.5% for per-install LRS) at the cost of 100% same-day duplicates and worse single-app sparse openers (7.3% back-to-back at p=0.4). I also checked whether weekend passes can steal weekday slots under the 5-per-week cap (Saturday exposures arrive after Mon to Fri has used the quota, so they are blocked and do not displace weekdays); that is not a problem.

## Moderation-load check

queue_load.py reproduces the addendum's numbers: 12.5 / 83 / 414 reviewer-hours a month at 15 / 100 / 500 senders (0.12 / 0.79 / 3.94 FTE at 105 productive hours), and the queue-flood figures of 8 to 27 hours for 200 shells at WIP 8. The inputs are all assumptions. They are plausible for creative reading time, but incomplete: no second-reviewer sample, blind canaries, report quarantine with a 4-business-hour response, landing-crawl failures, appeals or renewal flags; inheritance is assumed to always work although a ruleset change voids it; and the flood case assumes lint-detectable junk. At 15 senders the total stays in the 20 to 25 hour range even at 2 minutes a creative, so the pilot is feasible, but the 4-business-hour SLA is a staffing constraint (someone on call), not an hours figure. At 500 senders (3.9 FTE) the model is no longer small-team. Bait-and-switch hole: the family review is sound for the creatives it shows (per-creative decisions, no approve-all, subset approval signs a derived revision, so the signer never signs unreviewed bytes). It leaves open (a) inheritance across envelope contexts, because the key is content hash, facts hash and landing domain, not category, sensitivity class, geometry or schedule, (b) landing-page content changes after approval (cloaking is only caught by reports), and (c) the reviewer-fatigue risk of diff-based review of 5 of 6 wordings against a base.

## Scope assessment

Not small for a very small team. The v1 list is roughly a second product: AI authoring with two pipelines and eval gates, a family review console, a sharing and permissions system, a conversion pipeline with three entry points, a release-and-billing rework, and a 4-target rotation kernel. The addendum's 25-42 pw estimate is a judgement ([U]); added to PLAN (85-150 pw) and Addendum 01 (18-32 pw) it gives about 128-224 pw, or 32-56 engineer-months at 4 pw/month. A defensible v1 for the owner's request is: manual family editor with fact-lock and lint, one AI drafting call from the typed fact sheet, family review, on-device `select_variant`, C1 weekly release with no waiver, one code per family with CSV entry, and a read-only snapshot link. The pilot cannot use per-version analytics, so C3, `var`, per-version codes, the Redeem desk, partner attestation API, audit fleet and brand kit should move to G-1.1. The addendum's own 'never cut' list (claim-lock, family-in-revision, reviewer matrix, validators, human review) is right and is kept in that cut.

## Invariant-accounting problems (7)

- I2a (new clause) and I25 vs the AI extraction stage: literal addresses, phones and whole pages go to the LLM vendor by design; the free-text targeting parse also sends road and place names.
- I10 and I17 vs partial family shipping (n_shipped < n_declared) with one signature over all creative bytes and a 4-byte `vid` in the signed list: an unspecified verification path that is forgeable at about 2^32 if the device checks a body against `vid`.
- I24 as worded ('every history-derived field is at most 2 bits (none in v1)') is false: `var` is a 3-bit value derived from the on-device ledger. The 'I3 unchanged' claim needs the same fix.
- Billboard principle: widened to depend on this device's exposure counters, but presented as a clarification, and the headline says 'no existing invariant is weakened'. Report auto-pause and the M13 gate are also loosened, and report-with-`var` is missing from the data-flow table (PLAN 6.2 item 4).
- Event unlinkability precondition for `var`: 'single-use random nonce' contradicts the per-token daily counter and C_tok; the issuer-side unlink is policy-only until blind tokens (Phase 2), so the 'closes B2' claim is not earned.
- I21 is formally met by static per-version codes, but at pilot volumes (about 5 to 20 views per version per week) the anonymity set is tiny and loyalty-ID redemption at a till reveals near-ring presence for that person. Not a violation of the letter, a hole in the intent.
- I22: the 'late arrivals' row (weekly total minus partial data), the in-progress spend meter and the day-boundary budget stop are derived from exact counts and are not released cells. The meter and stop are declared as residuals, but the late-arrivals row is not and needs a source rule.

## Findings (15: 3 blocker, 8 important, 4 minor)

### S1. [BLOCKER] The release-and-billing design waives any week whose noisy count is below T=20, and the addendum calls the resulting bias 'of order one view a week'

The release-and-billing design waives any week whose noisy count is below T=20, and the addendum calls the resulting bias 'of order one view a week'. That is wrong. With discrete Laplace b=5 (sd 7.06) and T=20, billed views are 33% of true at 15 views/week, 67% at 20, 90% at 25 and 96% at 29 [MODEL, my recompute]. A sender who splits 600 views/week into 40 signposts of 15 each is billed for about 198 (33%), so about 67% of the impressions are free. Tier 1+ allows up to 200 signposts per r5 cell, and a multi-location chain does this by accident. Partner payout is 'billed x exact share', so partners go unpaid for real impressions while their own console shows the exact counts.

**Fix:** Bill every cell without a waiver (post-process the noisy value to max(0, noisy)), or bill at the sender-by-flight level instead of per signpost-week. Alternatively set T for display only and bill on a pooled sender-week cell. Re-run the pilot model's billed/true figure at T=20 and b=5. The existing 1.000 result comes from K=10 and sd 2.8, not these parameters. Add a fragmentation test to the G-V simulator.

### S2. [BLOCKER] Partial shipping contradicts the single signature

Partial shipping contradicts the single signature. Section 4.4 ships 'n_shipped < n_declared' creatives in dense cells, and section 4.1 says one 65 B signature covers all creative bytes. The device cannot verify a signature over bytes it was not sent unless the envelope signs per-creative digests, and the addendum does not say so. It also defines `vid` as the first 4 bytes of SHA-256 and says the device renders 'no creative whose vid is not in the signed list'. If the device checks a shipped body against a 4-byte vid, a second preimage costs about 2^32 hashes, so a compromised CDN edge or mirror could substitute an arbitrary body for a signed one. That breaks I10 and I17 in practice.

**Fix:** State that the signed envelope contains full 32-byte creative digests, and that the device verifies each shipped body against its full digest. `vid` stays a local ledger key only. Recompute F(N) with 32 B per declared creative (about 190 B at N=6) and re-derive the capacity table. Add a fuzz test: swap a body for another with the same 4-byte vid and expect rejection.

### S3. [BLOCKER] The new I2a clause says no literal street address or phone number reaches any LLM request, and I25 says no end-user or customer data does

The new I2a clause says no literal street address or phone number reaches any LLM request, and I25 says no end-user or customer data does. But stage (a) of the AI pipeline sends pasted text or the whole verified-domain page to Haiku to extract exactly those facts (`phone`, `address_line`, `hours`). The 600-character 'describe it in your words' box and the free-text targeting parse send street, road and place names to the vendor. The sole-proprietor phone on a page may be personal data. As worded, the invariant fails its own test on the day extraction ships.

**Fix:** Either narrow I2a to 'authoring coordinates and end-user data' and list business-supplied addresses and phones as an accepted, disclosed processor flow with a COUNSEL note, or drop the AI extraction stage in v1 and have the sender type facts (the manual form already exists). Do not ship the invariant text as written.

### S4. [IMPORTANT] The commuter walkthrough's driving case is not worked through

The commuter walkthrough's driving case is not worked through. A driver only sees the glanceable (7 words plus an icon). The diversity gate requires only 'distinct glanceable strings', with no distance threshold, so two glanceables differing by one word pass. The image, the headline and the code chip never reach a driver. The sample glanceable 'Fresh roast, 1.2 km ahead' contains a number that fails the addendum's own fact-span lint and cannot be true at every point on a corridor, because signed text is static and distance is an SDK chip. The Friday 'shows BREW10-C from her Saved card' step needs a Save tap while driving, which the addendum calls 'the only action while driving'. That is a driver-distraction question for COUNSEL and the app stores, and the walkthrough treats it as the normal path.

**Fix:** Apply the 0.35 distance gate to glanceables, since they are the only text a driver sees. Ban literal distances and directions in signed text. Fix the example. State plainly that for drivers the variation lives in the 7 words and icon, and that driver-to-redemption conversion is near zero unless the save is automatic or voice-based. Have counsel rule on the tap-to-save-while-driving question.

### S5. [IMPORTANT] The two-app and reinstall limits are declared 'unfixable without an identifier', but only one stateless design was tested

The two-app and reinstall limits are declared 'unfixable without an identifier', but only one stateless design was tested. The `DayIndexed` policy in variant_select.py uses a per-install offset and order. A family-global date-keyed order needs no identifier. In my run (N=6, 1,500 commuters per cell, two independent apps) the share of exposure-days that repeat a wording from the previous exposure-day was: per-install LRS 29.5% / 18.5% / 12.9% at p=1.0 / 0.7 / 0.4, against date-global 0.0% / 0.1% / 2.2% [MODEL, mine]. The price is same-day duplicates (100% / 54% / 25% of days, against 16.6% / 8.7% / 4.1% for LRS) and, for a single sparse app at p=0.4, a 7.3% back-to-back rate, against 0 for LRS. The sparse-opener loss and the cross-app win trade directly, and the addendum never presents the trade.

**Fix:** Put 'date-keyed family order as the tie-break or prior for the ledger' on the M0/M3 spike list and simulate it in proxim-sim, including a hybrid. At minimum reword section 5.5 to 'with a per-install ledger the cross-app repeat cannot be fixed', not 'cannot be fixed without an identifier'. Owner decision 2 should show the trade.

### S6. [IMPORTANT] The invariant label 'No existing invariant is weakened; one clarification' is not accurate

The invariant label 'No existing invariant is weakened; one clarification' is not accurate. (a) The billboard principle is widened to depend on this device's own exposure counters. That loosens it, and Addendum 01 criticised the same kind of 'clarification' as silent change A15. (b) `var` is a value derived from the device's history, because the shown version depends on the ledger. Yet I24 says every history-derived field is at most 2 bits, 'none in v1', and I3 is 'unchanged'. As worded I24 is false, since `var` is 3 bits. (c) The report-concentration quarantine needs `var` on user reports, but PLAN 6.2 item 4 (reports carry only signpost id and reason) is not in the 9.2 data-flow changes. (d) Report auto-pause becomes a minimum of 25 reports plus a ratio, with review within 4 business hours. For a 2,000-impression ad that is 1.25%, against the old 0.5%. A harmful ad can keep serving over a weekend. That loosens a safety control. (e) The M13 reconciliation gate moves from 1% to 2%/5%.

**Fix:** Relabel (a), (d) and (e) as loosenings with reasons. Reword I24 so it counts `var` as a 3-bit history-derived field and states the rule as 'at most 3 bits, `var` only', or remove `var` (see the next item). List `var` on reports in the data-flow table. Make quarantine immediate-hold (the ad stops serving) with the human review following.

### S7. [IMPORTANT] `var` in events, the per-version cube C3 and per-version codes carry the biggest privacy cost in the addendum, for almost no pilot value

`var` in events, the per-version cube C3 and per-version codes carry the biggest privacy cost in the addendum, for almost no pilot value. At 6 wordings, about 80% of pilot senders never reach the display rule for CTR (>=300 views per version over the flight) and 0.35% could finish a 2x A/B [MODEL, my recompute on the lognormal in pilot.py]. The addendum's '1.9% could finish a 2x test' is stale. It comes from pilot.py with variants=3, and its '48 views per version' implies N=5, not the recommended 6. The by-version table for a median sender is about 40 plus or minus 7 views per version, which is even by construction, with CTR shown as '-'. Per-version codes then have an anonymity set of roughly 5 to 20 devices per week (p10 and median senders). A named loyalty member who redeems a code tells the sender the person was near a 150 m ring in the last days, and the sender can add a per-sibling-signpost code. I21 is formally satisfied, but the k is tiny.

**Fix:** Cut `var`, C3, per-version codes and the by-version tab from v1. Use one code per family. Verify rotation evenness with the simulator, golden vectors, the audit fleet and an on-device test, none of which needs a server field. Add per-version reporting back at G-1.1 when the volume rule holds (for example, a sender with >=2,000 views per version in a period). That also removes the single-use-token precondition for `var` and most of the composition accounting. If the owner insists on per-version codes, gate them behind an estimated cohort of at least k devices per code.

### S8. [IMPORTANT] The epsilon and composition story is incoherent against its own framing

The epsilon and composition story is incoherent against its own framing. The addendum sizes noise to the per-device weekly cap, which is device-level adjacency. A device then touches every metric of a cube: views 1, taps 2, saves 1, dismiss 1, hide 1, summing to 6 per cube-week, 12 across C1 and C3, and 72 over a 6-week flight [MODEL]. The text counts views only ('about 2 per device-week, about 12'), a 6x understatement. The table labels C3 as 'same parameters, epsilon 1, Delta 5', but a 2-week cell has a sensitivity of 10 and so an effective epsilon of 2 per period, or 4 for the selectable 4-week period. Two apps and reinstalls multiply this again, and the caps are client-enforced.

**Fix:** Either state a Delta per period (5 times the number of weeks) and sum across metrics, or drop the epsilon language and the `privacy_budget_ledger` ceilings. Describe the release as 'thresholded and noised' with no budget. The addendum already forbids DP claims in copy, so a trigger-enforced budget built on understated numbers is false assurance. A one-line honest statement is better: 'device-level epsilon is in the tens; this is noise plus threshold, not DP'.

### S9. [IMPORTANT] The events-unlinkability precondition for `var` is described inconsistently

The events-unlinkability precondition for `var` is described inconsistently. Section 7.3 says tokens are 'single-use random nonces'. Section 10 and the `ingest_token_counters(token_hash, day, n)` table assume a per-token daily cap (C_tok, 20 events per token per day). A single-use token has n=1 always, and a multi-use token appears with every event of that device that day, so the day's `var` sequence is linkable by anyone who sees request headers. A single-use nonce also needs a server-side validity record, and the issuer knows which attested device got which nonce. That is a policy-only unlink until Phase 2 blind tokens. PLAN 6.3 admits this for phase 1, but the addendum calls the precondition a gate that 'closes' B2.

**Fix:** Say which it is. If tokens are multi-use per day, per-device-day linkage exists and `var` must not ship. If single-use, replace the per-token cap with an issuance cap per attestation and write down that the issuer link is policy until blind tokens. Tie `var` to Phase 2, or cut it (see above).

### S10. [IMPORTANT] The moderation load model understates the work and the bait-and-switch hole is open

The moderation load model understates the work and the bait-and-switch hole is open. queue_load.py counts base review (4-10 min) plus 0.45-1.5 min per creative. The addendum's own protections add work the model omits: a second reviewer on 10-20%, blind canaries, quarantine within 4 business hours, landing-crawl failures, appeals, and renewal flags. At 12.5 h/month for 15 senders (0.12 FTE) a 4-business-hour response is a coverage requirement, not an hours figure. It also assumes inheritance always works, but 'any ruleset change cancels inheritance', so every lexicon bump forces full re-reads on the next refresh. At 2 minutes per creative the 15-sender figure is about 20-25 h, which is still tolerable. At 500 senders 3.9 FTE is not a small-team figure. The queue-flood figure of 8 to 27 h assumes junk is lint-detectable. Plausible low-value content from tier-0 shells (all pilot senders start in tier 0) costs a full review at about 10 min per family, and is rate-limited only by reviewer speed. The bait-and-switch hole: inheritance keys on (content hash, facts hash, landing domain). It ignores the envelope context a creative sits in (geometry, schedule, category and sensitivity class, image set), so text approved in one context is inherited into another. The landing page's content can change after approval, and cloaking is 'detected only by user reports'.

**Fix:** Add the omitted workloads to queue_load.py with their own assumptions, and show a reviewer-coverage plan (who answers within 4 business hours, and what happens on weekends). Add the envelope context (category, sensitivity class, geometry digest, schedule class) to the inheritance key, or re-approve whenever geometry moves materially. Store a landing-page snapshot hash at approval and flag any crawl whose rendered text differs. Ruleset changes should flag the lexicon hit for human review only where the lint result changes, not void all inheritance.

### S11. [IMPORTANT] v1 is not small

v1 is not small. The addendum's v1 list includes: AI wizard plus two-stage extraction plus 100-brief and 40-injection eval gates; brand kit; facts library; image pipeline; family review UI with canaries and second review; external guest members with MFA, expiry and removal cascade; internal approval and comments; snapshot and preview links with a separate cookie-free origin and a SECURITY DEFINER reader; Redeem desk PWA; CSV and API conversions; release job with two cubes and a G-V simulator; weekly settlement; partner attestation API; reconciliation; audit fleet; IVT/C_tok; `select_variant` on four targets; variant-scope revocation; MDE calculator; export API. The 25-42 pw figure is described as a judgement, and it sits on top of PLAN's 85-150 and Addendum 01's 18-32. That is about 128-224 pw, or 32-56 engineer-months at the stated 4 pw/month.

**Fix:** Cut v1 to: manual family editor with fact-lock and lint; one AI call that drafts up to 5 wordings from the typed fact sheet (the owner asked for AI, so keep that, but defer URL import, extraction and the big eval corpora to about 30 briefs and 15 injection fixtures); family review; `select_variant`; C1 weekly release with no waiver; CSV conversion entry with one code per family; read-only snapshot link; guest members without MFA and internal approval. Move to G-1.1: C3 and `var`, Redeem desk, partner attestation API, audit fleet, brand kit, preview links, internal approval and comments, calculator, API conversions.

### S12. [MINOR] Rotation and caps depend on the host calling `markShown`

Rotation and caps depend on the host calling `markShown`. For the owner's headline partner (a navigation app in partner-fed, headless mode), a host that never calls it freezes rotation and also freezes the caps, so the same wording could be emitted on every pass beyond 2 per day and 5 per week. The addendum only says 'the TestKit must warn'.

**Fix:** Count a decision against caps at emission when no `markShown` arrives within a short window, and fall back to uniform rotation by emission count. Keep billing on `markShown` only.

### S13. [MINOR] The reconciliation gate of median 2% and p95 5% is unreachable for low-cadence partners by the addendum's own outbox arithmetic (a 0.3 opens/day app delivers only 86% of events with a 7-day TTL, so about 14% loss)

The reconciliation gate of median 2% and p95 5% is unreachable for low-cadence partners by the addendum's own outbox arithmetic (a 0.3 opens/day app delivers only 86% of events with a 7-day TTL, so about 14% loss). Daily comparison against a partner's same-day count is also biased by 1-24 h send delay and the 7-day TTL. Partner attestation counts that come from the same SDK `markShown` calls are not independent of Proxim's reports.

**Fix:** Gate on weekly totals with a lag, per-partner against a stated completeness model, and report loss separately. Define what 'independent' means (partner server-side impression logs, not SDK callbacks).

### S14. [MINOR] The statistics tables are lower bounds in ways the text understates

The statistics tables are lower bounds in ways the text understates. Release noise raises the views per arm needed to detect CTR 1% to 2% from 2,315 to about 3,000 (one 2-week cell), 3,470 (two cells), 4,200 (four cells) and 4,780 (six cells) [MODEL, tap-cell sd 3.51]. Rotation is deterministic, not randomized, so a wording's CTR is confounded with exposure order. 'New wordings show first' gives a refresh revision's new wordings the high-novelty first exposures, and N of 5 or 10 binds a wording to a weekday for a weekday commuter. Derived or refresh revisions renumber `var`, so C3 cells must key on (rev, var) or content hash.

**Fix:** State the noise-adjusted sample sizes next to the unadjusted ones. Forbid N in {5, 10} or randomise phase per install. Key per-version cells on content hash.

### S15. [MINOR] The rotation simulation does not test the logic that distinguishes `lrs_decay` from round-robin

The rotation simulation does not test the logic that distinguishes `lrs_decay` from round-robin. The two are identical in every cell of the report. There are no scenarios for dismiss penalty, gaps over 14 days, 60-day purge or the 2,000-row LRU, new-revision insertion, or autocorrelated opens (holidays, illness). The 'within 3 calendar days' metric is trivially 0 for N>=4 at one exposure per day, so it measures cadence, not fatigue. The section 5.4 'previous 7 weekdays' figure (45% at N=6) comes from a different simulation (the builder's commuter.py). The golden vectors are the Python file checked against itself, with no second implementation.

**Fix:** Add the missing scenarios to proxim-sim before M3. Replace the 3-day metric with 'wording seen in the last k exposures'. Make the first Rust port a CI gate on the golden-vector file.

