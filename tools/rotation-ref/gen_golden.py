"""Generate golden_v2.json from the NORMATIVE python reference (refkernel.py, envelope.py). Deterministic except ECDSA signatures (random k);
signatures are stored, verifiers check, nobody regenerates them in CI."""
import json, hashlib, random, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import refkernel as R, envelope as E
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives import serialization

rng = random.Random(20261010)
def hx(b): return b.hex()
G = {"version": "golden_v2",
     "constants": dict(ONE=R.ONE, SAT=R.SAT, DISMISS=R.DISMISS, PURGE_BELOW=R.PURGE_BELOW, MAX_AGE=R.MAX_AGE, SLACK=R.SLACK, K_MAX=R.K_MAX, RECENT_MAX=R.RECENT_MAX,
                       W_EMIT_MS=R.W_EMIT_MS, LATE_MS=R.LATE_MS, LRU_DEFAULT=2000, ROT_PREFIX_HEX=hx(R.ROT), CREATIVE_PREFIX_HEX=hx(E.CRE_PREFIX), ENV_PREFIX_HEX=hx(E.ENV_PREFIX)),
     "decay": R.DECAY}

# ---- sched_index
G["sched_index"] = [[d, m, R.sched_index(d, m)] for d in (0, 1, 4, 5, 6, 7, 11, 12, 13, 14, 20000, 20003, 20006, 20007) for m in (0x1F, 0x7F, 0x3F, 0x00, 0x15)]

# ---- creatives (digest vectors)
def fam_creatives(name, n):
    return [E.mk_creative(rng, name, i) for i in range(n)]
CR = {"brew": fam_creatives("brew", 6), "tyre": fam_creatives("tyre", 6), "yoga": fam_creatives("yoga", 4)}
G["digest_vectors"] = []
for f, cs in CR.items():
    for i, c in enumerate(cs):
        b = E.canon_creative(c); G["digest_vectors"].append(dict(family=f, idx=i, canon_hex=hx(b), digest_hex=hx(E.digest(b)), vid=E.vid_of(E.digest(b))))
DIG = {f: [E.digest(E.canon_creative(c)) for c in cs] for f, cs in CR.items()}
SID = {"brew": b"sp-brew-0001", "tyre": b"sp-tyre-0002", "yoga": b"sp-yoga-0003"}

# ---- derive
G["derive"] = []
for f in CR:
    v, p, ph = R.derive(SID[f], DIG[f])
    G["derive"].append(dict(family=f, signpost_id_hex=hx(SID[f]), digests_hex=[hx(d) for d in DIG[f]], vids=v, pos=p, phi=ph))

# ---- select traces
def trace(name, fam, days_avail, dow_mask=0x1F, lane=0, lanes=1, lru=2000, preload=None, dismiss_after=(), ops_extra=None, digests=None):
    sid = SID[fam]; dg = digests or DIG[fam]
    vids, pos, phi = R.derive(sid, dg)
    led = R.Ledger(lru=lru)
    if preload:
        led.rows = {v: (c, d) for v, c, d in preload["rows"]}; led.recent = list(preload["recent"]); led.last_day = preload["last_day"]
    ops = []; pend = None
    for k, ent in enumerate(days_avail):
        day, am = ent[0], ent[1]; ln, lns = (ent[2], ent[3]) if len(ent) > 2 else (lane, lanes); nocommit = len(ent) > 4 and ent[4]
        out = R.select(led, day, vids, pos, phi, am, dow_mask, ln, lns)
        o = dict(op="select", day=day, avail_mask=am, expect_idx=None if out is None else out[0], expect_new_count=None if out is None else out[1])
        if len(ent) > 2: o["lane"] = ln; o["lanes"] = lns
        ops.append(o)
        if out is None or nocommit: continue
        R.commit(led, vids[out[0]], day, out[1]); ops.append(dict(op="commit"))
        if k in dismiss_after:
            R.dismiss(led, vids[out[0]], day); ops.append(dict(op="dismiss"))
    return dict(name=name, signpost_id_hex=hx(sid), digests_hex=[hx(d) for d in dg], dow_mask=dow_mask, lane=lane, lanes=lanes, lru=lru,
                preload=preload, ops=ops, final=led.snapshot())

wk = [d for d in range(28) if d % 7 < 5]
full6 = 0b111111
T = []
T.append(trace("n6_weekdays_daily", "brew", [(d, full6) for d in wk]))
T.append(trace("n6_weekdays_daily_lane1of2", "brew", [(d, full6) for d in wk], lane=1, lanes=2))
T.append(trace("n4_weekdays_daily", "yoga", [(d, 0b1111) for d in wk]))
sparse_days = [0, 1, 4, 7, 8, 14, 15, 16, 22, 36, 37, 44, 60, 61, 130, 131, 132, 133, 134, 135, 136, 137]
T.append(trace("n6_sparse_gaps_to_130", "brew", [(d, full6) for d in sparse_days], dow_mask=0x7F))
T.append(trace("n6_dismiss_then_rest_7d", "brew", [(d, full6) for d in wk[:14] + [d for d in range(40, 60) if d % 7 < 5]], dismiss_after=(9,)))
T.append(trace("n6_partial_ship_then_revoke", "brew", [(d, m) for d, m in zip(wk[:18], [0b000111] * 6 + [0b001111] * 6 + [0b111101] * 6)]))
# preload for new revision: previous revision had digests of brew; new revision replaces 3 creatives
old_led = R.Ledger(); ov, op, oph = R.derive(SID["brew"], DIG["brew"])
for d in wk[:12]:
    o = R.select(old_led, d, ov, op, oph, full6, 0x1F, 0, 1); R.commit(old_led, ov[o[0]], d, o[1])
new_dig = DIG["brew"][:3] + DIG["tyre"][:3]
T.append(trace("n6_new_revision_3_of_6_new", "brew", [(d, full6) for d in wk[12:24]], preload=old_led.snapshot(), digests=new_dig))
T.append(trace("n1_single", "yoga", [(d, 0b1) for d in wk[:5]], digests=DIG["yoga"][:1]))
T.append(trace("n2_alternates", "yoga", [(d, 0b11) for d in wk[:8]], digests=DIG["yoga"][:2]))
T.append(trace("n6_lru4", "brew", [(d, full6) for d in wk], lru=4))
vids_b = R.derive(SID["brew"], DIG["brew"])[0]
T.append(trace("n6_purge_boundary_rows", "brew", [(60 + 20, full6), (81, full6), (111, full6), (112, full6)],
               preload=dict(rows=[[vids_b[0], 256, 20], [vids_b[1], 256, 19], [vids_b[2], 256, 21], [vids_b[3], 1000, 0]], recent=[vids_b[0], vids_b[1]], last_day=21)))
T.append(trace("n6_three_wordings_without_rows", "brew", [(d, full6) for d in wk[:8]],
               preload=dict(rows=[[vids_b[0], 700, 5], [vids_b[1], 900, 6], [vids_b[2], 800, 4]], recent=[vids_b[1]], last_day=6)))
T.append(trace("n6_calendar_all_days_mask7F", "brew", [(d, full6) for d in range(21)], dow_mask=0x7F))
T.append(trace("n6_mon_sat_mask3F", "brew", [(d, full6) for d in range(21) if d % 7 < 6], dow_mask=0x3F))
# slack boundary: counts exactly min, min+256 (eligible), min+257 (not) -- one select per lane so every wording gets to be the date-nearest
vb = R.derive(SID["brew"], DIG["brew"])[0]
sl = dict(rows=[[vb[0], 300, 50], [vb[1], 556, 50], [vb[2], 557, 50], [vb[3], 900, 50], [vb[4], 1000, 50], [vb[5], 1100, 50]], recent=[], last_day=None)
T.append(trace("n6_slack_boundary_min_plus_256_257", "brew", [(50, full6, l, 6, True) for l in range(6)], dow_mask=0x7F, preload=sl))
sl2 = dict(rows=[[vb[i], 300, 50] for i in range(6)], recent=[vb[0], vb[1], vb[2], vb[3]], last_day=49)
T.append(trace("n6_recent_window_k3_bans_last3_only", "brew", [(50, full6, l, 6, True) for l in range(6)], dow_mask=0x7F, preload=sl2))
# family_state expiry at exactly 90 / 91 days (recent window bans vs not), all counts equal so only the ban can change the choice
D0 = 40
fs = dict(rows=[[vb[i], 65535, D0] for i in range(6)], recent=[vb[0], vb[1], vb[2]], last_day=D0)
T.append(trace("n6_family_state_expiry_90_vs_91", "brew", [(D0 + dd, full6, l, 6, True) for dd in (90, 91) for l in range(6)], dow_mask=0x7F, preload=fs))
# row expiry at exactly 90 / 91 days: expired rows read as 'no row' and are shown first
rw = dict(rows=[[vb[i], 65535, D0] for i in range(3)] + [[vb[i], 300, D0 + 89] for i in range(3, 6)], recent=[], last_day=None)
T.append(trace("n6_row_expiry_90_vs_91", "brew", [(D0 + dd, full6, l, 6, True) for dd in (90, 91) for l in range(6)], dow_mask=0x7F, preload=rw))
# LRU tie: four rows share the oldest last_day, inserted out of vid order; a fifth commit must evict the smallest vid among them
vs = sorted(vb[:4]); lt = dict(rows=[[vs[2], 400, 10], [vs[0], 400, 10], [vs[3], 400, 10], [vs[1], 400, 10]], recent=[], last_day=10)
T.append(trace("n6_lru4_tie_on_last_day_evicts_smallest_vid", "brew", [(11, 0b110000), (12, 0b110000)], dow_mask=0x7F, lru=4, preload=lt))
# purge threshold: a row whose decayed count is 12 reads as 'no row' (shown first), 13 is live
pg = dict(rows=[[vb[0], 12, 60], [vb[1], 13, 60]], recent=[], last_day=None)
T.append(trace("n6_purge_threshold_12_vs_13", "brew", [(60, 0b000011, l, 6, True) for l in range(6)], dow_mask=0x7F, preload=pg))
G["select"] = T

# ---- pending / S12 state machine (fake clock). Each case: list of events -> expected log and ledger rows
def pending_case(name, evs):
    vids, pos, phi = R.derive(SID["brew"], DIG["brew"]); led = R.Ledger(); P = R.Pending(); log = []; day = 5
    def choose():
        o = R.select(led, day, vids, pos, phi, 0b111111, 0x1F, 0, 1); return o
    def do_commit(idx, nc): R.commit(led, vids[idx], day, nc)
    for ev in evs:
        if ev[0] == "emit": r = P.emit("f", ev[1], choose); log.append(["emit", ev[1], r])
        elif ev[0] == "tick": P.tick("f", ev[1], do_commit); log.append(["tick", ev[1], len(P.log)])
        elif ev[0] == "mark": r = P.mark_shown("f", ev[1], ev[2], do_commit); log.append(["mark", ev[1], r])
        elif ev[0] == "day": day = ev[1]
    return dict(name=name, signpost_id_hex=hx(SID['brew']), digests_hex=[hx(d) for d in DIG['brew']], dow_mask=0x1F, start_day=5, events=evs, expect_log=log, expect_ledger=led.snapshot(), pending_log=[list(x) for x in P.log])
G["pending"] = [
    pending_case("mark_in_window", [["emit", 0], ["mark", 4000, 3500], ["tick", 70000]]),
    pending_case("headless_never_marks", [["emit", 0], ["emit", 5000], ["tick", 59999], ["tick", 60000], ["day", 6], ["emit", 100000], ["tick", 170000]]),
    pending_case("late_mark_after_emit_commit_bills_once", [["emit", 0], ["tick", 60000], ["mark", 120000, 3500], ["mark", 130000, 3500]]),
    pending_case("too_late_mark_not_billed", [["emit", 0], ["tick", 60000], ["mark", 700000, 3500]]),
    pending_case("short_view_not_billed", [["emit", 0], ["mark", 2000, 400], ["tick", 90000]]),
]

# ---- envelope vectors
priv = ec.derive_private_key(int.from_bytes(hashlib.sha256(b"proxim-test-key").digest(), "big") % (2**255), ec.SECP256R1())
pub = priv.public_key()
pub_hex = hx(pub.public_bytes(serialization.Encoding.X962, serialization.PublicFormat.UncompressedPoint))
core = {f: b"core|" + f.encode() + bytes(rng.randrange(256) for _ in range(40)) for f in CR}
EV = []
def add(name, wire, f=None):
    ok, out = E.verify_entry(pub, wire)
    EV.append(dict(name=name, wire_hex=hx(wire), expect="accept" if ok else "reject",
                   shipped_idx=sorted(out) if ok else None, body_sha256_hex=[hx(hashlib.sha256(out[i]).digest()) for i in sorted(out)] if ok else None))
for f, mask in (("brew", 0b111111), ("brew", 0b000001), ("brew", 0b010101), ("tyre", 0b000111), ("yoga", 0b1111), ("yoga", 0b0101)):
    cs = CR[f]; add(f"valid_{f}_mask{mask:b}", E.build_entry(priv, core[f], cs, mask))
base = E.build_entry(priv, core["brew"], CR["brew"], 0b000111)
add("flip_sig_byte", base[:-5] + bytes([base[-5] ^ 1]) + base[-4:])
add("flip_core_byte", base[:5] + bytes([base[5] ^ 1]) + base[6:])
add("truncate_1", base[:-1])
add("trailing_byte", base + b"\x00")
add("mask_zero_base_missing", base[:2 + len(core['brew'])] + bytes([6, 0b000110]) + base[2 + len(core['brew']) + 2:])
add("n_zero", base[:2 + len(core['brew'])] + bytes([0, 1]) + base[2 + len(core['brew']) + 2:])
add("n_nine", base[:2 + len(core['brew'])] + bytes([9, 1]) + base[2 + len(core['brew']) + 2:])
# swaps: body i replaced by another family member / another family / forged
def with_body(f, mask, i, newbody):
    cs = CR[f]; n = len(cs); idx = [k for k in range(n) if (mask >> k) & 1]
    e = E.build_entry(priv, core[f], cs, mask); sig = e[-64:]
    ds = [E.digest(E.canon_creative(cs[k])) for k in range(n) if not (mask >> k) & 1]
    bodies = [E.canon_creative(cs[k]) for k in idx]; bodies[idx.index(i)] = newbody
    return E.lp16(core[f]) + bytes([n, mask]) + b"".join(ds) + b"".join(E.lp16(b) for b in bodies) + sig
add("swap_body1_for_body2_same_family", with_body("brew", 0b000111, 1, E.canon_creative(CR["brew"][2])))
add("swap_body0_for_other_family_body", with_body("brew", 0b000111, 0, E.canon_creative(CR["tyre"][0])))
add("swap_body2_for_unsigned_text", with_body("brew", 0b000111, 2, E.canon_creative(E.mk_creative(rng, "evil", 7))))
fx = json.load(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "vid_fixture.json")))
legit, forged = bytes.fromhex(fx["legit_hex"]), bytes.fromhex(fx["forged_hex"])
assert E.vid_of(E.digest(legit)) == E.vid_of(E.digest(forged))
# family whose creative 0 is the `legit` raw body
bodies = [legit] + [E.canon_creative(c) for c in CR["brew"][1:]]
ds = [E.digest(b) for b in bodies]; sig = E.sign(priv, E.signed_msg(core["brew"], ds))
def raw_wire(b0): return E.lp16(core["brew"]) + bytes([6, 1]) + b"".join(ds[1:]) + E.lp16(b0) + sig
add("vid_collision_legit_accepts", raw_wire(legit))
add("vid_collision_forged_same_4byte_vid_rejects", raw_wire(forged))
# two creatives with the same 4-byte vid in one signed family (a different 32-byte digest): device must fail closed (ledger key uniqueness). Use the fixture pair.
bodies2 = [legit, forged] + [E.canon_creative(c) for c in CR["brew"][2:]]
ds2 = [E.digest(b) for b in bodies2]; sig2 = E.sign(priv, E.signed_msg(core["brew"], ds2))
add("signed_family_with_vid_collision_inside_rejects", E.lp16(core["brew"]) + bytes([6, 0b000011]) + b"".join(ds2[2:]) + E.lp16(legit) + E.lp16(forged) + sig2)
G["envelope"] = dict(pubkey_uncompressed_hex=pub_hex, entries=EV, vid_collision=dict(legit_hex=fx["legit_hex"], forged_hex=fx["forged_hex"], vid_hex=fx["vid_hex"], search=fx["search"]))
blob = json.dumps(G, sort_keys=True, separators=(",", ":"))
G["sha256_of_everything_above"] = hashlib.sha256(blob.encode()).hexdigest()
json.dump(G, open("golden_v2.json", "w"), indent=1, sort_keys=True)
print("golden_v2.json written:", len(blob), "bytes;", len(G["select"]), "select traces;", len(G["pending"]), "pending cases;", len(EV), "envelope entries;", G["sha256_of_everything_above"])
print("envelope accept/reject:", sum(e["expect"] == "accept" for e in EV), sum(e["expect"] == "reject" for e in EV))
for t in G["select"][:3]: print(t["name"], [o["expect_idx"] for o in t["ops"] if o["op"] == "select"])
