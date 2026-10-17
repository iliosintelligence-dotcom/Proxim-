# Legal and launch gates

NOT LEGAL ADVICE. This file tracks gates that need qualified counsel; engineers do not close them. Status as of 2026-10-11: every gate is OPEN and no counsel sign-off has been recorded. Each gate has an accountable role, an approver who is not the author, and an evidence path (per PLAN-ADDENDUM-02 section 12). Evidence below is a placeholder until a real record exists.

## Gate table

| Gate | Scope | Kind | Needed before | Conditions (summary) | Accountable | Approver | Evidence | Status |
|---|---|---|---|---|---|---|---|---|
| G-V | Variants, release of aggregates, billing, one promo code per family | Pilot gate | First external sender in the pilot | Golden, scenario and mutation CI green; billing tests; review set; I21 to I26 enforced with named tests; counsel reviews charging the 20-view weekly minimum (100 for health_adjacent campaigns; health_facility flat fee, PLAN-ADDENDUM-02 7.7) | Owner (to name) | Counsel plus a reviewer who is not the author | `docs/legal-gates.md` entry with date, memo reference | OPEN |
| G-AI | AI-assisted wording (LLM vendor receives the sender's own screened wording) | Per-org feature gate | AI drafting enabled for an org (pilot can run on the manual editor if it slips) | Counsel sign-off on the vendor DPA and on whichever retention path is used; ONE of: (a) a zero data retention (ZDR) contract in force, or (b) the owner's explicit, recorded acceptance of the stated retention terms without ZDR ('not retained by default; up to 2 years if flagged', copied from the vendor's current page with a retrieval date and re-verified); under either path flagged content may still be kept up to 2 years; otherwise G-AI stays off (PLAN-ADDENDUM-02 3.4, 11); I25 test passing | Owner (to name) | Counsel | Same | OPEN |
| G-SHARE | Sharing: publish, teammates and expiring guests, read-only snapshot link | Per-org feature gate | Share features enabled for an org | Guest expiry enforced server side and in RLS; snapshot links expose only released noisy weekly numbers; tests RS-SNAP and RS-TEAM | Owner (to name) | Counsel plus security reviewer | Same | OPEN |
| G-1.1S | Everything cut from v1: URL import, flyer or PDF extraction, free-text targeting, CSV bulk, pictures and brand kit, per-version analytics, redeem desk, partial shipping | Release gate, per item | Any Appendix A item (G11S-01 to G11S-13) | Pilot evidence, counsel, an ADR and a passing test per item; some need an I25 amendment | Owner (to name) | Counsel | Cited G11S entry id | OPEN, not scheduled |

Parked and not gated for M0/M1: Addendum 01 owner mode, resorts and OS notifications (gates G-1.1, G-O0, G-O1). Third-party ads are never OS notifications.

## Health traffic gate (PLAN 6.5)

Live health-category traffic requires ALL of: written counsel sign-off recorded here (WA, NV, CT, FTC order terms, Apple 5.1.3, 2.5.18, 5.1.5, Google Play declarations); DPIA; partner DPA; OHTTP and blind tokens live and reviewed; production KMS keys with a passed key-compromise drill; registry governance; SDK-owned consent receipts; external privacy review. Until then health stays sandbox-only. Hospitals, clinics, K-12 and childcare are blocked pending counsel. Current status: OPEN.

## Counsel list

| # | Question | Needed for |
|---|---|---|
| C1 | Washington My Health My Data (geofence ban within 2,000 ft of health facilities, private right of action), Nevada, Connecticut: does any health-adjacent or health-facility use remain lawful | Health gate |
| C2 | FTC orders (Kochava structure), FTC health breach and unfair practices exposure | Health gate, G-V |
| C3 | Processor or service-provider position given on-device processing and no precise location received; who "collects"; DPA template; App Privacy label and Google Data safety answers | Partner contracts |
| C4 | Charging the 20-view weekly minimum (100 for health_adjacent) for views that did not occur (A02-D1) | G-V |
| C5 | LLM vendor DPA, retention and ZDR terms, and whether the no-ZDR path (owner accepts the stated retention terms) is acceptable (A02-D3) | G-AI |
| C6 | Whether any ad may reach a driver (COUNSEL A17), glanceable format | Pilot |
| C7 | Apple 5.1.3, 2.5.18, 5.1.5 and 4.5.4 scope for SDK-posted local notifications; Google Play location and data declarations | Store review, owner mode |
| C8 | COPPA: inert when child-directed; confirm the SDK-inert design is sufficient | Launch |
| C9 | "Public information" label and civic content (Addendum 01) | Later |
| C10 | ODbL derivative database question for public packs of derived shapes | Later |
| C11 | Pilot terms: invite-only, partner weekly server-side impression count clause (A02-D4) | Pilot |
| C12 | Sensitive-buffer policy for pharmacies (Addendum 01 A01-D6) | Owner mode, parked |

EU and UK are deferred pending a DPIA and counsel; US-only at launch.

## Sign-off log

| Date | Gate or question | Decision | Counsel | Evidence |
|---|---|---|---|---|
| (none recorded) | | | | |
