> **REVISED DRAFT (2026-10-10): critic findings C1-C11 addressed; see the review section for status.** The critic found 0 blockers and 7 important issues; each is now handled in the body (C1 iOS rows retagged from Apple's Core Location pages; C2 recall is detector recall by construction; C3 "solved" only for continuous-location partners; C4/C5/C6 ring-only and background-session rows relabelled as bands and upper bounds; C7 privacy guards; C8 Android DWELL; C9 tags; C10 lead wording; C11 Apple guideline anchors). Residuals are listed in the review section. All simulation numbers are [MODEL]. Nothing here is legal advice.

# Speed and Latency Playbook (Advertising only)

Status: revised draft of 2026-10-10 (critic findings C1-C11 applied in place). Scope: hosted mode only (third-party senders, in-app slot plus inbox, never OS notifications). Resorts and owner mode are out of scope.

Evidence tags: [V] verified on a primary source fetched today; [V-sec] verified only through secondary sources or search summaries; [M] measured; [MODEL] simulation or arithmetic on assumed distributions; [U] unverified. COUNSEL means a legal or store conclusion that is not ours to draw.

How these tags were obtained: the first draft was a synthesis of specialist reports, whose [V] tags came from pages they fetched on 2026-10-10 (developer.android.com location, background-limits, geofencing, battery, FGS, Play policy timeline, transitions, and Apple developer JSON for CLLocation speed fields). The critic then re-fetched and confirmed the Android pages, the Android Play timeline and the Apple pages listed in the review section. For this revision the iOS rows were retagged only where a fetched Apple page supports them (URLs are named at each row); the following were re-fetched again by the reviser on 2026-10-10: Apple's handling-location-updates-in-the-background article, the startMonitoring(for:) page, the startMonitoringSignificantLocationChanges() page, the monitoring-regions article (local copy) and the App Store Review Guidelines. Everything else keeps its earlier tag. The simulation scripts are in the scratchpad under speedsim/ (sim.py, run_all.py, results_*.csv) and sim/ (lead_sim.py, late_sim.py).

---

## 1. The answer in one page

The question: at driving speed the OS location report arrives minutes late, by which point the car has moved kilometres. How do we still show a roadside-sign-like ad at the right place?

Plain answer: we do not make the report faster. No lever found shortens it. Android's own documentation says geofence latency is "usually less than 2 minutes, even less when the device has been moving", about 2 to 3 minutes under background limits, up to 6 minutes when stationary [V]. Even 2 minutes is 3.0 km at 25 m/s [MODEL arithmetic]. The responsiveness setting is only a power hint: Google says lower values do not necessarily notify faster [V]. On iOS, Apple's legacy region-monitoring page says an app "can expect to receive the appropriate region entered or region exited notification within 3 to 5 minutes on average, if not sooner" [V: https://developer.apple.com/tutorials/data/documentation/corelocation/cllocationmanager/startmonitoring(for:).json]. That supports the plan's iOS p50 of about 4.3 minutes; the p95 of 10.7 minutes has no Apple source and stays [MODEL].

What we do instead is three levers.

Lever A, get our own fixes. When the app is visible, or a partner (navigation, ride-hail, transit, EV, parking) hands the SDK its own 1 Hz fixes and optionally its route, the SDK knows where the car is within seconds. Plan numbers: 60 m p50 and 150 m p95 late at 15 m/s; 100 m and 250 m at 25 m/s [MODEL]. Simulation: 99.4 to 100% detector recall given continuous fixes, by construction, at 1.4, 15, 25 and 30 m/s on both OSes (S4 foreground, S5 route-intent) [MODEL]. This is detector recall, not product recall: it assumes the app is open and fixes flow for the whole approach, uses a 1 km window and invented delay and error distributions, and fires at the start of the zone rather than at the 40 s ETA rule. Product recall = P(slot visible and fixes flowing in the last ~2 minutes before the sign) x detector recall, with P [U] until pilot telemetry exists. It costs no OS region slots and no new permission, because the host already pays for the location.

Lever B, move the trigger earlier. Approach rings, a re-anchor ring, leapfrog replanning, hold-awake and drive-start timers all try to wake us before the sign. They help for pre-arming but fail for point delivery:
- the report delay spread (p10 to p90) is about 3.6 km on Android and 6.0 km on iOS at 15 m/s, far wider than a 1 km sign window [MODEL];
- ring-only delivery for a 1 km window is a band, not a number: S2 on isolated anchors at 15 to 30 m/s lands at about 55 to 60% at the plan's nominal latency (N1) and about 57 to 86% at 0.5x latency, on both OSes [MODEL]. Even the band is an upper bound, because every ring-only strategy assumes a free 60 to 90 s high-accuracy burst after the wake that the Android background does not offer; with only the wake fix (about 4 s) the critic's quick hack gave 0.0% for S2 at Android 15 m/s isolated [MODEL, not a faithful model]. The "does not work" verdict rests on that missing burst plus the roughly 3.6 km wake-position spread, not on the latency figure;
- the strategies that reach 93 to 100% (hold-awake S2x, just-in-time timers S3J) need a background location session that the partner owns (Android location foreground service; iOS When-In-Use or Always plus the Location-updates capability and a CLBackgroundActivitySession or CLServiceSession, blue indicator [V per Apple's background-updates article, https://developer.apple.com/tutorials/data/documentation/corelocation/handling-location-updates-in-the-background.json]). If the partner already runs such a session, that is M2 and the rings are redundant. Google Play is also removing geofencing as an approved foreground-service use case on 2027-01-27 [V];
- and an earlier wake does not help show a card, because ads are in-app only and nobody holds the decision until the ETA.

Lever C, change the promise and how we fire.
1. Fire by time-to-anchor, not distance. A fixed 600 m is 19 s of warning at highway speed and 209 s in a traffic jam; an ETA rule gives about 40 s in both [MODEL].
2. Sell the roadside sign only with continuous fixes (M1, M2). In the background, offer only formats where a late report is harmless because the thing reported is a state that persists: near you (area), at the stop (arrival and still stopped), saved offer (inbox).
3. Tell senders, per mode, in plain words what will and will not happen (section 9).

What does not work, said plainly:
- Any background-only roadside sign at driving speed (App closed, narrow window).
- A bigger or earlier ring as a way to show a card at the right moment.
- Periodic background location on Android (throttled to a few fixes per hour [V]).
- A foreground-service burst started by an ad SDK (closing policy, and a surprise notification) [V].
- Batching, battery-optimisation exemption requests, Bluetooth car detection.

What the owner gets: a clear product line. Roadside sign: only in navigation-class and continuous-location apps. Everything else: area, stop and saved formats, evaluated on app open, with background wake as an optional extra for named partners.

How far the common case is solved: it is solved for continuous-location partners (navigation, ride-hail, transit, EV navigation). For open-occasionally apps (weather, shopping, news, social), delivery of the area and stop formats depends on app-open behaviour, which is unmeasured: nothing here says how often a user opens such an app while inside or near a site, and no segment-by-segment share of ad inventory is given. Pilot metric to add: the share of impression-eligible sessions in which fixes were flowing during the approach window (the last ~2 minutes before the anchor), reported per partner segment. That share is the P in the product-recall formula above, and it decides how much of total inventory the roadside format can ever reach.

---

## 2. The physics: distance = speed x delay

The plan's own numbers (PLAN-ADDENDUM-01 section 2.6, N1 to N4) [MODEL, anchored to one Google sentence for Android; for iOS only the 3 to 5 minute mean has a primary source (Apple's legacy startMonitoring(for:) page, [V]); the 10.7 minute p95 is [MODEL]]:

| Where the fix comes from | Delay p50 / p95 | Distance late at 15 m/s (p50 / p95) | At 25 m/s (p50 / p95) |
|---|---|---|---|
| Foreground or partner-fed 1 Hz | about 4 s / 10 s | 60 m / 150 m | 100 m / 250 m |
| Background ring, Android | 2.8 min / 6.5 min | 2.5 km / 5.9 km | 4.2 km / 9.8 km |
| Background ring, iOS | 4.3 min / 10.7 min | 3.9 km / 9.6 km | 6.5 km / 16 km |

Ring radius needed to land before the anchor with 90% probability (N3): Android 4.9 km and iOS 7.8 km at 15 m/s; 8.1 km and 13.1 km at 25 m/s.

The simulation reproduces these (400k draws: Android p50/p90/p95 = 2.80/5.39/6.50 min, iOS 4.30/8.77/10.71 min) [MODEL]. It also adds the spread that matters for a sign window. Distance the wake lands past the ring edge, p10 / p50 / p90:

| Speed | Android | iOS |
|---|---|---|
| 15 m/s | 1.3 / 2.5 / 4.9 km | 1.9 / 3.9 / 7.9 km |
| 30 m/s | 2.6 / 5.0 / 9.7 km | 3.8 / 7.7 / 15.8 km |

The p10 to p90 spread is 3.6 km (Android) and 6.0 km (iOS) at 15 m/s, and 7.1 / 11.9 km at 30 m/s. A roadside sign is readable over about 300 to 500 m, which is 9 to 36 s of driving at 14 to 33 m/s [MODEL arithmetic]. One ring cannot both land early enough and land inside that window.

Android's moving-device caveat. Google's sentence says latency is lower for a moving device [V]. Our N1 blends the stationary tail into the driving case, so Android driving distances may be smaller than N2 and N3 say. Arithmetic if the moving figure is 2 minutes: 1.8 km at 15 m/s, 3.0 km at 25 m/s. That is still 6 to 10 times the sign window. It is a hypothesis; the simulation's 0.5x latency runs bracket it. Ring-only recall is therefore a band between the nominal N1 run and the 0.5x run, not a point: on isolated anchors, 1 km window, Android 15 / 25 m/s, S2 is 58 / 57% nominal and 86 / 67% at 0.5x latency, and S2s (smart burst) is 80 / 73% nominal and 91 / 92% at 0.5x [MODEL, results_sens.csv]. The effect of halving latency is large (up to about 28 points), so the single ring-only figures must not be quoted. What keeps the negative verdict is not the latency model: it is that those strategies need a post-wake burst the Android background does not provide, plus the roughly 3.6 km wake-position spread against a sign window of a few hundred metres.

Decision where specialists differed: keep N1 as the planning basis until the drive test (T1, T2) reports. Reason: it is the conservative choice, the moving-device claim is Google's wording and not a measurement, and the planner degrades gracefully only if the latency table is set from field data (section 5, planner mismatch).

Lead arithmetic (pure definition, not a model): the distance at which the card appears = speed x lead.

| Speed | 20 s lead | 40 s lead | 75 s lead |
|---|---|---|---|
| 15 mph | 134 m | 268 m | 503 m |
| 30 mph | 268 m | 536 m | 1.01 km |
| 45 mph | 402 m | 805 m | 1.51 km |
| 65 mph | 581 m | 1.16 km | 2.18 km |
| 75 mph | 671 m | 1.34 km | 2.52 km |

---

## 3. Lever table per OS

Recommendation vocabulary: use_v1, later, only_some_partners, do_not_use.

### 3.1 Cross-platform levers (software, no OS dependency)

| Lever | What it does | Evidence | Recommendation |
|---|---|---|---|
| Partner-fed fixes (M2) with speed, course, accuracy, optional route and progress | ENTER and ETA from the partner's own 1 Hz fixes; ahead-of-sign firing from route ETA | Detector recall 99.4 to 100% at 1.4 to 30 m/s given continuous fixes, by construction [MODEL]; product recall also needs P(fixes flowing) [U]; the OS speed fields are real: Android speedAccuracy API 26 (68% confidence) [V], iOS speedAccuracy iOS 10 with negative = invalid [V] | use_v1 (best fidelity) |
| Foreground M1 (app visible, SDK or host requests fixes) | Same as above, only while the slot is visible | [MODEL] as above | use_v1 |
| Time-to-anchor (lead-seconds) firing | Makes speed a parameter; realized lead 40.0/41.3/42.5 s at 29 m/s for a 40 s target | [MODEL] | use_v1 |
| Lateness-tolerant formats (near you, at the stop, saved offer) | Late report is harmless because the state persists | [MODEL] survival tables in 6.3 | use_v1 |
| Wide lead window for background pre-arm (about 5 km) | S2 reaches 94 to 98% on both OSes for W = 5 km, but only as an upper bound under an oracle path (1-D, straight, constant speed, anchors on the driven road) and a free 60 to 90 s post-wake burst; see the box in section 5 | [MODEL], upper bound | later (only for named background partners) |
| Bigger/earlier approach ring to show a card at the right moment | Does not work: ad is in-app only, a wake has little time to run (about 10 s on iOS [U]) | reasoned from plan + [MODEL] | do_not_use |

### 3.2 Android

| Lever | Effect | Evidence | Recommendation |
|---|---|---|---|
| Foreground or partner location (FLP, visible activity or partner foreground service) | No throttling while foreground; seconds-level ENTER | [V] "no change in location sampling rates" in foreground; seconds figures [MODEL] | use_v1 |
| Partner's own location-type foreground service feeds the SDK | App counts as foreground with the screen off; route-ETA firing ahead of the sign. SDK must never start the service | [V] FGS exemptions; foregroundServiceType=location mandatory since Android 14 [V] | only_some_partners (nav, ride-hail, delivery, fleet) |
| Geofence ring sized for speed (background) | Wake 1 to 3 minutes (moving), up to 6 min (stationary) per Google | [V] for the documented sentence; moving-case benefit [U] | only_some_partners; pre-arm only; default OFF |
| setNotificationResponsiveness low on rings | Power hint; "doesn't necessarily" notify faster | [V] direction; default and magnitude [V-sec]/[U] | use_v1 where a ring exists (free), do not budget any saving until A/B (T3) |
| GEOFENCE_TRANSITION_DWELL with a loitering delay (and INITIAL_TRIGGER_DWELL) on site rings, for the F3 "at the stop" format | Fires only after the device has stayed inside for the loitering delay, which matches "arrived and still stopped", filters drive-by wakes, and makes a late report less harmful | API and the recommendation for "stops inside a geofence for a given period" [V, https://developer.android.com/develop/sensors-and-location/location/geofencing]; DWELL latency and verify-fix freshness [U] | use_v1 for F3 site rings (ENTER plus verify fix remains the fallback); test in T1/T4 (see 8.2) |
| INITIAL_TRIGGER_ENTER on approach/leapfrog rings | Closes the registration race for a moving car | [V] | later (moot while approach rings are removed from hosted v1); keep OFF on final site rings |
| Triggering location + age on the geofence event, dead-reckoned; at most one getCurrentLocation | Tells us how late we are; a fresh background fix is not guaranteed | Mechanism [V]; background freshness [U] | use_v1 (as the verify step); test T6 |
| Activity Recognition IN_VEHICLE | Drive-state signal only, no faster location; latency "might vary by device" | [V]; needs Android 10+ runtime permission; adds profiling optics | do_not_use in v1 (partner may feed a driving hint); later at most |
| Foreground service started from a geofence/transition event for a burst | Technically allowed today; policy-closed 2027-01-27; visible notification | [V] timeline, https://developer.android.com/distribute/play-policies (geofencing FGS removal dated 2027-01-27) | do_not_use |
| Passive location listener in background | May receive a navigating app's faster fixes at no GNSS cost | Doc statement [V]; real behaviour on Android 14 to 16 [U] | later (test T8 first) |
| Periodic FLP updates in background (PendingIntent) | "a few times each hour" regardless of requested rate | Page statement for background apps [V, https://developer.android.com/about/versions/oreo/background-limits]; that the PendingIntent variant is throttled the same way [U] (the page does not mention PendingIntent) | do_not_use (conclusion unchanged) |
| Batching (setMaxUpdateDelayMillis) | Adds latency by design | [V] | do_not_use |
| Battery-optimisation exemption request; Doze/Standby workarounds | Play prohibits unless core function is hurt | [V] | do_not_use (design around: prefetch, no network on wake) |
| ACCESS_BACKGROUND_LOCATION and Play approval | Entry ticket to geofencing, passive, FGS-from-background | [V] core-functionality language (https://developer.android.com/develop/sensors-and-location/location/background; that page does not mention advertising); the Play sentence about not requesting location for the sole purpose of advertising or analytics is [V-sec] only (support.google.com was unreachable) | only_some_partners; COUNSEL |
| Bluetooth/Android Auto "driving started" signal | Undocumented gain, invasive | [U] | do_not_use |
| Significant-motion sensor | Permissionless "started moving" hint; wake of a killed app undocumented | [V] description; behaviour [U] | later |

### 3.3 iOS

The first draft said the iOS levers were unresearched because the specialist returned nothing and the Core Location pages looked unreadable. That was wrong: Apple's documentation is served as JSON at developer.apple.com/tutorials/data/documentation/... and the critic (and the reviser, for the pages marked "re-fetched") read the pages named below. What is now sourced: region monitoring latency (mean only), the 20-condition cap, significant-change behaviour, and the background-updates authorization rule. Also fetched earlier: CLLocation.speed (informational only), speedAccuracy (iOS 10, negative = invalid), activityType (system may pause updates), CMMotionActivityManager (needs a usage-description key and its own prompt) [V]. Still unsourced: the p95 of region-monitoring latency, relaunch run time, speed validity after relaunch, and whether a When-In-Use session survives a relaunch. Planner notes should target CLMonitor, not the deprecated startMonitoring(for:), though the latency sentence below comes from the legacy page.

| Lever | What we can say | Evidence | Recommendation |
|---|---|---|---|
| Foreground or partner-fed fixes | Same logic as Android; sim identical for both OSes (S4 and S5) | [MODEL] | use_v1 |
| Region monitoring tripwires (20 conditions of any type per app) | Notification "within 3 to 5 minutes on average, if not sooner" [V]; p95 10.7 min is [MODEL]. Cap: "no single app may monitor more than 20 conditions of any type" [V]. After a relaunch the app must recreate the monitor with the same identifier, and monitoring works only after the first unlock following a reboot [V]. The simulation uses 8 usable rings; a third of iOS's gap to Android at driving speed is that 8-ring budget [MODEL] | Mean and 20-condition cap [V]: https://developer.apple.com/tutorials/data/documentation/corelocation/cllocationmanager/startmonitoring(for:).json (re-fetched; legacy page, deprecated in favour of CLMonitor) and the monitoring-regions article (local copy, not re-fetched); p95 and 8-ring budget [MODEL] | only_some_partners; near-you and at-the-stop only |
| Significant-location-change | Notification "as soon as the device moves 500 meters or more from its previous notification" and "not ... more frequently than once every five minutes"; the system relaunches the app on a new event [V]. Minutes-scale, so it cannot do point delivery; at most a coarse drive-start hint | [V, re-fetched: https://developer.apple.com/tutorials/data/documentation/corelocation/cllocationmanager/startmonitoringsignificantlocationchanges().json] | do_not_use for delivery; later at most as a coarse hint |
| Visits | Not researched | [U] | later, after research |
| Background location updates through CLServiceSession / CLBackgroundActivitySession (partner-owned, blue indicator) | Does NOT require Always. Apple says to create a CLServiceSession requiring whenInUse or always, or a CLBackgroundActivitySession, while the app is in the foreground, and to recreate it if the app terminates; the system may then show the blue indicator and keep the app running. This is what would enable hold-awake style strategies (S2x/S3J), but only inside a partner's own background session, which is M2 and makes the rings redundant. Not available to hosted ads (the SDK never starts it). Apple 5.1.5 fit for an advertising purpose: COUNSEL | Authorization rule and foreground-creation rule [V, re-fetched: https://developer.apple.com/tutorials/data/documentation/corelocation/handling-location-updates-in-the-background.json]; the CLBackgroundActivitySession page itself returned 404 as JSON, so the indicator wording rests on the article and the local allowsBackgroundLocationUpdates copy | only_some_partners (partner-owned = M2); COUNSEL |
| Motion activity (CMMotionActivityManager) | Separate prompt and key; SDK must not call it; partner may feed a hint | [V] for the prompt requirement | do_not_use by SDK |
| Relaunch wake runtime (about 10 s) | Quoted by the firing specialist without a source | [U] | assume nothing |

---

## 4. Time-to-anchor firing model (the speed fix)

Applies to the roadside format F1, on corridors, where fixes are continuous (M1, M2). All values are proposals; defaults are data in the signed envelope or policy, not code.

Per fix inputs: along-path distance to the anchor d_rem; Doppler speed v (smoothed; if speed is invalid derive it from position deltas over 3 s or more).

Estimator: v_f = EWMA of v with tau 8 s, re-seeded from the median of 3 fixes after a gap above 5 s; v_eff = max(v_f, v_floor) with floors 4 m/s vehicle, 2 m/s cycle, 0.7 m/s foot; T_hat = d_rem / v_eff. With a partner route (P2): v_eff = 0.5 x v_f + 0.5 x v_route(next 500 m) for T_hat up to 90 s, route ETA alone beyond that.

Fire rule: armed while d_lo <= d_rem <= d_hi, the speed band holds and the path gates hold (bearing within 30 degrees, closing over 3 or more fixes, accuracy 65 m or better). FIRE when T_hat <= lead_s + t_comp (2 consecutive confirming fixes at least 1 s apart), or d_rem <= d_lo (crawl rule). t_comp is 3 s by default (fix age, confirming fix, render) so the card is visible about lead_s before the anchor if the real fix-to-render time is 3 s; the real value is [U] until measured (T-M2 drive tests). Note on the simulation: lead_sim.py uses t_comp = 3.0 s in the fire threshold but models the card as visible 1.2 s after the firing fix (docstring and code agree; these are two different parameters, not a stale comment), so the simulated steady-road leads are about 1.8 s longer than the target (41.3 s median for a 40 s target at 29 m/s) [MODEL, checked in lead_sim.py]. Equivalent distance form: D_fire = clamp(v_eff x (lead_s + t_comp), d_lo, d_hi).

Defaults: lead_s 40 (sender range 15 to 120; presets 20 / 40 / 75); L_min = max(0.4 x lead_s, 10 s); d_hi 3000 m default, 5000 m hard ceiling; d_lo 150 m vehicle, 100 m or 3 x accuracy foot/cycle; d_pass = max(60 m, 5 s x v_eff); display cap = min(lead_s + 20, 90) s, minimum display 3 s; speed-band presets any vehicle at or above 2.5 m/s, free-flowing at or above 8.3 m/s, on foot 0.5 to 2.5 m/s, with 1 m/s hysteresis. Partners may only narrow lead and speed bounds.

Rules around it:
- Dead reckoning: a fix up to 10 s old is evaluated with d_rem minus v_eff x age; older is FIX_STALE (29 m/s x 10 s = 290 m).
- Latch: one fire per signpost per pass, in memory only; re-arm only after the anchor is passed and d_rem exceeds 1.2 x d_hi, or a reroute puts the anchor ahead again. A reroute that removes the anchor cancels the card.
- Late join (app opened late, GPS gap): fire with a shortened lead if T_hat >= L_min; otherwise drop TOO_LATE_FOR_LEAD (or save, per late_policy).
- Stop-and-go: because v_eff has a 4 m/s floor, a stopped car 300 m out does not fire; it fires when it creeps to about 170 m. A red-light stop after the fire does not re-fire.
- Background: F1 signposts are never evaluated from OS wakes (reason FORMAT_NEEDS_CONTINUOUS_MODE). Only the state formats F2 and F3 are.

Realized lead, target 40 s, 1 Hz fixes, 150 seeds per cell [MODEL, lead_sim.py]:

| Profile | ETA firing p10 / p50 / p90 | Fixed 600 m baseline |
|---|---|---|
| Highway 29 m/s | 40.0 / 41.3 / 42.5 s (1.19 km) | 18.5 / 19.3 / 20.0 s |
| Signalised arterial 15 m/s | 31.6 / 65.7 / 84.0 s (0.55 km); 19% get more than 2x target | 37.8 / 66.6 / 83.5 s |
| Exit-ramp braking 29 to 13 m/s | 66.5 / 67.6 / 68.6 s (braking after the fire adds about 27 s) | not run |
| Stop-and-go jam (about 3 m/s) | 45 / 71 / 104 s (163 to 258 m) | 176 / 209 / 259 s |
| Cyclist 5 m/s | 38.9 / 41.5 / 44.7 s (207 m) | not run |

Reading: when speed is steady the ETA rule makes lead nearly speed-invariant (spread about 2.5 s). Realized lead is bounded below on steady roads, but 10 to 20% below the target on arterials and bikes at the low end (arterial p10 is 31.6 s, about 21% below; cyclist p10 is 38.9 s, about 3% below, against a 40 s target), and the arterial case also fires too early: 19% of simulated arterial passes get more than 2x the target, which is the "card appears at the wrong place" failure for a roadside format. The console must show a range for urban roads, not a point. Route-segment ETAs did not help for ramps in the simulation (unproven). Still to add before any realized-lead range is published to senders: a dropout and speed-invalid scenario in lead_sim.py (the current sim has perfect along-path distance, 5 m position noise, no GPS gaps, no canyon error, and no speed-invalid fixes such as iOS speed = -1 at start), and a joint run with map-matching confidence (the 55 to 65% canyon figure) against a 300 to 500 m window.

The ETA rule fixes timing, not which road: correct-road accuracy is 94 to 98% in open sky and 55 to 65% in urban canyons [MODEL]; use P2 route intent where it matters.

---

## 5. Strategy comparison from the simulation

All [MODEL]. One straight road, constant speed, known direction, no stops or traffic, no dropped OS events, no Doze. Hit = exact geometry check fires inside the sender's zone [anchor - L - W, anchor - L], L = clamp(30 s x v, 150 m, 1500 m), W = 1 km. About 3000 anchors per cell, SE about 1 point; cells can be non-monotonic by up to 5 points. Slot budget: iOS 1 + 8, Android 1 + 40.

Strategies:
- S0: naive 150 m ring at each anchor. S1: static approach ring sized on p90. S2: S1 plus re-anchor ring plus leapfrog replanning on every wake.
- S2s: S2 with a smart burst that ends when the last reachable zone entry is confirmed (instead of a fixed 60 to 90 s). S2x: S2s plus hold-awake up to 240 s when a zone starts within 240 s.
- S3: S2 plus a drive-start hint and periodic low-rate fix used for replanning only. S3J: S3 plus just-in-time timers from the ETA.
- S4: foreground or partner-fed 1 Hz. S5: partner route-intent.

Recall (%) with isolated anchors (the honest worst case: no help from neighbouring anchors' rings). Columns: speed 1.4 / 15 / 25 / 30 m/s. How to read the recall figures: S4 and S5 are detector recall given continuous fixes, by construction (the window is about 33 s of slack at 30 m/s against about 10 s of invented fix delay); they are not product recall. S2 and S2s are upper bounds that assume a free 60 to 90 s burst after the wake. The S2x and S3J rows are shown for completeness only: they need a background location session or honoured timers that hosted ads do not have.

| Strategy | Android | iOS | GNSS s/h Android (1.4 / 15 / 25 / 30) | What it needs |
|---|---|---|---|---|
| S0 naive ring | 0 / 0.6 / 10.8 / 18.8 | 0 / 4.7 / 19.7 / 25.7 | 28 / 318 / 581 / 689 | OS rings only |
| S1 static approach ring | 91 / 25 / 16 / 14 | 90 / 10 / 15 / 17 | 45 / 390 / 610 / 706 | OS rings only |
| S2 + leapfrog | 98 / 58 / 57 / 60 | 98 / 55 / 59 / 59 | 297 / 861 / 1462 / 1664 | OS rings + 60-90 s burst (burst not available in the Android background: upper bound) |
| S2s smart burst | 99 / 80 / 73 / 78 | 99 / 65 / 80 / 82 | 42 / 239 / 411 / 579 | OS rings + burst (same caveat; uses true speed to end the burst) |
| S2x hold-awake | 99.8 / 98.6 / 97.5 / 98 | 99.8 / 92.6 / 98.5 / 98.4 | 60 / 793 / 1271 / 1634 | background location session: not available to hosted ads / partner-owned = M2 |
| S3 hint, replan only | 96 / 67 / 61 / 62 | 99 / 68 / 63 / 63 | 234 / 1140 / 1693 / 1915 | activity hint + periodic fixes (periodic fixes not offered by Android [V]) |
| S3J just-in-time timers | 100 all | 100 all | 156 / 513 / 813 / 1038 | requires honoured timers [U], infeasible on iOS; not available to hosted ads (not a candidate) |
| S4 foreground 1 Hz | 100 / 100 / 100 / 99.9 | same | 3600 (host pays) | app open; detector recall by construction |
| S5 route intent | 100 / 99.8 / 99.8 / 99.4 | same | 3600 (host pays) | partner route; detector recall by construction |

Ring-only with no burst (the case hosted M3 would actually face, since decision 2 below rules the burst out): the critic's quick hack, with the burst cut to about 4 s (the wake fix only, 3-fix confirm kept), took S2 at Android 15 m/s isolated from 58.0% to 0.0% [MODEL, not a faithful model]. Read it plainly: ring-only delivery of a narrow-window sign without a post-wake burst is effectively near zero, and the S2/S2s rows above are an unreachable upper bound, not a degraded mode.

With neighbours (0.5 anchors per km, Poisson): S2 Android 98 / 80 / 75 / 76, iOS 99 / 76 / 71 / 73; S2s Android 99 / 92.5 / 88.6 / 87.8, iOS 99 / 90 / 84 / 83; S2x Android 99.9 / 95 / 94 / 94, iOS 99.5 / 92 / 90 / 90. Denser fields raise ring recall only because each wake lands inside several zones, and the GNSS bill climbs toward continuous (S2x 2.7 to 3.1 ks/h at 2 to 8 per km).

Window width (isolated, Android 15 / 25 / 30 m/s): S2 recall at W = 0.5 / 1 / 2 / 5 km is 45-48 / 57-60 / 72-84 / 95-98. S1 at W = 5 km is 90 / 81 / 70 (iOS 72 / 39 / 45). Widening the lead window is the cheapest background lever; that is exactly a pre-arm, not a sign.

> **What this simulation cannot say (read before using any pre-arm number).**
> - The path is oracle-known: 1-D, straight, constant speed, every anchor on the road being driven; the smart burst ends using the true speed and the true distance to the next zone. There are no turn-offs, so no wasted rings and no wasted iOS slots.
> - OS delays are drawn independently per ring. Real latency is correlated per device and session, so per-user outcomes would be bimodal, not the averages shown.
> - No dropped events: reboot, Play services update, location toggle, or iOS monitoring only working after first unlock [V] are not modelled.
> - "Isolated" means regular 10 km spacing. The 95th-percentile wake spread can reach the next anchor at 25 to 30 m/s, so it is not truly isolated there.
> - The post-wake burst (60 to 90 s, first fix after 3 to 10 s) is free and always available. Hosted M3 has no such burst (decision 2 below).
> - Delay, error and gap distributions are invented or fitted to N1, not measured.
>
> Therefore the 94 to 98% figure for a W = 5 km pre-arm window is an upper bound under an oracle path and a free burst. Path uncertainty (heading-only prediction with turn-offs) must be added to the simulation before any pre-arm claim is used for planning.

Planner sensitivity: a static plan (S1) collapses when the planner's latency or speed assumptions are wrong (Android 15 m/s isolated falls to 3.5% if reality is 2x better than assumed). Leapfrog plus hold-awake (S2x) degrades gracefully. Field data must set the planner's latency table.

Winner by speed and OS (recall of 90% or more as "good enough", 1 km window, isolated anchors). S4/S5 figures are detector recall given continuous fixes, by construction; product recall = P(slot visible and fixes flowing in the last ~2 min before the sign) x detector recall, P [U]. Ring-only figures are bands from nominal N1 latency to 0.5x latency, and are upper bounds that assume a free post-wake burst.

| Speed | Android | iOS | Where nothing works |
|---|---|---|---|
| Walking 1.4 m/s | S2 or S2s rings (98 to 99%, upper bound with burst); S1 91% | S2 or S2s (98 to 99%, upper bound with burst); S1 90% | none; rings are fine for pedestrians |
| 15 m/s (54 km/h) | S4/S5 (detector recall about 100%). Background location session: not available to hosted ads / partner-owned = M2. Ring-only: no | S4/S5. Background location session: not available to hosted ads / partner-owned = M2. Ring-only: no | ring-only, any sign-like window: S2s band about 80 to 91% (Android) and 65 to 85% (iOS), nominal to 0.5x latency, with a free burst; without the burst effectively near zero. Verdict rests on burst unavailability plus the ~3.6 km wake spread, not on the latency model |
| 25 to 30 m/s | S4/S5. Background session: not available to hosted ads / partner-owned = M2 | S4/S5. Background session: not available to hosted ads / partner-owned = M2 (S2x inside such a session is slot-limited at 87 to 90% at 2 to 8 per km) | ring-only on both OSes: S2s band about 73 to 92% (Android) and 75 to 82% (iOS) with a free burst, near zero without; iOS at 30 m/s cannot even size S1's ring (needs 17.1 km vs the 15 km clamp) |
| Any speed, W = 5 km, pre-arm | S2 94 to 98%, upper bound under oracle path and burst | S2 94 to 98%, upper bound under oracle path and burst (S1 only 39 to 72%) | n/a; see the "what this simulation cannot say" box |

S3J (just-in-time timers, 100% in the simulation) is deliberately not a winner in this table: it requires honoured background timers [U] and is infeasible on iOS. S2x is shown only as the partner-owned case.

Important qualifiers on the S2s/S2x/S3J rows:
- They assume a first usable fix within 3 to 10 s of the wake and a 60 to 90 s high-accuracy burst or hold-awake. The Android specialist found that in the background an unthrottled fix stream exists only with a foreground service (FLP otherwise "a few times each hour" [V]), and the foreground-service route is policy-closing [V]. So on Android the S2/S2s rows are upper bounds; the real background wake has the event's triggering location plus at most one getCurrentLocation whose freshness is [U].
- iOS: background location updates do not need Always. Apple says to create a CLServiceSession (requiring whenInUse or always) or a CLBackgroundActivitySession in the foreground and to recreate it on relaunch [V]. But a session created at relaunch from a region wake, the run time available, and whether a When-In-Use session survives a relaunch are [U] (tests I2/I3), and the session would be the partner's own, i.e. M2.
- S3J assumes background timers are honoured with 0 to 25% lateness (recall falls to 81 to 94% at 100% slop and P = 300 s). Nothing verifies that assumption: iOS offers no timer that wakes a suspended or terminated app, Android Doze and standby delay or ignore inexact alarms, and exact alarms need a special permission [U, not fetched]. Treat S3J as infeasible for hosted ads. S3's drive-start hint alone is no better than S2 (61 to 68%) and costs 31 to 69 extra wakes per hour; do not build it.
- S3 and S3J on Android also lean on 60 to 300 s periodic background fixes, which Android does not offer [V].

Where the specialists disagreed and how I decided:
1. Simulation and Android specialist say approach rings can pre-arm; firing specialist says no hosted signpost needs one. Decision: drop approach rings and leapfrog from hosted v1. Reason: the plan already keeps hosted background OFF by default; for an in-app-only ad the content is already in the on-device pack, so pre-arm buys at most an inbox row; and a ring wake cannot hold the decision. Revisit as Stage 2 only if field data shows inbox fill is worth a Play background-location declaration.
2. Simulation recommends a smarter/longer burst; Android specialist says the burst is unsupportable in the background. Decision: no burst for hosted M3. The smart-burst result stays on file for owner mode and for partners who run their own background session.
3. Android specialist: 500 m minimum ring for driving; plan: 150 m. Moot for hosted v1 (no driving rings). Keep 150 m for site rings; reconsider only if approach rings return.
4. Activity-recognition: Android says later, simulation says useless for replanning, firing says SDK must not call it. Decision: SDK does not call it; partner may feed a motion hint.

---

## 6. Partner-segment matrix

### 6.1 Who gets what

| Partner type | Location source and mode | Experience we can offer | Do not promise |
|---|---|---|---|
| Navigation apps | Continuous 1 Hz road-snapped fixes, route, progress. M2, P2 (P1 without route) | F1 roadside sign with lead_s (default 40 s), glance variant; F3 at stops; F4 saved offers when parked | Delivery with screen off or voice-only; CarPlay/Android Auto (deferred, [U]); lane/turn/ramp copy; exact lead on signalised arterials; accuracy in urban canyons |
| Ride-hail rider apps | Fixes during trip; route/ETA fed by partner. M2, surface in_vehicle_passenger | F1 with longer leads (60 to 120 s allowed); F3 at pickup/drop-off while waiting; F4 | That the viewer is not driving; anything implying destination (last 1.5 km truncated); background delivery |
| Ride-hail driver, delivery, fleet | Continuous fixes during shifts; M2; M3 not used | F3 at stops (safe, valuable); F1 only if the partner enables it and COUNSEL clears (default off); F4 | Roadside signs to working drivers by default (distracted driving, COUNSEL); anything resembling worker monitoring; background delivery |
| EV charging and fuel apps | Foreground when searching/navigating; M1 or M2; M3 only if declared | F3 at the charger (long dwell), F3 at fuel via on-open, F2 nearby offers; F1 only where the partner navigates | Roadside signs for competing stations; background delivery at a fuel stop; charger availability claims |
| Parking apps | Foreground at session start/end; M1 (M2 if route fed) | F3 at "just parked"; F2 on walking away; F4 | Anything while circling for a space; underground precision; background gate detection |
| Transit apps | Foreground; M2 with timetable ETAs; M1 otherwise | "Next stop" style F1 with lead 60 to 120 s from the partner's timetable ETA; F3 at stops; F2 around interchanges | GPS ETAs underground; stop-level F3 from background reports |
| Fitness/outdoor apps | Foreground or partner session; M1/M2; likely sealed or health-category policy | Post-workout F3/F4 at finish; F2 on the summary screen | Roadside signs during a workout; foot leads below 100 m; health inferences |
| Weather, social, shopping, news (open occasionally) | When-in-use on open; M0 or M1; M3 default OFF | F2 near-you on open (one fix); F3 only if opened while stopped inside a site; F4 | Any roadside or time-to-anchor behaviour; delivery when the app is not opened; ad waiting after a background wake |

### 6.2 Format summary

- F1 Roadside sign ahead: needs continuous fixes. Glance variant (at most 7 words plus icon plus SDK-rendered distance chip). Late policy per signpost: drop (default), show short, or save.
- F2 Near you: area-level. Works late, but only if the area is large enough (6.3).
- F3 At the stop: arrival at a site and still stopped (speed 1.5 m/s or less). Works late if the user is still there.
- F4 Saved offer: inbox row, no location precision needed. Not an impression; not billed.
- Not offered: F5 "turn now" and exact-spot copy ("next exit", "in 200 m"); any roadside format from an OS wake; past-tense "you are passing X".

### 6.3 Late-report survival (used to choose formats; never in sender copy)

[MODEL; lognormal fits to N1; iOS mean of 3 to 5 minutes is [V] (Apple legacy page) but the iOS p95 is [MODEL]; dwell medians are my [U] assumptions; optimistic for area crossings (path through the centre).]

Chance a late OS ENTER report lands while the user is still inside a circular area during a through-trip:

| Case | r = 1 km | r = 2 km | r = 5 km |
|---|---|---|---|
| Android 15 m/s | 33% | 82% | 100% |
| Android 25 m/s | - | 46% | 95% |
| iOS 15 m/s | - | 52% | 96% |
| iOS 25 m/s | - | 19% | 79% |

Walking, within 500 m past the boundary: 93% (Android), 72% (iOS). Guidance: for background delivery pick areas of radius 5 km or more; below 2 km mostly lands after the user has left.

Chance the report lands before the user leaves a stop (Android / iOS): fuel stop (4 min median) 74% / 51%; drive-through 76% / 51%; curbside pickup (6 min) 88% / 70%; transit wait (3 min) 79% / 56%; EV fast charge (30 min) 100% / 100%; car park (45 min) 99% / 98%. Seeing the ad still requires the user to open the partner app.

---

## 7. Planner, SDK API and PLAN.md changes

The exact list is in the structured fields planner_and_plan_changes, sdk_api_additions and corrections_to_existing_docs of this deliverable. Summary:

Planner:
- Hosted v1 has no approach rings, no leapfrog. Re-anchor plus nearest-site ring selection stays (20 iOS regions cannot hold a metro's sites) and serves F2/F3 only, for named hosted_background partners.
- No burst on wake. Wake uses the OS event time, the verify fix (Android: triggering location, at most one getCurrentLocation [U]), then drop table.
- If rings return in Stage 2: scale re-anchor radius with speed, tighten rings the car is already inside, keep still-valid rings, plan rings only for the top 1 to 3 candidates after caps.
- Replace the three inconsistent clocks (2 x p95 latency, 30 s, N6 expiries) with per-format max_event_age_s and max_event_dist_m.

SDK: fix fields (speed, speedAccuracy, course, courseAccuracy, monotonic time, source); route.set(RouteIntent) plus route.progress; optional motion.update; setSurfaceReady and closed surface ids; optional moment assertion; decision timing; partner config; TestKit speed profiles. The earlier proposal to add report fields lead_bucket and fire_mode is withdrawn pending the privacy guards in section 10A (they are not reported, or only as k-thresholded aggregates with no device or session linkage). No new permission; the SDK never starts a foreground service and never asks for background location. RouteIntent, route.progress and motion hints are subject to the guards in section 10A (never persisted, logged or sent; pack sync never route-shaped).

---

## 8. Simulator scenarios and field tests

### 8.1 Simulator (proxim-sim-qa)
See simulator_scenarios_to_add. Core set: constant-speed approach at 8 to 35 m/s; signalised arterial; exit ramp; jam; red light; late join; GPS gap; fix-stale; wake at 3 and 10 minutes late with F1 ignored; F2 and F3 wake paths; two Android latency models (moving and stationary); recorded routes replayed at 50 / 90 / 120 km/h.

### 8.2 Field tests that settle [MODEL] and [U] numbers
Rule from the plan: publish a percentile only with 20 or more samples per OS and speed class.

Android (from the Android specialist):
- T1 drive-by latency: Pixel plus Samsung plus one other OEM, Android 14 to 16, 150 m / 500 m / 1 km rings at 50 / 90 / 120 km/h, screen off, ground-truth GNSS log. Report min, median, p90, max.
- T2 moving versus stationary (parked 30+ minutes versus already moving).
- T3 responsiveness A/B at 0, 30 s, 5 min, 20 passes each; no effect if the median difference is under 20 s.
- T4 initial trigger delay and leapfrog, plus GEOFENCE_TRANSITION_DWELL / INITIAL_TRIGGER_DWELL for F3: dwell-event latency after the loitering delay (try 1, 3 and 5 minute delays at fuel, EV and car-park sites), drive-by filtering (no event on a pass-through), and freshness of the verify fix at the dwell event; compare with ENTER plus verify fix. T1 should also log a DWELL ring beside each ENTER ring so both are measured on the same passes. T5 standby buckets and Battery Saver; T6 wake fix freshness (getCurrentLocation versus cached versus failed); T7 IN_VEHICLE detection latency; T8 passive listener while a map navigates; T9 rural/highway reliability with Wi-Fi scanning on and off.

iOS (proposed, revised after the critic; the test outcomes are all [U], the mechanisms are sourced in 3.3):
- I1 region-monitoring drive-by latency at 50 / 90 / 120 km/h, backgrounded and terminated, iPhones on current and previous iOS, 20 passes per speed. Compare against Apple's "3 to 5 minutes on average" [V] and replace the modelled p95 of 10.7 minutes with a measured one. Use CLMonitor, not the deprecated startMonitoring(for:), and check the first-unlock-after-reboot rule [V].
- I2 behaviour after a relaunch wake (region or significant-change): time to a first location, speed validity (speedAccuracy, negative = invalid), run time available.
- I3 the real gaps in background updates: whether a partner-created CLServiceSession or CLBackgroundActivitySession, on a When-In-Use grant, keeps updates flowing in the background, whether it must be created in the foreground and recreated at relaunch [V, article], and whether it survives a relaunch; blue-indicator behaviour and Always versus When-In-Use grants. (The earlier version of I3 assumed an Always grant, which Apple's article shows is not required.)
- I4 Background App Refresh off, Low Power Mode, and reduced-accuracy grants.

Both OSes: M2 drive tests with 3 Android and 3 iOS devices, 20 passes per speed class (urban 15 m/s, highway 29 m/s, crawl): record fix-to-render time (t_comp; the simulation used 3.0 s in the fire rule and 1.2 s to display) and realized lead, including dropout and speed-invalid cases; and a pilot telemetry item, the share of impression-eligible sessions with fixes flowing during the approach window, per partner segment (the P in the product-recall formula); on-open and late-wake tests at fuel, EV, car-park and transit sites to replace assumed dwell times.

Non-engineering: prepare the Play background-location declaration and the Apple purpose strings for one pilot partner whose core function genuinely needs background location; COUNSEL review before enabling hosted background for anyone.

---

## 9. What we promise senders and partners, in plain words

Senders (console text; no latency or distance figures for background until field data exist):

- In a navigation or ride-hail app that shares its route: "Your sign appears about [lead] before the point you marked, at the driver's speed. We show the speed-to-distance table for your lead (arithmetic) and one simulated trip range, labelled simulated."
- In an app that uses its own location while open: "Same rule. Not delivered if the app is opened late or the screen is off."
- App closed or in the background: "Road signs are not delivered. If you tick Save for later and the partner allows it, the offer appears in the app's inbox, usually after the driver has passed."
- Near-you areas: radius under 2 km "delivered only while the app is open"; 5 km or more "can also reach people whose app wakes late". No percentages.
- At-the-stop: EV charging and car parks "work when the app is opened or wakes late"; fuel, drive-through and transit "work when the app is open; late wakes often miss short stops".
- Always: "about", never a number, in copy. Copy with fixed distances or times ("500 m ahead") is rejected for F1 and F2; the SDK renders the distance chip.

Partners (integration page):

- Navigation or continuous-location app (M2): "Pass us your fixes with speed and course, and your route with segment durations. We never ask the user for location, never start a service, and never see coordinates on the server. You get ETA-timed roadside signs."
- App-open only (M1): "We use your fixes while the ad slot is visible. We evaluate near-you and at-the-stop once on open."
- Background wake (M3): "Off by default. We turn it on only if your declared purposes already include background location. It serves near-you and at-the-stop formats, minutes late, never roadside signs. You own the permission prompt and the store declaration."
- Never: "We will not show an OS notification for an ad, ask for Always, or ask your users to exempt us from battery optimisation."

---

## 10. Open owner decisions (maximum 4, recommended defaults)

1. Drop approach rings, leapfrog and the wake-time burst from hosted v1. Recommended: yes. Hosted background stays OFF until a partner's declared purposes include it.
2. Who may sell F1 roadside signs. Recommended: navigation-class partners only (glance variant, partner opt-in, COUNSEL on distracted driving first); off by default for delivery and ride-hail driver apps.
3. Hosted wake-time Saved writes. Recommended: no in v1; serve F2/F3 by on-open evaluation; allow wake writes only for a named hosted_background partner after the sensitive-zone test is clear.
4. Fund the field tests before any background promise. Recommended: yes, with Android T1/T2/T6 and iOS I1/I2 first, plus I3 as respecified above (the iOS lever research pass itself is largely done, see 3.3). The lead_bucket and fire_mode report fields are NOT added; see the privacy guards in section 10A. "Speed-band targeting" (letting senders target driving users) is a further decision for COUNSEL, listed in 10A.

---

## 10A. Store-policy anchors and privacy guards

### Store-policy anchors (Apple [V], reading for advertising is COUNSEL)

Fetched and read by the reviser on 2026-10-10 from https://developer.apple.com/app-store/review/guidelines/ (and confirmed earlier by the critic). Quoted as written; none of them speaks specifically to background location for an advertising purpose, so every ad-specific reading below is COUNSEL.

- 2.5.4 [V]: "Multitasking apps may only use background services for their intended purposes: VoIP, audio playback, location, task completion, local notifications, etc." COUNSEL: whether background location whose only consumer is ad delivery is an intended purpose; the safe reading is that it is acceptable only inside a partner whose own core function uses background location.
- 5.1.5 [V]: "Use Location Services in your app only when it is directly relevant to the features and services provided by the app." and "Ensure that you notify and obtain consent before collecting, transmitting, or using location data." COUNSEL: whether ad targeting is "directly relevant to the features and services provided by the app".
- 5.1.2(i) [V]: personal data requires permission before use, transmission or sharing, and "Data collected from apps may only be shared with third parties to improve the app or serve advertising (in compliance with the Apple Developer Program License Agreement)". COUNSEL: what counts as sharing when the SDK evaluates on device and sends no coordinates.
- 2.5.18 [V]: "Display advertising should be limited to your main app binary, and should not be included in extensions, App Clips, widgets, notifications, keyboards, watchOS apps, etc." This is the formal reason hosted ads must not become OS notifications or widgets, and it constrains any future CarPlay or widget surface. COUNSEL: how it applies to a CarPlay template or Live Activity.

Play (Android): the Play background-location policy language about core functionality is [V] (https://developer.android.com/develop/sensors-and-location/location/background; that page does not mention advertising). The sentence about not requesting location for the sole purpose of advertising or analytics is [V-sec] only, because support.google.com was unreachable when the critic tried; treat it as COUNSEL. The geofencing foreground-service removal is dated 2027-01-27 [V] (https://developer.android.com/distribute/play-policies).

### Privacy guards (invariant: the server never learns where the user is)

The critic found that the recommended SDK inputs and report fields could leak location or trajectory. These guards are proposed as invariants and need architect and COUNSEL sign-off:

1. RouteIntent, route.progress and partner motion hints are never persisted (no disk, no cache), never logged (including crash reports and analytics breadcrumbs) and never sent off the device. They live in memory for the session only, like the latch.
2. Pack sync is tile- or region-level and is never route-shaped. Corridor signposts must not be fetched by sending a corridor or polyline to the server; fetch by coarse tile or region, and match to the route on device. State the fetch granularity in the pack design before route pre-matching at route.set is built.
3. The report fields fire_mode and lead_bucket are NOT reported. If they are ever reported, only as aggregates above a k-threshold with no device or session linkage, because fire_mode=route plus lead_bucket tells the server that an impression came from a navigated route at a given speed class, which is low-k in sparse regions. The surface label in_vehicle_passenger is held to the same rule.
4. For ride-hail the destination stays truncated (last 1.5 km); navigation apps are not truncated, so guard 1 matters most there.
5. COUNSEL items to record: speed-band targeting (senders targeting driving users is ephemeral but is still context-based profiling), and driving-state inference from speed, route or motion hints.

---

## 11. Unverified

- All simulation numbers: [MODEL], constant speed, one straight road, no traffic, no dropped events, no Doze. The 1-D road hides S5's road-disambiguation benefit.
- Android moving-device latency: Google's sentence is [V]; the number under 2 minutes and how it compares with N1 are [U].
- iOS: the p95 of 10.7 minutes (the 3 to 5 minute mean is [V]), visits, relaunch run time, speed validity after relaunch, whether a When-In-Use CLBackgroundActivitySession survives a relaunch, and the ad-specific reading of Apple policy [U / COUNSEL]. The region-monitoring mean, the 20-condition cap, significant-change behaviour and the background-updates authorization rule are [V] (see 3.3).
- Product recall: P(slot visible and fixes flowing in the last ~2 minutes before the sign) is [U] until pilot telemetry; the share of inventory from open-occasionally apps that ever has continuous fixes is unmeasured; app-open behaviour of such apps near sites is unmeasured.
- What setNotificationResponsiveness really changes and its default; developers.google.com and support.google.com pages were not fetchable [V-sec].
- Whether a background getCurrentLocation from a geofence receiver is fresh; whether a PendingIntent FLP request is throttled like the background-limits page's "few times each hour" (that page does not mention PendingIntent); DWELL event latency; passive listener behaviour on Android 14 to 16; Activity Recognition detection latency [U].
- Play policy dates: the developer.android.com timeline dates both the geofencing foreground-service removal and the location-button item 2027-01-27 [V, https://developer.android.com/distribute/play-policies, checked by the critic]. The Play sentence about the sole purpose of advertising or analytics is [V-sec] and COUNSEL.
- Dwell times and survival tables; t_comp (fix to render); per-segment ETAs from navigation SDKs; speed validity after an iOS relaunch; CarPlay and Android Auto rules.
- Legal and store conclusions (ads using background location, activity data, dead reckoning, distracted driving, worker monitoring): COUNSEL.

---

# Critic review of this playbook

**Verdict:** The core conclusion holds: do not use OS geofence reports to deliver a roadside sign at driving speed, and use continuous fixes (M1/M2) for F1. It is supported by Google's own latency wording and by Apple's, and the simulation reproduces its stated numbers. Several parts are weaker than presented. (1) The iOS "research gap" is false. Apple's Core Location docs are fetchable through the developer.apple.com/tutorials/data/documentation/... .json endpoints, and local copies already existed in scratchpad/apple/. They change three iOS rows: there is now a primary source for region-monitoring latency, the "Always required" assumption is wrong, and significant-change is resolved. (2) The Lever A recall of 99.4-100% is true by construction. It uses a 1 km window, invented delay and error distributions, and 100% foreground coverage. It says nothing about the ETA rule, and the share of inventory where the app is open and fixes are flowing is unmodelled. (3) The "winner" rows S2x and S3J are not buildable for hosted ads. The ring-only numbers (S2/S2s) depend on a 60-90 s post-wake burst that is unavailable in the Android background. (4) The route-intent and report-field proposals (fire_mode, lead_bucket, route pre-matching) need explicit privacy guards before they match the "server never learns where you are" invariant. Items 1-3 should be fixed before the owner sees the playbook. The headline "the common case is already solved" should be softened to "solved for continuous-location partners; coverage of total inventory unknown".

## Claims re-checked against primary sources

- Android geofence latency: 'Usually the latency is less than 2 minutes, even less when the device has been moving. If Background Location Limits are in effect, the latency is about 2-3 minutes on average... stationary... up to 6 minutes'. CONFIRMED [V], fetched https://developer.android.com/develop/sensors-and-location/location/geofencing. The same page confirms the 100 geofence limit per app per device user, 'Setting lower values doesn't necessarily mean that users are notified within that time period', and the DWELL/loitering recommendation. N1 (p50 2.8 / p95 6.5 min) sits at or above the driving case, so it is a conservative model, not Google's number. The playbook's 'moving-device caveat' is correct.
- Android background location throttling: 'apps can receive location updates only a few times each hour... even when your app is requesting more frequent location updates'; 'Background apps can receive geofencing transition events more frequently than updates from the Fused Location Provider'; passive listeners 'may receive faster location updates if there are foreground apps requesting'. CONFIRMED [V], fetched https://developer.android.com/about/versions/oreo/background-location-limits. The page does NOT mention PendingIntent, so the playbook's [V] on the PendingIntent variant is really [U]. Foreground behaviour is unchanged on Android 8+.
- Activity Recognition: runtime permission android.permission.ACTIVITY_RECOGNITION from Android 10, and the Activity Recognition API returns nothing without it. CONFIRMED [V], fetched https://developer.android.com/about/versions/10/privacy/changes. The Transition API also needs the com.google.android.gms.permission.ACTIVITY_RECOGNITION manifest permission, and 'latency of event detection might vary by device'. CONFIRMED [V], fetched https://developer.android.com/develop/sensors-and-location/location/transitions. That page does not say whether IN_VEHICLE is faster than geofencing. Not covered by the playbook: an activity-recognition transition event is an exemption from the Android 12+ background foreground-service-start restriction, like geofencing [V, https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start]. However a location-type FGS needs a visible activity or ACCESS_BACKGROUND_LOCATION [V, same page]. Whether Play's FGS policy would accept a location FGS started from an activity-recognition event is untested [U].
- Background location is 'accessed in the background' unless an activity is visible or a foreground service with type location is running; geofencing requires ACCESS_BACKGROUND_LOCATION on API 29+. CONFIRMED [V], https://developer.android.com/about/versions/10/privacy/changes. Play background-location policy: 'restricting background location access to apps that need it for their core functionality... Adopting these best practices doesn't guarantee Google Play approves'. CONFIRMED [V], https://developer.android.com/develop/sensors-and-location/location/background. That page does not mention advertising. The 'never request location solely for advertising or analytics' wording is confirmed only through search-result summaries (V-sec). support.google.com/googleplay/android-developer/answer/9799150 was unreachable here (DNS failure).
- Play timeline: 'We're removing geofencing as an approved foreground services use case. Developers can instead use the Geofence API' dated January 27, 2027. CONFIRMED [V], fetched https://developer.android.com/distribute/play-policies. The location-button item is also dated 2027-01-27. The August 26, 2026 heading holds no geofencing item and there is no October 28, 2026 heading, so the playbook's unreconciled 2026-08-26 / 2026-10-28 doubts can be dropped.
- iOS region monitoring latency and cap: 'within 3 to 5 minutes on average, if not sooner' (legacy startMonitoring(for:) page, deprecated in favour of CLMonitor); 20-region cap; 'no single app may monitor more than 20 conditions of any type'; after reboot, monitoring works only after first unlock. CONFIRMED [V], fetched https://developer.apple.com/tutorials/data/documentation/corelocation/cllocationmanager/startmonitoring(for:).json, and the local copy of corelocation_monitoring-the-user-s-proximity-to-geographic-regions.json (documentation copy fetched earlier today; I did not refetch it). This contradicts the playbook's 'no primary source' for the iOS minutes figure (mean only; p95 remains unsourced).
- iOS significant-change: 'as soon as the device moves 500 meters or more... should not expect notifications more frequently than once every five minutes'; the system relaunches the app. CONFIRMED [V], fetched https://developer.apple.com/tutorials/data/documentation/corelocation/cllocationmanager/startmonitoringsignificantlocationchanges().json.
- iOS background updates without Always: Apple's background-updates article says to use CLServiceSession / CLBackgroundActivitySession requiring 'whenInUse or always', created in the foreground and recreated at relaunch. The allowsBackgroundLocationUpdates page says the system keeps the app running and may show the blue indicator. CONFIRMED [V], fetched https://developer.apple.com/tutorials/data/documentation/corelocation/handling-location-updates-in-the-background.json, plus local copies of the allowsBackgroundLocationUpdates and requestWhenInUseAuthorization pages (not refetched). The CLBackgroundActivitySession page itself returned 404 (JSON) and a local copy was HTML only. The playbook's assumption of 'Always plus background updates' for iOS partners is wrong or at least unnecessarily narrow.
- Apple App Store Review Guidelines: 2.5.4 (background services only for intended purposes, including location); 5.1.5 (location only when directly relevant, consent before collecting/transmitting/using); 5.1.2(i) (third-party sharing allowed 'to improve the app or serve advertising'); 2.5.18 (display advertising limited to the main app binary, not notifications, widgets, extensions). CONFIRMED [V], fetched https://developer.apple.com/app-store/review/guidelines/ (truncated after 5.3.4, which does not affect these items). No Apple sentence specifically governs background location for an advertising purpose, so the actual risk is a COUNSEL matter.

## Simulation sanity check

I read sim.py (501 lines), run_all.py, sanity.py, sanity_out.txt, lead_sim.py, late_sim.py and results_*.csv, and re-ran selected cells. The reported results reproduce exactly: S2 isolated Android 15 m/s 58.0% and iOS 55.0%; S2x 98.6% and 92.6%; GNSS 861 / 793 / 1001 / 752 s/h. Runs are deterministic (CRC32 seeds). The latency fit reproduces N2 within rounding. The sanity checks that exist hold: zero-latency recall is ~100% for rings (S1 98-100%, S2 100%), S0 is ~0% when truly isolated, the slot budget assertion never trips, and all fired leads lie in [L, L+W]. The suite is weak in these ways. (1) S4/S5 'recall' is by construction: W=1 km and ~10 s of delay, with invented noise parameters (fix delay lognormal, GPS-gap 3%, ETA eps sigma 0.15/0.5 mix, processing 1-2 s). They fire at the start of the zone (~92 s of lead at 15 m/s), not at the ETA rule's 40 s. (2) The world is oracle: 1-D, straight, constant speed, anchors all on the driven road, with true speed used for the smart-burst end. There is no branching, no turn-offs, no dropped events, and delays are i.i.d. per ring. (3) The post-wake burst (60-90 s, first fix 3-10 s) is free and always available. With a ~4 s burst, S2 recall at Android 15 m/s isolated goes to 0.0% in my quick hack, so ring-only numbers depend entirely on that assumption. (4) The winner among ring strategies depends on [U] inputs: the N1 latency model (the sensitivity table shows S2s Android 15 m/s isolated going from 79.7% to 90.6% at 0.5x latency), the iOS 8 and Android 40 usable-slot budgets, and burst availability. S3J's 100% depends on honoured timers, which are unverified. (5) The 'isolated' scenario is 10 km regular spacing, not truly isolated at 25-30 m/s. (6) lead_sim and sim.py are never combined. The negative conclusion (rings cannot do narrow-window signs at driving speed) is robust to all of these. The positive recall and lead figures are not independent evidence and should not appear as promises."]

## Findings (11)

### C1. [IMPORTANT] iOS levers are declared unresearched ('Core Location pages could not be read, JavaScript shell only') and every iOS row is tagged [U]

Status: addressed in sections 1 (plain answer, Lever B), 2 (table note), 3.3 (rewritten iOS table), 5 (qualifiers), 8.2 (I1-I3 respecified), 10A and 11. Accepted residual: the 10.7 min p95 stays [MODEL]; the CLBackgroundActivitySession class page itself returned 404 as JSON, so the background-session wording rests on the background-updates article (re-fetched); CLMonitor and visits are not researched.

iOS levers are declared unresearched ('Core Location pages could not be read, JavaScript shell only') and every iOS row is tagged [U]. The Apple JSON endpoints work, and scratchpad/apple/ already held ~72 fetched pages from 06:40 today, before the playbook was written. Several iOS claims can now be settled. (a) The playbook says the iOS 4.3/10.7 min figure has 'no primary source'. Apple's startMonitoring(for:) page says an app 'can expect to receive the appropriate region entered or region exited notification within 3 to 5 minutes on average, if not sooner' [V, fetched today]. That is a legacy-wording page, now marked deprecated in favour of CLMonitor. It supports the p50 of ~4.3 min but not the p95 of 10.7 min. (b) Region cap: 'no single app may monitor more than 20 conditions of any type' [V]. On relaunch the app must recreate the monitor with the same identifier, and monitoring only works after first unlock following a reboot [V]. (c) Significant-location-change: 'as soon as the device moves 500 meters... should not expect notifications more frequently than once every five minutes', with relaunch on a new event [V]. That is minutes-scale and cannot do point delivery. Its use is a coarse drive-start hint only. (d) Background updates do NOT require Always. Apple's background-updates article says to create a CLBackgroundActivitySession or CLServiceSession 'requiring... whenInUse or always', created in the foreground, and recreated at relaunch. The blue pill/indicator is shown. The playbook's iOS row, 'Always + background updates held by the partner', and the I3 test are therefore mis-specified.

**Fix:** Retag the iOS latency row: mean 3-5 min is [V] from the legacy Apple page; the p95 of 10.7 min stays [MODEL]/[U]. Replace 'Always + background updates' with 'When-In-Use + Location-updates capability + CLBackgroundActivitySession (blue indicator)', owned by the partner. Mark SLC as 'coarse hint only, do_not_use for delivery'. Target CLMonitor, not the deprecated startMonitoring(for:), in planner notes. Redo T-I1..I4 to test the real gaps: relaunch run time, speed validity after relaunch, and whether a When-In-Use session survives a relaunch.

### C2. [IMPORTANT] The Lever A headline ('99.4 to 100% recall at 1.4, 15, 25, 30 m/s on both OSes

Status: addressed in sections 1 (Lever A), 3.1, 5 (table notes, winner table) and 11. Accepted residual: the joint end-to-end test of fix noise, dropouts, map-matching confidence and the ETA rule against a 300 to 500 m window has not been run; P is [U] until pilot telemetry.

The Lever A headline ('99.4 to 100% recall at 1.4, 15, 25, 30 m/s on both OSes... this is the product at driving speed') is true by construction. In sim.py sim_continuous, S4 success means a fix delay of lognormal(4 s p50, 10 s p95) plus a 3% gap of up to 30 s is less than the 1 km window divided by speed. At 30 m/s that is ~33 s of slack against ~10 s of delay. S5 success uses invented ETA error (sigma 0.15, or 0.5 for a 5% mix) and a 1-2 s processing time. None of these parameters has a source. Three further problems. (i) S4/S5 fire at the start of the zone: median lead 1388 m (~92 s) at 15 m/s with L=450, W=1000. This is not the playbook's 40 s ETA rule, which is simulated separately in lead_sim.py without GPS gaps, canyon error or map-matching. The two were never combined end to end. (ii) The sim's 1 km window is 2-3x wider than the 300-500 m 'readable' window the playbook itself quotes. (iii) Recall is conditional on the app being open and fixes flowing for the whole approach, which is not modelled.

**Fix:** Re-label S4/S5 recall as 'detector recall given continuous fixes, by construction', not product recall. Report product recall as P(slot visible and fixes flowing in the last ~2 min before the sign) x detector recall, and mark P as [U] until partner telemetry or a pilot exists. Run one joint test of fix noise, dropouts, map-matching confidence (the playbook's 55-65% canyon figure) and the ETA rule against a 300-500 m window.

### C3. [IMPORTANT] The headline and decision table say 'the app is usually open or partner-fed when an ad shows, so the common case is already solved'

Status: addressed in sections 1 (new paragraph "How far the common case is solved"), 5 (winner table), 8.2 (pilot metric) and 11. Note: the quoted sentence did not appear verbatim in the body; the over-claim was carried by "This is the product at driving speed" and the winner table, both reworded. Accepted residual: app-open behaviour of open-occasionally apps stays unmeasured.

The headline and decision table say 'the app is usually open or partner-fed when an ad shows, so the common case is already solved'. This is an unsupported over-promise outside navigation-class partners. For weather, shopping, news and social apps the user is not in the app while driving. F1 is not sold there. F2/F3 on-open then depends on the user opening the app while inside or near a site, and that probability appears nowhere. M0 'works' only if the app is opened at the right place. The 'common case' is not quantified, and no segment-by-segment share of ad inventory is given.

**Fix:** Say: 'solved for continuous-location partners (nav, ride-hail, transit, EV navigation); for open-occasionally apps, F2/F3 delivery depends on app-open behaviour, which we have not measured.' Add a pilot metric: share of impressions-eligible sessions where fixes were flowing during the approach window.

### C4. [IMPORTANT] The 'Winner by speed and OS' table lists S3J (100%) and S2x (93-99%) as the answer when 'a background location session' exists

Status: addressed in sections 1 (Lever B), 3.3, 5 (strategy table, no-burst paragraph, winner table, qualifiers). Accepted residual: the no-burst figure (0.0%) is the critic's quick hack, not a faithful model, and the claims about iOS timers and Android Doze/exact alarms are [U] (not fetched).

The 'Winner by speed and OS' table lists S3J (100%) and S2x (93-99%) as the answer when 'a background location session' exists. Neither is buildable for hosted ads. S2x requires hold-awake location after a geofence wake. On Android that means a location foreground service started from the geofence event. I confirmed this is exempt from the background-start restriction today, but Google Play removes geofencing as an approved FGS use case on 2027-01-27 [V], and the playbook rates it do_not_use. If the partner already runs a continuous location session, that is M2/S4 and the rings are redundant. S3J also needs honoured background timers. iOS has no timer that wakes a suspended or terminated app. Android Doze and standby delay or ignore inexact alarms, and exact alarms need a special permission. The playbook neither verifies nor discusses any of this. The S2/S2s figures (58-80%) also assume a 60-90 s high-accuracy burst with the first fix 3-10 s after the wake, which the playbook admits is unavailable in the Android background. As a crude check, I set the burst to ~4 s (the wake fix only, with the 3-fix confirm kept) and Android 15 m/s isolated S2 recall fell to 0.0%. This is a quick hack, not a faithful model, but it shows the ring-only numbers are an unreachable upper bound, not a degraded mode.

**Fix:** Relabel the background-session column 'not available to hosted ads / partner-owned = M2'. Delete S3J from the winner table, or mark it 'requires honoured timers: [U], infeasible on iOS'. State the ring-only no-burst figure explicitly (effectively near zero for narrow windows). That strengthens the conclusion and removes the temptation to quote 80%.

### C5. [IMPORTANT] The 'ring-only is bad' numbers depend on the stationary-inclusive latency model N1, which is the [U] part

Status: addressed in sections 1 (Lever B), 2 (moving-device caveat) and 5 (winner table). Single figures (80%, 65%) removed; bands taken from results_sens.csv (nominal to 0.5x latency). Accepted residual: band ends come from two latency scales only, and cells can be non-monotonic by up to 5 points.

The 'ring-only is bad' numbers depend on the stationary-inclusive latency model N1, which is the [U] part. The Android doc (fetched) says 'Usually the latency is less than 2 minutes, even less when the device has been moving', and '2-3 minutes on average' under background limits; 'up to 6' is for a stationary device. N1's p50 2.8 min and p95 6.5 min sit at or above the top of the driving case. results_sens.csv shows how sensitive this is. Android isolated at 15 / 25 m/s: S2s is 79.7 / 73.2 at nominal latency but 90.6 / 92.1 at 0.5x latency. S2 is 58 / 57 at nominal but 85.9 / 66.7 at 0.5x. The playbook's wording ('lifts ring-only recall by about 8 to 28 points... does not make a narrow window reliable') understates this. 'Ring-only best case 80% / 65%' in the winner table is a function of an unverified latency model. The negative conclusion survives only because the burst is unavailable (see the gap above), not because of latency.

**Fix:** Present ring-only recall as a band (nominal N1 to 0.5x) and attribute the 'does not work' verdict to burst/background-fix unavailability plus the ~3.6 km wake-position spread. Do not quote single figures such as 80% or 65%.

### C6. [IMPORTANT] Hidden simulation assumptions beyond the ones listed

Status: addressed in section 5 (box "What this simulation cannot say", after the window-width paragraph) and section 3.1 (pre-arm row marked upper bound). Accepted residual: path uncertainty (heading-only prediction with turn-offs) and per-device correlated latency are still not in the simulation.

Hidden simulation assumptions beyond the ones listed. (i) The path is oracle-known: 1-D, straight, constant speed, and every anchor lies on the road being driven. The smart burst end uses the true speed and the true entry distance to the next zone. There are no turn-offs, so no wasted rings, and no iOS slot waste. (ii) OS delays are drawn i.i.d. per ring; in reality latency is correlated per device and session, so per-user outcomes are bimodal. (iii) No dropped events (reboot, Play services update, location toggle, iOS monitoring only after first unlock [V]). (iv) 'Isolated' means regular 10 km spacing, which the report itself shows can be reached by the 95th-percentile wake spread, so it is not isolated at 25-30 m/s. The decision table nevertheless quotes 'S2 delivers 94 to 98% of a 5 km window on both OSes' with no caveat. That figure assumes the oracle path and the 60-90 s burst (the wake-time burst is not supportable in hosted M3 per the playbook's own decision 2).

**Fix:** Add a 'what this sim cannot say' box next to the pre-arm row. Mark the 94-98% as an upper bound under oracle path and burst. Add path-uncertainty (heading-only prediction with turn-offs) before any pre-arm claim is used for planning.

### C7. [IMPORTANT] Privacy exposure from the recommended SDK inputs and report fields

Status: addressed in sections 7 (report fields withdrawn), 10 (decision 4) and 10A (privacy guards, speed-band targeting as a COUNSEL item). Accepted residual: architect and COUNSEL sign-off pending; pack-fetch granularity for corridor signposts is still to be specified in the pack design.

Privacy exposure from the recommended SDK inputs and report fields. (a) Proposed report fields lead_bucket and fire_mode: fire_mode=route plus lead_bucket tell the server that a specific impression came from a user on a navigated route passing anchor X at a given speed class. That is a mode and trajectory signal beyond 'an impression occurred', and it is low-k in sparse regions. The invariants forbid the server learning where the user is. The playbook defers this to architect and COUNSEL but proposes the fields anyway. (b) 'Route pre-matching at route.set' is fine on-device only if the signpost pack is fetched by region or tile. If packs are ever fetched by corridor, the partner's polyline is sent to the server. The playbook never states the pack-fetch granularity for corridor signposts. (c) route.set and route.progress put the user's full planned route and ETAs into the SDK. The playbook says the latch is 'in memory only' but says nothing about routes (disk persistence, logs, crash reports). The ride-hail destination is truncated to the last 1.5 km, but nav apps are not. (d) The surface label in_vehicle_passenger and the partner-fed motion hint are inferred sensitive attributes. Speed-band targeting lets senders target 'driving' users, which is ephemeral but is still context-based profiling for COUNSEL.

**Fix:** Add invariants: RouteIntent and motion hints are never persisted, logged or sent. Pack sync is tile- or region-level and never route-shaped. fire_mode and surface are not reported, or are reported only as aggregates above a k threshold with no device or session linkage. Record 'speed-band targeting' and 'driving-state inference' as COUNSEL items.

### C8. [MINOR] Missing lever for F3 on Android: GEOFENCE_TRANSITION_DWELL with a loitering delay

Status: addressed in section 3.2 (new DWELL row) and 8.2 (T4 extended). Accepted residual: DWELL latency and verify-fix freshness remain [U] until T4.

Missing lever for F3 on Android: GEOFENCE_TRANSITION_DWELL with a loitering delay. The geofencing doc (fetched) recommends it for 'stops inside a geofence for a given period', and INITIAL_TRIGGER_DWELL for the same purpose. It matches the 'at the stop, still stopped' semantics, filters drive-by wakes, and makes lateness less harmful. The playbook uses ENTER plus a verify fix and never mentions DWELL. The Android dwell/loiter latency is [U], but the API is documented.

**Fix:** Add DWELL to the Android lever table for F3. Test it in T1 or T4 (dwell latency and verify-fix freshness).

### C9. [MINOR] Evidence-tag inconsistencies

Status: addressed in the preamble (provenance paragraph) and sections 3.2 (PendingIntent row now [U] for the variant, Play dates, Play sentence), 10A and 11. Accepted residual: the Play "sole purpose of advertising or analytics" sentence stays [V-sec] and COUNSEL until support.google.com can be read.

Evidence-tag inconsistencies. (a) The lever table marks 'Periodic FLP updates in background (PendingIntent) ... a few times each hour [V]', but section 11 lists 'whether a PendingIntent FLP request is throttled' as [U]. The Android background-limits page I fetched says only that the location system service computes a new location for background apps 'only a few times each hour', and does not mention PendingIntent. The do_not_use decision still stands. (b) The playbook states it fetched nothing itself, so its [V] tags are carried over unchecked. I re-verified the ones below. (c) The Play date doubts ('2026-08-26 / 2026-10-28... FGS change may already apply') can be closed. The current developer.android.com/distribute/play-policies timeline shows geofencing FGS removal only under January 27, 2027. The August 26, 2026 heading contains only anonymous-chat and child-safety items, and there is no October 28, 2026 heading. The 'solely for advertising' Play wording is still only V-sec, because support.google.com was unreachable in this environment (DNS failure).

**Fix:** Fix the PendingIntent tag to [U] (conclusion unchanged). Remove the 'may already apply' caveat and the unreconciled dates. Keep the Play 'sole purpose of advertising or analytics' sentence as [V-sec] and flag it COUNSEL.

### C10. [MINOR] Inconsistencies in the firing section

Status: addressed in section 4 (fire-rule note on t_comp, realized-lead wording) and 8.2. Reconciled by reading lead_sim.py: the fire threshold uses t_comp = 3.0 s and the card is modelled as visible 1.2 s after the firing fix, so they are two parameters, not a stale comment; simulated steady-road leads run about 1.8 s above target. Accepted residual: the dropout and speed-invalid scenario is still to be added to lead_sim.py (this revision edits the playbook only); real fix-to-render time is [U].

Inconsistencies in the firing section. (i) 'Realized lead is bounded below by the target because drivers slow down rarely speed up' contradicts the playbook's own table: arterial p10 is 31.6 s and cyclist p10 is 38.9 s, both below the 40 s target. (ii) lead_sim.py's docstring says the card appears 1.2 s after the fix, while the playbook's t_comp is 3 s (a stale comment or a changed default: check which value produced lead_out.json). (iii) The lead_sim has perfect along-path distance, 5 m position noise, and no GPS dropouts, canyon error or speed-invalid fixes (iOS speed -1 at start). (iv) The arterial result ('19% get more than 2x target') shows an early-fire risk. That is exactly the 'card appears at the wrong place' failure for a roadside format.

**Fix:** Change the wording to 'bounded below on steady roads; 10-20% below target on arterials/bikes'. Reconcile the 1.2 s versus 3 s t_comp. Add a dropout and speed-invalid scenario to the lead sim before publishing realized-lead ranges to senders.

### C11. [MINOR] Policy framing for hosted background is thin

Status: addressed in section 10A (2.5.4, 5.1.5, 5.1.2(i), 2.5.18 quoted from the re-fetched guidelines, each COUNSEL for the ad-specific reading; 2.5.18 named as the formal reason for no notifications or widgets). Accepted residual: ad-specific interpretation is COUNSEL.

Policy framing for hosted background is thin. The playbook says Apple 5.1.5 / Play risk is 'unresearched' for iOS, but the guidelines were available. Verified today: Apple 2.5.4 limits background services to their intended purposes, with 'location' on the list. 5.1.5 says location must be 'directly relevant to the features and services provided by the app', with consent before collecting, transmitting or using location data. 5.1.2(i) allows sharing collected data with third parties 'to improve the app or serve advertising', with permission. 2.5.18 confines display advertising to the main app binary, not notifications, widgets, extensions or App Clips. This directly supports the 'in-app slot plus inbox, never OS notification' invariant and constrains any future CarPlay or widget surface. None of this is cited.

**Fix:** Add 2.5.4, 5.1.5, 5.1.2(i) and 2.5.18 as [V] anchors to the policy section, each marked COUNSEL for ad-specific interpretation. Note that 2.5.18 is the formal reason hosted ads must not become notifications or widgets.

