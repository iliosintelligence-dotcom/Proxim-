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
