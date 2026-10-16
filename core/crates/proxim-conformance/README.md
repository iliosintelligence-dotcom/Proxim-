# proxim-conformance

Black-box conformance suite for the Proxim decision engine, written from `docs/engine-spec.md`
(including its Implementation errata) and not from the implementation. `publish = false`.

It uses only public APIs: `proxim_ffi::decide_json` (and `proxim_wasm::decide_json`),
`proxim_geo::*` and `proxim_types::{canon, strict, ReasonCode, ...}`. The canonical-JSON checker,
SHA-256 and the reference trigonometry in this crate are independent re-implementations, so a bug the
engine shares with its own helpers cannot hide here.

## Run

```
cd core
cargo test -p proxim-conformance                 # everything, debug (deterministic, seeded)
cargo test -p proxim-conformance -- --ignored    # the known spec violations (must FAIL, see below)
PROXIM_FUZZ_SCALE=20 PROXIM_CONF_SCALE=5 cargo test --release -p proxim-conformance   # heavier sweeps
cargo run --release -p proxim-conformance --bin fuzz -- 1 1000000                      # long fuzz
cargo run --release -p proxim-conformance --bin fuzz -- one <case-seed>                # replay one case
```

## Layout (tests/)

| File | Covers |
|---|---|
| `analytic_fire.rs` | spec 11 / 6.2: 1247 / 645 / 215 / 172 m fire distances, crawl rule, d_hi clamp, late join, ENTER 140 m / EXIT 160 m, cool-down |
| `analytic_geo.rs` | spec 1-4: SCALE and SINCOS tables vs an independent WGS84/Taylor computation (all 19 601 rows), table digests, integer primitives, polygon/circle depth, polyline projection |
| `filters_global.rs` | F01-F10 alone, boundaries, every compatible pair (first in spec order wins), standoff behaviour |
| `filters_signpost.rs` | F11-F19 alone, all 383 compatible subsets of defects (first wins), reserved events, strict wire format |
| `filters_event.rs` | F20-F24 alone and all 31 subsets, consumed events, commit |
| `caps.rs` | 2/day, 5/week, 1800 s, local day/week buckets, high-water mark, clock backwards, pruning |
| `selection.rs` | F25 / one-slot selection, tie-breaks, revert of the loser |
| `approach_gates.rs` | gate order, P1 confidence ladder, must-not-fire list (U-turn, opposite carriageway, parallel road, stopped, jitter, consent, permission downgrade) |
| `path_exact.rs` | speed EWMA and heading blend re-implemented from 5.1/5.2 and compared bit for bit; d_rem / t_hat arithmetic |
| `lifecycle.rs`, `more_spec.rs`, `scenarios.rs` | gaps, pruning, revisions, policy narrowing, polygon ENTER, ids, L_min, d_pass, tunnels, 300 overlapping signposts |
| `invariants.rs` | I6 output hygiene, state well-formedness, caps over random worlds, consent withdrawal, reason-code table |
| `determinism.rs` | same bytes twice, shuffled signposts, seed, threads, key order, duplicate revisions |
| `roundtrip.rs` | `parse(write(x)) == x` and canonical writer for random valid values of every wire type |
| `never_panic.rs` | >60 000 fuzz cases: arbitrary strings and bytes, token soup, structure-aware mutations, exhaustive leaf x nasty-number sweeps, deep nesting on a 256 KiB stack, 10 MB strings |
| `contract.rs` | ffi == wasm bytes, all decide golden vectors replayed with independent hashing, manifest hashes |

## Known spec violations kept as `#[ignore]`

None open. Each violation is kept as an `#[ignore = "spec-violation: <id>"]` test that fails until fixed.
The first run found four, all fixed on 2026-10-16 (errata in docs/engine-spec.md):

| id | fix |
|---|---|
| `S-TZ-CAP-BYPASS` | cap entries record their timezone (E-D-TZ); plus the east-move case |
| `D-HEALTH-FLOOR` | health floor 700, never lowered (E-D-HEALTH) |
| `D-NESTED-POSITIONAL` | strict decode rejects positional structs at any depth (E-D-POS) |
| `D-TRIG-HEADER` | table files nest their header (E-G11) |

## Determinism rules

Every random source is a seeded `Rng` (xorshift64*) or `proptest` with a fixed `RngSeed`; no test
reads a wall clock for an expectation (the fuzz watchdog only aborts a hung call). Failing fuzz cases
print their seed and all six inputs; `fuzz one <seed>` replays them.
