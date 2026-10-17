"""OI-ROT-1 supplement (Addendum 02 5.1): the `new_count` cases golden_v2.json lacks, built by the
normative reference (refkernel.py) and written to core/testdata/golden/rotation/golden_v2_newcount.json.
golden_v2.json itself is frozen; this file adds traces in its `select` schema:

  * a one-wording family whose count grows by ONE each showing and saturates at SAT;
  * a six-wording family revoked down to one available wording (avail mask with one bit), where
    new_count = min(SAT, cn + ONE) ignores the higher counts of the unavailable members;
  * the same ledger with every wording available, for contrast (new_count = max(cn + ONE, top + 1));
  * R-2026-10-17: window expiry applied on every call (re-ask and markShown after the window, no tick);
  * boundary cases found by the independent review (2026-10-17): a dismissal 60 days after the
    showing (dismiss reads the decayed count), LRU eviction never removing the row just written,
    and the pending window at exactly W_EMIT_MS, LATE_MS and a 1,000 ms view.

A select op may carry "day" on a dismiss (the day it happens; golden_v2 dismisses on the commit day).

Run:  python3 -I gen_newcount.py            (dry run: exit 1 if the committed file differs)
      PROXIM_BLESS=1 python3 -I gen_newcount.py
"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import refkernel as R

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "..", "core", "testdata", "golden", "rotation", "golden_v2_newcount.json")
V2 = json.load(open(os.path.join(HERE, "..", "..", "core", "testdata", "golden", "rotation", "golden_v2.json")))
FAM = {d["family"]: d for d in V2["derive"]}


def trace(name, family, preload, steps, dow_mask=0x7F, lru=2000):
    f = FAM[family]
    digests = [bytes.fromhex(x) for x in f["digests_hex"]]
    sid = bytes.fromhex(f["signpost_id_hex"])
    vids, pos, phi = R.derive(sid, digests)
    led = R.Ledger(lru=lru)
    if preload:
        led.rows = {v: (c, d) for v, c, d in preload["rows"]}
        led.recent = list(preload["recent"])
        led.last_day = preload["last_day"]
    ops = []
    last = None
    for step in steps:
        if step[0] == "dismiss":
            R.dismiss(led, vids[last], step[1])
            ops.append({"op": "dismiss", "day": step[1]})
            continue
        day, avail = step
        o = R.select(led, day, vids, pos, phi, avail, dow_mask)
        ops.append({"op": "select", "day": day, "avail_mask": avail,
                    "expect_idx": None if o is None else o[0], "expect_new_count": None if o is None else o[1]})
        if o is not None:
            R.commit(led, vids[o[0]], day, o[1])
            ops.append({"op": "commit"})
            last = o[0]
    return {"name": name, "digests_hex": f["digests_hex"], "signpost_id_hex": f["signpost_id_hex"], "dow_mask": dow_mask,
            "lane": 0, "lanes": 1, "lru": lru, "preload": preload, "ops": ops, "final": led.snapshot()}


def strip_none(v):
    if isinstance(v, dict):
        return {k: strip_none(x) for k, x in v.items() if x is not None}
    if isinstance(v, list):
        return [strip_none(x) for x in v]
    return v


yoga = FAM["yoga"]
yoga_vids = R.derive(bytes.fromhex(yoga["signpost_id_hex"]), [bytes.fromhex(x) for x in yoga["digests_hex"]])[0]
brew = FAM["brew"]
bv = R.derive(bytes.fromhex(brew["signpost_id_hex"]), [bytes.fromhex(x) for x in brew["digests_hex"]])[0]
# Six brew wordings, the base (0) shown least; the others carry high counts from recent days.
heavy = {"rows": sorted([[bv[0], 300, 40]] + [[v, 4000 + 500 * k, 40] for k, v in enumerate(bv[1:])]),
         "recent": [bv[3], bv[4], bv[5]], "last_day": 40}

traces = [
    # yoga has 4 wordings; avail mask 1 makes it a one-wording family on the wire's terms.
    trace("n1_avail_only_base_daily", "yoga", None, [(d, 0b0001) for d in range(0, 6)]),
    trace("n1_saturates_at_sat", "yoga",
          {"rows": [[yoga_vids[0], 65400, 9]], "recent": [yoga_vids[0]], "last_day": 9}, [(9, 0b0001), (10, 0b0001)]),
    trace("n6_revoked_to_one_base_left", "brew", heavy, [(41, 0b000001), (42, 0b000001)]),
    trace("n6_revoked_to_one_non_base_left", "brew", heavy, [(41, 0b100000)]),
    trace("n6_contrast_all_available", "brew", heavy, [(41, 0b111111)]),
    trace("dismiss_reads_the_decayed_count", "brew", None, [(0, 0b000001), ("dismiss", 60)]),
    trace("lru1_never_evicts_the_row_just_written", "brew",
          {"rows": [[bv[0], 300, 10]], "recent": [bv[0]], "last_day": 10}, [(10, 0b100000)], lru=1),
]


def pending_case(name, evs, start_day=5):
    f = FAM["brew"]
    vids, pos, phi = R.derive(bytes.fromhex(f["signpost_id_hex"]), [bytes.fromhex(x) for x in f["digests_hex"]])
    led = R.Ledger(); P = R.Pending(); log = []; day = start_day
    choose = lambda: R.select(led, day, vids, pos, phi, 0b111111, 0x1F, 0, 1)
    do_commit = lambda idx, nc, d: R.commit(led, vids[idx], d, nc)
    for ev in evs:
        if ev[0] == "emit": log.append(["emit", ev[1], P.emit("f", ev[1], choose, do_commit, day)])
        elif ev[0] == "tick": P.tick("f", ev[1], do_commit); log.append(["tick", ev[1], len(P.log)])
        elif ev[0] == "mark": log.append(["mark", ev[1], P.mark_shown("f", ev[1], ev[2], do_commit)])
        elif ev[0] == "day": day = ev[1]
    return {"name": name, "signpost_id_hex": f["signpost_id_hex"], "digests_hex": f["digests_hex"], "dow_mask": 0x1F,
            "start_day": start_day, "events": evs, "expect_log": log, "expect_ledger": led.snapshot(),
            "pending_log": [list(x) for x in P.log]}


pending = [
    pending_case("window_edge_reask_and_tick", [["emit", 0], ["emit", 59999], ["tick", 59999], ["tick", 60000], ["emit", 60000]]),
    pending_case("late_mark_at_exactly_late_ms", [["emit", 0], ["tick", 60000], ["mark", 600000, 3500]]),
    pending_case("late_mark_one_ms_too_late", [["emit", 0], ["tick", 60000], ["mark", 600001, 3500]]),
    pending_case("view_of_exactly_1000_ms_bills", [["emit", 0], ["mark", 1000, 1000]]),
    pending_case("view_of_999_ms_does_not_bill", [["emit", 0], ["mark", 1000, 999]]),
    # R-2026-10-17 (5.3 rules 3 and 4): expiry applies on every call, not only on tick.
    pending_case("reask_after_window_commits_the_expired_decision_first", [["emit", 0], ["emit", 60000], ["tick", 130000]]),
    pending_case("reask_long_after_window_without_tick", [["emit", 0], ["day", 6], ["emit", 86400000]]),
    pending_case("mark_after_window_without_tick_bills_late_only", [["emit", 0], ["mark", 70000, 3500], ["mark", 80000, 3500]]),
    pending_case("mark_after_late_window_without_tick_is_too_late", [["emit", 0], ["mark", 700000, 3500]]),
    pending_case("short_view_after_window_without_tick", [["emit", 0], ["mark", 70000, 400]]),
]
doc = {"header": {"format": "proxim-golden/1", "kind": "rotation", "id": "golden_v2_newcount", "engine_spec": 1, "synthetic": True},
       "source": "tools/rotation-ref/gen_newcount.py (refkernel.py); companion to golden_v2.json (frozen)",
       "select": strip_none(traces), "pending": pending}
text = json.dumps(doc, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n"
old = open(OUT).read() if os.path.exists(OUT) else None
if old == text:
    print("same   ", OUT)
elif os.environ.get("PROXIM_BLESS") == "1":
    open(OUT, "w").write(text)
    print("written", OUT)
else:
    print("DIFFERS", OUT, "(PROXIM_BLESS=1 to write)")
    sys.exit(1)
