# ADR-001: Engine types, numerics and crate graph

Status: accepted for the M0/M1 build, 2026-10-11. Normative detail: [engine-spec.md](../engine-spec.md). Note: ADDENDUM-01 2.1 also calls its naming decision "ADR-001"; this file keeps the number the lead assigned, and the naming decision should be renumbered when it is written (open question).

## Context

PLAN 5.6 and 11 demand a pure, deterministic decision core whose traces are byte-identical on Rust, WASM, Android and iOS, with a native-reimplementation fallback if the UniFFI spike fails (M0 go/no-go). PLAN-REVIEW (the float-determinism finding) noted that std `sin/cos/atan2/exp/powf/ln` differ in the last ulp across platforms, and that `geo` and `h3o` call them. Location is noisy and carries an accuracy radius; headings are unreliable at low speed.

## Decisions

1. **Wire position is e7 degrees (`i32`); geometry is evaluated in integer ENU millimetres.** One projection per (signpost, fix) into a local plane, then only integer predicates (`i128` cross products, one integer square root). Rejected: f64 geometry via `geo` (platform-dependent rounding, boundary flips under jitter); raw e7 arithmetic (anisotropic, so every distance needs trig anyway); a global metric grid such as UTM (zone seams, trig).
2. **Per-signpost frame whose origin is a descriptor field, never derived from the fix.** The TS pack compiler and every engine then agree by construction. Scale is `SCALE[lat_q]` (WGS84 meridian and prime-vertical radii at a 0.01 degree latitude grid, Q32 mm per e7 unit). Rejected: one frame per H3 res-5 cell (up to 0.13% scale error at latitude 45 and 0.37% at 70 at the cell edge, against near zero at a signpost's own origin, and geometry straddling cells would need two frames); a frame centred on the current fix (frames would jitter and make state non-reproducible). Cost: about 0.1% scale error within 5 km at latitude 45, 0.25% at 70, growing linearly; acceptable against the 30 m minimum corridor buffer and 65 m accuracy cap. Latitudes above 80 degrees are unsupported (fail closed).
3. **Float confinement by quantized domains.** The only transcendental results are two finite tables (`SCALE`, 16001 entries; `SINCOS`, 3600 entries, 0.1 degree), computed with `libm` in one module and frozen as golden digests. A Kotlin or Swift fallback port embeds or exhaustively verifies them and needs no libm. H3 is the second float module, via `h3o` 0.11.0 (version re-checked on crates.io 2026-10-11 with `cargo info`; its source confirms `default-features = false` selects `functions-libm.rs` and the crate is `no_std`). `atan2`, `exp`, `pow`, `ln`, `tan` are not used at all: headings are Q30 unit vectors compared by dot product against cosine thresholds read from the table, and the speed and heading filters use a rational weight `dt/(tau+dt)` instead of `1-exp(-dt/tau)`.
4. **The five core crates are `#![no_std]` + `alloc`.** This makes the clippy `disallowed-methods` list (E1) a second line of defence rather than the first: std float methods, `HashMap`, clocks, files and threads do not exist in those crates (E2, E3, I1 reinforced). `proxim-sim`, `-ffi`, `-wasm` and tests use `std`.
5. **Units are integers with the unit in the name** (`_e7`, `_mm`, `_ms` as `i64`, `_mmps`, `_ddeg`, `_permille`, `_milli`). Time is one `i64` ms timeline; the engine has no clock. Caps use local calendar buckets from `clock.tz_offset_min` and a persisted high-water mark so a clock set backwards cannot reopen a cap.
6. **Accuracy is an input and the straddle rule is in the type system.** `acc_eff = max(acc, 10 m) x kappa`; `classify` returns Inside, Straddle or Outside; ENTER needs Inside, EXIT needs Outside, which yields a hysteresis band of `2 acc_eff`. The single exception (corridor tests use the centre point) is stated in the spec with its compensations and its false-positive cost.
7. **A shared `proxim-types` crate, and `proxim-detect` as its own crate. Deliberate deviation from PLAN 3,** whose layout lists neither. Without `proxim-types`, `ReasonCode`, `EventKind`, `Fix`, `Position` and the signpost descriptor would have to live in one of geo, path, detect or decide; detect needs the reason codes and decide needs detect, so any placement either creates a cycle or makes geo depend on decision vocabulary. `proxim-pack` (M2), `proxim-events` and `proxim-sched` also need the descriptor without pulling in the engine, and the canonical-JSON writer must be single-sourced. Graph: types <- geo <- path <- detect <- decide. `proxim-detect` follows ADDENDUM-01 4.1 (`detect(state, fix, triggers, clock)`). The lead must update the PLAN 3 layout (open question).
8. **Fire timing lives in detect, filters and selection in decide.** The time-to-anchor rule defines when an APPROACH event exists, so decide cannot delay it; decide owns the ordered hard filters, caps, event age and distance, and slot selection, and can revert a loser's detector to its pre-event state.
9. **State splits into `caps` (persistable, I3) and `mem` (memory-only: trace, filters, detectors).** The golden vectors hash both. A shell that persists `mem` violates I3; the device-store snapshot test (I3) enforces it.
10. **Canonical JSON is integer-only with sorted keys; `Output` carries no coordinates; reasons other than fires appear only under `explain`.** This keeps the golden bytes platform-independent and lets a shell log everything it receives without breaching I2 or I6. In M1 a fix inside any standoff suppresses all classes, wipes `mem` and reports the generic `SUPPRESSED`.

## Deviations from PLAN and addenda (all recorded so reviewers can object)

- ENTER/EXIT use ADDENDUM-01 4.1's `k_in`, `k_out`, `N_in`, `N_out`; exit hysteresis is `acc_eff`-based, not N6's departure margin (that applies to owner departures, parked).
- Approach re-arm follows the playbook (distance beyond `1.2 x d_hi` from the anchor); ADDENDUM-01's "or the anchor is passed" would re-fire on a U-turn, so passing alone does not re-arm.
- P1 needs heading within 45 degrees of the route tangent (PLAN 5.5); the fire rule narrows it to the trigger's `bearing_tol` (30 degrees default, playbook 4). Foot and cycle use lower speed minimums than the vehicle 2.5 m/s; their confidence is capped at 600.
- `CAP_APP_INTERVAL` (one fire per 30 min per app) is built in M1 although PLAN schedules full caps for M3, because it is one field and prevents back-to-back cards from two signposts.
- The M1 standoff is stricter than R2's text (all classes, not only health). Relaxing it needs the R2 wording settled with counsel.
- Health's 0.7 confidence floor is reachable by P1 only at accuracy <= 30 m with a bonus; P2 is reserved. Health signposts are therefore rare until M3.

## Consequences

- Positive: bit-reproducible traces, exact boundary predicates, ports verifiable without libm, no hidden state, small attack surface for fuzzing (JSON in, JSON out).
- Negative: `i128` arithmetic costs speed on 32-bit targets (acceptable: tier-0 is 500 signposts and fixes arrive at 1 Hz); the frame scale error grows with distance from the origin; two float modules remain to audit; the 0.01 degree latitude grid and 0.1 degree course grid are deliberate precision limits.
- Verification: golden vectors for geo, path, detect, decide, h3 and the two tables; proptest against turf.js (0.2% + 0.1 m band, since turf is spherical); property tests `decide_is_a_pure_function_of_its_arguments`, `decide_ignores_input_order`, `seed_has_no_effect_in_m1`; a repo test that fails if `f32`/`f64` appear in `proxim-types` or outside the two float modules.

## Open questions

- Lead: update PLAN 3 layout for `proxim-types` and `proxim-detect`; renumber the ADDENDUM-01 naming ADR.
- Counsel (M3): whether non-health ads may fire inside a standoff and what R2 means for the ledger.
- `core/clippy.toml` must list the methods in engine-spec 2.4; whether `disallowed-types` can ban the primitive `f64` in this toolchain is unverified, so the grep test above is the fallback.
