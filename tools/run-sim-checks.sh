#!/usr/bin/env bash
# All scenario-simulator checks in one command (docs/simulator.md):
# CI-sized scenario suite (debug), seed sweep (release), turf.js comparison, mutation check.
#   tools/run-sim-checks.sh          # 1,000-seed sweep, 200k turf cases
#   tools/run-sim-checks.sh --full   # also 1,000,000 turf cases
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
manifest="$root/core/Cargo.toml"
turf_cases=200000
if [[ "${1:-}" == "--full" ]]; then turf_cases=1000000; fi

echo "== scenario suite (CI-sized seeds, checked-in report)"
cargo test --manifest-path "$manifest" -p proxim-sim --test scenarios

echo "== scenario sweep, 1,000 seeds (release)"
PROXIM_SIM_SEEDS=1000 cargo test --manifest-path "$manifest" --release -p proxim-sim --test scenarios -- --nocapture

echo "== turf.js comparison ($turf_cases cases)"
npm ci --prefix "$root/tools/turf-compare" --no-audit --no-fund >/dev/null
cargo build --manifest-path "$manifest" --release -p proxim-sim --bin proxim-sim
bin="$root/core/target/release/proxim-sim"
"$bin" geo-cases --count "$turf_cases" --seed 1 | node "$root/tools/turf-compare/compare.mjs" > /dev/null
echo "turf: $turf_cases cases within tolerance"
if "$bin" geo-cases --count 20000 --seed 2 | node "$root/tools/turf-compare/compare.mjs" --perturb > /dev/null; then
  echo "turf: perturbed answers passed, the comparison is not checking anything"; exit 1
fi
echo "turf: perturbed answers rejected (as they must be)"

echo "== mutation check"
python3 -I "$root/tools/mutation-check.py"
