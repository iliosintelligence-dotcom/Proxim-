# ADR 000: Verified and unverified facts

Skeleton created 2026-10-11. Every row is "to verify". A research agent burns this list down at M0: fetch the primary source, record the URL, the date read and the exact value, then set Status to `verified`, `contradicted` (and open a conflict note for the lead) or `unreachable`. Figures in PLAN and the addenda tagged [MODEL] or [R] stay unproven until they appear here as `verified`.

Status values: to verify | verified | contradicted | unreachable.

| # | Fact to establish | Why it matters | Primary source to read | Value found | Date read | Status |
|---|---|---|---|---|---|---|
| V01 | Supabase `list_extensions` on the NEW project: postgis, pg_cron, pgtap, pg_net availability and versions | Data model, tests, jobs (PLAN 2) | Supabase MCP `list_extensions` after the owner confirms the project | | | to verify |
| V02 | Supabase plan limits (DB size, storage, egress, connections, Storage CDN behaviour for public buckets) | Pack hosting sizing | supabase.com pricing and limits docs | | | to verify |
| V03 | Vercel limits (function duration, memory, body size, cron frequency per plan, log retention, log drains) | API, cron every minute, log minimisation | vercel.com/docs/limits and cron docs | | | to verify |
| V04 | iOS region monitoring: limit of 20 monitored regions per app, minimum useful radius, relaunch behaviour | Tripwire planner | Apple Core Location documentation | | | to verify |
| V05 | iOS "region monitoring" latency scope: what Apple states about delay (PLAN-ADDENDUM-01 notes there is no primary source for a delay in minutes) | All iOS latency figures are [MODEL] | Apple docs and WWDC material | | | to verify |
| V06 | iOS CLMonitor and significant-change availability, background relaunch and reported regressions | Default API choice | Apple docs, developer forums | | | to verify |
| V07 | Android geofencing: 100 geofence limit per app and device user, recommended minimum radius (about 100 m), reboot and update behaviour | Background artifact | developer.android.com location geofencing docs | | | to verify |
| V08 | Android geofence latency text: documented responsiveness (the "about 2 minutes" style wording) and Doze or battery effects | Android latency figures | developer.android.com geofencing and location background docs | | | to verify |
| V09 | Android background location and WorkManager limits (15 minute minimum, Doze, standby buckets) | Scheduler design | developer.android.com | | | to verify |
| V10 | Google Play policy effective dates: background location declaration, data safety form, ads policies | Store gates (legal-gates C7) | support.google.com/googleplay/android-developer policy pages | | | to verify |
| V11 | Apple App Store Review Guidelines 5.1.3, 2.5.18, 5.1.5, 4.5.4 current text and update date | Store gates | developer.apple.com/app-store/review/guidelines | | | to verify |
| V12 | OHTTP relay availability: a commercially available relay operated by an organisation other than the gateway (Fastly is a candidate), terms and price | Phase 2 and health gate | Vendor pages and sales contact | | | to verify |
| V13 | RFC 9458 is Oblivious HTTP; RFC 9578 is Privacy Pass issuance (check numbers, status, errata) | PLAN 2 and 6.3 cite them | rfc-editor.org | | | to verify |
| V14 | Washington My Health My Data geofence provision wording (2,000 ft) and exceptions | Health standoff distance | Washington legislature RCW text | | | to verify |
| V15 | LLM vendor retention and zero-data-retention terms (flagged content up to 2 years) | G-AI | Vendor primary page | | | to verify |
| V16 | Crate versions in PLAN section 2 tagged R (h3o, geo, rstar, p256, libm, proptest, uniffi, wasm-bindgen) | Pinning at M0 | crates.io and the committed lockfile | | | to verify |
| V17 | JavaScript toolchain versions tagged R (TypeScript, Next, pnpm, turbo, hono, zod, vitest) | M2 and later pinning | npm registry | | | to verify |
