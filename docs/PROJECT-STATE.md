# Project state

The living record across sessions. Update it at the end of every session that changes anything: newest entries first, short, factual.

## Where the build stands (2026-10-16)

| Milestone | State | Notes |
|---|---|---|
| M0 foundations | Done | Workspace, guard tools, CI for Linux, macOS, iPhone simulator, Android emulator, WASM, cargo-deny |
| M1 engine (phases 1 and 2) | Done | geo, path, detect, decide; golden vectors; black-box conformance suite; category matching (spec 7.1); health on foot (E-P-FOOT); rank ceiling (E-D-RANK) |
| M1 simulator v1 (phase 3) | Done | 33 scenarios, oracle, noise, turf.js check (1M cases), mutation check, `ci-sim`; phone simulator with partner apps and routing |
| M2 signed packs | **Next** | `proxim-pack`: JWS ES256, manifest, seq/expiry/rollback, revocation, keyset, TS compiler, golden vectors (PLAN M2) |
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

**Scenarios**
- Confirm the definitions of S01 to S06 and S11, which were written by the simulator work (`docs/simulator.md`).

## Session log

- **2026-10-16:**
  - Phases 1 to 3 merged to `main` (pull requests 1 and 2).
  - Fixed the review findings: engine (ambiguous ids, canonical depth, fuzz replay) and plan documents.
  - Added the rank ceiling, `docs/ADVERTISING-STRATEGY.md`, this file and `CLAUDE.md`.
