"""[MODEL/M] Reference for the S2 fix: signed envelope = core || n_declared || FULL 32-byte creative digests.
Wire entry elides the digest of every SHIPPED creative (the device recomputes it from the body), carries the digest of
every UNSHIPPED creative, and signs once (ES256 P-256, raw r||s 64 B). A body that does not hash to the signed digest at its
index changes the signed message, so the whole entry fails closed. vid = first 4 B of the digest = local ledger key only.

Python 3, stdlib + `cryptography`.  Run with:  python3 -I envelope.py   (self-test + fuzz)
"""
import hashlib, struct, random, json, sys, os
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric.utils import decode_dss_signature, encode_dss_signature
from cryptography.exceptions import InvalidSignature

ENV_PREFIX = b"PROXIM-ENV-V2\x00"
CRE_PREFIX = b"PROXIM-CREATIVE-V2\x00"
MAX_N = 8


def lp16(b: bytes) -> bytes:
    assert len(b) < 65536
    return struct.pack(">H", len(b)) + b


def canon_creative(c: dict) -> bytes:
    """Canonical creative bytes (fixed field order, u16 length prefixes, UTF-8 NFC text assumed done by the compiler)."""
    return (lp16(c["locale"].encode()) + lp16(c["headline"].encode()) + lp16(c["body"].encode()) +
            lp16(c["glance"].encode()) + bytes([c["cta"], c["icon"], c["layout"]]) +
            lp16(c.get("image", b"")) + lp16(c.get("code", "").encode()))


def digest(body: bytes) -> bytes:
    return hashlib.sha256(CRE_PREFIX + body).digest()


def vid_of(d: bytes) -> int:
    return int.from_bytes(d[:4], "big")


def signed_msg(core: bytes, digests: list) -> bytes:
    assert 1 <= len(digests) <= MAX_N and all(len(d) == 32 for d in digests)
    return ENV_PREFIX + lp16(core) + bytes([len(digests)]) + b"".join(digests)


def sign(priv, msg: bytes) -> bytes:
    r, s = decode_dss_signature(priv.sign(msg, ec.ECDSA(hashes.SHA256())))
    return r.to_bytes(32, "big") + s.to_bytes(32, "big")


def verify_sig(pub, sig: bytes, msg: bytes) -> bool:
    if len(sig) != 64:
        return False
    try:
        pub.verify(encode_dss_signature(int.from_bytes(sig[:32], "big"), int.from_bytes(sig[32:], "big")), msg, ec.ECDSA(hashes.SHA256()))
        return True
    except InvalidSignature:
        return False


# ---- wire entry:  core(lp16) | n(1) | mask(1) | elided digests (ascending idx, unshipped only) | bodies (ascending idx, shipped only, lp16 each) | sig(64)
def build_entry(priv, core: bytes, creatives: list, mask: int) -> bytes:
    bodies = [canon_creative(c) for c in creatives]
    ds = [digest(b) for b in bodies]
    n = len(ds)
    assert mask & 1 and mask < (1 << n), "base (idx 0) must ship; mask within n"
    sig = sign(priv, signed_msg(core, ds))
    out = lp16(core) + bytes([n, mask])
    out += b"".join(ds[i] for i in range(n) if not (mask >> i) & 1)
    out += b"".join(lp16(bodies[i]) for i in range(n) if (mask >> i) & 1)
    return out + sig


def verify_entry(pub, entry: bytes, check_dup=True):
    """Return (True, {idx: body}) or (False, reason). Fail closed; never raises on malformed input."""
    try:
        pos = 0
        (cl,) = struct.unpack_from(">H", entry, pos); pos += 2
        core = entry[pos:pos + cl]; pos += cl
        if len(core) != cl:
            return False, "short core"
        n, mask = entry[pos], entry[pos + 1]; pos += 2
        if not (1 <= n <= MAX_N) or not (mask & 1) or mask >= (1 << n):
            return False, "bad n/mask"
        ds = [None] * n
        for i in range(n):
            if not (mask >> i) & 1:
                d = entry[pos:pos + 32]; pos += 32
                if len(d) != 32:
                    return False, "short digest"
                ds[i] = d
        bodies = {}
        for i in range(n):
            if (mask >> i) & 1:
                (bl,) = struct.unpack_from(">H", entry, pos); pos += 2
                b = entry[pos:pos + bl]; pos += bl
                if len(b) != bl:
                    return False, "short body"
                bodies[i] = b
                ds[i] = digest(b)                      # FULL 32-byte digest recomputed from the shipped body
        sig = entry[pos:pos + 64]; pos += 64
        if pos != len(entry) or len(sig) != 64:
            return False, "trailing/short"
        if check_dup and len(set(d[:4] for d in ds)) != n:
            return False, "vid collision in family"       # compiler also rejects; device fails closed
        if not verify_sig(pub, sig, signed_msg(core, ds)):
            return False, "signature"
        return True, bodies
    except Exception as e:                                   # struct.error, IndexError ...
        return False, "malformed:" + type(e).__name__


def strawman_verify(pub, entry_signed_without_bodies, shipped_bodies, signed_vids):
    """The behaviour the critic feared: signature covers a vid list; bodies checked only against 4-byte vid."""
    return all(vid_of(digest(b)) in signed_vids for b in shipped_bodies)


def mk_creative(rng, fam, i, nonce=""):
    words = "fresh warm local fast friendly quiet bright cosy crisp seasonal family-run hearty zesty classic".split()
    return dict(locale="en", headline=f"{rng.choice(words).title()} {fam} {rng.choice(words)} {i}", body=f"{rng.choice(words)} {rng.choice(words)} at {fam}. {nonce}",
                glance=f"{rng.choice(words)} {fam} {i}", cta=rng.randrange(6), icon=rng.randrange(40), layout=rng.randrange(4), code=f"{fam[:4].upper()}{i}10")


def self_test(seed=20261010, n_fuzz=20000, vid_fixture=None):
    rng = random.Random(seed)
    priv = ec.derive_private_key(int.from_bytes(hashlib.sha256(b"proxim-test-key").digest(), "big") % (2**255), ec.SECP256R1())
    pub = priv.public_key()
    fams = {}
    for f in ("brew", "tyre", "yoga"):
        fams[f] = [mk_creative(rng, f, i) for i in range(6)]
    core = {f: b"core|" + f.encode() + bytes(rng.randrange(256) for _ in range(40)) for f in fams}
    res = {}
    # 1. accept: all masks containing base, shipped bodies come back exactly
    ok = 0
    for f, cs in fams.items():
        for mask in range(1, 64, 2):
            e = build_entry(priv, core[f], cs, mask)
            good, out = verify_entry(pub, e)
            assert good, out
            assert out == {i: canon_creative(cs[i]) for i in range(6) if (mask >> i) & 1}
            ok += 1
    res["accept_cases"] = ok
    # 2. targeted swaps
    def swap_case(f, mask, i, newbody):
        e = build_entry(priv, core[f], fams[f], mask)
        # locate body i in wire and replace it (re-encode with new length)
        n = 6
        ds_el = [digest(canon_creative(fams[f][k])) for k in range(n) if not (mask >> k) & 1]
        bodies = [canon_creative(fams[f][k]) for k in range(n) if (mask >> k) & 1]
        idxs = [k for k in range(n) if (mask >> k) & 1]
        bodies[idxs.index(i)] = newbody
        sig = e[-64:]
        w = lp16(core[f]) + bytes([n, mask]) + b"".join(ds_el) + b"".join(lp16(b) for b in bodies) + sig
        return verify_entry(pub, w, check_dup=False)
    rej = tot = 0
    for f in fams:
        for mask in (0b000001, 0b000111, 0b111111, 0b010101 | 1):
            idxs = [k for k in range(6) if (mask >> k) & 1]
            for i in idxs:
                # (a) another member of the same family
                for j in range(6):
                    if j != i:
                        tot += 1; rej += not swap_case(f, mask, i, canon_creative(fams[f][j]))[0]
                # (b) a member of another family
                for g in fams:
                    if g != f:
                        for j in range(6):
                            tot += 1; rej += not swap_case(f, mask, i, canon_creative(fams[g][j]))[0]
                # (c) unsigned arbitrary body
                tot += 1; rej += not swap_case(f, mask, i, canon_creative(mk_creative(rng, "evil", 9)))[0]
    res["swap_cases"] = tot; res["swap_rejected"] = rej
    assert rej == tot
    # 3. vid-collision fixture (4-byte second preimage), if provided
    if vid_fixture:
        legit = vid_fixture["legit"]; forged = vid_fixture["forged"]
        assert vid_of(digest(legit)) == vid_of(digest(forged)) and legit != forged
        # build a family whose creative 0 is `legit`
        cs = [dict(fams["brew"][i]) for i in range(6)]
        # fixture creative encoded as raw canon bytes: wrap by monkeypatching canon via a sentinel dict
        class Raw(dict): pass
        base = Raw(); base["raw"] = legit
        def canon_any(c): return c["raw"] if isinstance(c, Raw) else canon_creative(c)
        bodies = [canon_any(base)] + [canon_creative(c) for c in cs[1:]]
        ds = [digest(b) for b in bodies]
        sig = sign(priv, signed_msg(core["brew"], ds))
        def wire(b0):
            return lp16(core["brew"]) + bytes([6, 0b000001]) + b"".join(ds[1:]) + lp16(b0) + sig
        assert verify_entry(pub, wire(legit))[0]
        res["forged_vid_body_rejected_by_full_digest"] = (not verify_entry(pub, wire(forged))[0])
        res["strawman_4byte_vid_check_accepts_forged"] = strawman_verify(pub, None, [forged], {vid_of(ds[0])})
        assert res["forged_vid_body_rejected_by_full_digest"] and res["strawman_4byte_vid_check_accepts_forged"]
    # 4. random mutation fuzz: any single/multi-byte mutation of any valid entry is either rejected or yields the identical body map
    base_entries = [(f, m, build_entry(priv, core[f], fams[f], m)) for f in fams for m in (1, 0b000111, 0b111111, 0b101011)]
    accepted_changed = rejected = unchanged = 0
    for t in range(n_fuzz):
        f, m, e = rng.choice(base_entries)
        b = bytearray(e)
        for _ in range(rng.choice((1, 1, 1, 2, 3))):
            op = rng.randrange(4)
            if op == 0 or len(b) < 8:
                b[rng.randrange(len(b))] ^= 1 << rng.randrange(8)
            elif op == 1:
                del b[rng.randrange(len(b))]
            elif op == 2:
                b.insert(rng.randrange(len(b)), rng.randrange(256))
            else:
                i, j = rng.randrange(len(b)), rng.randrange(len(b)); b[i], b[j] = b[j], b[i]
        good, out = verify_entry(pub, bytes(b))
        orig = verify_entry(pub, e)[1]
        if not good:
            rejected += 1
        elif out == orig and bytes(b) == e:
            unchanged += 1          # mutation was a no-op (e.g. swapped equal bytes)
        else:
            accepted_changed += 1
    res.update(fuzz=n_fuzz, fuzz_rejected=rejected, fuzz_noop=unchanged, fuzz_accepted_but_changed=accepted_changed)
    assert accepted_changed == 0
    return res


if __name__ == "__main__":
    fx = None
    p = os.path.join(os.path.dirname(os.path.abspath(__file__)), "vid_fixture.json")
    if os.path.exists(p):
        j = json.load(open(p)); fx = {"legit": bytes.fromhex(j["legit_hex"]), "forged": bytes.fromhex(j["forged_hex"])}
    print(json.dumps(self_test(vid_fixture=fx), indent=1))
