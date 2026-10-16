# Threat model (skeleton)

Status: M0/M1 in progress (2026-10-11). A STRIDE skeleton to be deepened per milestone. "Control" names what is intended; the Enforced column says whether a tool or test enforces it TODAY (see [privacy-invariants.md](privacy-invariants.md)). Almost everything is still planned.

## Assets

End-user location and movement; consent state; signing keys; sender content and billing; sender tenancy; partner trust.

## STRIDE by component

| Component | S Spoofing | T Tampering | R Repudiation | I Information disclosure | D Denial | E Elevation |
|---|---|---|---|---|---|---|
| Rust core / SDK | Fake fixes feeding the engine (mock location) | Modified pack or revocation list | n/a (no server log of decisions by design) | Network dependency leaking location (I1, enforced) | Malformed pack crashing the parser (fuzz, M2) | Headless integration enabling health classes (I12) |
| Pack CDN | Fake host serving packs | Swapped pack bytes (signature, digest, rollback checks, I10, I17) | n/a | Cell id in GET path (res 5 only, I2) | Cache poisoning, stale packs (TTL, revocation) | n/a |
| API and events | Emulator farm posting events (attestation, caps) | Replay or inflation of events | Disputed billing (signed released numbers, ledger) | IP and request metadata joins (I8c, declared residual) | Event flood (rate limits) | Publishable key used as secret |
| Console and database | Sender account takeover (MFA) | Edit of approved revision (revision pinning, re-review) | Missing audit trail (audit log) | Sender reads another sender's rows (RLS plus tests) | Authoring spam (quotas) | Guest or viewer role gaining submit rights |
| Signing and keys | Forged key set | Compromised online key (revocation by kid, drill) | Unsigned approvals | Key leak via env (OIDC to KMS, no long-lived secret) | KMS outage (cached validity windows) | Worker able to sign unreviewed content |

## Top abuse cases

| # | Abuse | Primary control | Test status |
|---|---|---|---|
| A1 | Sender geofences a hospital or clinic to reach patients, or sets a tiny radius to single out a person | Sensitive-zone standoff, 150 m floor (I18), health class rules (I7, I12) | planned M3 and M5 |
| A2 | Re-identification of a commuter from released numbers or per-device events | Released noisy cells only, closed event allowlist (I22, I24) | planned M5 and M9 |
| A3 | Forged or rolled-back signposts rendered to users | Signature, digest, expiry, rollback and revocation checks (I10, I17, I20) | planned M2 and M3 |
| A4 | Partner or sender smuggles an identifier or user attribute into targeting | SDK rejects unknown context keys, no IDFA or AAID (I5, BP) | planned M3 and M8 |
| A5 | Fake impressions or conversions inflate billing or payouts | Attestation, caps, conversions never feed billing (I23), partner count reconciliation | planned M5 and M9 |
| A6 | Cross-tenant data read | RLS on every table plus cross-sender tests | planned with the first migration |
| A7 | Malicious ad copy, phishing links, cloaked landing pages | Moderation, Web Risk, human approval, signed landing URL (I17, I21) | planned M5 and later |
| A8 | Sensitive content or locations sent to an LLM vendor | Typed prompt serializer, no coordinates or facts (I25) | planned M7b |
| A9 | Network dependency slipped into the core | `check-no-network` over cargo metadata (I1) | enforced today |
| A10 | Use of the existing healthcare Supabase project for ad data | `check-supabase-ref` allowlist (G1) | enforced today |
| A11 | Driver distraction from a signpost while moving | Glanceable-only format, no tap target while driving; COUNSEL A17 open | design rule, not tested |

## Key-compromise drills

Online signing key compromised: revoke by kid, ship a new key set signed by the offline root, confirm clients stop rendering the old kid at the next run. Root key: offline procedure and a client update. Both are M12 deliverables; no production key exists in M0/M1 and repo keys are test keys only.

## Open items

Fuzzing plan for the pack parser (M2); egress audit tool (M3); a full STRIDE pass per new component as it lands; counsel input on A11 and A1 (see [legal-gates.md](legal-gates.md)).
