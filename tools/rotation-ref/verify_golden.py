"""CI gate #1 (Python side): the COMMITTED golden_v2.json must be reproduced by the Python reference (refkernel.py, envelope.py).
Rust gate #2 is rustgold (cargo run --release -- golden_v2.json).  A change to either implementation or to the file that breaks one of them fails CI."""
import json, sys, os, hashlib
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import refkernel as R, envelope as E
from cryptography.hazmat.primitives.asymmetric import ec
# The one change from the working copy: the file now lives at its committed path (OI-ROT-1).
GOLDEN = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "core", "testdata", "golden", "rotation", "golden_v2.json")
g = json.load(open(os.environ.get("PROXIM_ROTATION_GOLDEN", GOLDEN)))
fails = []
if g["decay"] != R.DECAY: fails.append("decay")
for r in g["sched_index"]:
    if R.sched_index(r[0], r[1]) != r[2]: fails.append(("sched", r))
for r in g["digest_vectors"]:
    b = bytes.fromhex(r["canon_hex"]); d = E.digest(b)
    if d.hex() != r["digest_hex"] or E.vid_of(d) != r["vid"]: fails.append(("digest", r["family"], r["idx"]))
for r in g["derive"]:
    v, p, ph = R.derive(bytes.fromhex(r["signpost_id_hex"]), [bytes.fromhex(x) for x in r["digests_hex"]])
    if (v, p, ph) != (r["vids"], r["pos"], r["phi"]): fails.append(("derive", r["family"]))
nops = 0
for t in g["select"]:
    dg = [bytes.fromhex(x) for x in t["digests_hex"]]; vids, pos, phi = R.derive(bytes.fromhex(t["signpost_id_hex"]), dg)
    led = R.Ledger(lru=t["lru"])
    if t["preload"]:
        led.rows = {v: (c, d) for v, c, d in t["preload"]["rows"]}; led.recent = list(t["preload"]["recent"]); led.last_day = t["preload"]["last_day"]
    pend = None; last = None
    for op in t["ops"]:
        nops += 1
        if op["op"] == "select":
            o = R.select(led, op["day"], vids, pos, phi, op["avail_mask"], t["dow_mask"], op.get("lane", t["lane"]), op.get("lanes", t["lanes"]))
            if (None if o is None else o[0]) != op["expect_idx"] or (None if o is None else o[1]) != op["expect_new_count"]: fails.append(("select", t["name"], op["day"]))
            pend = (o, op["day"])
        elif op["op"] == "commit":
            (i, b), d = pend[0], pend[1]; R.commit(led, vids[i], d, b); last = (i, d)
        elif op["op"] == "dismiss":
            i, d = last; R.dismiss(led, vids[i], d)
    if led.snapshot() != t["final"]: fails.append(("final", t["name"]))
pub = ec.EllipticCurvePublicKey.from_encoded_point(ec.SECP256R1(), bytes.fromhex(g["envelope"]["pubkey_uncompressed_hex"]))
for e in g["envelope"]["entries"]:
    ok, out = E.verify_entry(pub, bytes.fromhex(e["wire_hex"]))
    if ("accept" if ok else "reject") != e["expect"]: fails.append(("envelope", e["name"]))
    if ok and sorted(out) != e["shipped_idx"]: fails.append(("envelope shipped", e["name"]))
# Pending window (added with R-2026-10-17; the working copy's gate did not replay these): the
# corrected kernel still reproduces the frozen cases.
for c in g["pending"]:
    vids, pos, phi = R.derive(bytes.fromhex(c["signpost_id_hex"]), [bytes.fromhex(x) for x in c["digests_hex"]])
    led = R.Ledger(); P = R.Pending(); log = []; day = c["start_day"]
    choose = lambda: R.select(led, day, vids, pos, phi, 0b111111, c["dow_mask"], 0, 1)
    do_commit = lambda i, nc, d: R.commit(led, vids[i], d, nc)
    for ev in c["events"]:
        if ev[0] == "emit": log.append(["emit", ev[1], P.emit("f", ev[1], choose, do_commit, day)])
        elif ev[0] == "tick": P.tick("f", ev[1], do_commit); log.append(["tick", ev[1], len(P.log)])
        elif ev[0] == "mark": log.append(["mark", ev[1], P.mark_shown("f", ev[1], ev[2], do_commit)])
        elif ev[0] == "day": day = ev[1]
    if log != c["expect_log"] or led.snapshot() != c["expect_ledger"]: fails.append(("pending", c["name"]))
fx = g["envelope"]["vid_collision"]
if E.vid_of(E.digest(bytes.fromhex(fx["legit_hex"]))) != E.vid_of(E.digest(bytes.fromhex(fx["forged_hex"]))): fails.append("vid fixture")
print("python gate:", "OK" if not fails else fails, f"({len(g['select'])} traces, {nops} ops, {len(g['pending'])} pending cases, {len(g['envelope']['entries'])} envelope entries)")
sys.exit(1 if fails else 0)
