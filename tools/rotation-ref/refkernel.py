"""NORMATIVE reference of select_variant v2 + pending-decision state machine (integer-only, pure). Every other implementation
(Rust first, then WASM/Kotlin/Swift) must reproduce golden_v2.json bit for bit.  No floats, no clock, no randomness, no install salt."""
import hashlib

ONE, SAT, DISMISS, PURGE_BELOW, MAX_AGE, SLACK, K_MAX, RECENT_MAX = 256, 65535, 512, 13, 90, 256, 3, 4
W_EMIT_MS, LATE_MS = 60_000, 600_000
DECAY = [round(65536 * 2 ** (-d / 14)) for d in range(MAX_AGE + 1)]     # shipped as constants in golden_v2.json; CI compares
ROT = b"PROXIM-ROT-V2\x00"


def sha(b): return hashlib.sha256(b).digest()
def u32be(b): return int.from_bytes(b[:4], "big")


def derive(signpost_id: bytes, digests):
    """vid = first 4 B of the creative digest (LOCAL LEDGER KEY ONLY, never an authenticator); pos = rank of key = u32(SHA256(ROT|'ord'|signpost_id|digest)); phi = u32(SHA256(ROT|'phase'|signpost_id))."""
    n = len(digests)
    vids = [u32be(d) for d in digests]
    assert len(set(vids)) == n, "vid collision inside a family: the compiler rejects, the device fails closed"
    keys = [u32be(sha(ROT + b"ord" + signpost_id + d)) for d in digests]
    order = sorted(range(n), key=lambda i: (keys[i], i)); pos = [0] * n
    for p, i in enumerate(order): pos[i] = p
    return vids, pos, u32be(sha(ROT + b"phase" + signpost_id))


def popcount(x): return bin(x).count("1")


def sched_index(day, mask):
    """day 0 = a Monday (days since 1970-01-05, device-local date). mask: 7 bits, bit0 = Monday; 0 reads as 0b1111111."""
    mask = mask or 0x7F
    dow = day % 7
    return (day // 7) * popcount(mask) + popcount(mask & ((1 << dow) - 1))


class Ledger:
    def __init__(self, lru=2000):
        self.rows = {}          # vid -> (count, last_day)
        self.recent = []        # newest last, <= RECENT_MAX
        self.last_day = None
        self.lru = lru

    def snapshot(self):
        return {"rows": sorted([v, c, d] for v, (c, d) in self.rows.items()), "recent": list(self.recent), "last_day": self.last_day}

    def cn(self, vid, day):
        r = self.rows.get(vid)
        if r is None: return 0, False
        c, t = r; dt = max(0, day - t)
        if dt > MAX_AGE: return 0, False
        v = (c * DECAY[dt]) >> 16
        return (v, True) if v >= PURGE_BELOW else (0, False)

    def eff_recent(self, day):
        if self.last_day is None or day - self.last_day > MAX_AGE: return []
        return self.recent


def select(led, day, vids, pos, phi, avail_mask, dow_mask=0x7F, lane=0, lanes=1):
    """returns (idx, new_count) or None.  new_count is what commit will write; it is fixed at decision time and stored in the pending decision."""
    n = len(vids)
    A = [i for i in range(n) if (avail_mask >> i) & 1]
    if not A: return None
    K = min(K_MAX, len(A) - 1)
    recent = led.eff_recent(day); banned = set(recent[max(0, len(recent) - K):]) if K > 0 else set()
    cand = [i for i in A if vids[i] not in banned]
    cs = {i: led.cn(vids[i], day) for i in A}                       # (count, has_live_row); no live row reads 0
    m = min(cs[i][0] for i in cand)
    elig = [i for i in cand if cs[i][0] <= m + SLACK]
    lane_off = (lane * n) // max(1, lanes)
    t = (sched_index(day, dow_mask) + phi + lane_off) % n
    best = min(elig, key=lambda i: (1 if cs[i][1] else 0, (pos[i] - t) % n, i))
    top = max((cs[j][0] for j in A if j != best), default=0)
    return best, min(SAT, max(cs[best][0] + ONE, top + 1))


def commit(led, vid, day, new_count):
    led.rows[vid] = (new_count, day)
    led.recent = (led.recent + [vid])[-RECENT_MAX:]
    led.last_day = day
    if len(led.rows) > led.lru:
        victim = min((k for k in led.rows if k != vid), key=lambda k: (led.rows[k][1], k))
        del led.rows[victim]


def dismiss(led, vid, day):
    led.rows[vid] = (min(SAT, led.cn(vid, day)[0] + DISMISS), day)


class Pending:
    """SDK-level state machine for S12. Memory-only. src of commit: 'mark' (markShown) or 'emit' (window expiry). Billing only on markShown.

    R-2026-10-17 (owner decision, Addendum 02 5.3): window expiry is applied on EVERY call, not only on tick. A decision
    whose window has passed is committed as an emission (rule 3) before a re-ask makes a new decision, and before a
    markShown is judged, so a late markShown can only bill (rule 4), never commit and bill as if it were in the window.
    Every commit is written with the day the decision was made (stored with it), whenever the commit happens.
    do_commit(idx, new_count, day)."""
    def __init__(self): self.p = {}; self.log = []
    def _expire(self, fam, now_ms, do_commit):
        cur = self.p.get(fam)
        if cur and not cur["done"] and now_ms - cur["t"] >= W_EMIT_MS:
            do_commit(cur["idx"], cur["base"], cur["day"]); cur["done"] = "emit"; self.log.append(("commit_emit", now_ms))
    def emit(self, fam, now_ms, choose, do_commit, day):
        cur = self.p.get(fam)
        if cur and not cur["done"] and now_ms - cur["t"] < W_EMIT_MS: return cur["idx"]      # re-ask inside the window: same decision
        self._expire(fam, now_ms, do_commit)
        idx, base = choose()
        self.p[fam] = dict(idx=idx, base=base, t=now_ms, day=day, done=None)   # `base` = new_count fixed at decision time
        return idx
    def tick(self, fam, now_ms, do_commit):
        self._expire(fam, now_ms, do_commit)
    def mark_shown(self, fam, now_ms, visible_ms, do_commit):
        self._expire(fam, now_ms, do_commit)
        cur = self.p.get(fam); bill = False
        if not cur: return "ignored"
        if cur["done"] == "mark": return "duplicate"
        if cur["done"] is None:
            do_commit(cur["idx"], cur["base"], cur["day"]); cur["done"] = "mark"; self.log.append(("commit_mark", now_ms))
            return "billed" if visible_ms >= 1000 else "committed_not_viewable"
        # already emission-committed: billing only, never a second charge
        if now_ms - cur["t"] <= LATE_MS and visible_ms >= 1000:
            cur["done"] = "mark"; self.log.append(("bill_late", now_ms)); return "billed_late"
        return "too_late"
