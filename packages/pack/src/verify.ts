// Second, independent verifier (docs/pack-format.md sections 2-5). The phone runs the Rust one;
// this one proves the format is implementable from the spec and checks the compiler's output.
import { createPublicKey, verify as cryptoVerify, type KeyObject } from "node:crypto";
import { inflateRawSync } from "node:zlib";
import { b64decode } from "./b64.ts";
import { type Json, canonical, parseCanonical, parseStrict } from "./canon.ts";
import {
  ENTRY_AUTH_ES256, FormatError, LIMITS, MAGIC, PACK_KIND, Reader, VERSION, creativeDigest, hex,
  parseCell, sha256, signedMessage, validKid,
} from "./format.ts";

export type Reason =
  | "SIGNATURE_INVALID" | "SEQ_ROLLBACK" | "KID_REVOKED" | "PACK_EXPIRED" | "REVOKED" | "NOT_YET_LIVE";

export class PackError extends Error {
  readonly reason: Reason;
  constructor(reason: Reason) {
    super(reason);
    this.reason = reason;
  }
}

const bad = (): never => {
  throw new PackError("SIGNATURE_INVALID");
};

export interface Guard {
  seq: bigint;
  sha256: string;
}

function checkGuard(prev: Guard | undefined, seq: bigint, digest: string): Guard {
  if (prev && (seq < prev.seq || (seq === prev.seq && prev.sha256 !== digest))) throw new PackError("SEQ_ROLLBACK");
  return { seq, sha256: digest };
}

function checkWindow(issued: bigint, expires: bigint, now: bigint) {
  if (!(issued <= now && now < expires)) throw new PackError("PACK_EXPIRED");
}

export function publicKey(sec1: Uint8Array): KeyObject | null {
  if (sec1.length !== 65 || sec1[0] !== 4) return null;
  try {
    const b = Buffer.from(sec1);
    return createPublicKey({
      key: { kty: "EC", crv: "P-256", x: b.subarray(1, 33).toString("base64url"), y: b.subarray(33).toString("base64url") },
      format: "jwk",
    });
  } catch {
    return null;
  }
}

export function verifyEs256(key: KeyObject, msg: Uint8Array, sig: Uint8Array): boolean {
  if (sig.length !== 64) return false;
  try {
    return cryptoVerify("sha256", msg, { key, dsaEncoding: "ieee-p1363" }, sig);
  } catch {
    return false;
  }
}

// ---- small typed accessors over strict JSON -------------------------------------------------

type Obj = { [k: string]: Json };
const obj = (v: Json | undefined): Obj => (v !== null && typeof v === "object" && !Array.isArray(v) && typeof v !== "bigint" ? v : bad());
const exact = (o: Obj, keys: string[]) => {
  const have = Object.keys(o).sort();
  if (have.length !== keys.length || have.some((k, i) => k !== [...keys].sort()[i])) bad();
  return o;
};
const int = (v: Json | undefined): bigint => (typeof v === "number" ? BigInt(v) : typeof v === "bigint" ? v : bad());
const uint = (v: Json | undefined, max: bigint): bigint => {
  const n = int(v);
  return n >= 0n && n <= max ? n : bad();
};
const U64 = 2n ** 64n - 1n;
const I64 = (v: Json | undefined) => {
  const n = int(v);
  return n >= -(2n ** 63n) && n < 2n ** 63n ? n : bad();
};
const str = (v: Json | undefined): string => (typeof v === "string" ? v : bad());
const arr = (v: Json | undefined): Json[] => (Array.isArray(v) ? v : bad());

// ---- JWS -----------------------------------------------------------------------------------------

export const TYP = { keyset: "proxim-keyset+jws", manifest: "proxim-manifest+jws", revocations: "proxim-revocations+jws" } as const;

interface Jws {
  kid: string;
  payload: Uint8Array;
  signingInput: Buffer;
  sig: Uint8Array;
}

export function parseJws(token: string, typ: string): Jws {
  if (token.length > 262_144) bad();
  const parts = token.split(".");
  if (parts.length !== 3) bad();
  const [h, p, s] = parts.map(b64decode);
  if (!h || !p || !s || s.length !== 64) bad();
  let header: Obj;
  try {
    header = obj(parseCanonical(h!));
  } catch (e) {
    if (e instanceof PackError) throw e;
    return bad();
  }
  exact(header, ["alg", "kid", "typ"]);
  if (header.alg !== "ES256" || header.typ !== typ || !validKid(str(header.kid))) bad();
  return { kid: header.kid as string, payload: p!, signingInput: Buffer.from(parts[0] + "." + parts[1], "ascii"), sig: s! };
}

function decodeDoc(payload: Uint8Array): Obj {
  try {
    return obj(parseCanonical(payload));
  } catch (e) {
    if (e instanceof PackError) throw e;
    return bad();
  }
}

// ---- key set -------------------------------------------------------------------------------------

export interface Root {
  kid: string;
  key: KeyObject;
}

interface Key {
  kid: string;
  purpose: "approve" | "manifest";
  notBefore: bigint;
  notAfter: bigint;
  key: KeyObject;
}

export interface Keyset {
  guard: Guard;
  keys: Key[];
  revokedKids: string[];
}

export function usable(ks: Keyset, kid: string, purpose: Key["purpose"], now: bigint): KeyObject {
  if (ks.revokedKids.includes(kid)) throw new PackError("KID_REVOKED");
  const k = ks.keys.find((x) => x.kid === kid);
  if (!k) return bad();
  if (k.purpose !== purpose || now < k.notBefore || now >= k.notAfter) throw new PackError("KID_REVOKED");
  return k.key;
}

export function acceptKeyset(token: string, roots: Root[], prev: Guard | undefined, now: bigint): Keyset {
  const j = parseJws(token, TYP.keyset);
  const root = roots.find((r) => r.kid === j.kid) ?? bad();
  if (!verifyEs256(root.key, j.signingInput, j.sig)) bad();
  const d = exact(decodeDoc(j.payload), ["format", "version", "issued_at_ms", "expires_at_ms", "keys", "revoked_kids"]);
  const keysIn = arr(d.keys);
  if (d.format !== "proxim-keyset/1" || keysIn.length > 64) bad();
  const revokedKids = arr(d.revoked_kids).map(str);
  const keys: Key[] = [];
  for (const raw of keysIn) {
    const k = exact(obj(raw), ["kid", "purpose", "pub", "not_before_ms", "not_after_ms"]);
    const kid = str(k.kid);
    const purpose = str(k.purpose);
    const notBefore = I64(k.not_before_ms);
    const notAfter = I64(k.not_after_ms);
    const fresh = validKid(kid) && !keys.some((x) => x.kid === kid) && !roots.some((r) => r.kid === kid) &&
      !revokedKids.includes(kid) && notBefore < notAfter && (purpose === "approve" || purpose === "manifest");
    if (!fresh) bad();
    const pub = b64decode(str(k.pub));
    const key = pub && publicKey(pub);
    if (!key) bad();
    keys.push({ kid, purpose: purpose as Key["purpose"], notBefore, notAfter, key: key! });
  }
  if (!revokedKids.every(validKid)) bad();
  checkWindow(I64(d.issued_at_ms), I64(d.expires_at_ms), now);
  return { guard: checkGuard(prev, uint(d.version, U64), hex(sha256(j.payload))), keys, revokedKids };
}

// ---- manifest ------------------------------------------------------------------------------------

export interface Tier {
  tier: number;
  bytes: number;
  sha256: string;
}

export interface Manifest {
  cell: bigint;
  guard: Guard;
  packKind: number;
  tiers: Tier[];
}

export function acceptManifest(token: string, ks: Keyset, cell: bigint, prev: Guard | undefined, now: bigint): Manifest {
  const j = parseJws(token, TYP.manifest);
  if (!verifyEs256(usable(ks, j.kid, "manifest", now), j.signingInput, j.sig)) bad();
  const d = exact(decodeDoc(j.payload), ["format", "cell", "seq", "pack_kind", "issued_at_ms", "expires_at_ms", "tiers"]);
  const tiersIn = arr(d.tiers);
  if (d.format !== "proxim-manifest/1" || d.pack_kind !== "hosted_tile" || parseCell(str(d.cell)) !== cell) bad();
  if (tiersIn.length < 1 || tiersIn.length > 3) bad();
  const tiers: Tier[] = [];
  for (const raw of tiersIn) {
    const t = exact(obj(raw), ["tier", "bytes", "sha256"]);
    const tier = Number(uint(t.tier, 255n));
    const bytes = Number(uint(t.bytes, 2n ** 32n - 1n));
    const digest = str(t.sha256);
    if (tier > 2 || (tiers.length && tiers[tiers.length - 1].tier >= tier) || bytes > LIMITS.packBytes || !/^[0-9a-f]{64}$/.test(digest)) bad();
    tiers.push({ tier, bytes, sha256: digest });
  }
  checkWindow(I64(d.issued_at_ms), I64(d.expires_at_ms), now);
  return { cell, guard: checkGuard(prev, uint(d.seq, U64), hex(sha256(j.payload))), packKind: PACK_KIND.hosted_tile, tiers };
}

// ---- revocations ---------------------------------------------------------------------------------

export type Scope =
  | { scope: "variant"; id: string; rev: number; vid: string }
  | { scope: "signpost_rev"; id: string; rev: number }
  | { scope: "signpost"; id: string }
  | { scope: "sender_org"; sender: string }
  | { scope: "kid"; kid: string };

export interface Revocations {
  guard?: Guard;
  entries: Scope[];
}

const validId = (s: string) => /^[A-Za-z0-9._:-]{1,64}$/.test(s);

export function acceptRevocations(token: string, ks: Keyset, prev: Guard | undefined, now: bigint): Revocations {
  const j = parseJws(token, TYP.revocations);
  if (!verifyEs256(usable(ks, j.kid, "manifest", now), j.signingInput, j.sig)) bad();
  const d = exact(decodeDoc(j.payload), ["format", "seq", "issued_at_ms", "expires_at_ms", "entries"]);
  if (d.format !== "proxim-revocations/1") bad();
  const entries: Scope[] = arr(d.entries).map((raw) => {
    const e = obj(raw);
    switch (e.scope) {
      case "variant":
        exact(e, ["scope", "id", "rev", "vid"]);
        if (!validId(str(e.id)) || !/^[0-9a-f]{8}$/.test(str(e.vid))) bad();
        return { scope: "variant", id: e.id as string, rev: Number(uint(e.rev, 2n ** 32n - 1n)), vid: e.vid as string };
      case "signpost_rev":
        exact(e, ["scope", "id", "rev"]);
        if (!validId(str(e.id))) bad();
        return { scope: "signpost_rev", id: e.id as string, rev: Number(uint(e.rev, 2n ** 32n - 1n)) };
      case "signpost":
        exact(e, ["scope", "id"]);
        if (!validId(str(e.id))) bad();
        return { scope: "signpost", id: e.id as string };
      case "sender_org":
        exact(e, ["scope", "sender"]);
        if (!str(e.sender)) bad();
        return { scope: "sender_org", sender: e.sender as string };
      case "kid":
        exact(e, ["scope", "kid"]);
        if (!validKid(str(e.kid))) bad();
        return { scope: "kid", kid: e.kid as string };
      default:
        return bad();
    }
  });
  checkWindow(I64(d.issued_at_ms), I64(d.expires_at_ms), now);
  return { guard: checkGuard(prev, uint(d.seq, U64), hex(sha256(j.payload))), entries };
}

// ---- pack ----------------------------------------------------------------------------------------

export interface Creative {
  index: number;
  vid: string;
  body: Buffer;
}

export interface Entry {
  index: number;
  id: string;
  rev: number;
  sender: string;
  kid: string;
  liveAt?: bigint;
  core: Obj;
  creatives: Creative[];
}

export interface Dropped {
  index: number;
  id?: string;
  reason: Reason;
}

export interface Opened {
  cell: bigint;
  seq: bigint;
  tier: number;
  entries: Entry[];
  dropped: Dropped[];
}

interface Slot {
  body?: Buffer;
  digest: Buffer;
}

function parseEntry(bytes: Buffer): { core: Buffer; slots: Slot[] } {
  const r = new Reader(bytes);
  const coreLen = r.u16();
  if (coreLen === 0 || coreLen > LIMITS.coreBytes) bad();
  const core = r.take(coreLen);
  const n = r.u8();
  const mask = r.u8();
  if (n === 0 || n > LIMITS.creatives || !(mask & 1) || mask >> n !== 0) bad();
  const shipped = (i: number) => (mask & (1 << i)) !== 0;
  const slots: Slot[] = [];
  for (let i = 0; i < n; i++) slots.push({ digest: shipped(i) ? Buffer.alloc(0) : r.take(32) });
  for (let i = 0; i < n; i++) {
    if (!shipped(i)) continue;
    const len = r.u16();
    if (len === 0 || len > LIMITS.bodyBytes) bad();
    const body = r.take(len);
    slots[i] = { body, digest: creativeDigest(body) };
  }
  const vids = slots.map((s) => hex(s.digest.subarray(0, 4)));
  if (new Set(vids).size !== vids.length) bad();
  if (!r.empty) bad();
  return { core, slots };
}

/** Steps a and c for one entry plus its signature (the golden_v2 envelope vectors). */
export function verifyEnvelope(key: KeyObject, entry: Uint8Array, sig: Uint8Array): { core: Buffer; bodies: [number, Buffer][] } {
  let raw: { core: Buffer; slots: Slot[] };
  try {
    raw = parseEntry(Buffer.from(entry));
  } catch (e) {
    if (e instanceof FormatError) return bad();
    throw e;
  }
  if (!verifyEs256(key, signedMessage(raw.core, raw.slots.map((x) => x.digest)), sig)) bad();
  const bodies: [number, Buffer][] = [];
  raw.slots.forEach((s, i) => {
    if (s.body) bodies.push([i, s.body]);
  });
  return { core: raw.core, bodies };
}

export function openPack(bytes: Uint8Array, m: Manifest, tier: number, ks: Keyset, rl: Revocations, now: bigint): Opened {
  const want = m.tiers.find((t) => t.tier === tier) ?? bad();
  const buf = Buffer.from(bytes);
  if (buf.length > LIMITS.packBytes || buf.length !== want.bytes || hex(sha256(buf)) !== want.sha256) bad();
  try {
    return openChecked(buf, m, tier, ks, rl, now);
  } catch (e) {
    if (e instanceof FormatError) return bad();
    throw e;
  }
}

function openChecked(buf: Buffer, m: Manifest, tier: number, ks: Keyset, rl: Revocations, now: bigint): Opened {
  const r = new Reader(buf);
  const headerOk = r.take(4).equals(MAGIC) && r.u8() === VERSION && r.u8() === m.packKind && r.u8() === ENTRY_AUTH_ES256 &&
    r.u8() === tier && r.u64() === m.cell && r.u64() === m.guard.seq;
  if (!headerOk) bad();
  const nKids = r.u8();
  if (nKids === 0 || nKids > LIMITS.kids) bad();
  const kids: string[] = [];
  for (let i = 0; i < nKids; i++) {
    const kid = r.take(r.u8()).toString("latin1");
    if (!validKid(kid) || kids.includes(kid)) bad();
    kids.push(kid);
  }
  const nEntries = r.u32();
  if (nEntries > LIMITS.entries) bad();
  const compressed = r.take(r.u32());
  const sigs: { kid: string; sig: Buffer }[] = [];
  for (let i = 0; i < nEntries; i++) {
    const kid = kids[r.u8()] ?? bad();
    sigs.push({ kid, sig: r.take(64) });
  }
  if (!r.empty) bad();
  let stream: Buffer;
  try {
    stream = inflateRawSync(compressed, { maxOutputLength: LIMITS.streamBytes });
  } catch {
    return bad();
  }
  const s = new Reader(stream);
  const records: Buffer[] = [];
  for (let i = 0; i < nEntries; i++) {
    const len = s.u32();
    if (len === 0 || len > LIMITS.entryBytes) bad();
    records.push(s.take(len));
  }
  if (!s.empty) bad();

  const out: Opened = { cell: m.cell, seq: m.guard.seq, tier, entries: [], dropped: [] };
  const seen: string[] = [];
  records.forEach((record, index) => {
    const { kid, sig } = sigs[index];
    let id: string | undefined;
    try {
      // a. parse; b. kid; c. signature; d. core.
      const raw = parseEntry(record);
      const key = usable(ks, kid, "approve", now);
      if (!verifyEs256(key, signedMessage(raw.core, raw.slots.map((x) => x.digest)), sig)) bad();
      let core: Obj;
      try {
        core = obj(parseCanonical(raw.core));
      } catch (e) {
        if (e instanceof PackError) throw e;
        return bad();
      }
      id = str(core.id);
      seen.push(id);
      const rev = Number(uint(core.rev, 2n ** 32n - 1n));
      const sender = str(core.sender);
      if (!validId(id) || core.pack_kind !== "hosted_tile") bad();
      const liveAt = core.live_at_ms === undefined ? undefined : I64(core.live_at_ms);
      // e. revocations.
      const revoked = rl.entries.some((e) =>
        (e.scope === "signpost_rev" && e.id === id && e.rev === rev) || (e.scope === "signpost" && e.id === id) ||
        (e.scope === "sender_org" && e.sender === sender) || (e.scope === "kid" && e.kid === kid));
      if (revoked) throw new PackError("REVOKED");
      const creatives: Creative[] = [];
      raw.slots.forEach((slot, i) => {
        const vid = hex(slot.digest.subarray(0, 4));
        if (rl.entries.some((e) => e.scope === "variant" && e.id === id && e.rev === rev && e.vid === vid)) {
          if (i === 0) throw new PackError("REVOKED");
          return;
        }
        if (slot.body) creatives.push({ index: i, vid, body: slot.body });
      });
      // f. live_at.
      if (liveAt !== undefined && now < liveAt) throw new PackError("NOT_YET_LIVE");
      out.entries.push({ index, id, rev, sender, kid, liveAt, core, creatives });
    } catch (e) {
      if (e instanceof FormatError) out.dropped.push({ index, id, reason: "SIGNATURE_INVALID" });
      else if (e instanceof PackError) out.dropped.push({ index, id, reason: e.reason });
      else throw e;
    }
  });
  if (new Set(seen).size !== seen.length) bad();
  return out;
}

/** The canonical summary recorded by the golden vectors (proxim_pack::testkit::summary). */
export function summary(p: Opened): Json {
  return {
    cell: p.cell.toString(16).padStart(15, "0"),
    seq: Number(p.seq),
    tier: p.tier,
    entries: p.entries.map((e) => {
      const v: Obj = { index: e.index, id: e.id, rev: e.rev, kid: e.kid, creatives: e.creatives.map((c) => ({ index: c.index, vid: c.vid })) };
      if (e.liveAt !== undefined) v.live_at_ms = e.liveAt;
      return v;
    }),
    dropped: p.dropped.map((d) => (d.id === undefined ? { index: d.index, reason: d.reason } : { index: d.index, id: d.id, reason: d.reason })),
  };
}

export { canonical, parseStrict };
