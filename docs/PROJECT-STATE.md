# Project state

The living record across sessions. Update it at the end of every session that changes anything: newest entries first, short, factual.

## Where the build stands (2026-10-16)

| Milestone | State | Notes |
|---|---|---|
| M0 foundations | Done | Workspace, guard tools, CI for Linux, macOS, iPhone simulator, Android emulator, WASM, cargo-deny |
| M1 engine (phases 1 and 2) | Done | geo, path, detect, decide; golden vectors; black-box conformance suite; category matching (spec 7.1); health on foot (E-P-FOOT); rank ceiling (E-D-RANK) |
| M1 simulator v1 (phase 3) | Done | 33 scenarios, oracle, noise, turf.js check (1M cases), mutation check, `ci-sim`; phone simulator with partner apps and routing |
| M2 signed packs | **Core done**, open items below | `proxim-pack` (Rust verifier: key set, manifest, revocation list, pack, envelope v2, rollback guards); `packages/pack` (TypeScript compiler and second verifier); golden vectors both ways; attack, fuzz and I10 tests; tier-0 size measured; format in `docs/pack-format.md` |
| M3 decision kernel extras and privacy lint | Planned | pacing draw, small-sender quota, slots, driving-safe, policy matrix, `privacy-lint`, health_facility class (I7) |
| M4 onwards | Planned | See the PLAN.md milestone table |

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
- A `health_facility` class in the engine: no device events and no pacing row (I7).
- PLAN 6.5 ("no pacing row") against Addendum 01 5.3 ("ledger row when a notification posts").
- Standoff measured from the facility perimeter (needs the M5 registry), not the centre.
- Pharmacies as `health_adjacent`.
- Counsel on foot routes that end on a clinic's street.

**M2 open items**
- OI-ROT-1 (Addendum 02, 13), the rotation reference. **Owner: Ilios Digital.** Technical work is done:
  - the file was recovered and committed unchanged;
  - the Rust port reproduces every vector;
  - the edge cases were added;
  - the two disagreements with Addendum 02 5.3 were fixed in favour of the text (R-2026-10-17).

  **Still needed:** a named approver who is not the author signs off. Until then the item stays open, and it blocks M2 exit and M3 entry.
- Signature-check timing on a real iPhone and a real Android phone. The host baseline is about 0.2 ms per entry. The numbers come with the device runs.
- Production keys: the offline root key ceremony and KMS signing keys (M5) are owner and ops tasks. The test keys in the code are public and must never be trusted.
- Not yet built: `open_pack` exposed to the iOS, Android and web SDKs through `proxim-ffi` and `proxim-wasm`; the database columns for signed revisions (M5).

**Scenarios**
- Confirm the definitions of S01 to S06 and S11, which were written by the simulator work (`docs/simulator.md`).

## Session log

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
