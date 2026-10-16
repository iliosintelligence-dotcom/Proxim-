# golden/decide

Golden vectors for `proxim-decide` (engine-spec 9, kind `decide`). One canonical-JSON file per
scenario plus a trailing newline: `{header, seed, policy, consent, signposts[], steps[{clock, fix,
expect:{output, state_sha256}}], trace_sha256}`. Synthetic data only (invariant I2): the origin is
latitude 45, longitude 7 and nothing here is a real place or a real person's trace.

Every binding must reproduce the `output` bytes and the SHA-256 of the canonical state after each
step. The runner starts at `State::empty()` and threads the state; `trace_sha256` hashes
`canon(output) + "\n" + state_sha256 + "\n"` over all steps.

| File | What it pins |
|---|---|
| `drive_in_enter` | drive into a circle: ENTER on the 4th inside fix |
| `approach_fire_on_route` | time-to-anchor fire at 15 m/s, one fire per pass |
| `drive_past_parallel_road` | a road 600 m aside: no event |
| `opposite_carriageway` | route axis driven the wrong way: BEARING_MISMATCH, never fires |
| `u_turn_before_anchor` | approach, then reverse: never fires |
| `standoff_suppressed` / `standoff_explained` | minimal standoff: generic SUPPRESSED (reason SENSITIVE_STANDOFF only with explain), mem wiped |
| `consent_denied` / `consent_unknown` / `child_directed_disabled` | fail-closed global gates |
| `cap_exhaustion` | 2 per local day (UTC+2), the third is CAP_SIGNPOST_DAY, next day fires |
| `clock_goes_backwards` | the persisted high-water mark clamps `now` |
| `malformed_fixes` | F03 to F09, one reason each, and the third-jump reset |
| `two_signposts_one_slot` | score wins, LOST_SLOT reverts the loser, CAP_APP_INTERVAL |
| `unsupported_events` | cross dwell exit pass sequence fail closed |
| `late_join_too_late` | first armed fix too close: TOO_LATE_FOR_LEAD |
| `health_gates` / `health_with_receipt` | health consent and TRIGGER_IN_STANDOFF |
| `boundary_jitter_never_enters` | accuracy circle straddling the edge never enters |
| `aged_clock_event_too_old` | F20 with a lagging clock |
| `superseded_and_ambiguous_revisions` | revision selection |
| `world_tour` | ENTER, EXIT and APPROACH over six signposts |

Regenerate (reviewed diff; a byte change in behaviour bumps `engine_spec`):

    PROXIM_BLESS=1 cargo run -p proxim-decide --example gen_golden

Without `PROXIM_BLESS` the example only compares and exits 1 on a difference. The test
`golden_regeneration_is_byte_identical_and_complete` does the same inside `cargo test`, and
`proxim-ffi` and `proxim-wasm` replay the files through their `decide_json`.
