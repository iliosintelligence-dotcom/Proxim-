// Tier-0 size budget (PLAN M2 "tier-0 size budget measured and set"; Addendum 01 N10; Addendum 02
// 4.4). A synthetic dense-metro snapshot: 500 families, realistic cores (60% circles, 40% route
// corridors with 12 vertices), pseudo-random wordings of 30 to 70 characters, 64-byte signatures.
// Measured numbers are printed and recorded in docs/pack-format.md section 8.
import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { b64encode } from "../src/b64.ts";
import { canonical } from "../src/canon.ts";
import { CompileError, compile, type Snapshot } from "../src/compile.ts";
import { LIMITS } from "../src/format.ts";

/** Deterministic byte stream from a label (so the measurement is reproducible). */
function prng(label: string) {
  let counter = 0;
  let buf = Buffer.alloc(0);
  return (n: number) => {
    while (buf.length < n) buf = Buffer.concat([buf, createHash("sha256").update(`${label}/${counter++}`).digest()]);
    const out = buf.subarray(0, n);
    buf = buf.subarray(n);
    return out;
  };
}

const WORDS = ("coffee fresh pastries open now lunch deal soup bread cheese fuel charge parking cinema tonight " +
  "sale books garden market bakery pizza noodles tea pharmacy repairs bikes gym yoga museum tickets kids menu " +
  "hours today weekend local family brunch vegan grill tacos sushi salad juice flowers hardware laundry").split(" ");
const CATS = ["food.cafe", "food.restaurant", "auto.fuel", "retail.grocery", "retail.books", "leisure.cinema", "retail.clothing"];

export function synthetic(families: number, wordings: number, label = "metro", incompressible = false): Snapshot {
  const rnd = prng(label);
  const n = (max: number) => rnd(4).readUInt32BE() % max;
  const words = (min: number, max: number) => {
    // Worst case: random letters, which DEFLATE cannot shrink below about 6 bits a character.
    if (incompressible) return Array.from(rnd(min + n(max - min)), (b) => "abcdefghijklmnopqrstuvwxyz     "[b % 31]).join("").trim() || "x";
    let s = "";
    while (s.length < min) s += (s ? " " : "") + WORDS[n(WORDS.length)] + (n(5) === 0 ? ` ${n(90) + 10}` : "");
    return s.slice(0, max);
  };
  const revisions = Array.from({ length: families }, (_, i) => {
    const lat = 450000000 + n(2_000_000);
    const lon = 70000000 + n(2_800_000);
    const route = n(10) < 4;
    const shape = route
      ? { kind: "route", buffer_m: 30, vertices: Array.from({ length: 12 }, (_, k) => ({ lat_e7: lat + k * 1800 + n(300), lon_e7: lon + n(400) })) }
      : { kind: "circle", center: { lat_e7: lat, lon_e7: lon }, radius_m: 100 + n(400) };
    const core = canonical({
      category: CATS[n(CATS.length)], class: "general", event: route ? "approach" : "enter",
      id: `sp-${i.toString().padStart(5, "0")}`, rev: 1 + n(4), sender: `sender-${n(200)}`, sensitivity: 0,
      pack_kind: "hosted_tile", rank_milli: 1000 + n(2001),
      trigger: { origin: { lat_e7: lat, lon_e7: lon }, shape },
      valid_from_ms: 1_791_000_000_000, valid_until_ms: 1_793_000_000_000,
    });
    const bodies = Array.from({ length: wordings }, () => words(30, 70));
    return { core, bodies: [...new Set(bodies)], kid: "approve-2026-1", sig: b64encode(rnd(64)) };
  });
  return { cell: "851f9a83fffffff", seq: 1, tier: 0, pack_kind: "hosted_tile", revisions };
}

test("tier-0 size at 500 families, N = 1 / 4 / 6", () => {
  const rows: string[] = [];
  for (const [wordings, worst] of [[1, false], [4, false], [6, false], [1, true], [4, true], [6, true]] as const) {
    const s = synthetic(500, wordings, "metro", worst);
    // Over budget is a build failure, so measure with the budget lifted via tier 1.
    const c = compile({ ...s, tier: 1 });
    const perFamily = Math.round(c.bytes / 500);
    rows.push(`${worst ? "random text " : "word text   "} N=${wordings}: ${c.bytes} B (${perFamily} B per family; signature table ${500 * 65} B)`);
    if (wordings === 1 || !worst) assert.ok(c.bytes <= LIMITS.tier0Budget, `must fit: ${c.bytes}`);
  }
  console.log("tier-0 size, 500 synthetic families:\n  " + rows.join("\n  "));
});

test("an over-budget tier-0 pack is a build failure, not a silent cut", () => {
  const s = synthetic(3000, 6, "huge");
  assert.throws(() => compile(s), (e: unknown) => e instanceof CompileError && /budget/.test((e as Error).message));
});
