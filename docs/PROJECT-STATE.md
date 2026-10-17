# Project state

The living record across sessions. Update it at the end of every session that changes anything: newest entries first, short, factual.

## Where the build stands (2026-10-16)

| Milestone | State | Notes |
|---|---|---|
| M0 foundations | Done | Workspace, guard tools, CI for Linux, macOS, iPhone simulator, Android emulator, WASM, cargo-deny |
| M1 engine (phases 1 and 2) | Done | geo, path, detect, decide; golden vectors; black-box conformance suite; category matching (spec 7.1); health on foot (E-P-FOOT); rank ceiling (E-D-RANK) |
| M1 simulator v1 (phase 3) | Done | 33 scenarios, oracle, noise, turf.js check (1M cases), mutation check, `ci-sim`; phone simulator with partner apps and routing |
| M2 signed packs | **Core done**, open items below | `proxim-pack` (Rust verifier: key set, manifest, revocation list, pack, envelope v2, rollback guards); `packages/pack` (TypeScript compiler and second verifier); golden vectors both ways; attack, fuzz and I10 tests; tier-0 size measured; format in `docs/pack-format.md` |
| M3 decision kernel extras and privacy lint | **In progress** (engine_spec 2) | Slices below |
| M4 onwards | Planned | See the PLAN.md milestone table |

**M3 slices** (PLAN M3; Addendum 01 11.3; Addendum 02 13), in order:

| Slice | What | State |
|---|---|---|
| 1 | Per-sender cap (3/day), per-app cap (4/day, up to 8), dismiss / hide / report | **Done** (E-D-CAPS, E-SPEC2) |
| 2 | Wording rotation wired into `decide` (step 7b), with the chosen wording in the output | **Done** (E-ROT-WIRE); the SDK pending window (5.3) is still to come |
| 3 | Health classes `health_adjacent`, `health_facility` (memory-only, Option A), `prohibited_anchor` (R4) | **Done** (E-HEALTH-CLASSES). R3 destination suppression waits for the route input |
| 4 | Score V x Q x R x P, pacing draw (`PACED_OUT`), up to 3 slots | **Done** (E-SCORE). The 30% small-sender share is a server rule and waits for the owner's definition of "small"; the R bonuses (surface tags, Preference Center) wait for those inputs |
| 5 | Driving-safe format, dayparts and quiet hours | **Done** (E-DRIVE-TIME); counsel item A17 (distracted driving) stays open |
| 6 | Policy matrix as data with exhaustive tests | **Done** (E-MATRIX). Health classes now need a Sealed partner with the health tier (PLAN 6.5). Background and owner rows are in the table but not reachable until M4/M11 and M15 |
| 7 (next) | `privacy-lint`, egress audit, write-spy harness | Planned |

The recommended path after M2 is in `docs/ADVERTISING-STRATEGY.md`: M3, then a thin sender console and ledger, then a one-town pilot.

## Waiting on the owner

**CI**
- GitHub Actions minutes or spending limit. Jobs get no runner, so CI cannot run until this is raised or the month resets.

**Commercial (from `docs/ADVERTISING-STRATEGY.md`)**
- Take rate.
- Whether to publish the fee waterfall.
- The small-sender quota and its definition.
- An agent-ready sender API.

**Health, before any live health traffic**
- **Decided 2026-10-17 (owner): Option A** for `health_facility` frequency caps. **Built** (engine spec 7.4).
  - Showings are counted in memory only and never written to the persisted state.
  - The cap holds within one app session, and a restart resets it.
  - Nothing on disk records a walk towards a clinic.
  - The Addendum 01 5.3 ledger row applies only to owner-mode notifications, which may not carry health classes (Addendum 01 R6), so there is no conflict. Built in M3 slice 3.
- Standoff measured from the facility perimeter (needs the M5 registry), not the centre.
- Pharmacies as `health_adjacent`.
- **Health only in Sealed health-tier apps (M3 slice 6, built as the safe default).** PLAN 6.5 says health adverts may be received only by a Sealed partner app with the health tier enabled by a signed addendum. The engine now applies this to both health classes. Owner question: should `health_adjacent` adverts (gyms, pharmacies, wellness) also be allowed in ordinary apps? Today they are not.
- **Gaps recorded by the slice 6 privacy review (SDK, server and counsel work):**
  - `profile` and `health_tier` must come only from the verified signed partner config, never from the host app.
  - The server must never give them to a web integration.
  - A planned test should prove that host-supplied values are ignored.
  - `surface` is declared by the SDK, and the engine trusts it. Nothing yet stops a host app from re-posting a fire as its own notification. That needs SDK-owned rendering with no advert text handed to host code before display (I11, I13, both still planned) and a clause in the partner terms (counsel).
  - Counsel item C13 in `docs/legal-gates.md`: do pharmacies and gyms count as health care under Washington's My Health My Data Act? The answer decides whether `health_adjacent` can ever be loosened.
- Counsel on foot routes that end on a clinic's street.

**M2 open items**
- OI-ROT-1 (rotation reference): **closed 2026-10-17**, approved by the owner (Ilios Digital owns rotation).
- Signature-check timing on a real iPhone and a real Android phone. The host baseline is about 0.2 ms per entry. The numbers come with the device runs.
- Production keys: the offline root key ceremony and KMS signing keys (M5) are owner and ops tasks. The test keys in the code are public and must never be trusted.
- Not yet built: `open_pack` exposed to the iOS, Android and web SDKs through `proxim-ffi` and `proxim-wasm`; the database columns for signed revisions (M5).

**Scenarios**
- Confirm the definitions of S01 to S06 and S11, which were written by the simulator work (`docs/simulator.md`).

## Session log

- **2026-10-17 (later):**
  - Pull request 6 merged; the owner approved OI-ROT-1 and it is closed, so M2 exit and M3 entry are unblocked.
  - M3 slice 1: sender and app day caps, dismiss / hide / report; engine_spec 2.
  - M3 slice 2: rotation wired into `decide`; the full engine reproduces the approved reference.
  - M3 slice 3: health classes; `health_facility` leaves nothing on disk (Option A); the simulator's I6 oracle caught facility memory surviving a standoff, now fixed.
  - M3 slice 4: full score, pacing draw, up to 3 slots; a test caught the default slot count briefly following the new ceiling (fixed before commit).
  - M3 slice 5: driving-safe format (glanceable / inbox / off), dayparts, quiet hours.
  - M3 slice 6: policy matrix as data (`core/policy/matrix.json`, 256 cells, 6 allow). Health now needs a Sealed health-tier partner. The decide vectors changed only in their policy inputs, with no output or state change.
- **2026-10-17:**
  - Pull request 4 merged (M2 core).
  - OI-ROT-1 committed: `golden_v2.json` (unchanged), `tools/rotation-ref/`, the `new_count` supplement, the Rust rotation port, and the M2 entry layout aligned to the vectors.
  - Owner named the rotation owner, Ilios Digital, and ruled that the plan text wins: window expiry is applied on every call, and late reports only bill.
- **2026-10-16 (later):**
  - Pull request 3 merged (plan fixes, strategy, project memory).
  - M2 core built: signed packs end to end in Rust and TypeScript, checked against each other, on Linux and WebAssembly.
- **2026-10-16:**
  - Phases 1 to 3 merged to `main` (pull requests 1 and 2).
  - Fixed the review findings: engine (ambiguous ids, canonical depth, fuzz replay) and plan documents.
  - Added the rank ceiling, `docs/ADVERTISING-STRATEGY.md`, this file and `CLAUDE.md`.
