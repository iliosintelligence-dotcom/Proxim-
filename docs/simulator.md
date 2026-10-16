# Scenario simulator (proxim-sim, M1 "simulator v1")

Nobody can drive past a hospital for every test. This simulator replays realistic movement
against the real engine (`proxim_decide::decide`, one fix per call, state threaded exactly as an
SDK threads it) and checks exactly which signposts fire, where and when. The checks use the true,
noise-free path. It validates the engine's logic. It does not validate OS wake behaviour: that
needs real devices (PLAN M4 and M11).

Code: `core/crates/proxim-sim/src/` (`trace`, `noise`, `scenario`, `oracle`, `runner`, `junit`,
`geocases`, `stationary`, `bin/proxim-sim.rs`). Scenarios: `core/testdata/scenarios/*.json`.
Synthetic routes: `core/testdata/routes/`. Checked-in report:
`core/testdata/sim/scenario-report.json`. The phone simulator in the same crate is separate
(`docs/phone-simulator.md`).

## Running it

| What | Command (from the repo root) |
|---|---|
| Everything, CI-sized (part of `cargo test --workspace`) | `cargo test --manifest-path core/Cargo.toml -p proxim-sim --test scenarios` |
| Full sweep, 1,000 seeds per noisy scenario (about 100 s release) | `PROXIM_SIM_SEEDS=1000 cargo test --manifest-path core/Cargo.toml --release -p proxim-sim --test scenarios -- --nocapture` |
| CLI with JSON report and JUnit | `cargo run --manifest-path core/Cargo.toml --release -p proxim-sim --bin proxim-sim -- run core/testdata/scenarios --report report.json --junit junit.xml [--seeds N]` |
| turf.js comparison (1M cases locally, 200k in CI) | `npm ci --prefix tools/turf-compare && cargo run --manifest-path core/Cargo.toml --release -p proxim-sim --bin proxim-sim -- geo-cases --count 1000000 --seed 1 \| node tools/turf-compare/compare.mjs` |
| Mutation check | `python3 -I tools/mutation-check.py` (`--list` shows the mutations) |
| All simulator checks in one go | `tools/run-sim-checks.sh` (add `--full` for 1,000 seeds and 1M turf cases) |

The CLI exits 1 if any scenario fails and 2 on a usage or load error. `PROXIM_SIM_SEEDS` sets the
seed count for every noisy scenario. A scenario whose noise model has no random term runs once.
After a deliberate behaviour change, regenerate the checked-in report with
`PROXIM_BLESS=1 cargo test -p proxim-sim --test scenarios` and review the diff.

CI (`.github/workflows/ci-sim.yml`) runs the 1,000-seed sweep in release on Linux and macOS. It
compares the two 1,000-seed reports byte for byte, and it compares the CLI's CI-sized report with the
checked-in file. The same job runs the turf comparison (200k cases, plus a `--perturb` run that must
fail) and the mutation check. The CI-sized suite also runs inside `cargo test --workspace` on the
iPhone simulator, the Android emulator and wasm32-wasip1 (no threads, no clock, test data located
through `PROXIM_CORE_DIR`).

## Writing a scenario

A scenario is one JSON file. Unknown fields are rejected, so a typo fails loudly. Geometry is
in whole metres east (`e`) and north (`n`) of `origin`, and the loader converts it with the engine's
own frame. A trigger's origin follows the compiler rule (spec 1.2): the route anchor, the circle
centre, or the polygon's bounding-box centre.

```json
{
  "id": "expressway-inbound", "title": "...", "source": "PLAN M1; engine-spec 11",
  "origin": {"lat_e7": 450000000, "lon_e7": 70000000},
  "signposts": [{"id": "exit-cafe", "event": "approach",
                 "shape": {"kind": "route", "vertices_m": [[0, -5000], [0, 0]], "buffer_m": 30},
                 "trigger": {"lead_s": 40, "mode": "vehicle"}}],
  "trace": {"kind": "path", "start_m": [0, -6000], "moves": [{"to_m": [0, 500], "speed_mmps": 29000}]},
  "noise": {"profile": "open_sky"},
  "seeds": 20,
  "expect": {"signposts": [{"signpost": "exit-cafe", "fires": [1, 1],
                            "true_d_rem_m": [1000, 1500], "true_xt_max_m": 30, "t_hat_min_s": 30}]}
}
```

- **Signposts.** `id`, `event`, `shape` (`circle` `center_m radius_m`, `polygon` `vertices_m
  [inradius_m]`, `route` `vertices_m buffer_m [anchor_along_m]`), and optionally `class`,
  `category`, `sender`, `sensitivity`. `trigger` holds extra trigger fields and `extra` holds extra
  signpost fields; both are merged verbatim and parsed strictly by the engine's types. A
  `fixture: {path, sha256}` loads a verbatim signpost file. Its digest is pinned, so a changed
  byte stops the load.
- **Trace.** `path` (moves `{"to_m", "speed_mmps"}`, `{"wait_s"}`, `{"gap_s", "then_at_m"}`
  for time with no fixes, for example the app not running, optionally ending elsewhere), `gpx`
  (`file`, its own timestamps or `speed_mmps`), `geojson` (`file`, the first LineString, at
  `speed_mmps`). `period_ms` (default 1000), `start_ms` (default Monday 2026-10-12 08:00 UTC).
- **Noise.** `{"profile": "none|open_sky|suburban|urban", "set": {field overrides}}`.
  `dropouts`: `{from_s, to_s}` or `{along_from_m, along_to_m}` (true distance travelled: a tunnel).
- **Context.** `consent` (default: location granted), `consent_changes [{at_s, consent}]`,
  `policy` (engine JSON, default `{"explain": true}`), `standoff_zones_m [{id, center_m,
  standoff_m?}]`, `clock {tz_offset_min, lag_ms, events [{at_s, shift_ms, tz_offset_min}]}`, and
  `restarts_s` (process restarts: only `caps` survives).
- **Expectations per signpost.** `fires [min, max]` per run. These must hold at every fire:
  `true_d_rem_m [lo, hi]` and `true_xt_max_m` (routes), `true_depth_min_m` and `latency_s [lo, hi]`
  after the true entry (zones), `fire_time_s`, `t_hat_min_s` and `min_gap_s`. These must hold at
  least once per run: `reasons_include [codes]` (decisions or notes) and `max_transitions_per_10min`.
  `min_runs_ok_permille` (default 1000) is the share of runs that must meet all of these.
- **Global expectations.** `no_fire_within [{center_m, radius_m}]` (judged on the true position),
  `fix_reasons_include`, `mem_empty_after [codes]` (the state's memory must be empty after such a
  step: I4, I6) and `max_total_fires`.

YAML is what the plan names, but the scenario format is JSON. The serde YAML crates are either
archived (`serde_yaml`) or have a RustSec advisory history (`serde_yml`). The workspace already
parses JSON strictly, and the simulator adds no dependency at all.

## Generators and noise model

- **Synthetic legs.** Constant speed between waypoints, waits and gaps, all in integer mm and ms.
  The course comes from the engine's own SINCOS table (no `atan2`).
- **GPX / GeoJSON import.** Minimal scanners with no XML or GeoJSON crate. Decimal degrees are parsed
  from their text straight into e7 integers (exponents included), so no float is ever formed.
  GPX times must be UTC (`...Z`).
- **Noise ([MODEL] parameters, to be replaced by M4 field logs).** Position error per axis is
  AR(1), `x' = rho x + sqrt(1 - rho^2) sigma w`, with `w` from a 12-uniform Irwin-Hall integer
  Gaussian. Jumps (multipath) add an offset of random direction for a few fixes, and the reported
  accuracy does not show them. Reported accuracy is a mean plus a spread. Doppler speed and course
  carry noise, and course is omitted below 0.8 m/s. After a dropout, a few reacquisition fixes have
  poor accuracy and extra error.

  | Profile | sigma per axis | rho (1 s) | acc reported | Jumps | Speed / course noise |
  |---|---|---|---|---|---|
  | open_sky | 3 m | 0.90 | 5 m +- 1 m | 15-40 m, 1-3 fixes, 0.2% per fix | 0.3 m/s, 3 deg |
  | suburban | 6 m | 0.93 | 8 m +- 2 m | 20-60 m, 1-5 fixes, 1% per fix | 0.5 m/s, 6 deg |
  | urban | 12 m | 0.95 | 15 m +- 5 m | 30-120 m, 2-10 fixes, 3% per fix | 1 m/s, 15 deg |

- **Determinism.** Randomness comes from xorshift64* seeded through SplitMix64 from the seed index
  (1..=N). Every random draw is made for every sample, so a dropout does not shift the noise after
  it. There is no clock, no thread and no float anywhere. The report is canonical JSON
  (`proxim_types::canon`) and includes a SHA-256 over every engine output of every run, so any
  divergence across OSes or builds shows as a byte difference. The debug test build and the release
  CLI produce the same bytes.

## The oracle

Engine outputs never carry a position (I2), so the oracle keeps the truth sample of every fix.
For each fire it computes:
- **Routes.** The true remaining distance along the authored route and the true cross-track at
  the fire.
- **Zones.** The true signed depth, and the latency since the latest true entry.

Each run is judged separately, and the scenario passes when the required share of runs passes.
It also counts the detector's inside/outside transitions (from the `in` state in `mem.det`) and
the naive transitions (the reported centre crossing the edge), so flapping suppression is
measured, not assumed.

**Every assertion can fail.** `tests/scenarios.rs` wraps the real engine in deliberately broken
engines, and each must turn a known scenario red with the expected message:
- one that never fires (must-fire scenarios);
- one that fires everywhere (negative scenarios and the hospital no-fire radius);
- one that fires twice;
- one that fires at the first fix, which breaks the window and false-enter checks;
- one that fires 20 s late, which breaks the latency check;
- one that leaves a breadcrumb in `mem` inside a standoff;
- one that flaps the detector;
- one that hides a reason code.

A changed byte in the cross fixture pin and a misspelt expectation field are both rejected at load.

## Scenarios S01-S06 and S11

ADDENDUM-01 M1 names S01-S06 and S11, but no document defines them (ADDENDUM-01 open items,
PLAN-REVIEW). The definitions below are this simulator's. Each tests a claim of ADDENDUM-01 4.1/4.2
or engine-spec 6/11 that the other named scenarios do not.

| Id | Definition |
|---|---|
| S01 | Drive-in ENTER at 15 m/s into a 150 m circle: one fire, truly inside, latency 1-8 s (4.2: "about 3-5 s after the boundary") |
| S02 | Walk-in ENTER at 1.4 m/s: the 10 s dwell for slow movers, one fire, latency 5-30 s |
| S03 | Drive-through: one fire, EXIT hysteresis prevents a second on the way out |
| S04 | U-turn 1.5 km before the anchor (before D_fire = 645 m at 15 m/s): no fire |
| S05 | Late join from a side road 400 m before the anchor at 29 m/s: no fire, `TOO_LATE_FOR_LEAD` |
| S06 | Stop-and-go at 3 m/s with 8 s stops: one fire near the crawl distance (spec 11: about 172 m) |
| S11 | Three commutes in one day: exactly two fires (cap 2/day), `CAP_SIGNPOST_DAY` on the third; S11b repeats it with a +14 h time-zone change before the third (errata E-D-TZ) |

## Results (1,000 seeds per noisy scenario, release, 2026-10-16)

All 33 scenarios pass in every run. "Measured" values are from the truth at the fire, given as
min-max over all runs. The CI-sized numbers are in the checked-in report.

| Scenario | Signpost | Expected fires | Runs OK | Fires | Measured (truth at fire) |
|---|---|---|---|---|---|
| expressway-inbound | exit-cafe | 1 | 1000/1000 | 1000 | d_rem 1186-1215 m; t_hat 41-42 s |
| expressway-inbound-suburban | exit-cafe | 1 | 1000/1000 | 1000 | d_rem 983-1273 m; t_hat 34-43 s |
| expressway-outbound | outbound-diner | 1 | 1000/1000 | 1000 | d_rem 1186-1215 m; t_hat 41-42 s |
| gpx-expressway-inbound | exit-cafe | 1 | 1000/1000 | 1000 | d_rem 1186-1215 m; t_hat 41-42 s |
| opposite-carriageway (25 m away, inside the buffer) | exit-cafe | 0 | 1000/1000 | 0 | `BEARING_MISMATCH` |
| overpass | exit-cafe | 0 | 1000/1000 | 0 | |
| angled-crossing (40 deg) | exit-cafe | 0 | 1000/1000 | 0 | `BEARING_MISMATCH` |
| parallel-frontage-road (80 m) | exit-cafe | 0 | 1000/1000 | 0 | `OFF_CORRIDOR` |
| walking | shop-walk | 1 | 1000/1000 | 1000 | d_rem 89-115 m; t_hat 46-87 s |
| geojson-walk-corners | shop-walk | 1 | 1000/1000 | 1000 | d_rem 87-108 m |
| tunnel (40 s, before the fire point) | exit-cafe | 1 | 1000/1000 | 1000 | d_rem 1186-1215 m |
| tunnel-over-fire-point | exit-cafe | 0..1 | 1000/1000 | 997 | d_rem 403-548 m; t_hat 14-20 s (3 late joins, no fire) |
| health-standoff | hosp-approach | 1 | 1000/1000 | 1000 | d_rem 875-1050 m (3.6-3.8 km from the hospital); t_hat 35-42 s |
| health-standoff | cafe-near-hospital | 0 | 1000/1000 | 0 | `SUPPRESSED` seen, `mem` empty after every suppressed fix, no fire within 1.1 km |
| health-no-receipt | hosp-approach | 0 | 1000/1000 | 0 | `HEALTH_CONSENT_MISSING` |
| s01-drive-in-enter | cafe | 1 | 1000/1000 | 1000 | latency 2-6 s; depth 30-90 m |
| s02-walk-in-enter | cafe | 1 | 1000/1000 | 1000 | latency 5-28 s; depth 8-40 m |
| s03-drive-through | cafe | 1 | 1000/1000 | 1000 | latency 1-5 s |
| s04-u-turn | exit-cafe | 0 | 1000/1000 | 0 | |
| s05-late-join | exit-cafe | 0 | 1000/1000 | 0 | `TOO_LATE_FOR_LEAD` |
| s06-stop-and-go | exit-cafe | 1 | 1000/1000 | 1000 | d_rem 140-167 m |
| s11-cap-exhaustion | exit-cafe | 2 | 1000/1000 | 2000 | d_rem 1016-1219 m; `CAP_SIGNPOST_DAY` |
| s11b-cap-timezone-change | exit-cafe | 2 | 1000/1000 | 2000 | as S11 |
| boundary-jitter | cafe | 0 | 1000/1000 | 0 | detector transitions 0 (naive edge crossings up to 133) |
| drive-by (50 m) | cafe | 0 | 1000/1000 | 0 | |
| drive-by-tangent (0 m) | cafe | 0 (>= 97.2% of runs) | 1000/1000 | 0 | 0 false enters (model allows 2.8%) |
| stationary-outside (25 m, 10 min) | cafe | 0 | 1000/1000 | 0 | detector transitions 0 (naive up to 4) |
| flap (12 m outside, suburban) | cafe | 0 | 1000/1000 | 0 | detector transitions 0 (naive mean 12.8, max 42 per 10 min) |
| re-arm | cafe | 2 | 1000/1000 | 2000 | return inside the 30 min cool-down quiet; return after it fires |
| signed-cross-fixture | gate-cross | 0 | 1000/1000 | 0 | `UNSUPPORTED_EVENT` |
| permission-downgrade | exit-cafe | 1 (after re-grant) | 1000/1000 | 1000 | `CONSENT_DENIED` with empty `mem`; fire 6 s after re-grant |
| clock-set-back | cafe | 1 | 1000/1000 | 1000 | second visit refused as `FIX_FROM_FUTURE` |
| restart-after-fire | exit-cafe | 1 | 1/1 (no noise) | 1 | re-armed after the restart, held by `CAP_APP_INTERVAL` |
| overlapping-signposts (11 nested circles) | ov-400 / others | 1 / 0 | 1000/1000 | 1000 / 0 | one fire in total; the rest `CAP_APP_INTERVAL` |

**Stationary statistic (ADDENDUM-01 M1 verify, [MODEL] target).** A phone stood 25 m outside a
150 m circle for 10 minutes in open sky, over 1,000 seeds. It produced **0 false enters** and **at
most 0 detector edge transitions per 10 minutes** (target: at most 4). The reported centre crossed
the edge up to 4 times (mean 0.31). Under suburban multipath 12 m outside (`flap`), the naive edge
test flips a mean of 12.8 and up to 42 times per 10 minutes, and the detector flips 0 times.

**Observations.**
- **Health fire window.** In a first 1,000-seed run, one health approach (seed 540) fired at
  875 m instead of the usual 1,025-1,050 m (t_hat 35 s), outside the 900 m bound I had first
  written. Health needs confidence 700 (errata E-D-HEALTH), so one noisy fix can delay arming by a
  few fixes. The spec promises a lead of at least L_min, not a fixed distance, so the expectation is
  now `d_rem 400-1500 m` plus `t_hat >= 16 s`. The safety property (no fire within the standoff)
  held in every run.
- **tunnel-over-fire-point.** Fires with t_hat down to 14 s. L_min (16 s) applies to the arming
  fix (spec 6.2), and the fire comes two confirming fixes later. The scenario asserts at least 10 s,
  as the conformance suite does.

## turf.js comparison

`proxim-sim geo-cases` writes random cases with Proxim's answers: distance, point-in-polygon and
point-to-polyline projection (distance and along). Cases are integer e7, and each is evaluated in
the frame of a random origin at latitudes -70..70, with geometry within 1, 5 or 20 km of the origin.
`tools/turf-compare/compare.mjs` checks them against `@turf/turf` 7.4.0 (pinned exactly, with a
lockfile) and against an exact WGS84 ECEF tangent-plane reference.

**Tolerances**, derived from the spec's error sources and not fitted to the data:
- **Rounding.** 25 mm absolute. Each e7 coordinate is within 5.6 mm of its point, plus the frame
  projection's 1 mm rounding and the isqrt floor.
- **Frame.** Proxim scales by the frame origin's latitude (spec 1.2), so a point r from the origin
  is off by a relative `tan|lat| r / R` plus `r / R` curvature, plus the 0.01 degree grid (E-G10):
  `TOL_FRAME(d) = 25 mm + d (1.25 tan|lat| r/R + 2 r/R + 1e-4 (1 + tan|lat|))`.
- **Sphere.** turf uses a sphere, and WGS84's radii of curvature differ from it by up to 0.57%:
  `TOL_TURF(d) = TOL_FRAME(d) + 0.006 d`.
- **Segments.** A segment that is straight in e7 bows from the tangent-plane chord by about
  `L^2 (1 + tan|lat|) / (8 R)`, which is added to the line checks. For along, the sphere's shear
  adds `0.0067 dist`, and along is compared only when the nearest segment is unambiguous.
- **Point-in-polygon.** The e7 to ENU map is affine per frame, so turf and Proxim must agree
  except within 25 mm of an edge. Proxim's `boundary` matches either answer.

**Results, 1,000,000 cases (seed 1), 0 failures.** "Worst" is the largest error as a share of its
tolerance.

| Check | Cases | Max abs error | Worst (of tolerance) | Max abs error within 5 km |
|---|---|---|---|---|
| distance vs WGS84 ENU | 333,334 | 149 m (20 km, lat 70) | 0.45 | 9.2 m (r 1 km: 0.57 m) |
| distance vs turf | 333,334 | 253 m | 0.87 | 51.5 m |
| point-in-polygon vs turf | 333,333 | 26 disagreements, all within 25 mm of an edge | n/a | n/a |
| point-to-line distance vs WGS84 ENU | 333,333 | 110 m | 0.74 | 6.3 m (r 1 km: 0.44 m) |
| point-to-line distance vs turf | 333,333 | 163 m | 0.87 | 37.8 m |
| along vs WGS84 ENU (unambiguous, 199,919) | 199,919 | 88 m | 0.36 | 10.8 m |
| along vs turf | 199,919 | 230 m | 0.80 | 51.7 m |

The large absolute errors are the frame's documented scale behaviour far from the origin. The
engine evaluates a signpost within its reach, a few km, where the errors are metres or less. The
`--perturb` mode (distances x 1.02, point-in-polygon answers flipped more than 1 m from an edge)
must and does fail.

## Mutation check

`tools/mutation-check.py` copies `core/` to a temporary directory and applies one exact text change
to the engine in the copy. It then runs every scenario with the CLI and requires a red run that
includes the expected scenarios. The working tree is hashed before and after and never changes.
Results:

| Mutation | Expected red | Result |
|---|---|---|
| ENTER without N_in persistence (fire on the 2nd inside fix) | flap | killed: boundary-jitter, flap, s02-walk-in-enter, s03-drive-through, stationary-outside |
| OFF_CORRIDOR gate disabled | parallel-frontage-road | killed: parallel-frontage-road |
| bearing check flipped | expressway-inbound, walking | killed: 15 scenarios |
| bearing gate removed | angled-crossing | killed: angled-crossing, opposite-carriageway |
| F10 standoff suppression disabled | health-standoff | killed: health-standoff |
| day cap off by one | s11-cap-exhaustion | killed: s11-cap-exhaustion, s11b-cap-timezone-change |

The first version of the script copied files with their original mtimes, and Cargo then reused the
previous mutation's objects from the shared target directory, so one mutation reported another's
failures. The copy now gives every file a fresh mtime, and the table above comes from the fixed
script.

## Not covered (needs a device or a later milestone)

- **OS behaviour.** Background wake latency, geofence registration, reboot and force-quit, and
  Low Power mode (PLAN M4 SimulatedOS, M11 field drives). The noise profiles are [MODEL] values
  until M4 field logs replace them.
- **Real sensors.** Real GNSS receivers, urban canyons, the Doppler residual inflation (kappa,
  reserved), and how often a device delivers fixes faster than 3 Hz (errata E-P12: CLOSING needs
  4 s of trace).
- **Pack signing.** "Signed" for the cross fixture currently means pinned by SHA-256 in the
  scenario. JWS ES256 pack signatures are M2 (`proxim-pack` is a stub), so a pack-level signed
  fixture belongs with M2.
- **Real routes.** No real trace exists anywhere in the fixtures (I2): every route is synthetic
  and generated.
- **SDK replay.** The SDK replay (Web Playwright, Android, iOS `simctl` playback) starts at M6, M8
  and M10. This simulator drives the Rust engine directly.
