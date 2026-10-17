// Strict JSON parse and canonical JSON write (engine spec 3.2), mirroring
// core/crates/proxim-types/src/{strict,canon}.rs: integers only (bigint beyond 2^53), no null,
// no duplicate keys, keys sorted by UTF-8 bytes, only `"` `\` and controls escaped.

export type Json = boolean | number | bigint | string | Json[] | { [k: string]: Json };

export class JsonError extends Error {}

const MAX_DEPTH = 32;

export function parseStrict(text: string): Json {
  let i = 0;
  const fail = (): never => {
    throw new JsonError(`invalid JSON at ${i}`);
  };
  const ws = () => {
    while (i < text.length && " \t\n\r".includes(text[i])) i++;
  };
  const str = (): string => {
    if (text[i] !== '"') fail();
    i++;
    let out = "";
    for (;;) {
      if (i >= text.length) fail();
      const c = text[i++];
      if (c === '"') return out;
      if (c.charCodeAt(0) < 0x20) fail();
      if (c !== "\\") {
        out += c;
        continue;
      }
      const e = text[i++];
      const simple: Record<string, string> = { '"': '"', "\\": "\\", "/": "/", b: "\b", f: "\f", n: "\n", r: "\r", t: "\t" };
      if (e in simple) out += simple[e];
      else if (e === "u") {
        const hex = text.slice(i, i + 4);
        if (!/^[0-9a-fA-F]{4}$/.test(hex)) fail();
        i += 4;
        const cu = parseInt(hex, 16);
        if (cu >= 0xd800 && cu <= 0xdbff) {
          const lo = text.slice(i, i + 6);
          if (!/^\\u[dD][c-fC-F][0-9a-fA-F]{2}$/.test(lo)) fail();
          i += 6;
          out += String.fromCharCode(cu, parseInt(lo.slice(2), 16));
        } else if (cu >= 0xdc00 && cu <= 0xdfff) fail();
        else out += String.fromCharCode(cu);
      } else fail();
    }
  };
  const value = (depth: number): Json => {
    ws();
    const c = text[i];
    if (c === "{" || c === "[") {
      if (depth + 1 > MAX_DEPTH) fail();
    }
    if (c === "{") {
      i++;
      const obj: { [k: string]: Json } = {};
      ws();
      if (text[i] === "}") {
        i++;
        return obj;
      }
      for (;;) {
        ws();
        const k = str();
        if (Object.prototype.hasOwnProperty.call(obj, k)) fail();
        ws();
        if (text[i++] !== ":") fail();
        obj[k] = value(depth + 1);
        ws();
        const d = text[i++];
        if (d === "}") return obj;
        if (d !== ",") fail();
      }
    }
    if (c === "[") {
      i++;
      const arr: Json[] = [];
      ws();
      if (text[i] === "]") {
        i++;
        return arr;
      }
      for (;;) {
        arr.push(value(depth + 1));
        ws();
        const d = text[i++];
        if (d === "]") return arr;
        if (d !== ",") fail();
      }
    }
    if (c === '"') return str();
    if (text.startsWith("true", i)) {
      i += 4;
      return true;
    }
    if (text.startsWith("false", i)) {
      i += 5;
      return false;
    }
    const m = /^-?(0|[1-9][0-9]*)/.exec(text.slice(i, i + 24));
    if (!m || m[0] === "-0" || /^[.eE]/.test(text.slice(i + m[0].length, i + m[0].length + 1))) fail();
    i += m![0].length;
    const big = BigInt(m![0]);
    if (big < -(2n ** 63n) || big >= 2n ** 64n) fail();
    return Number.isSafeInteger(Number(big)) ? Number(big) : big;
  };
  const v = value(0);
  ws();
  if (i !== text.length) fail();
  return v;
}

const utf8 = new TextEncoder();

function cmpBytes(a: string, b: string): number {
  return Buffer.compare(Buffer.from(utf8.encode(a)), Buffer.from(utf8.encode(b)));
}

function writeString(s: string): string {
  let out = '"';
  for (const ch of s) {
    const cp = ch.codePointAt(0)!;
    if (ch === '"') out += '\\"';
    else if (ch === "\\") out += "\\\\";
    else if (cp < 0x20) out += "\\u00" + cp.toString(16).padStart(2, "0");
    else out += ch;
  }
  return out + '"';
}

export function canonical(v: Json, depth = 0): string {
  if (typeof v === "boolean") return v ? "true" : "false";
  if (typeof v === "bigint") return v.toString();
  if (typeof v === "number") {
    if (!Number.isSafeInteger(v)) throw new JsonError("not an integer");
    return Object.is(v, -0) ? "0" : String(v);
  }
  if (typeof v === "string") return writeString(v);
  if (depth + 1 > MAX_DEPTH) throw new JsonError("too deep");
  if (Array.isArray(v)) return "[" + v.map((x) => canonical(x, depth + 1)).join(",") + "]";
  if (v === null || typeof v !== "object") throw new JsonError("unsupported value");
  const keys = Object.keys(v).sort(cmpBytes);
  return "{" + keys.map((k) => writeString(k) + ":" + canonical(v[k], depth + 1)).join(",") + "}";
}

/** Parse and require the exact canonical bytes. */
export function parseCanonical(bytes: Uint8Array): Json {
  const text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: false }).decode(bytes);
  if (text.charCodeAt(0) === 0xfeff) throw new JsonError("BOM");
  const v = parseStrict(text);
  if (canonical(v) !== text) throw new JsonError("not canonical");
  return v;
}
