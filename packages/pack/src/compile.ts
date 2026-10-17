// Deterministic pack compiler (docs/pack-format.md sections 4, 7 and 8; Addendum 02 4.4).
// Input is a snapshot of signed revisions as the database holds them (signed once at approval);
// the compiler never signs entries, so the same snapshot always gives the same bytes.
import { deflateRawSync, constants } from "node:zlib";
import { b64decode } from "./b64.ts";
import { parseCanonical } from "./canon.ts";
import {
  LIMITS, PACK_KIND, type Row, creativeDigest, encodeEntry, encodePack, encodeStream, hex, parseCell, sha256,
} from "./format.ts";

export interface SnapshotRevision {
  /** Canonical JSON of the Signpost, exactly as signed. */
  core: string;
  /** Creative bodies in declared order (index 0 is the base wording). */
  bodies: string[];
  kid: string;
  /** Base64url of the 64-byte ES256 signature over the envelope v2 message. */
  sig: string;
}

export interface Snapshot {
  cell: string;
  seq: number | bigint;
  tier: number;
  pack_kind: "hosted_tile";
  revisions: SnapshotRevision[];
  /** Test-only: per-id ship masks. The v1 compiler is all-or-none, so production omits this. */
  ship_masks?: Record<string, number>;
}

export interface Compiled {
  pack: Buffer;
  sha256: string;
  bytes: number;
  ids: string[];
}

export class CompileError extends Error {}

/** v1 cap on declared wordings per revision (Addendum 02 4.1; the wire allows 8). */
export const V1_MAX_CREATIVES = 6;

const enc = new TextEncoder();

export function compile(s: Snapshot): Compiled {
  const cell = parseCell(s.cell);
  if (cell === null) throw new CompileError("cell must be 15 lowercase hex digits");
  if (s.pack_kind !== "hosted_tile") throw new CompileError("only hosted_tile packs exist before M15");
  const items = s.revisions.map((r) => {
    const core = Buffer.from(r.core, "utf8");
    let doc: { [k: string]: unknown };
    try {
      doc = parseCanonical(core) as { [k: string]: unknown };
    } catch (e) {
      throw new CompileError(`core is not canonical JSON: ${(e as Error).message}`);
    }
    const id = String(doc.id);
    if (doc.pack_kind !== "hosted_tile") throw new CompileError(`${id}: core must carry pack_kind hosted_tile`);
    if (r.bodies.length === 0 || r.bodies.length > V1_MAX_CREATIVES) throw new CompileError(`${id}: 1 to ${V1_MAX_CREATIVES} wordings`);
    if (core.length > LIMITS.coreBytes) throw new CompileError(`${id}: core too large`);
    const bodies = r.bodies.map((b) => Buffer.from(enc.encode(b)));
    if (bodies.some((b) => b.length === 0 || b.length > LIMITS.bodyBytes)) throw new CompileError(`${id}: wording length`);
    const vids = bodies.map((b) => hex(creativeDigest(b).subarray(0, 4)));
    if (new Set(vids).size !== vids.length) throw new CompileError(`${id}: two wordings share a vid`);
    const sig = b64decode(r.sig);
    if (!sig || sig.length !== 64) throw new CompileError(`${id}: signature must be 64 bytes`);
    const rank = typeof doc.rank_milli === "number" ? doc.rank_milli : 1000;
    const full = (1 << bodies.length) - 1;
    const mask = s.ship_masks?.[id] ?? full;
    return { id, rank, row: { entry: encodeEntry(core, bodies, mask), kid: r.kid, sig } as Row };
  });
  const ids = items.map((i) => i.id);
  if (new Set(ids).size !== ids.length) throw new CompileError("two revisions of one signpost in a pack");
  // Rank descending, then id ascending (byte order): the order is part of the deterministic output.
  items.sort((a, b) => b.rank - a.rank || Buffer.compare(Buffer.from(a.id), Buffer.from(b.id)));
  const rows = items.map((i) => i.row);
  const compressed = deflateRawSync(encodeStream(rows), {
    level: 9, windowBits: 15, memLevel: 8, strategy: constants.Z_DEFAULT_STRATEGY,
  });
  const pack = encodePack({ packKind: PACK_KIND.hosted_tile, tier: s.tier, cell, seq: BigInt(s.seq) }, compressed, rows);
  if (s.tier === 0 && pack.length > LIMITS.tier0Budget) {
    throw new CompileError(`tier-0 pack is ${pack.length} bytes, over the ${LIMITS.tier0Budget} budget`);
  }
  return { pack, sha256: hex(sha256(pack)), bytes: pack.length, ids: items.map((i) => i.id) };
}

/** The manifest payload for compiled tiers (canonical JSON is the caller's: see canon.ts). */
export function manifestPayload(cell: string, seq: number | bigint, issuedAtMs: number, expiresAtMs: number, tiers: { tier: number; c: Compiled }[]) {
  return {
    format: "proxim-manifest/1", cell, seq: typeof seq === "bigint" ? seq : BigInt(seq), pack_kind: "hosted_tile",
    issued_at_ms: issuedAtMs, expires_at_ms: expiresAtMs,
    tiers: tiers.map(({ tier, c }) => ({ tier, bytes: c.bytes, sha256: c.sha256 })),
  };
}
