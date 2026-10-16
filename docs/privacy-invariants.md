# Privacy invariants

Status: M0/M1 in progress, 2026-10-11. This file is machine-read by `tools/check-invariants.py`
(CI job `check-invariants`). Do not reformat the tables: keep the column headers and one row per line,
and do not put the pipe character inside a cell.

Sources: [PLAN.md](PLAN.md) 6.1 (I1-I12), [PLAN-ADDENDUM-01.md](PLAN-ADDENDUM-01.md) 2.4 and 10.1 (amendments,
I13-I20, I2a), [PLAN-ADDENDUM-02.md](PLAN-ADDENDUM-02.md) 9.2 (amendments, I21-I26, the loosened
billboard principle). Where an addendum changed a wording, the row carries the master (latest) wording and
names the change in the Kind column.

## How this file is checked

`python3 -I tools/check-invariants.py` fails the build when:

- a row has no status, no milestone or no test id;
- a row is `enforced` but one of its test ids does not resolve to a file and a test name that exist in the repo today;
- a row is `design-rule` but a path matching its Guards glob exists (a guarded feature was built without flipping the row and adding the test);
- an ID is duplicated, or one of I1-I12, I2a, I13-I26 has no row (an invariant with no test fails the build).

`planned` rows only warn and are listed at the end of the run. If every test of a `planned` row already
exists the tool tells you to flip it to `enforced`.

### Status

| Status | Meaning |
|---|---|
| enforced | A named test or tool exists in the repo today and CI runs it. Nothing else may carry this label. |
| planned | The invariant is binding now, but its test does not exist yet. The Test ID is the proposed name and location; the milestone says when it lands. Until then it is NOT enforced by automation, only by review. |
| design-rule | No guarded code path may exist yet (parked owner mode and later stages). The Guards column lists globs that must match nothing. Building the feature requires the named test and a status flip in the same change. |

### Kind (relation to PLAN.md)

`base` = PLAN 6.1 wording unchanged; `scoped`, `clarified`, `tightened` = addendum rewording that does not weaken it;
`loosened` = an addendum weakened or widened it (labelled honestly, accepted by the owner on 2026-10-11, see
[DECISIONS.md](DECISIONS.md)); `new` = added by an addendum; `engineering` and `repo-guard` = rules that back the
invariants but are not themselves user-privacy promises.

### Test ID grammar

`path/to/file::test_name` (the file must exist and contain the name as a whole word; for Rust module paths only the
last segment is searched) or `tool:NAME` (`tools/NAME.py` exists and `tools/tests/test_tools.py` has a self-test that
mentions it). Several ids are separated by a semicolon. Names marked `planned` are proposals: the agent that writes the
test may rename it by editing this row in the same change.

## A. Privacy invariants

| ID | Kind | Status | Milestone | Test ID | Guards | Source | Wording |
|---|---|---|---|---|---|---|---|
| I1 | base | enforced | M0 | `tool:check-no-network`; `tools/tests/test_tools.py::test_check_no_network_rejects_transitive_denylisted_crate_and_prints_path` | - | PLAN 6.1 | The core (every crate under core/) has no network dependency, direct or transitive, and no socket use. Checked by check-no-network over cargo metadata with --all-features, and by cargo-deny bans in CI. The check is vacuous until core/Cargo.toml exists. |
| I2 | scoped | planned | M3 | `tools/privacy-lint/tests/test_privacy_lint.py::test_lat_field_injected_into_fixture_fails` | - | PLAN 6.1; A01 10.1 | No request, header, log or fixture that originates from an end-user device (or is derived from one) contains a coordinate pair, a route, or an H3 index finer than res 5. Sender-authoring routes are covered by I2a instead. |
| I2a | new | planned | M5 | `apps/api/test/authoring_routes.test.ts::coordinate_routes_are_post_only_no_store_no_log` | - | A01 9.5; A02 9.2 | Sender-authoring routes that carry coordinates are tagged, POST-only, no-store, excluded from logs, OpenTelemetry and Sentry, rate-limited, and store only r5-resolution rejections. Clause 8: no coordinate, geometry, place or address search text and no picked feature id is ever sent to a third party including any LLM; the console HTTP client is host-allowlisted. |
| I3 | clarified | planned | M3 | `core/crates/proxim-decide/tests/store_schema_snapshot.rs::device_store_has_only_allowed_tables` | - | PLAN 6.1; A01 2.4; A02 9.2 | Matching, path, caps, preferences and any history (including the variant ledger, family_state and the Seen on the road list) live only on the device, encrypted and excluded from backups. The trail ring is memory-only. No visit state is persisted in v1 (zone_state is reserved for owner dwell, I19). Nothing derived from history leaves the device except the events I24 allows. In M0/M1 the engine performs no I/O at all (E3). |
| I4 | base | enforced | M1 | `core/crates/proxim-decide/tests/consent.rs::consent_missing_or_unknown_denies_everything` | - | PLAN 6.1; PLAN 6.4 | Consent fails closed: unknown or missing means denied. Withdrawal wipes the outbox and affected cached data. M1 ships the fail-closed default in the kernel; the consent fuzz over all purpose combinations lands at M3. |
| I5 | base | planned | M8 | `sdks/android/sdk/src/test/kotlin/ManifestMergeTest.kt::base_aar_has_no_ad_id_permission`; `sdks/ios/Tests/ProximTests/PrivacyManifestTests.swift::no_idfa_no_att_usage` | - | PLAN 6.1 | No IDFA or AAID, no ATT prompt, no cross-app identifier, in any SDK artifact. |
| I6 | tightened | enforced | M3 | `core/crates/proxim-decide/tests/sensitive_zone.rs::nothing_logged_or_queued_inside_standoff` | - | PLAN 6.1; A01 10.1 | Inside any sensitive buffer nothing is logged, queued, written to a ledger, or learned from (also when the sensitivity test returns no data). The one test fix is memory-only. The owner-mode wake order (I16) is design-rule. |
| I7 | tightened | planned | M3 | `core/crates/proxim-decide/tests/sensitive_zone.rs::health_facility_emits_no_device_events` | - | PLAN 6.1; A01 10.1 | health_facility signposts produce no device events at all, and no exit, dwell or sequence events on health or prohibited anchors in any mode. |
| I8 | tightened | planned | M3 | `tools/egress-audit/tests/test_egress_audit.py::test_tile_request_with_key_or_cookie_fails` | - | PLAN 6.1; A01 2.4 | Restated in three parts. (a) Location-derived requests (/t/*) carry no publishable key, attestation token, cookie, bearer token or device, install or session id. (b) Credentialed requests (/v1/config, /v1/attest, /v1/events, /v1/reports) carry no location-derived value. (c) Declared residual: until OHTTP (Phase 2) requests of (a) and (b) from one device share an IP and can be joined by anyone who sees both logs. /o/* is reserved and parked. |
| I9 | base | enforced | M3 | `core/crates/proxim-decide/tests/policy_matrix.rs::child_directed_app_disables_engine` | - | PLAN 6.1 | Child-directed apps hard-disable the engine (SDK inert). |
| I10 | tightened | planned | M2 | `core/crates/proxim-pack/tests/tamper.rs::unsigned_expired_rolled_back_or_revoked_is_never_rendered` | - | PLAN 6.1; A01 10.1; A02 9.2 | Unsigned, expired, rolled-back or revoked content is never rendered. Revoked means on the current verified list; a list older than the class window renders nothing of that class. A creative counts as signed only if its full SHA-256 digest equals the signed digest at its index; a 4-byte vid is never evidence of authenticity. Any parse, duplicate or signature failure drops the whole entry. |
| I11 | tightened | planned | M8 | `sdks/android/sdk/src/test/kotlin/ManifestMergeTest.kt::base_aar_has_no_background_location_or_post_notifications` | - | PLAN 6.1; A01 10.1 | The base Android artifact never contains ACCESS_BACKGROUND_LOCATION, nor POST_NOTIFICATIONS or notification-posting code. iOS background lives in a separate SwiftPM product. |
| I12 | tightened | planned | M3 | `core/crates/proxim-decide/tests/consent.rs::health_class_requires_sdk_issued_receipt` | - | PLAN 6.1; A01 10.1 | Health-class signposts require an SDK-issued consent receipt (and receipts for local_notifications, background_location_for_triggers and background_location_for_offers when those exist). Headless integrations cannot enable health classes. |
| I13 | new | design-rule | Stage 2 (M16) | `sdks/android/sdk-background/src/test/kotlin/NotificationGateTest.kt::each_gate_condition_removed_alone_gives_no_post` | `core/crates/**/notif*.rs` | A01 2.4 | Notification gate: an OS notification is posted or scheduled only for owner mode, service class, general sensitivity, a bound app, a verified signature, an SDK consent receipt, full OS permission, a positively CLEAR sensitive-buffer test, fresh revocation list and passing caps. Owner mode is parked; no notification code may exist. |
| I14 | new | design-rule | Stage 2 (M16) | `core/crates/proxim-sched/tests/identity_firewall.rs::arm_state_never_changes_any_request` | `core/crates/**/owner*.rs` | A01 2.4 | Identity firewall: Proxim never receives push tokens, user ids, entitlements, arming lists or per-device enter, exit, cross, dwell or arrival events; no network request is initiated or timed by a location wake; requests to credentialed routes are arm-independent; no host callback on trigger events in owner mode. Declared residual: OS-visible facts in the host process. |
| I15 | new | design-rule | Stage 2 (M15) | `apps/worker/test/class_b_bundle.test.ts::class_b_bundle_holds_no_coordinate_but_the_ring` | `core/crates/**/owner_bundle*.rs` | A01 2.4 | No confidential content in packs or bundles: they are public by extraction; Class B venues carry no geometry finer than the registered ring; linted for secrets. |
| I16 | new | design-rule | Stage 2 (M16) | `core/crates/proxim-decide/tests/wake_order.rs::nothing_written_until_sidecar_clear` | `core/crates/**/sidecar*.rs` | A01 2.4 | Sensitive buffers: no notification surface is evaluated, queued or deferred inside a buffer, and none proceeds when the sidecar test returns NO_DATA (dropped, never deferred). Nothing is written until the test is CLEAR. |
| I17 | tightened | planned | M2 | `core/crates/proxim-pack/tests/envelope.rs::flip_of_any_signed_field_is_rejected` | - | A01 2.4; A02 9.2 | Signed envelope: the signature covers geometry, event parameters, event types, content_mode, delivery, experience, text, label key, intent, aud, sensitivity, expiry, creative count and the full 32-byte digest of every creative, family policy, cta kind, promo code, claims hash, landing URL and asset hashes. The signature and signing key id are modelled in the schema and the pack. |
| I18 | new | planned | M5 | `supabase/tests/signpost_triggers.test.sql::radius_below_150m_rejected_for_every_sender` | - | A01 2.4 | Micro-geometry: no trigger below the 150 m floor can exist for any sender in v1 (CHECK constraint, authoring validator, signer, compiler and kernel agree). |
| I19 | new | design-rule | Stage 3 | `core/crates/proxim-decide/tests/zone_state.rs::zone_state_encrypted_bucketed_and_wiped` | `core/crates/**/zone_state*.rs` | A01 2.4 | Visit state (zone_state) exists only for owner dwell, which is Stage 3 and not built; 15-minute buckets, 12 h TTL, never uploaded; no exit, departure or cap logic may read it. |
| I20 | tightened | planned | M3 | `core/crates/proxim-decide/tests/revocation.rs::revoked_scopes_stop_rendering_and_purge` | - | A01 2.4; A02 9.2 | Revocation: kill switch, withdrawal and revocation (scopes signpost+rev, signpost, sender_org, kid, and variant) stop rendering at the next app run; online worst case about 10 minutes, unsynced up to the hosted hard TTL. A02 moves this from design-rule to enforced for the hosted variant at M3. |
| I21 | new | planned | M3 | `core/crates/proxim-pack/tests/cta.rs::outbound_values_are_pure_functions_of_family_fields` | - | A02 9.2 | Every value that leaves the SDK toward a landing page, directions target, dialler, promo code or sender is a pure function of the signed family-level fields; it does not vary by wording, install, impression, tap, day or device state. One promo code per revision. No redirect hop through Proxim. |
| I22 | new | planned | M9 | `supabase/tests/report_releases.test.sql::sender_roles_read_only_released_cells` | - | A02 9.2 | Sender-visible numbers come only from released cells (noisy value stored once, thresholded for display), published formulas and sender-supplied aggregates; no charge may be waived because a released value is small; no exact, live, per-partner or late-arrival count reaches a sender role. Declared residual: stricter-only pack flags. |
| I23 | new | planned | M9 | `supabase/tests/conversions.test.sql::conversions_never_feed_billing_ranking_or_pacing` | - | A02 9.2 | Sender-reported conversions never feed billing, ranking, quality, pacing, refunds or suppress flags. |
| I24 | new | planned | M5 | `supabase/tests/events_schema.test.sql::events_raw_has_only_allowlisted_columns` | - | A02 9.2 | Events carry only the closed allowlist: event_id, signpost_id, rev, type (view, tap, save, dismiss, hide), slot, UTC day, sdk and os version buckets, attested, simulated. No token, device, session, hour, cell, IP, user, version-index or creative-id column exists; var, exp, rpt and cta are CHECK-disabled. |
| I25 | new | planned | M7b | `apps/worker/test/llm_prompt.test.ts::prompt_contains_only_allowlisted_fields` | - | A02 9.2 | AI boundary: an LLM request is built only by the typed LlmPromptV1 serializer from instruction text, closed-list values, placeholder names and the sender-authored wording with placeholders in place of fact values (never expanded before the call); brand name, fact values, coordinates, targeting, documents, end-user data and report rows are never sent; AI runs only on Proxim servers at authoring time. Gate G-AI applies. |
| I26 | new | planned | M3 | `core/crates/proxim-decide/tests/rotation_golden.rs::select_variant_matches_golden_v2` | - | A02 9.2 | Rotation purity: the rotation kernel is a pure integer function of the signed family, local day number, partner lane, availability mask and this install ledger; it reads no install salt, identifier, clock or random source. Rotation is NOT built in M1. |
| BP | loosened | planned | M3 | `tools/privacy-lint/tests/test_privacy_lint.py::test_user_attribute_field_fails_build` | - | PLAN 1; A01 2.4; A02 9.2 | Billboard principle (loosened by A02): a signpost may depend on where the device is and is heading, the time, the partner closed-taxonomy context, topics the user chose on the device, the user suppress actions and, for choosing among already-approved wordings only, the local date, the schedule mask, the app public lane and this install own wording counters. Never an inferred attribute, health input or identifier. The SDK rejects unknown context keys. |

## B. Engineering rules and repository guards

These back the invariants above. They are listed here so the same checker covers them.

| ID | Kind | Status | Milestone | Test ID | Guards | Source | Wording |
|---|---|---|---|---|---|---|---|
| E1 | engineering | enforced | M1 | `core/clippy.toml::disallowed-methods` | - | task brief M0/M1 | All transcendental math (sin, cos, tan, atan2, exp, pow, ln) in the decision path goes through the libm crate; the std f64 methods are forbidden by clippy disallowed-methods in the core crates. The decision path otherwise uses integer or fixed-point math. |
| E2 | engineering | enforced | M1 | `core/clippy.toml::disallowed-types` | - | task brief M0/M1 | No HashMap or HashSet iteration-order dependence in the core (BTreeMap or sorted Vec). |
| E3 | engineering | enforced | M1 | `core/crates/proxim-decide/tests/determinism.rs::decide_is_a_pure_function_of_its_arguments` | - | task brief M0/M1 | No clocks, randomness or I/O inside the engine: time, location and the seed are arguments. Outputs serialize to canonical JSON (stable key order, no float formatting surprises). |
| E4 | engineering | planned | M1 | `.github/workflows/ci-core.yml::golden-vectors` | - | PLAN 11 | Golden decision traces are byte-identical across Rust, WASM and (later) Android and iOS, and across operating systems. The CI job exists as a placeholder until core/testdata/golden lands. |
| E5 | engineering | enforced | M1 | `core/crates/proxim-decide/tests/events.rs::unsupported_event_is_ignored_fail_closed` | - | A01 4.1 | The engine implements enter and approach (plus internal exit hysteresis). cross, dwell, pass and sequence are reserved in the event enum; the kernel ignores them fail-closed with reason UNSUPPORTED_EVENT. |
| G1 | repo-guard | enforced | M0 | `tool:check-supabase-ref`; `tools/tests/test_tools.py::test_supabase_ref_rejects_forbidden_ref_as_project_id` | - | PLAN 2, 9 M0; DECISIONS | CI fails if any supabase config, .env file or workflow references the existing ilios Healthcare project ref, or any project ref that is not on tools/allowed-supabase-refs.txt (empty until the owner confirms a new project). |
| G2 | repo-guard | enforced | M0 | `tool:check-invariants`; `tools/tests/test_tools.py::test_invariants_enforced_without_any_test_id_fails` | - | PLAN 9 M0; A01 11.3 | check-invariants fails the build for an invariant with no test id, an enforced row whose test does not exist, and a design-rule row with a guarded path present. |

## C. What is and is not enforced today (2026-10-11)

- Enforced by automation: I1 (only once core/Cargo.toml exists; before that the check refuses to pass vacuously), G1, G2.
- Everything else in table A is binding policy that is enforced by review only, until its test lands. This file
  does not claim otherwise: the planned list is printed on every CI run.
- Owner mode and resorts are parked (DECISIONS.md, 2026-10-10). I13-I16 and I19 are design-rules: their Guards
  globs make CI fail if someone builds the guarded feature without the test.
- The sensitive-zone rules R1-R6 (PLAN 5.8) are kernel rules rather than invariants and are tracked by the M3
  policy-matrix tests; they are not listed here.
- Addendum 02 labels seven rows LOOSENED and two REMOVED (9.2). The ones that touch this file: the billboard
  principle (BP) and the event allowlist (I24, adds `save`). The owner accepted these on 2026-10-11. The phrase
  "no invariant is weakened" must not appear in any document.
- Test names in `planned` rows are proposals made by the repo scaffolder. The owning agent may rename them in the
  same change that lands the test.
