// The Rust-made golden vectors replay in the TypeScript verifier with identical results.
import { test } from "node:test";
import assert from "node:assert/strict";
import { b64decode } from "../src/b64.ts";
import { canonical } from "../src/canon.ts";
import { parseCell } from "../src/format.ts";
import { PackError, acceptKeyset, acceptManifest, acceptRevocations, openPack, summary, type Guard } from "../src/verify.ts";
import { type Any, golden, roots } from "./support.ts";

const g = golden();
const now = BigInt(g.now_ms);
const cell = parseCell(g.cell)!;
const prev = (c: Any): Guard | undefined => (c.prev ? { seq: BigInt(c.prev.seq), sha256: c.prev.sha256 } : undefined);
const outcome = (f: () => Any) => {
  try {
    return f();
  } catch (e) {
    if (e instanceof PackError) return { error: e.reason };
    throw e;
  }
};

test("key set cases", () => {
  for (const c of g.keyset_cases) {
    const got = outcome(() => ({ version: Number(acceptKeyset(c.token, roots(g), prev(c), now).guard.seq) }));
    assert.equal(canonical(got), canonical(c.expect), c.name);
  }
});

test("manifest cases", () => {
  const ks = acceptKeyset(g.keyset, roots(g), undefined, now);
  for (const c of g.manifest_cases) {
    const got = outcome(() => ({ seq: Number(acceptManifest(c.token, ks, cell, prev(c), now).guard.seq) }));
    assert.equal(canonical(got), canonical(c.expect), c.name);
  }
});

test("pack cases", () => {
  const ks = acceptKeyset(g.keyset, roots(g), undefined, now);
  const m = acceptManifest(g.manifest, ks, cell, undefined, now);
  const rl = acceptRevocations(g.revocations, ks, undefined, now);
  for (const c of g.pack_cases) {
    const got = outcome(() => summary(openPack(b64decode(c.pack)!, m, 0, ks, c.revocations ? rl : { entries: [] }, now)));
    assert.equal(canonical(got), canonical(c.expect), c.name);
  }
});
