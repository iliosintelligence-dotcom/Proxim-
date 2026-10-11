# Architecture: trust boundaries and data flow

Status: M0/M1 in progress (2026-10-11). Source: PLAN 5.1, 6.2, 6.3. If this file and PLAN.md disagree, PLAN.md wins and this file is the bug.

## 1. Principle

Decisions are made on the device. The server publishes signed, public, content-addressed packs; the device downloads the packs for a coarse area and matches locally. The server never receives a coordinate, a route or a per-device event stream. This is why the decision core (`core/`) is forbidden any network dependency (I1), clock, randomness or I/O: time, location and seed are arguments, outputs are canonical JSON.

## 2. Components and trust boundaries

```
 [Sender]--HTTPS-->[Console / API]--[Worker: compile + moderate + sign]--(KMS)
                         |                      |
                    [Postgres, RLS]       signed packs (public CDN bucket)
                                                |  GET, res-5 cell, no key
 ==================== device boundary ============================================
 [Partner app]--embeds-->[Proxim SDK shell]--calls-->[Rust core: verify, match, decide]
        |                      |                          (no network, no I/O)
        |                      +-- OS location (device only), local encrypted store
```

| Boundary | Trusted side | Untrusted side | Control |
|---|---|---|---|
| Device boundary | Core and SDK shell on the device | Everything off device, including our own servers for location | Core has no network crate (I1); egress audit planned (I2, I8) |
| Pack boundary | Verified signed pack | Pack bytes from CDN | JWS ES256 verify against pinned root keys before any use (I10, I17); planned M2 |
| Partner boundary | SDK | Host app code | SDK owns consent receipts, label and rendering; unknown context keys rejected (BP) |
| Sender boundary | Compiler output | Sender-authored text and geometry | Moderation, 150 m floor, sensitive-zone rules, human approval in v1 |
| Tenant boundary | Postgres under RLS | Another sender's session | RLS on every table; cross-sender read tests in `supabase/tests` |
| Signing boundary | KMS key | Everything else | Online key in KMS; test keys only in repo; offline root signs the key set |

## 3. Data-flow inventory: the ONLY things that may leave a device

Anything not in this table is a bug. In M0/M1 the engine performs no I/O, so nothing leaves at all; the table is the contract for later milestones.

| # | Flow | Contents | Must NOT contain | Phase |
|---|---|---|---|---|
| F1 | Tile manifest and pack GETs, global metadata GETs | res-5 cell (about 250 km2 class resolution), IP, time | publishable key, cookie, attestation token, bearer token, device, install or session id (I8a) | M3 |
| F2 | Config GET | publishable key, SDK version | any location-derived value (I8b) | M3 |
| F3 | One-time attestation exchange | platform attestation token | location | M3 |
| F4 | Events, one per request, fixed padded size | `event_id, signpost_id, rev, type (view, tap, save, dismiss, hide), slot, UTC day, sdk and os version bucket, attested, simulated, token` | hour, cell, session, device, IP-derived, user, variant or creative id (I24); none at all for `health_facility` (I7) | M5; random 1 to 24 h delay, per-install daily cap, dropped on consent withdrawal |
| F5 | "Report this signpost" | signpost id and reason | anything else | M5 |

Declared residual (PLAN 6.3): until OHTTP (Phase 2), F1 and the credentialed requests (F2 to F5) from one device share an IP and can be joined by anyone who sees both logs. Phase 1 is non-sensitive classes only.

## 4. What stays on the device

Matching state, path and heading estimates, caps, preferences, consent receipts and any history (I3). The trail ring is memory-only. Owner-mode visit state is not built (I19).

## 5. Server-side data

Senders, organisations, signposts and revisions, compiled packs, signatures and key ids, moderation records, released aggregate cells, billing ledger. No end-user location, no persistent device or advertising identifier. Retention (PLAN 6.6): raw events 14 days, request logs 7 days, aggregates 25 months, billing 7 years, audit log 7 years.

## 6. Out of scope for M0/M1

Networking, packs, signing, scheduler, events outbox, rotation, console, API, SDK shells, owner mode and resorts. Their places in the tree are reserved in PLAN section 3.
