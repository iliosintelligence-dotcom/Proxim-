"""[MODEL] Reference selector for the revised section 5 (pure, integer-only, no clock, no randomness, no install salt).
Policies:
  lrs_install : the draft's rule (decayed least-recently-shown, tie by a per-install static key)           -- baseline
  date_global : family-global, date-keyed, stateless: pos = (day + phi) mod n  (critic's alternative)
  hybrid      : date-keyed PRIOR + per-install LEDGER GUARD.  eligible = avail minus last_vid, with decayed count <= min + SLACK;
                pick the eligible wording whose position is nearest at/after today's date position  (+ optional lane offset)
State (per install x family): rows {vid:(count,last_day)}, last_vid, last_day.  Constants are integers; DECAY is shipped as a constant table.
"""
import hashlib

ONE, SAT, DISMISS, PURGE_BELOW, MAX_AGE = 256, 65535, 512, 13, 90
RECENT_MAX = 4
DECAY = [round(65536 * 2 ** (-d / 14)) for d in range(MAX_AGE + 1)]


def cn_eff(row, day):
    """decayed count; a row older than 90 days or below PURGE_BELOW is 'purged' (reads as 0) -- evaluated at READ, so ports agree."""
    if row is None:
        return 0
    c, t = row
    dt = day - t
    if dt < 0:
        dt = 0
    if dt > MAX_AGE:
        return 0
    v = (c * DECAY[dt]) >> 16
    return 0 if v < PURGE_BELOW else v


class FamilyState:
    __slots__ = ("rows", "last_vid", "last_day", "recent")

    def __init__(self):
        self.rows = {}
        self.last_vid = None
        self.last_day = None
        self.recent = []            # most recent shown vids, newest last (kept to RECENT_MAX)

    def eff(self, day):
        """(last_vid, recent) as seen at `day`: family_state older than MAX_AGE reads as empty (so ports agree without a purge job)."""
        if self.last_day is None or day - self.last_day > MAX_AGE:
            return None, []
        return self.last_vid, self.recent

    def commit(self, vid, day, lru_limit=None, newc=None):
        c = cn_eff(self.rows.get(vid), day)
        self.rows[vid] = (min(SAT, c + ONE) if newc is None else newc, day)
        self.last_vid, self.last_day = vid, day
        self.recent = (self.recent + [vid])[-RECENT_MAX:]
        if lru_limit is not None and len(self.rows) > lru_limit:      # evict oldest last_day, ties by smaller vid
            victim = min(self.rows, key=lambda k: (self.rows[k][1], k))
            if victim != vid:
                del self.rows[victim]

    def dismiss(self, vid, day):
        c = cn_eff(self.rows.get(vid), day)
        self.rows[vid] = (min(SAT, c + DISMISS), day)


def counts_plain(st, vids, idxs, day):
    """{idx: (decayed count, has_live_row)}; a wording with no live row reads 0 (draft semantics: 'new wordings show first')."""
    out = {}
    for i in idxs:
        row = st.rows.get(vids[i]); c = cn_eff(row, day)
        out[i] = (c, row is not None and c > 0)
    return out


def new_count(st, vids, avail, i, day):
    """value written on commit: max(own + 256, 1 + highest count among the OTHER available wordings).  Equals the draft's own+256 in steady state;
    it stops a freshly added wording (count 0) from monopolising the rotation until it catches up (S15 revision scenario)."""
    ci = cn_eff(st.rows.get(vids[i]), day)
    top = max((cn_eff(st.rows.get(vids[j]), day) for j in avail if j != i), default=0)
    return min(SAT, max(ci + ONE, top + 1))


def u32(*parts) -> int:
    return int.from_bytes(hashlib.sha256(b"|".join(parts)).digest()[:4], "big")


def pick_lrs_install(st, day, vids, avail, ukey, floor=False):
    """vids: list of vid per index; avail: list of indices; ukey[i]: per-install static tie key. argmin (cn, [no-row first], ukey, idx) excluding last.
    floor=False is the DRAFT rule exactly (no family_state purge either: kept for the cross-check)."""
    lv = st.last_vid if not floor else st.eff(day)[0]
    cand = [i for i in avail if not (len(avail) >= 2 and vids[i] == lv)]
    if not floor:
        return min(cand, key=lambda i: (cn_eff(st.rows.get(vids[i]), day), ukey[i], i))
    return min(cand, key=lambda i: (cn_eff(st.rows.get(vids[i]), day), ukey[i], i))


def pick_date_global(day, n, avail, phi=0):
    t = (day + phi) % n
    for k in range(n):
        i = (t + k) % n
        if i in avail:
            return i
    raise ValueError


def pick_hybrid(st, day, vids, avail, pos, slack, phi=0, lane_off=0):
    """pos[i]: position of creative i in the family-global order (0..n-1, unique).  Returns creative index."""
    n = len(vids)
    t = (day + phi + lane_off) % n
    cand = [i for i in avail if not (len(avail) >= 2 and vids[i] == st.last_vid)]
    cs = {i: cn_eff(st.rows.get(vids[i]), day) for i in cand}
    m = min(cs.values())
    elig = [i for i in cand if cs[i] <= m + slack]
    return min(elig, key=lambda i: ((pos[i] - t) % n, i))


def sched_index(day, dow_mask=0b1111111):
    """index of `day` among scheduled days. day 0 is a Monday (days since 1970-01-05). Days not in the mask share the previous scheduled day's index."""
    dow = day % 7
    d = bin(dow_mask).count("1")
    return (day // 7) * d + bin(dow_mask & ((1 << dow) - 1)).count("1")


def pick_hybrid_k(st, day, vids, avail, pos, K, slack, phi=0, lane_off=0, dow_mask=0b1111111, shift=False):
    """date prior (schedule-aware index) + vetoes: (a) not among the K most recently shown vids (K capped at |avail|-1),
    (b) decayed count <= min + slack among the remaining.  Nearest at/after today's position wins."""
    n = len(vids)
    si = sched_index(day, dow_mask)
    t = (si + (si // n if shift else 0) + phi + lane_off) % n
    k = min(K, max(0, len(avail) - 1))
    recent = st.eff(day)[1]
    banned = set(recent[-k:]) if k > 0 else set()
    cand = [i for i in avail if vids[i] not in banned]
    cs = counts_plain(st, vids, cand, day)
    m = min(cs[i][0] for i in cand)
    elig = [i for i in cand if cs[i][0] <= m + slack]
    return min(elig, key=lambda i: (cs[i][1], (pos[i] - t) % n, i))     # no-row (new) wordings first, then nearest at/after the date position


