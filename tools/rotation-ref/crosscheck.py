import sys, random, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import refkernel as R, kernel2 as K
r = random.Random(1); bad = 0; steps = 0
for t in range(4000):
    n = r.choice([1, 2, 3, 4, 5, 6, 8]); vids = r.sample(range(1, 2**32), n)
    order = list(range(n)); r.shuffle(order); pos = [0] * n
    for p, i in enumerate(order): pos[i] = p
    phi = r.getrandbits(32); mask = r.choice([0x7F, 0x1F, 0x3F, 0x5, 0x7F]); lane = r.randrange(3); lanes = 3
    a = R.Ledger(lru=r.choice([2000, 4, 7])); b = K.FamilyState(); lru = a.lru
    day = r.randrange(0, 20)
    for s in range(r.randint(1, 40)):
        day += r.choice([1, 1, 1, 2, 3, 6, 12, 40, 100])
        am = r.getrandbits(n) or 1
        avail = [i for i in range(n) if (am >> i) & 1]
        x = R.select(a, day, vids, pos, phi, am, mask, lane, lanes)
        y = K.pick_hybrid_k(b, day, vids, avail, pos, 3, 256, phi, (lane * n) // lanes, mask)
        steps += 1
        if x[0] != y: bad += 1; break
        nc = K.new_count(b, vids, avail, y, day)
        assert nc == x[1], (nc, x)
        R.commit(a, vids[x[0]], day, x[1]); b.commit(vids[y], day, lru, newc=nc)
        if r.random() < 0.1:
            R.dismiss(a, vids[x[0]], day); b.dismiss(vids[y], day)
        assert a.snapshot()["rows"] == sorted([v, c, d] for v, (c, d) in b.rows.items()), "ledger rows diverged"
print("refkernel vs kernel2: mismatches", bad, "over", steps, "steps (4000 random families: n 1..8, partial avail, gaps up to 100 d, dismissals, LRU 4/7/2000)")
