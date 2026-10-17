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
| 4 (next) | Scoring V x Q x R x P, pacing draw (`PACED_OUT`), slots above one, small-sender quota | Planned |
| 5 | Driving-safe format, daypart and quiet hours | Planned |
| 6 | Policy matrix as data with exhaustive tests | Planned |
| 7 | `privacy-lint`, egress audit, write-spy harness | Planned |

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
