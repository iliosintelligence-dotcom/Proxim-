> **DRAFT - critic review complete, revisions pending (2026-10-10).** The critic's verdict: *not ready as written*. 3 blockers, 8 important, 4 minor, plus 7 invariant-accounting problems (see the end of [`PLAN-REVIEW.md`](PLAN-REVIEW.md)). The three blockers: (1) the billing waiver below a noisy threshold lets a sender who splits one campaign into many small signposts pay for about a third of their views; (2) shipping only some wordings of a family cannot be verified under one signature and a 4-byte id; (3) the AI fact-extraction step contradicts the new LLM-privacy invariants. The headline claim 'no existing invariant is weakened' is also inaccurate, and v1 is too large for a very small team. **This text does not yet incorporate those corrections.** Nothing here is legal advice.

# PLAN-ADDENDUM-02: Sender-facing product

Campaign builder, creative families (variants), on-device rotation, sharing, delivery and conversion measurement

Status: proposed, for owner decision. Date: 2026-10-10. Scope: advertising only, hosted mode (third-party senders; in-app slot plus inbox). Owner mode and resorts stay parked. Builds on PLAN.md, PLAN-REVIEW.md and PLAN-ADDENDUM-01.md. Where this document conflicts with them, this document says so explicitly (section 9 and the plan-change lists).

Evidence tags: [V] verified on a primary source a specialist fetched; [V-sec] secondary only; [M] measured by a specialist; [MODEL] simulation or arithmetic on stated assumptions; [U] unverified; [MODEL arithmetic] small calculation I added while synthesising. I fetched and measured nothing new for this document. All tags are inherited from the five specialist reports (campaign builder and AI; rotation and fatigue; measurement and conversions; privacy and AI policy; abuse and fraud). I read only the addendum's invariant list to avoid numbering clashes. Legal conclusions are not drawn here; COUNSEL marks items for qualified review. Numbers tagged [MODEL] or [U] are for the team and must not appear in sender-facing copy (addendum 2.6, `check-promises`).

---

## 1. What changes (one page)

**What a business can do in v1**
1. Create a campaign with AI help or by entering and uploading its own copy. Both paths feed one structured editor. AI is an untrusted author: its text passes the same validators, sender confirmation, human review and signature as hand-typed text.
2. Turn the campaign into a **family**: one base message plus up to 5 other wordings (6 creatives; 4 for new, unverified senders; 3 for health classes). A family sits inside ONE signpost revision, so it is reviewed once, signed once, and has one set of caps, dismiss, hide and report controls.
3. Lock the facts. Price, offer, dates, hours, phone, address and promo code are typed, sender-attested "facts". Wordings may only reference them. A deterministic check, not the AI, enforces this.
4. Share. Teammates, agency staff and clients join as members (guests expire). Read-only report and preview links are expiring snapshots. "Publish" means approval, signing and the pack head advancing for the chosen partner categories.
5. See honest numbers: delivered (billed viewable impressions), taps, saves, dismiss and hide rates, per-wording counts when volume allows, and redemptions the sender reports against a static per-wording code.

**What changes on the phone.** The device picks which approved wording to show using a pure, integer-only, least-recently-shown rule over its own day-level ledger. Nothing about the choice leaves the phone.

**What we promise and do not promise**
- Promise: "eligible every weekday when a partner app is open on your road" and "even rotation with no back-to-back repeat on a phone".
- Do not promise: "delivered every weekday" (the commuter must have a partner app open, consent, a free slot, pacing and caps passing), "no ad fatigue" (the same wording returns about every 6th exposure; only new angles and refresh revisions fix fatigue), per-person frequency or unique reach (no identifier exists), or "which wording won" for small senders (volume is far too low).

**What the plan gains**
- Invariants: I17 extended. New I21 to I25. Billboard principle clarified. I2a gets an AI clause. I3 stays unchanged in v1.
- Pack: a tier-0 pack is chosen by rank then packed by bytes. It no longer holds a fixed 500.
- Billing: bill on the released weekly total (sender sees what they pay). This deliberately declines the PLAN-REVIEW #6 suggestion of exact counts.
- Gates: G-V (variants and conversions), G-AI, G-SHARE. The M13 reconciliation gate moves from 1 percent to 2 percent median and 5 percent p95, and "CTR gap vs behavioral baseline" stops being pass/fail.

**Honest limits in one breath.** Presence on a road is unverifiable. Two partner apps mean two ledgers. Reinstall resets rotation. A median pilot sender sees about 119 viewable deliveries a week and about 0 redemptions in six weeks [MODEL, inputs U]. Variant-level winners need thousands of impressions per wording. Conversion analytics worth the name needs roughly 16 times the pilot audience.

**Effort.** Roughly 25 to 42 person-weeks gross for everything in v1 scope [U, judgement; the builder's 17.5 to 31 and the abuse report's 15 to 26 overlap and partly replace the single-creative editor already in M7].

---

## 2. The owner's commuter scenario, day by day

Cast: Maya drives Route 9 on weekdays. Brew & Co wants its ad "every weekday". Maya has a navigation app (App A, opened most mornings) and a parking app (App B, opened some days). Brew & Co's family has 6 wordings, cadence preset "every weekday" (1 per day, 5 per week).

| When | What happens | What Proxim sees | What Proxim does NOT see or cannot guarantee |
|---|---|---|---|
| Week 0 | The café gives facts (price, hours, address, code prefix), picks the road, Mon to Fri 06:30 to 09:30. AI drafts a base and 5 other wordings from the confirmed fact sheet. The owner accepts each. One review item shows all six. A human approves; one signature; the pack head advances. | The sender's submission, claims, review and signature | AI suggestions are priors, not a traffic forecast. "Live" means partner apps pick it up within about a day (soft TTL 24 h) |
| Mon wk 1, 08:05 | Maya drives; App A is open. The device matches the approach, passes filters and caps, wins the slot. `pick_variant` chooses by this install's own order. She is driving, so only the 7-word glanceable shows (for example "Fresh roast, 1.2 km ahead"). After 3 s of display the device records a viewable impression and queues ONE padded event (version index, day, no hour or cell) with a random 1 to 24 h delay | Later: one unlinkable event: signpost, rev, version index, `view`, UTC day | Who Maya is, the hour, the cell, which road she was on within the ring, whether this is her 1st or 20th exposure |
| Tue | She does not open any partner app (or the slot is taken, or pacing declines). Nothing is shown. Rotation does not advance | Nothing | That she passed at all. "Every weekday" was eligibility, not delivery |
| Wed | The device shows the least-recently-shown wording, never the one shown last. She may tap "save" on the glanceable; the code chip is only on the card, which she sees in Saved signposts later | A `view` and possibly a `save` event with version index | The wording she saw is not linked to Tuesday or Monday on the server |
| Thu | She opens App B near the car park. App B has its own ledger and salt and may show the same wording she saw in App A the same morning | Two independent installs' events | Cross-app repeats and same-day duplicates cannot be prevented or even detected. About 16 percent same-day duplicates in the builder's two-app model [MODEL] |
| Fri | She walks in and shows "BREW10-C" from her Saved card. Staff tap +1 on the Redeem desk for that code | A count for (BREW10-C, Friday) entered by the café, "reported by you" | Not linked to any view, device or Maya. Not used for billing. A code is shared by everyone who saw that wording |
| Week 2, Mon | A wording returns after about 6 exposures (8 to 10 calendar days; mean gap 8.4 days, p90 10 [MODEL]). Never back-to-back, never within 3 days when the family has 4 or more wordings and the commuter opens App A daily | Same event types | Whether Maya "feels" repetition; the weekly cap of 5 keeps frequency bounded |
| Week 2, Tue | She dismisses the card. The whole family rests 7 days. The dismissed wording is held back for about two further exposures afterwards. If she hides it, the whole family is gone for good; variants cannot get round hide or dismiss | `dismiss` or `hide` event (version index) | That the same person is also annoyed in App B |
| Reinstall | The ledger resets. The first ad equals the last one with probability 1/N (16.6 percent at N=6) [MODEL] | Nothing | That it happened |
| Week 4 | The console reminds the café to refresh. A new revision with new angles goes through delta review. New wordings show first because the ledger keys on content hash | A new revision | Whether fatigue is real; only evenness of rotation is visible |
| Reports | Week 1's billed total appears at week end plus 8 days (the 7-day outbox TTL). A two-week per-wording table appears 8 days after the period if cells clear the threshold. Redemptions appear as entered | Released, noisy, thresholded cells | Per-person reach or frequency, hour, partner app, geometry, raw events. Per-wording CTR for a small sender is "not enough data" |

Takeaway: the café gets rotation without back-to-back repeats on each phone, honest counts of billed deliveries, and its own redemption tally. It does not get a guarantee of daily delivery, proof that Maya never saw the same wording twice across two apps, or proof which wording sold coffee.

---

## 3. Campaign creation

### 3.1 One editor, two entry paths
- **AI path (sender tier 0 and above, after gate G-AI).** Structured brief wizard (one question per screen, autosave): the offer facts, where and when (place search, road, address entry as the non-map path), goal and tone, optional "describe it in your words" box (600 characters, structured by the model, every field confirmed by the sender). The model drafts a base plus up to 5 other wordings with an `angle` each. The sender must accept each draft (`accepted_by`). No autopilot.
- **Upload path (v1).** Manual form with live counters; "import from your verified website page" (fact candidates the sender ticks); duplicate a campaign; Proxim-written starter templates (at most 20, token slots, no user content). The console says plainly: "Your ad is text plus one small picture. We will help you turn a flyer into that." Finished banners, HTML, video, SVG and animated images are not accepted.
- **Later in the upload path.** Flyer, menu or PDF to fields extraction and CSV bulk for multi-location are v1.1 (they need the injection corpus and PII gate first; they are also the first cuts if over budget).

### 3.2 Facts (claim-locking)
Facts are family-level, typed, attested and evidenced: `brand_name, price, original_price, discount_pct, offer_start, offer_end, promo_code, phone, address_line, hours, exit_or_road_ref, url`. At most 12 per revision.
1. **Prompt side.** The model never receives or emits literal digits, currency, percentages, dates, URLs, phone numbers, emails or entity names. It writes placeholders (`{price}`, `{offer}`, `{hours}`, `{address}`, `{phone}`) and the server substitutes attested values before storage. The signed text is final text; the SDK stays a dumb renderer. This is the abuse report's grammar rule, adopted over the builder's looser "price as a value in the prompt". It also keeps addresses and phones out of third-party prompts (I2a).
2. **Fact-span lint on the expanded text of EVERY creative**, including human-typed ones: any number, currency, percent, date, time, phone, URL, address-like span or number word ("half off") must resolve to an attested fact. Unresolved: plain-language error ("This version says $4. Add a price fact or remove it"). New facts are declared once at family level, never inside a wording.
3. **Qualitative-claim lexicon** (superlatives, "best", "#1", "guaranteed", "free" without an offer fact, "award-winning", comparatives, medical and financial claims, testimonials, ratings, urgency tricks, second-person statements about habits or location history such as "back again?", "your usual road"): blocked, or forced to the reviewer with a flag. Each AI sentence must cite `used_fact_ids`.
4. **Dated claims bind validity.** `valid_until = min(flight_end, end of offer_end day in campaign time zone)`, signed in the envelope.
5. **Text in images.** OCR on every uploaded image as an advisory flag; any fact-like span must match a fact. The reviewer also sees the image at final size.
6. **Advisory LLM check** lists "new assertions vs the base" and can only add flags.
- Residual: a fact-free semantic claim ("kid friendly", "open late") can pass the lint. Lexicon, assertion list and the human reviewer are the controls.

### 3.3 Creative fields, brand kit, images
- Per creative: `headline` (40 characters), `body` (90), `glanceable` (7 words and 40 characters; mandatory, because the driving surface shows only this), `cta_kind` (closed list: directions, call, url, save, book, order, menu, offer), `image_ref` (hash or none), `icon_id` and `layout` (SDK-owned closed sets; layouts text, text_logo, text_image), `angle` (closed enum), optional `promo_code` (references a fact). Word counting: whitespace split, numerals count as words.
- **Angles in v1:** benefit, convenience, price (needs a price fact), location_cue, question, plain_info. Excluded in v1: social_proof, scarcity, comparative, superlative, and time_cue (time_cue needs per-creative date windows, a v1.1 feature).
- Landing URL, sensitivity class, sender identity, geometry and cadence are family-level. A wording cannot differ in any of them.
- **Brand kit (small):** logo, display name (must equal the verified name the SDK renders), tone preset (friendly, professional, playful, calm), banned phrases, facts library, default CTA. No custom colours or fonts; partners own theme tokens and the SDK owns chrome. (The rotation report's `accent_id` is dropped from v1; confirm with design owners.)
- **Images:** server-side re-encode to WebP at most 40 KB (picture) or 64 KB (logo), EXIF stripped, perceptual hash against known logos and the org's own, OCR flag, content-addressed asset row with rights attestation. At most 2 distinct images per family; logo shared by hash; images ship only for ranked-in signposts. Every wording must also render text-only.
- **Attestation at submit:** "I have the right to use this text, logo and images, and every fact is true", recorded by user and time. Any fact change needs re-attestation.

### 3.4 Quotas and cost (provisional policy data [U])
LLM prices [V per builder and abuse reports]: Sonnet 5.5 $2 in / $10 out per MTok; Haiku 5.5 $0.10 / $0.50. A 6-wording generation costs about $0.015 to $0.024 on Sonnet [MODEL, assumed tokens]; a full campaign about $0.09, about $0.20 with retries; about $400 a month for 100 orgs by 20 campaigns [MODEL]. Honest spend is small; the quotas exist to stop abuse (an unmetered script at 1 request per second, max-size, is about $6,900 a day [MODEL]).
- Per call: input at most 6k tokens, output at most 1.5k, instruction text at most 500 characters.
- Sender tier 0 (email-verified): 2 free generations, then 5 per day and 20 per month, $0.25 a day. Tier 1 (domain proof): 30 per day and 300 per month, $2 a day. Beyond the 2 free, generation needs tier 1 or a card.
- Per user 10 per minute; per IP 60 per hour; at most 3 new orgs per IP/ASN per day; first 24 h of a new org at half quota; per-org dollar meter; global daily circuit breaker at 10 times expected honest spend (floor $20).
- Queue caps: 25 pending creatives per org (sender tier 0: 8); sender tier 0: 10 submissions a day and 3 live signposts.
- Model IDs and prompt template versions are pinned policy data; a model upgrade never regenerates or edits approved text; an LLM outage degrades the console to the manual editor.

### 3.5 AI pipeline (server only) and injection defence
1. **Two stages.** (a) Extraction (Haiku-class, no tools, no network, schema JSON out) turns pasted text or the verified-domain page into candidate facts. The sender must tick each fact with its source quote. (b) Generation sees only the confirmed fact sheet plus tone options. Raw documents never reach the generator.
2. **Structured outputs.** The documented schema subset does not support length bounds, and a refusal or max_tokens stop can return invalid text [V]. So lengths are enforced after the call by Zod; one targeted repair call for failing fields; still invalid means dropped; text is never truncated.
3. **Diversity gate.** Reject near-duplicate wordings: headline difflib distance at least 0.35 (hand-written pairs had min 0.48, lazy paraphrases 0.02 to 0.26 [M]) and distinct glanceable strings. Calibrated on 150 hand pairs only [U].
4. **Verified-domain import only in v1.** SSRF-safe shared fetcher (https only, private/link-local blocked after resolution, no off-domain redirects, 2 MB, 10 s, text/html, no JS, 5 fetches an hour per org).
5. **No coordinates, polylines, cells, end-user data, report rows or customer data in any prompt** (I25). Targeting suggestion is deterministic (search plus engine recipes plus category defaults); the model only parses free text and explains. The endpoint is tagged `x-proxim-coords`, POST, no-store, bodies excluded from logs, OpenTelemetry and Sentry.
6. **Moderation advisory is a separate call and prompt**; it may add flags, never remove them or approve.
7. **Eval gate.** At least 100 briefs (including adversarial ones such as "say it cures anxiety", "add a 5-star review", "make the price $1") and at least 40 injection fixtures gate any model, prompt or schema change.

### 3.6 Staging of AI
| Stage | Content |
|---|---|
| v1 (M7a ships manual first; M7b adds AI, external use after G-AI) | Brief wizard; fact sheet; base plus up to 5 wordings; regenerate; verified-domain import; quotas; validators; eval and injection corpora; targeting explanation (first cut if over budget) |
| v1.1 | Flyer, menu, PDF to fields with quarantined extraction and PII gate; CSV bulk |
| Later | AI refresh suggestions from released aggregates (still reviewed); localisation (needs multilingual moderation); AI product-scene backgrounds under strict conditions (no people, logos or text; indemnified vendor; provenance row) |
| Not building | Image generation in v1; chat-style open agent; AI auto-publish; on-device or serve-time LLM; per-user generated copy |

---

## 4. Variants: the creative family model

### 4.1 Shape
A signpost revision holds `creatives[0..5]` (idx 0 base). One signature (65 B) covers the whole envelope. Decisions:
- **One revision, not N signposts and not N signatures.** Dismiss (7-day cooldown), hide and report only mean something if all wordings share one signpost identity; caps (2 per day, 5 per week per signpost; 3 per day per sender) must not multiply by N. One review item, one audit record.
- **Per-variant signatures rejected.** They cost 23 to 30 percent more bytes at N of 4 to 8 [MODEL] and are unnecessary: the family hash is covered by the single signature, so the splice attack the abuse report worries about (moving a signed variant from family X to Y) does not exist.
- **Concurrent revisions per slot rejected** (abuse report). Adding or changing a wording is a new immutable revision that supersedes the previous one; delta review keeps that cheap. Waves or flighted variant sets are later.
- **Content id.** `vid` = first 4 bytes of SHA-256 of the canonical creative bytes. It is a device ledger key only; the compiler rejects collisions within a family. Events carry the revision-scoped version index (0 to 5, 3 bits).
- **Caps (policy data [U]).** Creatives per revision: 6 (sender tier 1 and above, general class); 4 for sender tier 0 (the minimum that gives zero repeats within 3 days for a weekday commuter [MODEL], so lower is a deliberate trade); 3 for `health_adjacent` and `health_facility`, manual authoring only (no AI generation in health or regulated classes). Claims at most 12. Images at most 2. Format reserves 8 slots for later. Review queue and pack bytes grow linearly while gains beyond 6 are slow [MODEL].
- **Not sender-configurable in v1:** rotation order, weights, sequencing ("B only after A"), exposure-history targeting. Sequencing would make the creative depend on seen-history beyond even rotation.

### 4.2 Review model: what a human approves, what is automated
| Layer | Who | What |
|---|---|---|
| Envelope | Human, once per envelope version | Category and sensitivity class, geometry (existing rules), schedule, landing origin and at most 4 exact URLs, facts with evidence, forbidden terms, asset set, brand names. Any change creates a new revision |
| Each creative | Human, v1 = 100 percent | Rendered in the real card and glanceable renderers, normalised text, diff against the base and against the last approved revision, lint hits, fact-containment results, image at final size. Explicit decision per new or changed creative, no "approve all", minimum dwell time |
| Inherited creatives | Human approval already exists | A creative whose content hash, facts hash and landing domain are unchanged since the last human-approved revision is collapsed ("inherited approved") but still re-linted with the current ruleset. Any ruleset, fact or landing-domain change cancels inheritance |
| Automated | Machine | Validators V0 to V9 (charset: NFKC, no bidi or zero-width; length; digit and URL ban in AI spans; fact containment; brand and impersonation lexicon with lookalike match; sensitivity and age-restricted lexicon; near-duplicate and drift; landing consistency; advisory entailment); URL scan and Web Risk; OCR flag |
| Always human, never auto | Human | health_adjacent or health_facility; anything tripping the health lexicon (reclassifies the whole family, escalate-only); finance, credit, insurance, crypto, gambling, alcohol, tobacco, cannabis and other age-restricted categories (prohibited in v1 because there is no age signal; COUNSEL); any new landing domain or URL; any unfurnished price, discount, free, guarantee, comparative, rating, award or statistic; any new image; government, hospital, bank, carrier or utility names; first N families of an org; orgs with an account-risk flag; agency-submitted revisions within 72 h of a recovery or factor change; anything the advisory LLM flagged |

- **Subset approval.** If the reviewer approves 4 of 6, the system writes a derived revision r+1 containing only the approved creatives (`derived_from_rev`, `derived_by='review_subset'`) with hash-identical rows, and signs r+1. We never sign bytes different from a reviewed revision.
- **Reviewer protections.** Flags first; Approve disabled until every non-inherited creative has been viewed; blind canary variants seeded into the queue (target catch rate at least 95 percent); a second reviewer samples 10 to 20 percent during the pilot; a user report concentrated on one wording quarantines the revision (section 10).
- **Hosted auto-approval lane.** Not in v1, not extended from the owner lane. Reconsider after the pilot only if reviewer cost justifies it (saves about 38 percent at 500 senders [MODEL]) and a corpus gate shows zero false approves. The LLM may only escalate to a human, never de-escalate.
- **Reviewer load [MODEL, assumptions U].** Family batch review: 12.5 reviewer-hours a month at 15 senders (0.12 FTE), 83 at 100, 414 at 500 (3.9 FTE). Independent per-variant review would be 22.5, 152 and 748. The builder estimated family review is 3.8 times cheaper than N separate reviews, the abuse report 1.8 times; both are assumptions. Queue-flood by 200 shell orgs costs 8 to 27 reviewer-hours, hence per-org WIP caps, verified-first priority and a cheap lint pre-reject.

### 4.3 Signature and immutability
- I17 is extended (section 9). The signature covers all creatives, `family_policy` (cadence), `claims_sha256`, landing URL, promo codes, asset hashes, `valid_until`, `expiry`, plus everything it covers today.
- The signer signs only if an independent DB read shows an approved review at the envelope and at every creative hash, and a deterministic fact-containment re-run inside the signer succeeds (it does not trust the API).
- `expiry` is at most `min(valid_until, 14 days for orgs younger than 90 days or 30 days otherwise)`. Renewal re-signs identical content into an append-only `signpost_revision_signings` table and requires a passing landing crawl and no flags. Revisions stay immutable.
- No dynamic creative: no server-side macros, no on-device generation, no per-user text. Rotation order is unsigned policy and can change which approved wording shows, never what it says.

### 4.4 Pack-size budget and N cap
- **Measured marginal cost** per extra creative: 40 to 52 B gzip on 60 hand-written creatives in 10 families [M]. Planning figure for the compiler: **110 B per extra creative** (rotation report; deliberately pessimistic 0.65 compression ratio, and real AI text may compress worse) [MODEL, U]. Family bytes F(N) = 515 + 110 N, including the 65 B signature.
- **Capacity of a pack tier-0 (350 KB gzip) [MODEL]:** 560 / 476 / 366 / 297 / 250 families at N of 1 / 2 / 4 / 6 / 8. At the measured 52 B the six-creative figure is about 387. So "top 500" was an N=1 number. Six separate signposts would cost about 4,200 B against about 926 to 1,175 B for one family.
- **Compiler allotment (deterministic, byte-identical output):** rank families as today; give every family 1 creative using 80 percent of the budget; spend the rest in rounds adding the r-th creative to every family whose `cadence_class` is D (daily or weekday, up to its ceiling), then O (at most 3 a week, up to 3), class S (single or event) keeps 1. A family that ships fewer than declared is marked `n_shipped < n_declared` and the console says "rotating with 3 of 6 versions in this area".
- **Does not bind at pilot scale** (probably at most 100 families in the metro, about 140 KB at N=8). It exists so the M2 size gate cannot be silently broken at metro density. M2 must re-measure with real copy and real AI output.
- Asset cache: 5 MB over 6 cells is about 853 KB per cell, about 21 images at 40 KB [MODEL]. At most 2 images per family and logo sharing keep one family to about 10 percent of a cell's image budget (PLAN-REVIEW #2).

---

## 5. On-device rotation and fatigue control

### 5.1 Algorithm (`select_variant`, new step 7b in `proxim-decide`, pure and integer-only)
Inputs: family record, ledger rows `(vid, count, last_day)`, `family.last_vid`, install salt, local day, locale. State changes only on a viewable `markShown`, so reopening the app, a second slot or a surface that was not ready returns the same wording (no reroll).
```
cn[v]  = (row[v].count * DECAY[day - last_day]) >> 16      # 14-day half-life constant table, 0 after 90 days
cand   = creatives of the locale minus family.last_vid     # exclude the last shown when N >= 2
deficit[v] = floor(BQ[k[v]] * Ctot / Bsum) - cn[v]         # k = 0 for all in v1
pick argmax deficit; ties by smaller SHA-256(install_salt | family | vid), then smaller vid
on markShown: count = min(65535, cn + 256), last_day = day, last_vid = vid   (skipped inside any sensitive buffer, R2/I6)
dismiss: +512 to that wording only, after the family rule has applied
```
- With a static set and no overlay this equals round-robin in every simulated cell; its value is robustness (no pointer to lose, handles set changes, dismissals, long gaps).
- A creative with no row is chosen first, so new wordings in a refresh revision show first, once. Keying on content hash also recognises unchanged wording across revisions.
- Driving mode shows only the creative's glanceable. The inbox row is bound to the vid chosen at queue time; opening the inbox does not advance rotation.
- Golden vectors on Rust, WASM, Kotlin and Swift. Integer-only so ports agree bit for bit.

### 5.2 Ledger
`variant_ledger(family_id, vid, count, last_day)` and `family_state(family_id, last_vid, last_day)`: whole days, rows purged when decayed count is below 13 (about 60 days), hard max 90 days, LRU at most 2,000 rows, excluded from backup, encrypted, wiped by consent withdrawal and `eraseAllLocalData`, listed in `privacyReport`, never written inside a sensitive buffer, readable only by the selector (reader allowlist enforced by `check-invariants`).

### 5.3 Interplay with caps, cadence and user controls
- **Cap unit is the family** (the signpost id). Per signpost 2 per day and 5 per week, per sender 3 per day, per app 1 per 30 minutes and 4 per day are applied before wording choice and are unchanged.
- **Signed `cadence`** (stricter-only): `max_per_day` 1 or 2, `max_per_week` at most 5, `min_gap_h`. Console preset "every weekday" = 1 per day, 5 per week. Reason: with the defaults of 2 per day and 5 per week, a commuter who passes twice a day with the app open each time gets an ad on only 61 percent of weekdays (Mon 2, Tue 2, Wed 1, nothing Thu or Fri). With 1 per day it is 100 percent at full open rate; 70 versus 91 percent at 70 percent open; 61 versus 64 percent at 40 percent open [MODEL].
- **True 7-day "daily" cannot be met** under the 5-per-week ceiling (owner decision 2).
- **"Repeat-exposure decay 0.5^seen_count" (PLAN 5.6 step 7) is clarified** as a per-wording ranking weight with a 14-day half-life. Across families it is only `S' = S x 2^(-c_min/256)`, a ranking factor, never a gate and never multiplied into `p_serve`. Read literally as a serving gate it yields about 5.3 / 4.8 / 4.0 exposures in 60 days against 44 / 30.8 / 17.6 opened commute days at p of 1.0 / 0.7 / 0.4 [MODEL], which would destroy "deliver daily".
- **User controls survive rotation.** Dismiss (7 days), hide (permanent), report (permanent for that sender) key on the family and are checked before wording choice. The selector reads the dismissed wording only to deprioritise it after the family gate passes; it can never re-enable a suppressed family. Golden vectors prove this. (This resolves the privacy report's "selector must not read dismiss state" against the rotation report's variant penalty: the penalty is allowed because it cannot raise exposure.)
- **Pacing** is unchanged and independent of wording. Daily-cadence families should normally run at `p_serve` = 1; a thinned day is a budget effect and the report says so.
- **Hosted background stays off**, so there is no wake-up delivery.

### 5.4 Simulation results (all [MODEL]; weekday commuter, 60 days = 44 commute days, 2,000 simulated commuters per cell, start Monday)
Single app, least-recently-shown (identical to round-robin). Shown: back-to-back repeat / same wording within 3 calendar days.

| Open rate per commute day | N=2 | N=3 | N=4 | N=6 | N=8 |
|---|---|---|---|---|---|
| 100 percent | 0 / 60.5 percent | 0 / 39.5 | 0 / 0 | 0 / 0 | 0 / 0 |
| 70 percent | 0 / 41.7 | 0 / 13.9 | 0 / 0 | 0 / 0 | 0 / 0 |
| 40 percent | 0 / 17.8 | 0 / 2.7 | 0 / 0 | 0 / 0 | 0 / 0 |

- Minimum N for under 5 percent back-to-back and under 10 percent within 3 days: 4 at 100 and 70 percent, 3 at 40 percent. Recommended default 6, cap 6 in v1.
- Mean days before the same wording returns at 100 percent: N=4 5.6; N=6 8.4 (p90 10); N=8 11.2.
- Uniform random never meets the thresholds: at N=12, 8.2 percent back-to-back and 15.6 percent within 3 days. Random with a no-repeat rule needs N of 12 / 8 / 3 (at p of 1.0 / 0.7 / 0.4) and spreads exposures unevenly (spread max-min about 5 against at most 1). A day-indexed stateless rotation needs N of 4 / 5 / 8 and cannot honour dismissals.
- The builder's independent simulation (90 percent open, N=6, weekly cap 5): least-seen has 0 percent back-to-back but **45 percent of exposures repeat a wording from the previous 7 weekdays** (uniform random: 17 percent back-to-back, 56 percent). So rotation guarantees no immediate repeat, not novelty. N=10 drops it to 13 percent.
- **Two partner apps (independent ledgers).** Rotation model, N=6, each app opened every commute day: 12.5 percent back-to-back and 41.3 percent within 3 days (88 exposures). At 70 percent open 11.5 / 29.2 percent. Builder model, N=6, 60 percent open each: 10 percent back-to-back, 16 percent same-day duplicates, 63 percent seen in the previous 7 weekdays (N=10: 6 / 10 / 40 percent). A larger N cannot fix this: under 5 percent back-to-back needs about N=16, under 10 percent within 3 days needs 25 to 32 at full open. Partitioning wordings across apps does not help. Cross-app capping needs an identifier (I5).
- **Reinstall:** first ad after reinstall equals the last with probability 1/N (24.9 percent at N=4, 16.6 percent at N=6).
- **Coverage under the caps:** see 5.3. **Overlay skew** and **bandit reweighting** are later (5.5).

### 5.5 Honest limits and what is deferred
1. Proxim cannot verify or report per-person frequency. The guarantee is per install and is shown by the simulator and on-device tests, not by production measurement (the exposure bits that would measure it are off in v1; section 7).
2. A user with two apps, a reinstall, a new device, or a headless host that never calls `markShown` (rotation and caps freeze; the TestKit must warn) defeats or freezes rotation.
3. Rotation fixes wording repetition only, not frequency fatigue or banner blindness. The cap of 5 per week bounds frequency. The console recommends a refresh revision about every 4 weeks.
4. A dense cell can ship fewer wordings than declared (4.4).
5. Variant-quality varies; the lint catches near-duplicates, not bland copy. Driving commuters only see 7-word lines, so wording variety lives in the glanceable, icon and layout.
6. **Later (fields reserved):** a weekly server overlay with weights -1/0/+1 computed only from released cells, applied only for N of 6 or more, guarded by a compile-time 60-day simulation (skew creates repeats: champion weighting at N=4 gave 11.6 percent within 3 days, which fails). It is inert at pilot volume: right 0 / 43 / 99 percent after 8 weeks at 200 / 1,000 / 5,000 impressions per family per week [MODEL, assumed tap rates]. v1 rotates uniformly.

---

## 6. Sharing and collaboration

Interpretation of "share": build (a) collaborate and share reports with teammates, clients and agencies, and (b) publish to partner apps via the existing submit, review, sign flow. (c) templates others can reuse is later.

### 6.1 Roles
Existing roles (viewer, editor, finance, admin, owner) plus on `org_members`: `external` (agency or client guest), `expires_at` (default 90 days, renewable, max 12 months), `invited_by`, `last_seen_at` (auto-suspend after 90 days of inactivity), a quarterly access-review email to owners, and a new `redeemer` role (Redeem desk only, no report access).

| Capability | viewer | editor | finance | admin | owner |
|---|---|---|---|---|---|
| View campaigns, reports, comments | yes | yes | yes | yes | yes |
| Comment and resolve | yes | yes | yes | yes | yes |
| Create/edit drafts, run AI, upload | no | yes | no | yes | yes |
| Request internal approval | no | yes | no | yes | yes |
| Approve an internal request (not own) | no | no | no | yes | yes |
| Submit to Proxim review | no | if `editor_can_submit` | no | yes | yes |
| Set budget, flight, top up | no | no | yes | yes | yes |
| Create report link | no | no | yes | yes | yes |
| Create draft preview link | no | yes | no | yes | yes |
| Invite members, change roles | no | no | no | yes | yes |

- **External members** are limited to viewer or editor (never finance, admin or owner), individually authenticated (no shared logins), email-verified, MFA step-up (aal2) for submit and share-link actions when an org has an external member, their revisions are flagged to reviewers. A person can belong to many orgs, so an agency is "a user invited into each client org". Money stays with the client org; the agency proposes and the client approves with a diff, including budget changes.
- **Removal cascade** within 5 minutes: sessions revoked, API keys and share links they created listed for rotation, drafts cancelled; audit log records actor and actor org.
- **Accountability.** The advertiser (sender org) is responsible for claims. An owner or admin accepts the advertiser attestation per campaign and on any fact change; an agency may prepare and submit under it. COUNSEL.
- Org-to-org delegation and cross-client agency dashboards are later.

### 6.2 Approval and comments
Internal approval (`approval_requests`, requester differs from approver, 14-day expiry, audit logged) is separate from Proxim moderation; solo owners skip it. Comments are org-internal (on signpost, revision or creative; 1,000 characters). Reviewer feedback from Proxim shows per creative on the status page and is not a comment thread. Internal approval and draft preview links are first cuts if over budget.

### 6.3 Links (capability URLs)
- **Report snapshot link.** Frozen `report_releases` rows for chosen campaigns (at most 20 signposts), default expiry 14 days, max 30, at most 10 active per org, optional view cap and passcode, instant revoke. Token is at least 128 bits (256 recommended), only the hash stored, carried in the URL fragment and exchanged by POST for a short-lived cookie so Referer, server logs and link unfurlers never see it. Headers `Referrer-Policy: no-referrer`, `X-Robots-Tag: noindex`, `Cache-Control: no-store`, no third-party scripts, separate cookie-free origin. Served by a `share_reader` DB role that can execute one SECURITY DEFINER function. Unknown, expired and revoked tokens return the same 404 with equalised timing; views and last view are counted, no IP stored.
- It never shows raw events, coordinates or geometry (place and road names only), partner-app breakdown, drafts, or arbitrary date ranges. Non-general classes show family-level cells only. A re-fetch returns identical numbers because releases are stored once.
- **Draft preview link.** Renders creatives (card and glanceable), the facts list, a plain-words targeting summary and schedule. No map and no approve button; approval requires an authenticated guest member. Default expiry 7 days, max 14.
- **Publish.** A state toggle: Draft, Needs approval, In review, Changes requested or Approved, Scheduled, Live, Paused, Ended. Controls are partner categories (navigation, transit, parking, EV/fuel, ride/delivery, fitness) with category and listed-partner exclusions. Partner policy (allow/block, sensitivity ceiling, caps) applies on the device. A required sensitive-category question routes to the health path. Console copy says "appears in apps within about a day". A sandbox test (simulated route, synthetic traffic only) precedes go-live.
- **Templates (c), later.** A copy creates a new draft, inherits no approval, sender name or domain. Not in v1 because facts must be stripped, trademark and copy-theft questions arise, and one approved template replicated widely is a spam multiplier. The `template_sha256` clone key is stored from v1 so reviewers can use it later.

---

## 7. Delivery tracking and conversion measurement

### 7.1 Funnel definitions
| Stage | Definition | Measured | Reaches Proxim |
|---|---|---|---|
| Eligible / matched, selected / queued, rendered | Device-side gates; matching and selection never leave the device | Device only | No (a "matched" event would be a presence event). Drive-it and TestKit show them in simulation |
| **Delivered = viewable impression** | At least 50 percent of the card's pixels visible for at least 1 continuous second, app foreground, screen on, not occluded, "Sponsored" label visible; glanceable fully shown for at least 3 s; inbox row 50 percent for 1 s. One `view` per delivery, de-duplicated on the device. Matches PLAN section 7 and the MRC display standard [V-sec; MRC text not fetched] | Device measures, server counts | Yes: `view` |
| Engagement | `tap` (with `cta`: web, directions, call, other), `save` (to Saved signposts; the only action while driving), `dismiss`, `hide` | Device | Yes, one event each |
| Landing | Page load in SFSafariViewController / Custom Tabs; no hop through Proxim | Sender's own analytics | Only if the sender enters optional daily sessions |
| Conversion | Redemption of a static per-version code, or sender-reported landing goal | Sender | As aggregate counts by (code, day) |

Rules: eligible, selected and rendered never feed billing. Landing and conversion never feed billing, ranking, Q or pacing (I23). CTR = taps / views. Saves are reported separately.

### 7.2 What is not measurable (stated in console and docs)
Store visits and footfall (needs post-exposure location; deferred in PLAN section 8); view-through conversion; cross-app or person-level reach, frequency and de-duplication; per-person journeys or multi-touch attribution; incrementality (redemptions cannot be linked to devices; switchbacks are far below detectable size for a single café); hour of day, position along the corridor, lead distance or cell; deliveries that did not happen. Reach and frequency are **not shown in v1** because the only identifier-free way (exposure-ordinal bits) needs an I3 amendment and is default OFF (7.8). A coarse "apps fetched your area" authoring hint from tile-fetch counts is later.

### 7.3 Events (additive to PLAN 6.2 item 3; closed enums, padded to a fixed size)
`{event_id, signpost_id, rev, var (0..5), type: view | tap | save | dismiss | hide, slot, UTC day, cta (tap only), sdk/os version buckets, attested, simulated}`.
- No token, device id, session id, hour, cell, IP or user id in the stored row. The ingest token is validated and discarded; replay and rate limits use `ingest_token_counters(token_hash, day, n)` with no event reference, purged at 72 h. Tokens must be single-use random nonces with no install component and issuance-capped (gate: this is a precondition for adding `var`, privacy report I1; if events were linkable, a 10-day commuter sequence over 5 wordings is about 23 bits [MODEL]).
- Not in v1: `render` sampling, `exp`, `rpt`, `exposure_bucket`.
- No `var` for `health_adjacent`; no events at all for `health_facility` (I7).
- Device: one event per request with the random 1 to 24 h delay; **outbox TTL 7 days** (the server rejects `day` older than 8 days); per-install daily event cap 16 [U]. The reason for the 7-day TTL: an app opened 0.3 times a day delivers only 52 percent of events with a 3-day TTL and 86 percent with 7 days [MODEL]. Low-cadence apps (parking, EV) are still biased low; this is reported per partner as "reporting completeness".

### 7.4 Pipeline
1. `POST /v1/events`: closed schema (unknown field 422), duplicate `event_id` rejected, attestation and rate checks, IVT flags; insert-only role; raw purge at 14 days (60 days under a dispute hold).
2. Nightly exact rollup into internal `event_aggregates_daily` (idempotent, recomputed while a day is open). **Never readable by console roles.** Feeds pacing, fraud, partner payout, reconciliation and the pack builder.
3. Release job, the only path to sender numbers (I22). Fixed schedule: weekly cube at week end plus 8 days; two-week cube at period end plus 8 days. No ad-hoc queries, no other releases. Each cell is released once and stored; noise is a deterministic function of the cell key and a KMS-held secret, so a crash and re-run yields the same value. Discrete Laplace with integer sampling (avoids the floating-point side channel [V-sec; integer sampler sufficiency U]). The display threshold applies to the NOISY value; below it the console shows "insufficient data". Negative values clamp to 0.
4. Weekly billing close from the released cell (7.7).
5. Daily reconciliation (7.9).
- Dashboard freshness is stated in the UI: numbers appear up to about two weeks after the day they describe. A "late arrivals" row shows weekly total minus any partial data.

### 7.5 Release design (a synthesis; parameters to be fixed by the G-V simulator)
This section resolves the biggest disagreement among the specialists: the measurement report sized noise to a single event (Delta = 1, cheap, accurate for small senders, but a weekday commuter contributes up to 5 events and shifts a weekly cell by about 1.8 standard deviations); the privacy report sized it to the device (strong, but hides impressions, needs history flags, and almost nothing releases at pilot volume); the abuse report proposed exact numbers above k = 20. The synthesis keeps the measurement report's cubes and schedule and sizes noise to the **per-device weekly cap**.

| Cube | Cell | Metrics: sensitivity Delta, epsilon, scale b = Delta/epsilon |
|---|---|---|
| C1 weekly | (signpost, week) | view: 5, 1.0, b=5 (sd 7.06); tap: 5, 2.0, b=2.5 (sd 3.51); save: 5, 1.0, b=5; dismiss: 1, 1.0, b=1 (sd 1.36); hide: 1, 1.0, b=1. View is the billing basis |
| C3 by version, disjoint periods | (signpost, version, period) | Same metrics and parameters; period 2 weeks by default (1 or 4 selectable at flight creation), never overlapping or nested, never cumulative |

- Standard deviations are [MODEL arithmetic] for the discrete Laplace with alpha = exp(-epsilon / Delta).
- **Threshold on the noisy value:** T = 20 (general class). Health_adjacent: family-level C1 only, T = 100, no version breakdown. Health_facility: none. G1/G2: one flight-total cell (addendum 9.1).
- Taps carry epsilon 2 because the sender sees taps in its own logs (PLAN 9.1 residual); noise protects what only Proxim knows.
- **Composition (honest).** A device is bounded by the caps to 5 views a week per signpost, so a cell's device-level epsilon is about 1. A weekday commuter appears in C1 and C3: about 2 per device-week for views, about 12 over a 6-week flight by basic composition [MODEL arithmetic]; the measurement report's event-level design implied about 10 per week and 60 over six weeks. The caps are enforced by the client and the same person may have two apps. **No differential-privacy or "anonymous" claim appears in any sender, partner, store-label or marketing copy.** The wording is "thresholded and noised, bounded per device by the app's caps". Per-signpost epsilon ceilings live in `release_policy` and `privacy_budget_ledger`; a trigger refuses releases beyond them; adding a cube needs an ADR. Sibling or overlapping signposts from one sender are not yet budgeted against each other (known gap; collusion between senders cannot be prevented).
- **What this costs small senders [MODEL arithmetic on the measurement report's pilot quantiles; inputs U].** P(weekly family view cell displayed): about 92 percent at a true 29 views (p10 sender) and about 100 percent at the median 119. A per-version cell over 2 weeks: about 100 percent displayed at the median sender (about 48 views per version), about 10 percent at the p10 sender (about 12). Noise at the median: about 6 percent of a weekly total, about 15 percent of a per-version two-week cell. Daily cells and cumulative running releases are not in v1.
- **Daily family-level cell (C2)** is deferred to v1.1. It is the first thing to add only if the G-V simulator (lone commuter, decoy farm at threshold, nested differencing, pilot cell survival) passes. This is a deliberate narrowing of the measurement report to meet the privacy report's concerns on composition and decoys.
- Anything in a public pack derived from reports (shrunk CTR in Q; later, overlay weights) is computed from released cells and quantised, never from exact aggregates.

### 7.6 Conversion mechanisms
**Per-version codes (v1).** One code per version per revision: `PREFIX-<letter>` (for example BREW10-A to BREW10-F; prefix at most 9 B stored once per signpost). The SDK renders a "Show code" chip on the card and in Saved signposts (not on the driving glanceable). Optional channel suffixes (`-P` print, `-S` social) separate a leakage baseline. Codes are unique per sender namespace, bound to (signpost, version) in the immutable revision, never reused, and expire with the revision. A QR of the code string for POS scanning is v1.1.

**Why per-delivery, per-device, single-use and per-tap values are banned (I21).** A unique code redeemed at a till joins a place and a day to a POS record or loyalty ID; the sender then learns that a named person was on that road on that day. It would be an identifier, a presence oracle, a data type Apple would count as tracking [V], would require per-device data in an otherwise identical content-addressed pack, and would give any code an anonymity set of one. Enforced by schema (`conversion_codes.scope = 'variant'`), compiler lint (no macros), SDK (no substitution code path) and a CI property test (N simulated devices emit identical URL, QR and code sets).

**Landing URL.** The signed URL, opened exactly as compiled, host equal to the verified sender domain, https, no shorteners, no macros, no hop through Proxim. Default parameters are campaign-level and static: `utm_source=proxim&utm_medium=signpost&utm_campaign=<slug>`. A static per-version `utm_content` is opt-in per campaign, general class only, and its retention is COUNSEL (privacy report). Health classes get no parameter. The measurement report's `pxv` and `pxk` MAC parameters are dropped (not needed; saves 17 B per signpost and a verify endpoint).

**Reporting paths (all aggregate by (code, day); tally counters only, no per-redemption rows):**
1. **Redeem desk (v1).** Hosted PWA, role `redeemer`: staff pick or enter the code and tap +1; undo only within the session. Camera scanning is untested; manual entry is the fallback.
2. **Console entry and CSV (v1).** Columns: date, code, count, optional value in cents. Extra columns rejected; a cell matching an email or phone pattern rejects the whole file; the file is parsed in memory and not kept (hash and row counts only).
3. **API (v1).** `POST /v1/conversions` (`{code, day, count}` batches; Idempotency-Key; sender secret key with a `conversions:write` scope).
4. Closed-schema hook, e-commerce pollers (Stripe promotion-code redemptions [V-sec], Shopify counters [V-sec], asynchronous and may lag) are v1.1. Proxim does not accept order webhooks (PII at the edge).
- **Attribution** is by the code used. Last-seen-code bias is built in; coupon-site leakage inflates counts; the dashboard shows "redemptions on days with no deliveries in the window". Window 7 days default, 30 max. Conversions are bucketed by the sender's local date while deliveries are UTC, so compare weekly. Implausible counts raise a flag and are never blocked.
- **Tap-bound conversions with a per-tap nonce in the landing URL (abuse report's C2) are NOT in v1 and not recommended.** A per-tap random value reaching a third party is exactly what I21 forbids. If ever wanted, it needs an explicit owner decision to except I21 for general-class G0 signposts, counsel review, and a data-flow inventory entry. Single-use vouchers need a live claim call and wait for OHTTP (post M12c).

### 7.7 Billing versus display reconciliation (decision)
**Bill on the released weekly total (C1), not on exact counts.** What the sender sees is what the sender pays.
- Statement line = released C1 views x rate card, waived (0) when the noisy value is below T; charge = min(released x rate, remaining budget). Revenue noise: sd about 7 views per signpost-week, about $0.035 at $5 CPM [MODEL arithmetic]; waiver bias of order one view a week for the smallest senders.
- **Settlement is weekly per (week, signpost).** Partner payout is allocated by exact shares: `payout_p = billed x (1 - margin) x exact_p / sum(exact)`, so the ledger closes exactly. Partners see their own exact app-level counts in the partner console.
- **Pacing and budget caps use exact internal counts.** The sender-visible in-progress spend is a weekly meter in 10 percent budget steps. Budget exhaustion is applied at the next UTC day boundary (to avoid revealing the exact moment a cumulative count crosses a threshold; the residual is declared).
- Late invalid traffic found after release becomes a separate whole-cent "invalid traffic credit" ledger entry; the cube is not re-released.
- Disputes can only be about invalid traffic or partner fraud, not arithmetic. Staff compare exact internals, partner counts and the audit fleet; a `raw_hold` extends raw events for 60 days.
- **This declines** the PLAN-REVIEW #6 suggestion of raw counts shown as exact to non-health senders (it creates a presence oracle at 150 m rings and via decoys, and makes the "noised" claim false for the headline metric) and the abuse report's "exact above k = 20 and invoice equals report" (the invoice equality is kept; exactness is not). It also declines the privacy report's 250-impression block rounding up (systematic overcharge of 20 to 35 percent at median pilot volume; the same invoice-equals-display property is obtained without it). Owner decision 3 can switch to exact billing at the stated price.
- **Independent verification** (replacing the M13 "within 1 percent from the same SDK report"): (1) partner server-side daily aggregates via `POST /v1/partner/attestations` (aggregate-only, contract and COUNSEL note); gate median daily discrepancy at most 2 percent and p95 at most 5 percent over 4 weeks [U], findings above 10 percent investigated, above 20 percent payout held; (2) audit fleet of Proxim-owned devices with scripted drives, known-truth views in a separate table (about 10 drives a week); (3) canary signposts in empty areas; (4) sender cross-checks (landing sessions not above taps; redemptions on no-delivery days); (5) a network-wide tile-fetch upper bound used only as a coarse per-r5-cell anomaly check. Per-partner attribution from tile fetches is impossible (keyless, I8), so per-partner checks use keyed `/v1/attest` counts and cohort statistics. A 1 percent gate is not reachable given outbox loss.

### 7.8 Exposure bits for reach and fatigue (not in v1)
The measurement report would add `exp` (2 bits: 1st, 2nd, 3rd to 4th, 5th or later exposure) and `rpt` (1 bit) to events, enabling reach (count of `exp` = 0 views, an overcount of about 1.22 times people in a model with 20 percent two-app users [MODEL, inputs U]) and a fatigue curve; the privacy report proposed first-in-epoch flags; the builder and rotation reports said not in v1. Decision: **not in v1.** The columns are reserved and CHECK-disabled. Reasons: they touch I3; at pilot volume the fatigue curve is unreadable (median sender about 7 taps over six weeks); and rotation quality can be shown by the simulator, golden vectors and per-version evenness. Revisit at G-1.1 with pilot data, the privacy reviewer and counsel. Conditional I3 amendment text is in section 9.

### 7.9 Statistics small senders can actually use
Sample sizes below are lower bounds (repeated exposure of the same commuter clusters views; true design effect unknown [U]) and ignore release noise.

| Goal | Views needed [MODEL] |
|---|---|
| CTR of one version at 1 percent, relative 95 percent interval of +/-100 / +/-50 / +/-25 percent | 380 / 1,521 / 6,085 |
| Detect CTR 1 to 2 percent (2x), per arm | 2,315 |
| Detect 1 to 1.5 percent, per arm | 7,747 (rotation report: 7,751) |
| Detect 0.5 to 1.0 percent | 4,670 |
| Compare redemption rates 0.09 to 0.18 percent | 26,124 |
| Detect 1.0 versus 1.2 percent | about 42,700 |

Minimum detectable CTR ratio for the best 2 of k rotating versions, base 1 percent: at 100 views a week and k=2, 6.2x over 4 weeks, 4.1x over 8; at 400 a week, 2.9x and 2.3x; at 1,600 a week, 1.8x and 1.6x; at 6,400 a week with k=4, 1.6x and 1.4x.

**What a pilot sender will actually see [MODEL; inputs U: 20,000 weekly-active devices across 2 to 3 location-core apps, 20 senders, CTR 1 percent, 0.06 redemptions per tap, CPM $5]:**

| | p10 | median | p90 | best of 20 |
|---|---|---|---|---|
| Viewable per week | 29 | 119 | 466 | about 930 |
| Views in a 6-week flight | about 180 | about 720 | about 2,800 | about 5,600 |
| Taps in the flight | about 2 | 7 | 29 | about 56 |

Redemptions in 6 weeks: median 0, p90 3; 48 percent of senders see at least one, 1 percent see ten or more. About 1.9 percent of pilot senders could finish a 2x A/B test in a flight (24.9 percent at 5 times the audience, 79 percent at 25 times). The median sender would need about 16 times the audience to reach 10 redemptions in 6 weeks. Worked example: a café at 300 views a week for 8 weeks has 2,400 views, about 24 taps (CTR 1.0 percent, 95 percent interval 0.67 to 1.48 percent), about 2 redemptions (95 percent interval 0.24 to 7.2), $12 spent and about $6 per redemption (interval $1.7 to $50).

**Display rules.**
- A per-version count shows when the noisy cell clears T; otherwise the versions are pooled as "Other versions".
- CTR with a 90 percent interval only when the version has at least 300 views and at least 5 taps in the window, with release noise added in quadrature; otherwise "-".
- Cost per tap and per redemption only when the count is at least 5, with Garwood intervals.
- The table is labelled "descriptive, not a test", with an MDE badge ("at your volume you can only detect a difference of at least X times in N weeks"). No winner badges, no "95 percent likely better", no auto-promotion, no bandit in v1.
- Pilot posture: prefer 8 or fewer senders on high-flow corridors, flights of at least 8 weeks, show the code on the card, count `directions` and `call` taps as micro-conversions, and present the pilot as "honest counts, rotation without repetition, conversion proof-of-life".
- Pre-registered 2-arm tests with O'Brien-Fleming-type weekly boundaries (false-positive rate 4.8 to 5.3 percent against 17 to 18 percent for naive weekly peeking and 15 to 27 percent for a Bayesian 95 percent rule [MODEL]) are v1.1. The overlay stays neutral during a test.

---

## 8. Analytics dashboard and exports

- **KPI strip (flight to date)**, each number with a badge (Billed, Estimate +/-, Sender-reported, "<T"): Delivered (billed, C1); Spend (released weeks plus a coarse in-progress meter); Taps, CTR and cost per tap; Saves; Dismiss and hide rate; Redemptions (sender-reported) and cost per redemption; Landing sessions (optional, sender-reported).
- **Tabs:** By version (pooling rule, MDE badge, evenness of rotation; the "freshness" panel in v1 is evenness only); Over time (weekly; UTC note; late-arrivals row); Funnel (viewable, taps, landing, redemptions, with the not-measurable stages stated); "Can I test?" calculator; Add conversions (code entry, CSV, Redeem desk link); Segments by sibling signposts (at most 3 per corridor, each at least 500 m and above the 150 m floor; no hour, position, lead-distance or cell dimension).
- **Exports/API.** `GET /v1/orgs/{org}/reports/cells` and CSV/JSON return released cells only (`value`, `kind` billed / released_noisy / sender_reported / suppressed, `noise_sd`, `epsilon`, `release_id`, `mechanism_version`); repeated exports are identical. Statements at `/v1/statements/{id}`. A `report.released` webhook is later. Rate limited; no endpoint returns raw events or exact aggregates to a sender role.
- **Console screens (plain words: "message versions" and "facts").** Campaigns list; start chooser; AI brief wizard; creative workspace (facts panel, up to 6 versions with card and driver view, checks panel); version editor; where and when (map editor with keyboard path); preview with "A commuter's week" (the real WASM picker for one simulated phone, labelled "one simulated phone; real phones vary"); budget and schedule (arithmetic only, no forecast); review and submit; status timeline with per-version feedback and one-click fix and resubmit; reports; team and sharing; brand kit and settings; public report and preview pages. Reviewer admin app: family matrix page (owned by moderation). Target: first publish in about 10 minutes with AI [U, usability test]. Accessibility: button equivalents for drag, aria-live counters, status not colour-only, keyboard-only script and axe on the key screens.

---

## 9. Privacy analysis and invariant changes

### 9.1 What Proxim learns and what it never learns
Learns: IP, time and res-5 cell on tile fetches (no key, I8); IP at ingest (pre-OHTTP, not logged together with the event body; declared residual); events: signpost, rev, version index, type, slot, UTC day, cta, version buckets, attested flag. Never: coordinates, a persistent or cross-app identifier, hour, cell, device or session id on events, the order of one phone's events, who the user is, whether someone has two apps, or anything about customer lists (not accepted at all).
Three blockers from the privacy report are closed in this design: (B1) per-recipient or per-tap values in codes, QR or URLs (I21); (B2) reporting that cannot carry variants and leaks through decoys, differencing and exact billing side channels (7.5, 7.7, I22); (B3) customer lists or personal data into Proxim or the AI vendor (no list, CRM, custom audience, lookalike, hashed-email or lead-form feature; CSV, XLSX and VCF uploads rejected; I25). Other risks: the sender's landing site sees IP, time, user agent and any cookie and may infer the visitor was near the geometry (state-law "precise geolocation" radii of about 1,750 to 2,000 ft [V-sec] versus Proxim's 150 m minimum; COUNSEL); the host app can read the SDK ledger (same process; disclosed, not hidden); store labels must match the real event allowlist.

### 9.2 Invariant and wording changes (every change, old and new)
| Item | Old wording | New wording | Why |
|---|---|---|---|
| I1, I2, I4 to I12 (except as below), I13 to I16, I18, I19 | as in PLAN and Addendum 01 | **No change** | Variants, rotation and measurement fit inside them |
| I2a (authoring routes) | Authoring routes carry coordinates; bodies not logged (addendum 9.5) | Add clause: "Authoring coordinates, and literal street addresses or phone numbers, are never included in any third-party or LLM request." | The AI path adds a third-party processor of sender content. I2 itself (end-user devices) is unaffected |
| I3 | "Matching, path, caps, preferences and any history live only on the device, encrypted and excluded from backups. The trail ring is memory-only." (with the addendum's `zone_state` amendment) | **Unchanged in v1.** The variant ledger is device-only history under the existing ledger rules. Conditional future text, only if exposure bits are enabled at G-1.1: add "The only history-derived value that may leave the device is `exp`, 2 bits (1st, 2nd, 3rd to 4th, 5th or later exposure of the signpost), on `view` events of the general class; no counts, timestamps or creative identities beyond the signed version index." | Resolves the disagreement between measurement (`exp`, `rpt`), privacy (first-in-epoch flags), builder and rotation (not v1) |
| Billboard principle | "A signpost may depend on where the device is, where it is heading, the time and the app's declared context (plus Preference Center topics and suppress actions)." | Append: "...and this device's own on-device exposure counters (caps, wording rotation) for that signpost. Versions may differ in wording, picture and CTA kind, never in offer terms, price, geometry or landing domain. Senders cannot configure sequencing or exposure-history targeting. It may never depend on who the user is." | Rotation and caps already use on-device counts; state it so rotation is permitted and personalisation is not |
| I17 (signed fields) | "...geometry and event parameters, event types, content_mode, delivery, experience, text, label key, intent, audience_scope, relation, arm_scope, selector, sz_sha256, live_at, ... expiry" | Add: `creatives[]` (each with text fields, cta_kind, image hash, icon_id, layout, optional promo_code, `vid`), `family_policy` (cadence, rot_v), `claims_sha256`, landing URL, asset hashes, `valid_until`. "A device renders no creative whose vid is not in the signed list and, once a creative is revoked by variant scope, none matching it." | Tightens; one signature covers the family |
| I20 (revocation scopes) | signpost+rev, signpost, sender_org, kid | Add scope `variant` (signpost, rev, vid; about 12 to 16 B per entry) and a `quarantine` reason; entries pruned after the maximum signed expiry (30 days). Hosted young-revision rule extended to creatives: for 72 h after `live_at` a new revision renders only if the device's revocation list is at most 6 h old | Instant retirement of a bad wording without a new revision; closes the publish-and-run window for offline devices |
| PLAN 6.2 item 3 (data-flow inventory) | `{event_id, signpost_id, rev, event_type, slot, UTC day, sdk/os version, attested, simulated, token}` | `{event_id, signpost_id, rev, var (0..5), event_type incl. save, slot, UTC day, cta, sdk/os version, attested, simulated}`; the token is validated and discarded, not stored | Allowlist change through privacy-lint; token column dropped |
| **I21 (new)** Outbound uniformity | n/a | Every value that leaves the SDK toward a landing page, QR, promo code or sender is a pure function of signed envelope fields (signpost, rev, version); none varies per install, impression, tap or day. No runtime substitution; no redirect hop through Proxim | Prevents an identifier shared with senders and a presence oracle (privacy B1) |
| **I22 (new)** Sender-facing numbers | n/a | Sender-visible numbers come only from `report_releases` cells (stored once, noisy-value thresholded), sender-supplied aggregates and a coarse spend meter. No exact, daily-by-partner, per-partner or live count reaches a sender role; exact aggregates feed only pacing, fraud, payout, reconciliation and the pack builder | Closes PLAN-REVIEW #6 at the headline metric |
| **I23 (new)** Conversions are sender input | n/a | Sender-reported conversion data never feeds billing, ranking, Q, pacing or refunds | No incentive to forge; separates trust |
| **I24 (new)** Closed event allowlist | n/a | Events carry only allowlisted closed-enum fields; no token, device, session, hour, cell or IP column exists in `events_raw`; every history-derived field is at most 2 bits (none in v1) | Keeps events unlinkable |
| **I25 (new)** AI boundary | n/a | No coordinates, polylines, cells, end-user or event data, report rows, member emails or customer personal data reaches any AI vendor. AI runs only server-side at authoring time; no LLM on the device, in the pack compiler or in serving. An LLM may add friction (flags, escalation) but never remove a human requirement | Containment of the new AI surface |
| "Human approval of every signpost" (PLAN section 8, addendum 9.3a; not numbered) | In v1 every signpost gets human approval | Clarified: every signed revision carries a human approval record by a reviewer shown the full family and a diff against the last approved revision. Byte-identical previously approved creatives are collapsed and re-linted rather than re-read; inheritance is void on fact, ruleset or landing-domain change; subset approval signs a new derived revision. Hosted auto-lane is not enabled | Needs owner confirmation (decision 4) |
| PLAN 5.6 caps and `0.5^seen_count` | "per signpost"; "repeat-exposure decay 0.5^seen_count rotates creatives" | Signpost = family; add signed stricter-only `cadence`; decay is a per-wording ranking weight (14-day half-life), across families a ranking factor only, never a gate or a `p_serve` multiplier | Section 5.3 numbers |
| PLAN 5.2 and N10 tier-0 pack | "top 500 by rank, 350 KB gzip including signatures; about 635 B per signpost" | Rank then pack by bytes, with the allotment rules in 4.4 | An N=1 number |
| PLAN 4 `events_raw.token`; ledger settlement per (day, signpost, partner) | token column; daily settlement | no token column (counters table); weekly settlement per (week, signpost) with exact-share partner allocation | Unlinkability; billing on released cells |
| PLAN 9 M13 gates | partner versus Proxim counts within 1 percent; promo-code and geo-holdout evidence of CTR gap vs behavioral baselines | median 2 percent and p95 5 percent against independent sources; report observed counts with intervals, no pass/fail on CTR gap | 7.7 and 7.9 |
| PLAN 7 report auto-pause (0.5 percent after 2,000 viewable) | permanent auto-pause | quarantine for human review within 4 business hours; absolute minimum 25 reports and the ratio; attestation-weighted; threshold rises 4 times after a benign confirmation; a concentration on one version quarantines the revision | Report-bombing with about 10 reports pauses a 2,000-impression campaign [MODEL arithmetic] |

---

## 10. Abuse and fraud controls, with milestone placement

| Threat | Control | Milestone |
|---|---|---|
| Harmful variant hidden in a benign family; bait-and-switch revisions | Family-batch review with per-creative decisions, no bulk approve, dwell time, blind canaries, 10 to 20 percent second review; signer reads review rows independently; new wording is a new revision; creative cap | M7a, M13 |
| Fake facts (price, awards, testimonials) | Claim-lock, fact-span lint, lexicon, OCR flag, human review, attestation, `valid_until` | M7a |
| Prompt injection via uploads or URLs | Two-stage pipeline, verified-domain import only in v1, schema-only extraction, no digits/URLs/entity names in AI spans, one-way advisory LLM, injection corpus in CI | M7b; upload extraction v1.1 |
| LLM cost abuse and shell-org farms | Quotas, dollar meters, global breaker, verified signup, per-IP/ASN limits, WIP caps | M7b |
| Review-queue flooding | Per-org WIP caps, verified-first priority, cheap pre-reject, sender tier 0 daily limit | M5, M7a |
| Landing-page swap or cloaking | Domain binding (DNS TXT, weekly re-proof); per-distinct-URL crawl (at most 4 URLs per family, all on the verified origin) at approval, every 6 h for 7 days after a change, then daily; two-egress mobile-UA SSRF-safe fetcher; failure revokes the family; short signed expiry with renewal gated on a crawl pass (about 600 fetches a day at 100 senders x 3 families x 2 URLs [MODEL]). Cloaking is detected only by user reports (residual) | M7a, M12 |
| Impression inflation by a rogue partner or tampered SDK; presence is unverifiable | Attested single-use tokens with issuance caps (`C_tok`), server-side per-token daily cap, cohort statistics from `/v1/attest` counts, canary signposts, 30-day payout hold, clawback and advertiser credit, partner server-side counts. Bound: attested tokens x C_tok x CPM (1,000 devices x 20 x $8 CPM = $160 a day [MODEL]). A genuine attested app on a GPS-spoofed real phone is indistinguishable; Play Integrity/App Attest farm resistance is [U] | M5 (ingest and attest moved up), M9 |
| Self-clicking to burn budget or game quality | Billing is CPM on viewable impressions; clicks and conversions are never billed; CTR term off in Q at pilot, later attested-only winsorised | M9 |
| Report-bombing a rival | Quarantine instead of permanent pause, minimum 25 reports, attestation weighting | M9 |
| Forged conversions, code farming | Conversions never billed or ranked (I23); redemption-rate anomaly flags; codes expire with the revision; label "reported by you" | M9 |
| Revoked wording still showing | Revoke scopes (variant, revision, family, sender_org, kid); inbox and Saved store the signed envelope and re-verify signature, expiry and revocation at render and at tap; online card gone in about 10 min (N5 design target, not measured); unsynced device up to 72 h then nothing renders; young-revision rule. Expected stale impressions of a revoked 60-per-hour wording in the first hour about 2 online [MODEL, weak] | M3, M12 drills (20 runs per OS) |
| Ex-employee or compromised agency | Member expiry, 90-day auto-suspend, removal cascade, aal2 on submit, external flag on revisions, no finance for externals, "this was not me" freeze | M5, M7a |
| Share-link leakage | 7.3 and 6.3 controls | M7a |
| Age-restricted and high-harm categories (no age signal exists) | Prohibited in v1; no AI generation in health or regulated classes; list is policy data (COUNSEL) | M7a |
| Variant count as a fingerprint | At most 6 wordings and at most 4 URLs/codes per family (at most 3 bits of variant information to a sender); no per-tap nonce | M5 constraints |

---

## 11. AI-content policy position and COUNSEL list

**Position.** AI is assistive: it drafts from a sender-confirmed fact sheet, the sender reviews, edits, accepts and attests, a human at Proxim reviews every creative, and the signature covers frozen text. Proxim does not "write your ads" and does not run AI at serve time. Text only in v1. No testimonials, reviews, ratings, endorsements, "before and after", scarcity, comparative or superlative claims; no second-person statements about the person's habits, location history, identity, health, finances, age, religion, politics or relationships; no photoreal people. No on-card AI label for text (Why-this notes AI only for AI-generated images, which are later). Provenance (`source`, `ai_run_id`, model id, template hash, fact sheet hash) is recorded in the revision so EU Art. 50 enablement later needs no format change.
- **Vendor handling.** Inline text only (no Files API), pinned model, no end-user identifiers, vendor listed as a sub-processor of sender content, ZDR pursued before accepting sensitive documents, raw uploads at most 30 days, job metadata (hashes) 12 months, AI request bodies excluded from logs. The specialists disagree on the default retention fact: the builder (secondary source) recorded up to 30 days; the privacy report (fetched primary page) recorded not retained by default except Covered Models at 30 days, flagged sessions up to 2 years even under ZDR, and Files API data retained until deleted. Treat as unresolved until checked for the pinned model before G-AI.
- **Images (not v1).** Reasons: text-first card, tight asset cache, a second vendor and data flow (Claude does not generate images), copyrightability (prompts alone are not authorship per the US Copyright Office Part 2 report [V-sec]), likeness and New York synthetic-performer disclosure in force since 2026-06-09 [V-sec], moderation load. v1 has non-generative image tools only (crop, resize, compress, background cleanup, a deterministic brand-name card art).

**COUNSEL list (no conclusions drawn):**
1. Disclosure of AI-assisted ad text to end users; state AI-disclosure laws; New York GBL 396-b and EU AI Act Art. 50 (EU deferred).
2. FTC deceptive-claim, substantiation and endorsement exposure; the FTC consumer-reviews rule covers fake and AI-generated testimonials (effective about 2024-10-21 [V-sec]); creator versus publisher liability, Section 230 treatment of platform-authored text.
3. Whether a tap-through that discloses IP and the sender's knowledge of the geometry is "precise geolocation" or a "sale" (CCPA/CPRA, Oregon, Maryland, Washington MHMDA); landing link-out in store privacy labels; joint-controller risk.
4. Controller versus processor posture now that Proxim bills and ranks across partners; agency role map; sender terms (no combining visits with identity to build movement profiles, no unique-recipient codes, no personal data to Proxim or the AI, authority warranty for agencies).
5. Partner-side aggregate logging for reconciliation; whether event fields (`var`, `save`, `cta`) change the partner's App Privacy and Data Safety answers; the 60-to-90-day on-device ledger disclosure.
6. Age-restricted and financial category prohibition without an age signal; coupon and promo terms; health_adjacent conversion totals; storing uploaded documents that may contain personal data; sub-processor and DPA terms with the LLM vendor; the optional `utm_content`.

---

## 12. Data-model, API, pack and SDK changes (compact)

**Data model**
- `revision_creatives(revision_id, idx 0..5, angle, headline<=40, body<=90, glanceable, cta_kind, image_asset, icon_id, layout, promo_code_claim, source human|ai|upload|human_edited, ai_run_id, vid, content_sha256, inherited_approved)`; immutable after submit (trigger plus pgTAP). Creative fields move out of `signpost_revisions`.
- `signpost_revisions` add: `creative_count` (1..6; at most 4 for sender tier 0; at most 3 for health classes), `family_sha256`, `claims_sha256`, `template_sha256`, `valid_until`, `cadence` (max_per_day, max_per_week, min_gap_h), `cadence_class` D|O|S, `redeem_prefix` (<=9), `derived_from_rev`, `derived_by`. `jws` is replaced by envelope, sig and signed_kid (addendum 11.7).
- `signpost_revision_signings` (append-only; supports renewal). `revision_claims`, `campaign_fact_sheets` and `advertiser_attestations`, `claims_library`, `brand_kits`, `assets` (content-addressed, provenance column with AI value CHECK-disabled), `source_documents` (v1.1), `ai_runs`, `ai_quotas`, `moderation_review_items` (per creative; inheritance flags).
- Collaboration: `comments`, `approval_requests`, `share_links` (kind, scope, `token_sha256`, expiry, `revoked_at`, views), `org_members` (+external, expires_at, invited_by, last_seen_at, suspended_at; role `redeemer`).
- Measurement: `events_raw` (+`var`, `cta`; type adds `save`; token dropped; partitioned by day; 14-day purge; `raw_hold`), `ingest_token_counters`, internal `event_aggregates_daily` (no console grants; key adds var, cta), `report_releases` (replaced: cube, period, version, metric, noisy_value, suppressed, b, T, epsilon, mechanism_version; unique so stored-once), `release_policy` (data rows per class), `privacy_budget_ledger` (trigger-enforced caps), `conversion_codes` (scope CHECK = variant), `conversion_counts_daily` plus history (written only through a SECURITY DEFINER function), `conversion_batches`, `billing_statement_lines` (quantity must equal the referenced release), `partner_attestation_counts`, `reconciliation_runs/findings`, `audit_events`, `ivt_flags`, `partner_app_stats_daily`. `events_raw.exp/rpt/exposure_bucket` reserved and CHECK-disabled.
- `revocations` (+variant scope and quarantine reason); `pack_compile_reports`; `policy_rules` keys (creative caps by tier and class, lexicon version, AI quotas and model ids, share limits, report pause thresholds, submission rate limits). Device SQLite: `variant_ledger`, `family_state`.
- Every new table: RLS enabled and forced; org tenancy via `private.is_org_member`; cross-tenant and anon-denial pgTAP cases; console roles have zero grants on `events_raw` and `event_aggregates_daily`.

**API (OpenAPI, versioned)** Console-session only: AI jobs and generation (`/v1/ai/*`; rejects sender secret keys), sources and fetch-url, fact sheets, claims, families and revisions with `creatives[]`, approval requests, share links, quota. Sender or console: `POST /v1/conversions`, `GET /v1/orgs/{org}/reports/cells` (+export), `/v1/statements/{id}`, `GET /v1/signposts/{id}/power`. Partner: `POST /v1/partner/attestations`. Public: `GET /r/{token}` page with fragment-token exchange. No endpoint returns raw events or exact aggregates to a sender role.

**Pack and SDK**
- Envelope v2: `creatives[]`, `family_policy`, `claims_sha256`, `valid_until`; old SDKs fail closed on unknown fields. `rot_v` in the pack: a version skew degrades to uniform rotation.
- Compiler: rank then bytes with allotment rounds, `n_shipped`, assets by hash for ranked-in signposts only.
- Kernel: `select_variant`, family-keyed caps and cadence, vid-scope revocation, young-revision rule, golden vectors across four targets. SDK: `markShown(visibleMs)` advances rotation (documented), events carry `var`, `save`, `cta`, outbox TTL 7 days, per-install cap 16, code chip, landing opener accepts only the signed URL, ledger in `privacyReport` and erase, TestKit "why this version" inspector and warning for repeated decisions with no `markShown`.
- Console: creative workspace, version editor, commuter-week preview with the real WASM picker, facts panel, brand kit, team and sharing, reports, Redeem desk, public pages. `proxim-sim` scenarios: `commuter_60d` (one and two apps), `reinstall`, `refresh_revision`, `driving_glanceable_only`.
- Store-label and DPA generators list the new event fields and the LLM sub-processor.

---

## 13. Milestone changes with verification

| Milestone | Change | Verification |
|---|---|---|
| M0 | Add burn-down: `share_reader` SECURITY DEFINER pattern, WebAuthn/aal2, golden-vector spike for `select_variant` | Spike passes on all four targets; Supabase items confirmed |
| M2 | Envelope v2 with `creatives[]`; compiler rank-then-bytes with allotment; re-measure bytes per creative with at least 60 real or AI-style creatives | Byte-identical packs; budget never exceeded including signatures; capacity table reproduced with measured numbers; flipping any creative byte fails verification (I10) |
| M3 | `select_variant`, ledger, cadence, family-keyed caps, decay clarification, vid-scope revocation, young-revision rule | Golden vectors; `commuter_60d` table reproduced; no back-to-back repeat; hide/dismiss/report suppress the family; mutation test turns the suite red |
| M5 | Tables, RLS, ingest and attest moved up, token not stored, single-use token issuance caps, release schema, conversion tables | pgTAP cross-tenant for every new table; schema lint (no token/device/hour/cell column); event contract tests |
| M7a (new split) | Manual and upload editor, facts, validators, family-batch review, subset approval and delta inheritance, per-URL crawl, roles, snapshot and preview links, brand kit, image pipeline, prohibited categories | 60+ labelled creative review set with zero false approves; role matrix and link tests; accessibility run; usability test (5 users) |
| M7b (new split) | AI text layer, quotas, eval and injection corpora, verified-domain import | Eval set (at least 100 briefs) and at least 40 injection fixtures; AI egress test; outage falls back to manual |
| M8 / M10 | SDK emits `view/tap/save/dismiss/hide`, `var`, `cta`; viewability conformance; code chip; outbox rules; ledger | 49 percent / 0.99 s emits nothing; 50 percent / 1.0 s exactly one; nothing written in a sensitive buffer; erase and withdrawal wipe the ledger |
| M9 | Release job, billing close on released C1, weekly settlement, Redeem desk, conversions API and CSV, partner attestations, reconciliation, audit fleet, IVT/C_tok, report quarantine, G-V simulator | Statement lines equal released x rate exactly; re-run returns identical cells; budget-ledger trigger works; G-V simulator results (lone commuter, decoy farm, nested differencing, pilot cell survival) reviewed |
| M12a / M12 | Pen-test scope adds prompt injection, share links, brand-URL SSRF, agency takeover; revocation drills at variant and family scope | 20 drills per OS; zero rendering after the stated bound |
| M13 | Pilot gates: G-V, G-AI (before any external sender uses AI), G-SHARE; reconciliation 2 percent / 5 percent; report observed counts with intervals; human review of 100 percent of creatives | Gate checklist signed; pilot posture adopted (8 or fewer senders, flights of at least 8 weeks) |
| M13.1 (G-1.1) | Decide exposure bits; daily family cell; pre-registered A/B; per-version `utm_content` decision; flyer extraction; CSV bulk; QR; per-version pause in console; e-commerce pollers; templates | Each gated by pilot data and counsel |

---

## 14. Risks
1. Owner and senders hear "no fatigue" or "delivered every day" as guarantees. Mitigation: contract and console promise only eligibility and even rotation; preview labelled as a simulation; model numbers stay internal.
2. Pilot volumes make most variant cells and conversions invisible, and pressure builds to loosen release rules. Mitigation: decide the promise now; show the owner the pilot simulation first; any loosening needs an ADR and a re-run of the decoy simulations.
3. Release parameters (Delta, epsilon, T) are a synthesis not yet simulated; the privacy report's figures are hand-computed. Mitigation: G-V simulator before the pilot; fall back to coarser periods or the privacy report's device-bounded design if it fails.
4. Reviewer rubber-stamping across 6 creatives; reviewer staffing. Mitigation: matrix, dwell time, canaries, second-review sample, caps.
5. AI claims drift, prompt injection, cost abuse. Mitigation: grammar-level ban on digits/URLs/entities in AI spans, fact lint, two-stage extraction, quotas, one-way LLM.
6. Rogue partner inflation; presence unverifiable. Mitigation: layered controls (7.7, section 10); honest residual.
7. Pack and asset budgets with variants. Mitigation: byte ranking, 2 images per family, M2 measurement with real AI output (marginal bytes may be 2 times the measured 40 to 52 B).
8. Event linkability (reusable token or IP-with-body logging) turns versions into a route diary. Mitigation: single-use tokens as a precondition for `var`; no body logging; OHTTP remains the hard gate for health.
9. Cross-app duplicates, reinstall and low-cadence under-reporting. Accepted and disclosed.
10. Landing site becomes a tracking surface; precise-geolocation treatment of taps. Mitigation: no parameter beyond static campaign UTM, disclosure in Why-this, sender terms, COUNSEL.
11. Anthropic dependency (outage, deprecation, retention terms). Mitigation: manual editor always works; pinned models; spend caps.
12. Scope for a very small team. Mitigation: manual variants before AI; documented cut order; never cut claim-lock, family-in-revision, reviewer matrix, validators or human review.
Cut order if over budget: AI targeting explanation; draft preview links; internal approval; CSV bulk; flyer extraction; QR; daily cell.

---

## 15. Open owner decisions (maximum 5, each with a recommended default)

1. **What "share" means.** Options: (a) collaborate and share reports with teammates, clients and agencies; (b) publish to partner apps and the network; (c) let others reuse a campaign as a template. Recommended: build (a) and (b) in v1 (guest members with expiry, expiring read-only snapshot links, preview links, publish toggle by partner category); (c) later. Agencies are guests in each client's org in v1; delegation after the pilot.
2. **Promise language and the weekly ceiling.** Recommended: promise "eligible every weekday when a partner app is open on your road" and "even rotation, no back-to-back repeat on a phone"; never "delivered every day" or "no ad fatigue". Keep 5 per week (covers weekday commuters; true 7-day "every day" is not possible).
3. **Reporting, billing and pilot promise.** Recommended: bill on the released weekly total (displayed equals billed; accept noise of about 7 views per signpost-week); no daily cells, no reach, frequency or exposure-history bits in v1 (I3 untouched); no "differential privacy" or "anonymous" wording; pilot promise = honest counts, rotation without repetition and conversion proof-of-life, with at most 8 senders and flights of at least 8 weeks. Alternative: exact billing and exact weekly totals (weakens the "noised" claim for the headline metric).
4. **Review rule.** Recommended: accept that each signed revision still has a human approval record, with byte-identical already-approved creatives collapsed and re-linted rather than re-read, subset approval creating a derived revision, and 100 percent human review of AI creatives. Confirm reviewer staffing (about 12.5 hours a month at 15 senders [MODEL]). If the owner reads the rule as "re-read everything", refresh review cost rises roughly 1.8 to 3.8 times.
5. **AI scope.** Recommended: assistive text only in v1 (no AI images, no autopilot, sender accepts and attests each creative); Anthropic as a disclosed sub-processor of sender content with standard retention for the closed beta and a sender notice, pursue ZDR before sensitive documents; counsel on author liability and disclosure before any external sender uses AI (G-AI).

---

## 16. Unverified facts

- Every pilot audience and behaviour input: 20,000 weekly-active devices, median 1.5 percent pass fraction, open and window rates, CTR 1 percent, 0.06 redemptions per tap, 0.03 percent direct redemptions, CPM $5. They drive the headline "119 views a week, about 0 redemptions" [MODEL, U]. No field data exists.
- Commuter open rates are independent per day in the rotation simulation; real opens are autocorrelated. Two-app models assume independence between apps; the two specialists' simulations use different assumptions. Pacing, competition and consent are not modelled. Only a Python reference exists; cross-language bit identity is untested until the M0 spike.
- Pack numbers: 110 B per creative is a planning assumption; measured 40 to 52 B came from 60 hand-written creatives; real AI output may compress worse. The fixed 450 B family part is inferred.
- Release parameters in 7.5 (Delta, epsilon, T = 20, two-week periods) and the P(display) figures are my arithmetic on the specialists' inputs; the privacy report's own figures were hand-computed. Event-level versus device-level composition, the client-trusted caps, and integer discrete-Laplace sufficiency have not been externally reviewed.
- MRC viewability text was not fetched; Stripe and Shopify counter behaviour is from search summaries; Mironov's attack from search results.
- LLM retention: the two reports disagree (see section 11). Zero-data-retention eligibility of the exact features, LLM token counts and per-call costs are assumptions. Prompt-injection robustness is unproven; the design assumes the model can be fooled.
- Reviewer times (90 s overhead, 12 s per creative, 6 min base, 0.75 min per diff-reviewed creative), reject rates, the 10 to 20 percent second-review sample and canary catch rate are assumptions.
- Diversity threshold 0.35 was calibrated on 150 hand-written pairs. The 10-minute first-publish target is a target, not a measurement. Effort ranges are judgement.
- Hidden-text detection in PDFs, OCR accuracy on stylised flyers, crawler cloaking resistance, Play Integrity/App Attest resistance to spoofing farms, whether SFSafariViewController and Custom Tabs preserve query parameters or share cookies, and camera scanning in the Redeem desk PWA are untested.
- FTC primary sources, Google Play policy pages and several state-law texts were unreachable to the privacy specialist; US-law items are [V-sec] at best. Whether any state requires disclosure of AI-assisted text in ads is open. Store-label mappings and whether the SDK-bundled layout and icon sets satisfy "partners control theme tokens, not content or chrome" are untested.
- Revocation timings (about 10 minutes online) are the N5 design target, not measurements; the offline-device share is unmeasured. The tile-fetch bound assumes about one pack fetch per device-cell per day.
- Supabase features assumed (guest invites, session limits, WebAuthn, SECURITY DEFINER share reader) are checked at M0.
