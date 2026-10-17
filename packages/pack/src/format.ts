// Byte layouts of docs/pack-format.md section 4, shared by the compiler and the verifier.
import { createHash } from "node:crypto";

export const MAGIC = Buffer.from("PXPK", "ascii");
export const VERSION = 1;
export const ENTRY_AUTH_ES256 = 1;
export const PACK_KIND = { hosted_tile: 1 } as const;
export const ENVELOPE_DOMAIN = Buffer.from("PROXIM-ENV-V2\0", "ascii");
export const CREATIVE_DOMAIN = Buffer.from("PROXIM-CREATIVE-V2\0", "ascii");

export const LIMITS = {
  packBytes: 4_194_304,
  streamBytes: 4_194_304,
  entries: 4096,
  kids: 16,
  entryBytes: 65_536,
  coreBytes: 16_384,
  creatives: 8,
  bodyBytes: 1024,
  tier0Budget: 350_000,
} as const;

export function sha256(...parts: Uint8Array[]): Buffer {
  const h = createHash("sha256");
  for (const p of parts) h.update(p);
  return h.digest();
}

export function creativeDigest(body: Uint8Array): Buffer {
  return sha256(CREATIVE_DOMAIN, body);
}

const u16 = (n: number) => {
  const b = Buffer.alloc(2);
  b.writeUInt16BE(n);
  return b;
};
const u32 = (n: number) => {
  const b = Buffer.alloc(4);
  b.writeUInt32BE(n);
  return b;
};
const u64 = (n: bigint) => {
  const b = Buffer.alloc(8);
  b.writeBigUInt64BE(n);
  return b;
};

/** "PROXIM-ENV-V2" 0x00 || u16(len core) || core || u8(n) || d_0 .. d_(n-1) */
export function signedMessage(core: Uint8Array, digests: Uint8Array[]): Buffer {
  return Buffer.concat([ENVELOPE_DOMAIN, u16(core.length), core, Buffer.from([digests.length]), ...digests]);
}

export function encodeEntry(core: Uint8Array, bodies: Uint8Array[], shipMask: number): Buffer {
  // Withheld digests first, then shipped bodies, each in index order (golden_v2 envelope layout).
  const shipped = (i: number) => (shipMask & (1 << i)) !== 0;
  const parts: Uint8Array[] = [u16(core.length), core, Buffer.from([bodies.length, shipMask])];
  bodies.forEach((b, i) => {
    if (!shipped(i)) parts.push(creativeDigest(b));
  });
  bodies.forEach((b, i) => {
    if (shipped(i)) parts.push(u16(b.length), b);
  });
  return Buffer.concat(parts);
}

export interface Row {
  entry: Uint8Array;
  kid: string;
  sig: Uint8Array;
}

export interface Header {
  packKind: number;
  tier: number;
  cell: bigint;
  seq: bigint;
}

export function encodeStream(rows: Row[]): Buffer {
  return Buffer.concat(rows.flatMap((r) => [u32(r.entry.length), r.entry]));
}

/** Header, kid table (first-use order), counts, compressed stream, signature table. */
export function encodePack(h: Header, compressed: Uint8Array, rows: Row[]): Buffer {
  const kids: string[] = [];
  for (const r of rows) if (!kids.includes(r.kid)) kids.push(r.kid);
  return Buffer.concat([
    MAGIC,
    Buffer.from([VERSION, h.packKind, ENTRY_AUTH_ES256, h.tier]),
    u64(h.cell),
    u64(h.seq),
    Buffer.from([kids.length]),
    ...kids.flatMap((k) => [Buffer.from([k.length]), Buffer.from(k, "ascii")]),
    u32(rows.length),
    u32(compressed.length),
    compressed,
    ...rows.flatMap((r) => [Buffer.from([kids.indexOf(r.kid)]), r.sig]),
  ]);
}

export class Reader {
  private pos = 0;
  private readonly b: Buffer;
  constructor(b: Buffer) {
    this.b = b;
  }
  take(n: number): Buffer {
    if (n > this.b.length - this.pos) throw new FormatError();
    const out = this.b.subarray(this.pos, this.pos + n);
    this.pos += n;
    return out;
  }
  u8(): number {
    return this.take(1)[0];
  }
  u16(): number {
    return this.take(2).readUInt16BE();
  }
  u32(): number {
    return this.take(4).readUInt32BE();
  }
  u64(): bigint {
    return this.take(8).readBigUInt64BE();
  }
  get empty(): boolean {
    return this.pos === this.b.length;
  }
}

export class FormatError extends Error {}

export const validKid = (k: string) => /^[a-z0-9-]{1,32}$/.test(k);

export const hex = (b: Uint8Array) => Buffer.from(b).toString("hex");

/** H3 cell as 15 lowercase hex digits. */
export function parseCell(s: string): bigint | null {
  return /^[0-9a-f]{15}$/.test(s) ? BigInt("0x" + s) : null;
}
