// OI-ROT-1: the 19 envelope v2 entries of the frozen golden_v2.json (partial ship masks, swaps,
// the 4-byte vid second preimage) get the reference verdict from the TypeScript entry parser.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { sha256 } from "../src/format.ts";
import { PackError, publicKey, verifyEnvelope } from "../src/verify.ts";
import { type Any, coreDir } from "./support.ts";

const g: Any = JSON.parse(readFileSync(join(coreDir, "testdata/golden/rotation/golden_v2.json"), "utf8"));

test("golden_v2 envelope entries", () => {
  const key = publicKey(Buffer.from(g.envelope.pubkey_uncompressed_hex, "hex"))!;
  let partial = 0;
  for (const e of g.envelope.entries) {
    const wire = Buffer.from(e.wire_hex, "hex");
    let got: Any;
    try {
      if (wire.length < 64) throw new PackError("SIGNATURE_INVALID");
      const r = verifyEnvelope(key, wire.subarray(0, wire.length - 64), wire.subarray(wire.length - 64));
      got = { expect: "accept", shipped_idx: r.bodies.map(([i]) => i), body_sha256_hex: r.bodies.map(([, b]) => sha256(b).toString("hex")) };
    } catch (err) {
      if (!(err instanceof PackError)) throw err;
      got = { expect: "reject" };
    }
    const want = e.expect === "accept" ? { expect: "accept", shipped_idx: e.shipped_idx, body_sha256_hex: e.body_sha256_hex } : { expect: "reject" };
    assert.deepEqual(got, want, e.name);
    if (e.expect === "accept" && e.shipped_idx.length < 6 && e.name.startsWith("valid_brew")) partial++;
  }
  assert.equal(g.envelope.entries.length, 19);
  assert.ok(partial >= 2, "partial ship masks are exercised");
});
