# Proxim engine specification (normative, spec version 1)

Status: M0/M1, 2026-10-11. Ports (Kotlin/Swift/TS) MUST match the golden vectors (section 9) byte for byte. This file wins over PLAN.md and the addenda on engine behaviour. [P] = provisional default: data in `proxim-types::defaults`, never hard-coded in logic. Sources: PLAN 5.5, 5.6, 6.1; ADDENDUM-01 2.2, 4.1; PLAYBOOK 4.

## 0. Scope

Crates `proxim-types`, `-geo`, `-path`, `-detect`, `-decide`: pure integer functions for foreground and partner-fed fixes (PLAN 5.3 M1/M2), hosted ads, one fix per call; no network, clock, randomness, file or OS access (I1, E3). Built in M1: `enter`, `approach`, internal exit hysteresis, path tiers P0/P1, consent fail-closed, per-signpost caps, minimal standoff, one-slot selection. Everything else is reserved (section 10).

## 1. Numeric model

### 1.1 Units (every field name carries its unit)

`Position {lat_e7, lon_e7}`: `i32` degrees x 1e7 (about 11 mm). `Enu {e_mm, n_mm}`: `i32` mm east/north of a frame origin. `_ms`: `i64` ms since Unix epoch UTC (`fix.t_ms` and `clock.now_ms` share one timeline). `_mmps`: `u32` (invalid speed = absent, never -1). `_ddeg`: `u16`, 0.1 degree clockwise from true north, 0..3599. `acc_mm`: 68% horizontal radius. `_permille`/`_milli`: 1000 = 1.0. Unit vectors: `[i32;2]` Q30 (east, north). Authored `_m`/`_s` fields are multiplied by 1000 at load. No engine type, input, output or golden file contains a float; shells convert with one binary64 multiply-add and floor (`lat_e7 = floor(deg*1e7 + 0.5)`). e7 is the wire form; geometry is evaluated in integer ENU mm (ADR-001); the trace is kept in e7 and re-projected per signpost frame.

### 1.2 Frame, projection, origin rule

`Frame {origin, kn_q32, ke_q32}`. The origin is never derived from the fix: it is the required `trigger.origin` (compiler rule: route anchor, circle centre, polygon bounding-box centre), so compiler and engines agree by construction.
- `lat_q = div_round(origin.lat_e7, 100000)` (0.01 degree grid; |lat| > 80 degrees -> `E_UNSUPPORTED_LATITUDE`).
- `(kn_q32, ke_q32) = SCALE[lat_q]`: mm per e7 unit in Q32, WGS84 at phi = lat_q x 0.01 degree: `kn = M(phi) pi/180 1e-4`, `ke = N(phi) cos(phi) pi/180 1e-4`, computed once per `lat_q` with `libm` in `proxim-geo::trig`, rounded. Normative table: `golden/trig/frame_scale_table.json`.
- `project(p)`: `dlon = wrap(p.lon - o.lon)` into [-1.8e9, 1.8e9) (`i64`); `e_mm = rsr(dlon*ke_q32, 32)`, `n_mm = rsr(dlat*kn_q32, 32)`, `rsr(x,s) = (x + 2^(s-1)) >> s` (arithmetic shift) in `i128`. `|e_mm|` or `|n_mm|` > 150_000_000 -> `E_OUT_OF_FRAME`.

Vertices are e7, projected at prepare time (memoizing is allowed). A fix farther than `trigger.reach_m` (optional; else geometry extent + `d_hi`/buffer + 100 m) from the origin is not a candidate.

### 1.3 Integer primitives

`div_round(a,b)` (b>0) = `floor((2a+b)/(2b))`; `isqrt(n:u128)` = exact floor sqrt; `dist = isqrt(dx^2+dy^2)`. Point-to-segment: clamp the dot-product parameter; perpendicular case `isqrt(cross^2 / len2)` (u128, floor). Within the 150 km frame every product is below 7.2e17 (`i64`) and every squared cross below 5.2e35 (`u128`). Arithmetic is checked: overflow -> `E_OVERFLOW`; no wrap, saturation, panic or unchecked indexing.

### 1.4 Accuracy and the straddle rule

`acc_eff = max(acc_mm, 10_000) x kappa_permille / 1000` (kappa 1000 [P]; inflation on Doppler residuals reserved). For signed depth `d` (inside positive) and factor `k`: `Inside` iff `1000 d >= k acc_eff`; `Outside` iff `1000 d <= -k acc_eff`; else `Straddle`. ENTER needs `Inside` (a fix whose accuracy circle straddles the boundary never counts; a straddle resets the candidate count); EXIT needs `Outside`. The one stated exception is corridor membership: it tests the centre point (`|xt| <= buffer`) because 65 m accuracy exceeds a 30 m buffer, compensated by 3 consecutive fixes, direction and closing gates and the 65 m cap (parallel roads within buffer + accuracy can fire, section 11).

## 2. Determinism and float confinement

1. Integer or fixed-point everywhere. Floats exist only in `proxim-geo::trig` and `::h3`, each with an explicit `#[allow(clippy::float_arithmetic)]`; elsewhere `float_arithmetic` and `float_cmp` are denied.
2. `trig` calls `libm` (sin, cos, sqrt) only on QUANTIZED finite domains and converts to integers at once: `SCALE[lat_q]` (16001 entries) and `SINCOS[course_ddeg]` (3600 entries, Q30, round-half-away; `cos(tol)` = `SINCOS[tol].cos`, tol <= 900). These are the only transcendental results in the engine; ports verify them exhaustively against golden digests or embed them. `atan2`, `exp`, `pow`, `ln`, `tan` are never used.
3. `h3` converts e7 to f64 degrees by one IEEE division for `h3o` 0.11.0 (current on crates.io 2026-10-11; `default-features = false` routes its trig through `libm`); only integer cell indices leave.
4. `core/clippy.toml` `disallowed-methods` lists std f64 `sin cos tan asin acos atan atan2 sinh cosh tanh exp exp2 exp_m1 ln ln_1p log log2 log10 powf powi cbrt hypot mul_add` (E1). The five crates are `#![no_std]` + `alloc`, so those methods, `HashMap`/`HashSet`, clocks, files and threads do not exist there (E2, E3); tests and tools may use `std`.
5. Signposts are processed and lists emitted in ascending byte order of `id` (ties `rev`); input order MUST NOT change output (`decide_ignores_input_order`).
6. Functions return `Result<_, EngineError>` (`E_PARSE E_OVERFLOW E_OUT_OF_FRAME E_UNSUPPORTED_LATITUDE E_DEGENERATE_GEOMETRY E_LIMIT`). A per-signpost defect is a drop reason, not an `Err`. A shell MUST treat any `Err` from `decide` as "no decision, state unchanged".

## 3. Crates, types, canonical JSON

### 3.1 Graph (deviation from PLAN 3, ADR-001)

`proxim-types` <- `proxim-geo` (+`h3o`, `libm`) <- `proxim-path` <- `proxim-detect` <- `proxim-decide`; `-pack`, `-events`, `-sched`, `-sim`, `-ffi`, `-wasm` depend on `proxim-types` (and decide), never the reverse. `proxim-types` holds wire types, `ReasonCode`, `EngineError`, `defaults` and the canonical writer. Limits: 4096 signposts, 1024 route / 256 polygon vertices, 256 standoff zones, trace 16, 4096 detector entries (excess -> `TRIGGER_INVALID` or `E_LIMIT`).

### 3.2 Canonical JSON (inputs, outputs, state, golden files)

UTF-8, no whitespace; keys ascending by UTF-8 bytes (via `serde_json::Value`, `BTreeMap`-backed: `preserve_order` is never enabled); integers only (no `+`, leading zeros, exponent or `-0`); no `null` (absent = omitted); array order is semantic; escape only `"`, `\` and controls < 0x20 as `\u00xx` lowercase; `Id` = ASCII `[A-Za-z0-9._:-]{1,64}`; enums lowercase snake_case, `ReasonCode` SCREAMING_SNAKE. Parsing is strict (unknown fields -> `E_PARSE`) except the reserved fields of section 10; any consent value other than exactly `"granted"` parses as `unknown` (I4).

### 3.3 Types (`?` optional, default in parentheses)

- `Fix {t_ms, lat_e7, lon_e7, acc_mm, speed_mmps?, speed_acc_mmps?, course_ddeg?, course_acc_ddeg?, is_mock?(false)}`. `Clock {now_ms, tz_offset_min:i16}`.
- `Consent {location_matching, nearby_health_events, aggregate_reporting, local_notifications: granted|denied|unknown (absent = unknown); health_receipt?}`.
- `Policy {simulator_mode?, child_directed?, explain?, categories_blocked?[], senders_blocked?[], sensitivity_ceiling?(2), lead_s_min?(15), lead_s_max?(120), cap_signpost_day?(2), cap_signpost_week?(5), app_min_interval_s?(1800), slots?(1), max_acc_mm?(100000), standoff_zones?[{id, center, standoff_m(1000)}]}`. Partners only narrow: values clamp to platform ceilings (caps 2/5, slots 1, accuracy 100000, lead 15..120).
- `Signpost {id, rev, class: general|health, category, sender, sensitivity:u8, event, valid_from_ms, valid_until_ms, min_path_tier?(0; approach implies 1), min_path_conf_milli?(500), rank_milli?(1000), trigger}`; `EventKind = enter|approach|exit|cross|dwell|pass|sequence` (`exit` internal, not authorable).
- `Trigger {origin, reach_m?, shape, max_event_age_s?, max_event_dist_m?}` and, for `approach`, `{lead_s(40), d_lo_m?, d_hi_m?(3000, max 5000), mode: vehicle|cycle|foot, speed_min_mmps?, speed_max_mmps?, bearing_tol_ddeg?(300, 150..450)}`. `shape` = `{kind:circle, center, radius_m >= 150}` | `{kind:polygon, vertices[3..256] (no closing repeat), inradius_m?}` | `{kind:route, vertices[2..1024], buffer_m 30..500, anchor_along_m?(route length)}`. `enter` needs circle|polygon, `approach` needs route.
- `Decision {signpost, rev, outcome: fire|drop, reason, event, t_ms, path_tier, path_conf_milli, copy: near_you|ahead|on_route, d_rem_mm?, t_hat_ms?}`.
- `Output {v:1, t_ms, fix:{ok, reason}, path:{tier, conf_milli, speed_mmps?, heading_valid}, decisions[], notes?[{signpost, reason}]}`. No position ever appears in an Output (I2, I6). Without `policy.explain`, `decisions` holds only `fire` entries (at most `slots`), `notes` is absent, and a standoff reads `fix:{ok:false, reason:"SUPPRESSED"}`, so logging reasons cannot reveal proximity to a health facility. With `explain` (tests, simulator) drops and notes are listed.
- `State {v:1, caps, mem}`. Only `caps` MAY be persisted (encrypted, no backup, I3); `mem` MUST NOT be persisted, logged or sent; a new process starts from `State::empty()`.
  - `caps {hwm_ms, last_fire_ms?, signposts[{id, day_idx, day_n, week_idx, week_n}] by id}`; entries from earlier weeks are pruned each call.
  - `mem {last_t_ms?, jump_rejects, trace[{t_ms, lat_e7, lon_e7, acc_mm, v_mmps?}] (<=16, oldest first), vf_mmps?, h?[x,y] Q30, h_t_ms?, det[{id, rev, st}] by id}`. Idle detectors are absent; a changed `rev` replaces the entry; entries absent from the input and unseen for 900 s are pruned. `st` (tag `s`): `cand{n,first_t,last_t}` | `in{xn,x_first_t?,last_t}` | `cool{until_ms}` | `armed{confirm_t?,last_t}` | `latched{why: fired|late, last_t}`.

## 4. proxim-geo

| Function | Semantics |
|---|---|
| `frame_for(Position)`, `Frame::project` | 1.2 (an inverse `unproject` exists for tools only) |
| `orient2d(a,b,c)->i8`, `dist_mm`, `dist_point_segment_mm` | exact cross-product sign; distances per 1.3 |
| `point_in_polygon(poly,p)->Inside/Boundary/Outside` | exact even-odd, half-open crossing rule; `Boundary` (on an edge) tested first; holes reserved |
| `signed_depth_polygon_mm`, `signed_depth_circle_mm` -> `i64`, `classify(depth, acc_eff, k_permille)` | +distance to the nearest edge inside, - outside, 0 on boundary; circle `r - dist`; `classify` per 1.4 |
| `Polyline::new(v)` | rejects < 2 vertices and zero-length segments (`E_DEGENERATE_GEOMETRY`); `seg_len = isqrt(len2)`, `cum` = their prefix sums (the route length IS the sum); Q30 tangents |
| `project_to_polyline(pl,p,window?)->{seg, xt_mm, along_mm, dist_mm, tangent}` | nearest segment by squared distance (only segments overlapping the along window if given), ties lowest `seg`; `xt` signed (left of travel positive); `along = cum[seg] + clamp(floor(dot x seg_len / len2), 0, seg_len)` |
| `rem_along_mm(anchor, along)`, `unit_from_course`, `cos_tol_q30`, `dot_q30`, `normalize_q30(x,y)->Option` | `anchor - along` (negative once passed); tables per 2.2; `normalize`: `n=isqrt(x^2+y^2)`, `div_round(x<<30, n)`, `None` if `n=0` |
| `h3::cell_of(p,res)->u64`, `cell_center`, `cell_boundary`, `dist_to_cell_edge_mm(p,cell)` | res 0..=15; a cell leaving the device MUST be res <= 5 (I2); SDK prefetches neighbours only within 3 km of an edge |
| `h3::grid_disk(cell,k)`, `cover_circle(center,r_mm,res)` | sorted ascending `Vec<u64>`; `cover_circle` = cells meeting the circle (compact regional set) |

`cross_segment` is reserved. Frame scale error: about 0.1% within 5 km of the origin at latitude 45 (0.25% at 70), growing linearly.

## 5. proxim-path

`path_update(mem, fix) -> (mem', PathSummary)`, accepted fixes only.

### 5.1 Speed filter (exact)

`v_meas`: `speed_mmps` if present and (`speed_acc_mmps` absent or <= 5000); else `dist x 1000 / dt_ms` (floor) to the newest trace entry aged 3000..10000 ms; else none. Rational EWMA, tau 8000 ms (no `exp`): `alpha_q16 = dt x 65536/(8000+dt)`, `dt = clamp(fix.t - last_t, 0, 60000)`; `vf' = vf + rsr((v_meas - vf) x alpha_q16, 16)`. Seed `vf' = v_meas` when `vf` is absent or `dt > 5000` (after a gap: the median of the last up to 3 `v_meas` values, `sorted[(n-1)/2]`). Without `v_meas`, `vf` is unchanged and hidden after 10 s. `v_eff = max(vf, floor)`, floors vehicle 4000, cycle 2000, foot 700.

### 5.2 Heading filter (exact)

Measurement `m`: the OS course vector if `course_ddeg` is present, its accuracy is absent or <= 450 and `vf >= 800`; else the unit displacement to the newest trace entry aged 3000..15000 ms if that is >= max(15000, 3 acc_eff) mm; else none. Tau 4000 ms: `a = dt x 65536/(4000+dt)`, `h' = normalize(rsr(h x (65536-a) + m x a, 16))`; reseed `h' = m` if `h` is absent, `dt > 5000` or the blend is zero. `heading_valid` iff updated within 10 s.

### 5.3 Tiers, confidence, copy ladder

- P0 presence: confident `Inside` (ENTER rule); conf 500; copy `near_you`.
- P1 corridor: `acc_eff <= 65000` on the last 3 fixes; >= 3 fixes within 60 s; `vf` >= mode minimum (vehicle 2500, cycle 1500, foot 500); heading valid and `dot(h, tangent) >= cos 45 deg`; `abs(xt) <= buffer` on the last 3 fixes; CLOSING. `conf = 600 +100 [>=5 fixes in 60 s pass] +100 [dot >= cos 15 deg] -100 [max acc_eff of last 3 > 30000] -200 [> 50000]`, foot capped at 600 (range 400..800); copy `ahead`.
- P2 route-confirmed (RESERVED): conf 800-900; copy `on_route`. P3 swath (RESERVED): conf <= 500, widener only.

CLOSING over the last three fixes (along a0,a1,a2; `tol = max(acc_eff)/2`): `a2-a0 >= max(5000, tol)`, `a1-a0 >= -tol`, `a2-a1 >= -tol`. U-turn, opposite carriageway and a stopped vehicle fail CLOSING or the heading gate. Health's floor 700 is reachable by P1 only with accuracy <= 30 m and a bonus, never by P0.

### 5.4 Time to anchor

`d_rem = rem_along(anchor, along) - v_eff x age_ms / 1000` (dead reckoning, floor 0; `age_ms = now - fix.t` must be <= 10000 for approach else `FIX_STALE`). `t_hat_ms = d_rem x 1000 / v_eff` (floor; `v_eff >= 700`).

## 6. proxim-detect

`detect(st, sp, path, fix, now_ms, explain) -> (st', Option<Event>, Option<ReasonCode>)`; state is an explicit input and output. `Event {kind, t_ms, d_rem_mm?, t_hat_ms?, dist_to_shape_mm}`; the reason is a non-terminal "why not". Fire timing lives here because it defines when an event exists (ADR-001).

### 6.1 ENTER with EXIT hysteresis (circle, polygon)

`d` = signed depth in the shape's frame; `k_in = k_out = 1000` [P]. A shape whose inradius (circle radius; polygon `inradius_m` if given) is below `k_in x acc_eff` yields `ACC_TOO_COARSE`; circles need `acc_mm <= 50000`.
- Idle -> `cand{n=1}` on an `Inside` fix. In `cand`: `Inside` with gap <= 10000 ms increments `n`; anything else returns to idle (restarting at `n=1` if the new fix is `Inside`). ENTER fires at `n >= 3` and `last_t - first_t >= 3000` ms (10000 if `vf < 1500`) -> `in{xn=0}`.
- `in`: an `Outside` fix increments `xn` (the first sets `x_first_t`); any other fix zeroes it. Exit confirms at `xn >= 5` or (`xn >= 2` and `last_t - x_first_t >= 10000`) -> `cool{until = last_t + 1_800_000}`. EXIT is internal: no Event, nothing delivered or persisted. State survives gaps up to 120 s; a gap > 900 s becomes `cool{until = prev_last_t + 1_800_000}`. `cool` ignores fixes until `fix.t >= until`, then idle.

### 6.2 APPROACH (route)

`proj = project_to_polyline(route, fix, window)` in the trigger's frame; `window = [a_prev - 100 m, a_prev + 500 m + v_eff x 30 s]` once armed (blocks parallel-leg and loop confusion), else the whole route; `anchor = anchor_along_m x 1000`. Parameters: `lead_s` clamped by policy; `d_lo` (vehicle 150 m; foot/cycle `max(100 m, 3 acc_eff)`); `d_pass = max(60 m, 5 s x v_eff)`; `L_min = max(0.4 lead_s, 10 s)`; `t_comp = 3000 ms`; speed band from the trigger (vehicle >= 2500 mmps, free-flowing >= 8300, foot 500..2500) with 1000 mmps hysteresis once armed.

Gates, in order (first failure -> idle, reason noted): `FIX_STALE` (age > 10 s), `FIX_ACCURACY_TOO_COARSE` (`acc_eff > 65000`), `OFF_CORRIDOR` (`abs(xt) > buffer`, last 3 fixes), `SPEED_OUT_OF_BAND`, `BEARING_MISMATCH` (`dot(h, tangent) < cos(bearing_tol)`; invalid heading fails), `NOT_CLOSING`, `BEYOND_WINDOW` (`d_rem > d_hi` or `d_rem < d_pass`), `PATH_TIER_LOW`/`PATH_CONFIDENCE_LOW`.

idle -> `armed` on the first fix where all gates hold. At that fix, if `t_hat < L_min` and `d_rem > d_lo`: `latched{late}` and the terminal drop `TOO_LATE_FOR_LEAD` (late join). In `armed`: FIRE when `t_hat <= lead_s x 1000 + t_comp` or `d_rem <= d_lo` (crawl rule) has held on two accepted fixes >= 1000 ms apart (`confirm_t` = first confirming fix; a non-confirming armed fix or a gap > 5000 ms clears it). Equivalent: `D_fire = clamp(v_eff x (lead_s + 3 s), d_lo, d_hi)`. FIRE -> `latched{fired}`: one fire per pass. Re-arm (-> idle) only when the fix is farther than `1.2 x d_hi` (Euclidean) from the anchor point, or on a new `rev`; passing the anchor does not re-arm. A fix beyond `reach_m` returns armed to idle.

## 7. proxim-decide

`decide(state, fix, signposts, policy, consent, clock, seed) -> Result<(Output, State)>` is a function of its arguments only. `seed` is carried but UNUSED in M1 (a test asserts it).

`now = max(clock.now_ms, caps.hwm_ms)` (a clock set backwards cannot reopen caps), stored as `caps.hwm_ms`. `day_idx = floor_div(now + tz_offset_min x 60000, 86_400_000)`; `week_idx = floor_div(day_idx + 3, 7)` (Monday start). Hard filters in order, one reason code each:

Global (first failure ends the call):
- F01 `CONSENT_DENIED`: `location_matching != granted` -> `mem := empty`, nothing processed or retained (I4); `caps` kept. F02 `ENGINE_DISABLED`: `child_directed` -> `mem := empty` (I9).
- F03 `FIX_INVALID` (range), F04 `FIX_MOCK` (`is_mock`, not `simulator_mode`), F05 `FIX_ACCURACY_TOO_COARSE` (> `max_acc_mm`), F06 `FIX_FROM_FUTURE` (`t > now + 5000`), F07 `FIX_STALE` (`now - t > 30000`), F08 `FIX_OUT_OF_ORDER` (`t <= mem.last_t_ms`).
- F09 `FIX_JUMP`: against the newest trace fix with `dt <= 60000`: implied speed > 83000 mmps and distance > both accuracies + 100000 mm. Reject and count; the 3rd consecutive reject resets trace and detectors and accepts the fix.
- F10 `SENSITIVE_STANDOFF`: some `standoff_zones` entry with `dist(fix, center) <= standoff + acc_eff`. M1 minimal R1/R2: ALL classes are suppressed (stricter than R2's health-only text; relaxing is an M3 decision), `mem := empty` (no breadcrumb), `caps` untouched, call ends (I6).

Then `path_update`. Then per signpost in ascending `id`:
- Static (failure -> drop record, detector entry removed): F11 `UNSUPPORTED_EVENT` (E5); F12 `TRIGGER_INVALID` (shape/event mismatch, limits, radius < 150 m, buffer/lead/`d_hi` out of range, bad validity window, geometry outside the frame or degenerate); F13 `NOT_YET_LIVE`; F14 `EXPIRED`; F15 `HEALTH_CONSENT_MISSING` (`class=health` without `nearby_health_events=granted` and `health_receipt`, I12); F16 `PARTNER_CATEGORY_BLOCKED`; F17 `SENDER_BLOCKED`; F18 `SENSITIVITY_ABOVE_CEILING`; F19 `TRIGGER_IN_STANDOFF` (health geometry within a zone's standoff; defence in depth for R1).
- Detect (section 6) with the policy-narrowed trigger; quick reject (idle and beyond `reach_m`) records nothing. Gate reasons are non-terminal notes; `TOO_LATE_FOR_LEAD` is terminal.
- Event filters (a dropped Event is consumed, not retried): F20 `EVENT_TOO_OLD` (`now - event.t > max_event_age_s`; default enter 30 s, approach 10 s); F21 `EVENT_TOO_FAR` (`dist_to_shape > max_event_dist_m`, default 5000 m; zone: distance outside, route: cross-track); vacuous for a fresh foreground fix, they bind for late events (M3/M4) and are tested with an aged clock. F22 `CAP_SIGNPOST_DAY` (`day_n >= cap`), F23 `CAP_SIGNPOST_WEEK`, F24 `CAP_APP_INTERVAL` (`now - last_fire_ms < app_min_interval_s x 1000`).
- Selection: survivors ordered by `(score desc, t_hat_ms asc, absent last, id asc)`, `score = rank_milli x path_conf_milli / 1000` (u64). The first `slots` fire; the rest get F25 `LOST_SLOT` and revert to their pre-event detector state (they may fire at a later fix, subject to F24).
- Commit: winners get `day_n += 1`, `week_n += 1` (a new index resets the count), `last_fire_ms = now`, detector latched; nothing else is written. Decisions are sorted fire first, then `id`.

## 8. Reason codes

Stable strings and `repr(u16)` ids, never reused, consecutive from each row's first id. I = emitted in M1; R = reserved.

| First id | Codes |
|---|---|
| 0 (I) | OK, SUPPRESSED (output mask of SENSITIVE_STANDOFF) |
| 100 (I) | CONSENT_DENIED, ENGINE_DISABLED |
| 110 (I) | FIX_INVALID, FIX_MOCK, FIX_ACCURACY_TOO_COARSE, FIX_FROM_FUTURE, FIX_STALE, FIX_OUT_OF_ORDER, FIX_JUMP, FIX_OUT_OF_FRAME, ACC_TOO_COARSE |
| 120 (I) | SENSITIVE_STANDOFF |
| 200 (I) | UNSUPPORTED_EVENT, TRIGGER_INVALID, NOT_YET_LIVE, EXPIRED, HEALTH_CONSENT_MISSING, PARTNER_CATEGORY_BLOCKED, SENDER_BLOCKED, SENSITIVITY_ABOVE_CEILING, TRIGGER_IN_STANDOFF, OUT_OF_RANGE |
| 300 (I) | OFF_CORRIDOR, SPEED_OUT_OF_BAND, BEARING_MISMATCH, NOT_CLOSING, BEYOND_WINDOW, PATH_CONFIDENCE_LOW, TOO_LATE_FOR_LEAD, PATH_TIER_LOW |
| 400 (I) | EVENT_TOO_OLD, EVENT_TOO_FAR, CAP_SIGNPOST_DAY, CAP_SIGNPOST_WEEK, CAP_APP_INTERVAL, LOST_SLOT, NO_EVENT (explain only) |
| 500 (R) | Reserved, appended and never renumbered: pack verification (REVOKED, SIGNATURE_INVALID, SEQ_ROLLBACK, KID_REVOKED, PACK_EXPIRED; M2); the rest of PLAN 5.6 (CAP_SENDER_DAY, CAP_APP_DAY, COOLDOWN_DISMISSED, HIDDEN_BY_USER, SENDER_REPORTED, PACED_OUT, SURFACE_NOT_READY, DRIVING_UNSAFE_FORMAT, QUIET_HOURS, OUTSIDE_DAYPART, DESTINATION_SUPPRESSED, PROHIBITED_ANCHOR; M3); HOSTED_BACKGROUND_OFF, BACKGROUND_PERMISSION_MISSING, REDUCED_ACCURACY (M4/M11) |

## 9. Golden vectors

`core/testdata/golden/<kind>/<id>.json`, kinds `decide detect path geo h3 trig`; canonical JSON plus a trailing `\n`; synthetic data only (I2). Header: `{"format":"proxim-golden/1","kind","id","engine_spec":1,"synthetic":true}`.
- `decide`: `{header, seed, policy, consent, signposts[], steps[{clock, fix, expect:{output, state_sha256}}], trace_sha256}`. The runner starts at `State::empty()`, threads `state'` and compares the canonical bytes of `output` and the SHA-256 (lowercase hex) of the canonical bytes of `state'`; `trace_sha256` hashes `canon(output) + "\n" + state_sha256 + "\n"` over all steps.
- `geo path detect h3`: `{header, cases[{fn, args, result}]}` (integer args), including 1.3 rounding edges and every worked number here. `trig` tables: `{header, entries: every 100th row, sha256 of all rows as "idx,v1,v2\n"}`.
- Conformance entry (`proxim-ffi`, `proxim-wasm`): `decide_json(state, fix, signposts, policy, consent, clock, seed) -> {output, state}` in canonical JSON; CI `golden-vectors` needs identical bytes on every binding and OS.
- `MANIFEST.json` lists `{path, sha256}` per vector; regeneration needs `PROXIM_BLESS=1 proxim-sim golden --bless` and a reviewed diff; a byte change bumps `engine_spec`.

## 10. M1 built versus reserved

Reserved (types exist, fail closed; whatever section 0 omits): `exit` delivery, `cross`, `pass`, `dwell`, `sequence` (-> `UNSUPPORTED_EVENT`); polygon holes, gate, junction; P2, P3, kappa, FINE class; sender/app caps, dismiss/hide/report, V x Q x R x P scoring, pacing, slots > 1, driving-safe format, full R1-R6, pack and revocation checks, daypart/quiet hours, `late_policy=save`, background behaviour (M4, M11). Reserved input fields (accepted, ignored): Signpost `intent content_mode precision_class min_mode max_acc_m arm_scope relation selector live_at_ms fresh_required_s cooldown_s creatives rot_days`; Policy `rot_lane rot_lanes`. Rotation (ADDENDUM-02 5) is not built (when built: integer-only, I26).

## 11. Behaviour contract, false positives and negatives

`proxim-sim-qa` measures these on the fixture routes.
- Radius 150 m, `acc_eff` 10 m: ENTER at <= 140 m, EXIT at >= 160 m; jitter around the edge never ENTERs.
- Time-to-anchor (steady, 1 Hz, `lead_s` 40, `d_lo` 150 m): `D_fire = clamp(v_eff x 43 s, 150 m, 3000 m)`: 29 m/s -> 1247 m (realized lead about 41 s), 15 m/s -> 645 m, cycle 5 m/s -> 215 m, jam 3 m/s (floor 4) -> 172 m.
- Must not fire: U-turn, opposite carriageway, unknown consent, inside a standoff (`caps` byte-identical, `mem` empty), a first armed fix with `t_hat < L_min` (e.g. after a 3 min GPS drop), a 3rd fire in a local day, a 6th in a week.
- False positives: a parallel same-direction road at distance `D` when `D < buffer + acc_eff` (tracked metric; P2 is the remedy). False negatives: stop-and-go below the 4 m/s floor fires at about 172 m, not earlier; heading is invalid below 800 mmps; urban-canyon correct-road accuracy is not promised (ADDENDUM-01 4.4); a spoofed GPX is indistinguishable (attestation is server-side, M9); `d_rem` follows the authored polyline, not the road.

Change log: v1 (2026-10-11) initial; normative changes bump `engine_spec` and are recorded here.
