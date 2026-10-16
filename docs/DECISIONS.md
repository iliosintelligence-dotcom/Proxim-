# Owner decisions log

Decisions made by the owner, with the date and the exact words that made them. Recommended defaults come from the addenda; this file records what was ACCEPTED.

## Addendum 02 (sender product) - ACCEPTED 2026-10-11

Owner message: *"accept all five and start M0 and M1"*. All five decisions below were accepted as recommended. Consequences that engineering relies on: v1 sender scope is text + icon only; billing is per campaign-week with a 20-view minimum (100 views for `health_adjacent`, flat fee for `health_facility`; Addendum 02 7.7) and no waiver (COUNSEL to review charging the minimum before G-V); the billboard-principle loosening and the other labelled loosenings are accepted; the LLM-vendor flow carries sender-authored wording only, with placeholders for facts (counsel sign-off on the vendor DPA before G-AI); pilot is invite-only.

### A02-D1. ACCEPTED

Minimum weekly charge (billing). Choose between a 20-view minimum per live campaign-week (about $0.10 at a $5 CPM [U]; bills views that did not occur; keeps a small week from revealing more than one bit) and clamp-at-zero with no minimum (no charge for nothing, but a lone commuter in a near-empty cell is detectable with accuracy 0.63 at 26 weeks vs 0.99 raw). Both are unexploitable; any pilot discount must be a flat per-campaign credit. RECOMMENDED: the 20-view minimum, COUNSEL to review charging it. 'Share' is not touched by this decision.

### A02-D2. ACCEPTED

Accept the labelled loosenings and removals: billboard principle (the wording shown may depend on the device's own exposure counters and the app's lane), event allowlist plus `save`, decay as ranking weight, human-approval inheritance under the context-extended key, x4 report threshold after a benign confirmation, the M13 reconciliation gate (weekly, independent count, stability not absolute accuracy), removal of the CTR-gap/geo-holdout evidence and of the epsilon ledger. RECOMMENDED: accept all; the rotation feature you asked for cannot exist without the first, and 'no invariant weakened' is withdrawn.

### A02-D3. ACCEPTED

Send the sender's own screened ad wording to the LLM vendor (base wording and up to 3 highlights with fact values replaced by placeholders; never brand name, facts, targeting, pages or documents) as a disclosed processor flow. RECOMMENDED: accept for v1 (needed for 'make variants of my own copy'), counsel sign-off on the vendor DPA before G-AI, and let the pilot run on the manual editor if G-AI slips (G-AI is a per-org gate). The strict alternative (closed lists only) cannot vary your own text.

### A02-D4. ACCEPTED

Pilot operating terms. RECOMMENDED: invite-only, at most 15 senders (planned cohort 8 chosen from about 20 candidates, at least half with mostly non-driving views); a 12-week pilot window (first reconciliation verdict needs at least 70 days); a contract clause that each billing-live partner supplies a weekly server-side impression count (IC-A), otherwise it runs in credit mode with the gate recorded NOT EVALUABLE; at least 2 trained reviewers before the first external sender and the promise 'held automatically, reviewed within 4 business hours' with no weekend human response; partners must pass the 95% markShown sandbox gate.

### A02-D5. ACCEPTED

v1 product scope, including what 'share' means. RECOMMENDED: text plus icon only (no pictures, flyer/PDF/URL import or brand kit until G-1.1S); 'upload your campaign' = paste or type existing copy and duplicate a past campaign; 'share' = (a) PUBLISH (admin submits, human approves, device delivers) plus (b) invite teammates and expiring read-only guests plus (c) a read-only, expiring snapshot link to released weekly numbers; driving surface has no tap target and keeps a 'Seen on the road' row (COUNSEL A17 on whether any ad may reach a driver stays open); one promo code per family, hidden below 100 viewable per 14 days and never on health classes (fallback: no chip in the pilot). If you meant 'share' as only one of (a) to (c), say which.

## Addendum 01 (platform, events, notifications; resort/owner mode parked) - PENDING

Not yet answered by the owner. Owner mode and resorts are parked (2026-10-10: *"forget resorts for now let us focus on ads"*), so these do not block M0/M1. Recommended defaults, for reference:

- **A01-D1** (pending): 1. Notifications for advertising (retained, unchanged): accept that third-party adverts are never OS notifications (they stay in-app and inbox) and notifications are a first-party owner-mode product. Recommended: accept; ad notifications would need written Apple and Google guidance first and no engineering before it.

- **A01-D2** (pending): 2. First non-ad pilot partner and store outcome: the venue's own app run by the same organisation (O1), a rural/remote destination property passing the 9.6 site test, Android first, signed LOI; accept that Android notifications exist only if Google approves background location for the partner's real app (plan B drops the product, no retry), that tenants/sponsors/franchisees/managing agents are hosted or out of scope, and chains over 20 venues are unsupported. Recommended: yes to all; no urban hotel, medical campus or mall first.

- **A01-D3** (pending): 3. Scope and sequencing: accept the re-tiered v1 (enter and approach only; exit/cross/dwell shown disabled; four pickers), owner engineering only after the hosted pilot is live, owner mode staffed as a second product, and the M13 pilot counted as PLAN 6.3 Phase 1 (OHTTP gates M14/GA, not M13). Recommended: accept.

- **A01-D4** (pending): 4. Owner notification behaviour: W_service=72h (alternatives 24h or 7d); stale guests get no arrival notification and a partner pre-arrival open/refresh() is a contract term; departure means leaving while armed; always passive title-only; tap opens guide/allowlisted route only; text/geometry/domain changes wait 24h for human review. Recommended: accept all five.

- **A01-D5** (pending): 5. 'The AI decides it is beneficial' (retained, stricter): accept the five-layer meaning (declared intent classes, review-time AI on sender content, deterministic on-device rules, user controls, owner's declarative arming), with no runtime AI on user data in Proxim and no decision callback into the owner's app (hostVeto removed for good). Recommended: accept; tell the resort partner up front.

- **A01-D6** (pending): 6. Evidence and policy defaults: counsel timing (hosted at M5 exit, owner at LOI) and device/venue funding; sub-150 m only for verified claims on public-access places; armed exit for owner mode only, dwell Stage 3; 'Public information' label stays OFF; I16 buffer unchanged (no pharmacy exclusion without a counsel memo). Recommended: yes to all.

## Other standing instructions

- 2026-10-10: advertising only; resorts / owner mode parked.
- 2026-10-11: start M0 and M1 (monorepo, CI, toolchain spike, geometry/path/decide kernel, location simulator).
- Creating a new Supabase project (cost, organisation, region) needs explicit owner confirmation and is NOT part of the M0/M1 build until given. The existing 'ilios Healthcare' project must never be used (CI guard).
