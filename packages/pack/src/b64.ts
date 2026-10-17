// Base64url without padding, strict (pack-format 2): one encoding per byte string.

export function b64encode(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString("base64url");
}

export function b64decode(s: string): Uint8Array | null {
  if (!/^[A-Za-z0-9_-]*$/.test(s) || s.length % 4 === 1) return null;
  const out = new Uint8Array(Buffer.from(s, "base64url"));
  return b64encode(out) === s ? out : null;
}
