# Proxim: instructions for every Claude session

Proxim delivers location-triggered "signpost" adverts through partner apps. The phone decides on its own, with no user data and no auctions. Read `docs/PROJECT-STATE.md` first: it says where the work stands, what is waiting on the owner, and what comes next. Update it before you end a session that changed anything.

## Map

- **The engine:** `core/`, a Rust workspace. Pure `decide()` in `proxim-decide`. Geometry is in `proxim-geo`, path tiers in `proxim-path`, detectors in `proxim-detect`, wire types in `proxim-types`.
- **Simulators:** `proxim-sim`, the phone simulator (`phone.rs`, `town.rs`) and the scenario simulator (`scenario.rs`, `runner.rs`, `testdata/scenarios/`).
- **Black-box suite:** `proxim-conformance`, written from the spec.
- **Normative spec:** `docs/engine-spec.md`. Its errata at the end are append-only; add an `E-...` entry for every behaviour change.
- **Plan:** `docs/PLAN.md` plus `docs/PLAN-ADDENDUM-01.md` and `docs/PLAN-ADDENDUM-02.md`. The addenda override the base plan where they say so.
- **Privacy rules:** `docs/privacy-invariants.md`, checked by `tools/check-invariants.py`. Health rules are in PLAN 6.5. Legal gates are in `docs/legal-gates.md`.
- **Product direction:** `docs/ADVERTISING-STRATEGY.md`, the owner's "democratic advertising" brief mapped to the build. Other docs: `docs/phone-simulator.md`, `docs/simulator.md`.
- **Specialist agents:** `.claude/agents/` (trigger engine, SDK, console, architect, sim/QA, privacy reviewer, security reviewer).

## Hard rules

- Never use, reference or link the Supabase project `wjsjkemamttsuilhihip` (ilios Healthcare). `tools/check-supabase-ref.py` enforces this. Creating a new Supabase project needs the owner's explicit confirmation.
- Never weaken a privacy invariant to make something work. Health adverts stay at least 1,000 m from any health facility and never fire on ENTER at a facility (PLAN 1, 6.5; errata E-D-HEALTH, E-P-FOOT). Get a privacy review (the `proxim-privacy-reviewer` agent) for any change that touches location, consent or health.
- No user attributes, advertising IDs, auctions or live server decisioning in v1. This is the billboard principle, PLAN 5.1 and 5.6.
- Treat legal statements as counsel questions, not conclusions.

## Engineering rules

- **Lints:** the workspace denies float arithmetic, `unwrap`/`expect`/`panic`/indexing outside tests, and std float transcendentals. Use integers and fixed point, and return `Result`. Test files add `#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]`.
- **Determinism:** `decide` is a pure function. Golden vectors live in `core/testdata/golden`. Check them with `cargo run -p proxim-decide --example gen_golden`, and regenerate with `PROXIM_BLESS=1` only after reviewing the diff.
- **Checked-in reports:** `core/testdata/sim/*.md|json`. Regenerate with `PROXIM_BLESS=1` (see `docs/simulator.md`, `docs/phone-simulator.md`).
- **Tests also run on the iPhone simulator, the Android emulator and WebAssembly (WASI, no threads):**
  - find test data through `PROXIM_CORE_DIR`, with a fallback to `CARGO_MANIFEST_DIR/../..`;
  - spawn no threads on wasm (use `proxim_conformance::fuzz::on_stack`).
- **Before every push**, from `core/`:
  - `cargo fmt --all -- --check`;
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
  - `cargo test --workspace --all-features`.

  Then, from the repo root: `python3 -I tools/tests/test_tools.py`, `tools/check-no-network.py`, `tools/check-invariants.py` and `tools/check-supabase-ref.py`.
- **CI:**
  - `ci-core`: Linux and macOS, plus cargo-deny;
  - `ci-wasm`: Wasmtime;
  - `ci-mobile`: iPhone simulator and Android emulator;
  - `ci-sim`: 1,000-seed sweep, turf.js, mutation check.

  If every job fails within seconds with no runner, that is the GitHub Actions minutes or spending limit, not the code: say so, and do not "fix" the code.

## Git and pull requests

- Work on the session's assigned branch. If its pull request is already merged, restart the branch from `origin/main`.
- Open a pull request only when the owner asks, and merge only when the owner asks. The owner usually asks for both. Write pull request descriptions in plain language.
- Commit messages say what changed and how it was verified. Do not put model names in commits or docs.

## Working with the owner

- The owner is the product lead, not a Rust engineer. Explain results in plain words: what works, what does not, and what you need from them. Show results (tables, pages) rather than only describing them.
- State what you verified and how. Say plainly when something was not run, for example because CI could not run.
- Record every question that needs an owner or counsel decision in `docs/PROJECT-STATE.md` under "Waiting on the owner".
