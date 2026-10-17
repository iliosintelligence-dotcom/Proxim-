import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { b64decode } from "../src/b64.ts";
import { parseStrict } from "../src/canon.ts";
import { publicKey, type Root } from "../src/verify.ts";

export const coreDir = process.env.PROXIM_CORE_DIR ?? join(dirname(fileURLToPath(import.meta.url)), "../../../core");
export const goldenDir = join(coreDir, "testdata/golden/pack");

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Any = any;

export function golden(): Any {
  return parseStrict(readFileSync(join(goldenDir, "pack_v1.json"), "utf8"));
}

export function roots(g: Any): Root[] {
  return g.roots.map((r: Any) => ({ kid: r.kid, key: publicKey(b64decode(r.pub)!)! }));
}
