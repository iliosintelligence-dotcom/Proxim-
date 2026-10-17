# Proxim

Proxim delivers location-triggered "signposts" (short, signed, sponsor-labelled messages) inside partner apps. A sender (a local business, for example) ties a signpost to a place or a corridor. A partner app embeds the Proxim SDK and the SDK shows the signpost to a person who is near the place, or heading toward it.

## The billboard principle

A roadside billboard knows where it is, not who is looking. A Proxim signpost may depend on where the device is and where it is heading, the time, the partner's closed-taxonomy context, and topics the user chose on the device. It never depends on an inferred attribute, a health input or an identifier. Matching runs on the device against signed public packs: raw location never leaves the phone, and this is made a build-time property (the core has no network dependency, invariant I1). The exact wording and its labelled loosenings are in [docs/privacy-invariants.md](docs/privacy-invariants.md) (row BP).

## Status (2026-10-11)

M0 (monorepo, CI, toolchain) and M1 (geometry, path and decision kernel, location simulator) are IN PROGRESS. Scope is advertising only; resorts and owner mode are parked. Nothing here is released. Of the invariants, only I1 and the repository guards are enforced by tools today; the rest are binding policy with their tests planned (see the status column in the invariants file). M2 has started: `proxim-pack` verifies signed keysets, manifests, packs and envelope v2 entries ([docs/pack-spec.md](docs/pack-spec.md)); the pack compiler and KMS signing are not built. No networking, rotation, scheduler or events outbox exist yet (stubs only).

## Repo map

| Path | What |
|---|---|
| `core/` | Cargo workspace: the on-device engine (geo, path, decide, and stubs). No network crates, no clocks, no randomness, no I/O. |
| `tools/` | Python (stdlib only) checks: `check-no-network.py`, `check-invariants.py`, `check-supabase-ref.py`, with self-tests in `tools/tests/`. |
| `supabase/` | Migrations and database tests (owned by the data architect). |
| `docs/PLAN.md`, `docs/PLAN-ADDENDUM-0*.md` | Source of truth for stack, data model and milestones. |
| `docs/DECISIONS.md` | What the owner accepted, with dates. |
| `docs/architecture.md` | Trust boundaries and the data-flow inventory. |
| `docs/privacy-invariants.md` | Invariants, status and the test that enforces each. |
| `docs/threat-model.md` | STRIDE skeleton and top abuse cases. |
| `docs/legal-gates.md` | Launch gates and counsel list (not legal advice). |
| `docs/adr/` | Decision records and the dated verified-facts list. |

Planned directories (`sdks/`, `apps/`, `packages/`, `samples/`, `fixtures/`) are described in PLAN section 3 and do not exist yet.

## Running the checks

Toolchain: Rust 1.97.0 (edition 2024, pinned in `rust-toolchain.toml`) and Python 3.13. Use a build directory outside the repo.

```
export CARGO_TARGET_DIR=/path/outside/repo/target
python3 -I tools/check-no-network.py          # I1: no network crate in the core, direct or transitive
python3 -I tools/check-invariants.py          # every invariant has a test id; enforced rows must resolve
python3 -I tools/check-supabase-ref.py        # no forbidden or unlisted Supabase project ref
python3 -I -m unittest discover -s tools/tests  # self-tests of the tools
cargo test --manifest-path core/Cargo.toml --workspace
cargo clippy --manifest-path core/Cargo.toml --workspace --all-targets -- -D warnings
```

macOS, Xcode and the Android NDK are not needed for M0/M1 work in this repository; the iOS and Android SDKs come later.

## Contributing rules in one paragraph

Determinism first (integer or fixed-point math, transcendental functions only through `libm`, no hash-order dependence). No network crate anywhere in the workspace. Migrations are files, never applied by hand. The existing "ilios Healthcare" Supabase project must never be used. Nothing in these docs is legal advice.
