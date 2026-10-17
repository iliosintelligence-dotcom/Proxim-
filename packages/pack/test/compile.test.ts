// The compiler is deterministic, its output opens in the TypeScript verifier with the same
// verdicts as the Rust golden, and the checked-in compiled pack (which Rust opens in
// core/crates/proxim-pack/tests/golden.rs) is current. PROXIM_BLESS=1 rewrites it.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { b64encode } from "../src/b64.ts";
import { canonical, parseStrict } from "../src/canon.ts";
import { CompileError, compile } from "../src/compile.ts";
import { parseCell } from "../src/format.ts";
import { acceptKeyset, acceptRevocations, openPack, summary, type Manifest } from "../src/verify.ts";
import { type Any, golden, goldenDir, roots } from "./support.ts";

const g = golden();
const now = BigInt(g.now_ms);
const compiledPath = join(goldenDir, "compiled_v1.json");

test("same snapshot, byte-identical pack", () => {
  const a = compile(g.snapshot);
  const b = compile(structuredClone(g.snapshot));
  assert.equal(a.sha256, b.sha256);
  assert.ok(a.pack.equals(b.pack));
  // Input order does not matter: the compiler sorts by rank, then id.
  const shuffled = { ...g.snapshot, revisions: [...g.snapshot.revisions].reverse() };
  assert.equal(compile(shuffled).sha256, a.sha256);
  assert.equal(a.ids[0], "rank-1", "highest rank first");
});

test("compiled pack opens with the golden verdicts", () => {
  const c = compile(g.snapshot);
  const ks = acceptKeyset(g.keyset, roots(g), undefined, now);
  const rl = acceptRevocations(g.revocations, ks, undefined, now);
  const m: Manifest = { cell: parseCell(g.cell)!, guard: { seq: 9n, sha256: "" }, packKind: 1, tiers: [{ tier: 0, bytes: c.bytes, sha256: c.sha256 }] };
  const got = summary(openPack(c.pack, m, 0, ks, rl, now)) as Any;
  const want = g.pack_cases.find((x: Any) => x.name === "opens").expect;
  // Same verdict per signpost; indices differ because the compiler orders by rank.
  const byId = (s: Any) => {
    const ids = g.snapshot.revisions.map((r: Any) => parseStrict(r.core) as Any).map((x: Any) => x.id);
    const pick = (i: number, order: string[]) => order[i];
    return {
      entries: s.entries.map((e: Any) => ({ ...e, index: undefined })).sort((a: Any, b: Any) => (a.id < b.id ? -1 : 1)),
      dropped: s.dropped.map((d: Any) => ({ id: d.id ?? pick(d.index, s === want ? ids : c.ids), reason: d.reason })).sort((a: Any, b: Any) => (a.id < b.id ? -1 : 1)),
    };
  };
  assert.deepEqual(byId(got), byId(want));
});

test("checked-in compiled pack is current", () => {
  const c = compile(g.snapshot);
  const text = canonical({ header: { format: "proxim-golden/1", kind: "pack", id: "compiled_v1", engine_spec: 1, synthetic: true }, compiler: "packages/pack", snapshot: "pack_v1.json#snapshot", sha256: c.sha256, bytes: c.bytes, ids: c.ids, pack: b64encode(c.pack) }) + "\n";
  if (process.env.PROXIM_BLESS === "1") writeFileSync(compiledPath, text);
  assert.equal(readFileSync(compiledPath, "utf8"), text, "stale: PROXIM_BLESS=1 npm test in packages/pack");
});

test("compiler refusals", () => {
  const rev = g.snapshot.revisions[0];
  const bad = (revisions: Any[], extra: Any = {}) => assert.throws(() => compile({ ...g.snapshot, ...extra, revisions }), CompileError);
  bad([rev, rev]);
  bad([{ ...rev, bodies: ["a", "b", "c", "d", "e", "f", "g"] }]);
  bad([{ ...rev, bodies: ["same", "same"] }]);
  bad([{ ...rev, sig: "AAAA" }]);
  bad([{ ...rev, core: rev.core.replace('"pack_kind":"hosted_tile",', "") }]);
  bad([{ ...rev, core: " " + rev.core }]);
  bad([rev], { cell: "XYZ" });
});
