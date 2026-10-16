#!/usr/bin/env node
// Compare Proxim geometry (integer ENU millimetres in a per-signpost frame, engine-spec 1-4)
// with turf.js and with an exact WGS84 local tangent-plane (ECEF -> ENU) reference.
//
// Input: JSON lines from `proxim-sim geo-cases` (stdin, or a file path argument).
//   cargo run --release -p proxim-sim --bin proxim-sim -- geo-cases --count 1000000 --seed 1 \
//     | node tools/turf-compare/compare.mjs
//
// Tolerances (documented in docs/simulator.md, "turf.js comparison"):
//   rounding   R_ABS = 25 mm: each e7 coordinate is within 5.6 mm of the point it stands for,
//              the frame projection rounds to 1 mm and isqrt floors (spec 1.3).
//   frame      Proxim projects with the scale of the frame origin (spec 1.2), so a point r away
//              from the origin carries a relative scale error of about tan|lat| x r / R (east-west
//              scale changes with latitude) plus (r / R) curvature, plus the 0.01 degree latitude
//              grid (tan|lat| x 8.7e-5, errata E-G10).
//              TOL_FRAME(d) = R_ABS + d x (1.25 tan|lat| r/R + 2 r/R + 1e-4 (1 + tan|lat|))
//   sphere     turf works on a sphere of radius 6,371,008.8 m; the WGS84 meridian and prime
//              vertical radii differ from it by at most 0.57% (M at the equator is 6,335 km).
//              TOL_TURF(d) = TOL_FRAME(d) + 0.0060 d
//   pip        the e7 -> ENU map is affine per frame, so turf's lon/lat point-in-polygon and
//              Proxim's must agree except within R_ABS of an edge (Proxim "boundary" matches
//              either answer).
//   segment    Proxim's segment is straight in the e7 plane; mapped to the tangent plane it bows
//              from the chord by a sagitta of about L^2 (1 + tan|lat|) / (8 R) for a segment of
//              length L (turf's geodesic bows differently again): SAG(L) is added to every
//              point-to-line distance and along tolerance, with L the longest segment.
//   along      compared only when the nearest segment is unambiguous (second-nearest segment at
//              least 2 TOL + 1 m farther); tolerance TOL(along + dist) + SAG + 0.0067 dist for
//              turf (the sphere's isotropic scale against the ellipsoid's M != N shears the
//              perpendicular foot by up to e^2 = 0.67% of the distance).
// Exit 1 if any case exceeds its tolerance; prints a JSON summary either way, with every check
// also broken down by the case radius (`name@r1000`, `@r5000`, `@r20000`).
//
// `--perturb` corrupts Proxim's answers (distances x 1.02, point-in-polygon answers flipped more
// than 1 m from an edge) to prove the comparison can fail: it must then exit 1.

import { createReadStream } from "node:fs";
import { createInterface } from "node:readline";
import * as turf from "@turf/turf";

const R_EARTH = 6371008.8;
const R_ABS = 0.025;
const A = 6378137.0;
const F = 1 / 298.257223563;
const E2 = F * (2 - F);
const D2R = Math.PI / 180;

const e7 = (v) => v / 1e7;
const ll = (p) => [e7(p[1]), e7(p[0])]; // [lon, lat]

function ecef(latDeg, lonDeg) {
  const phi = latDeg * D2R;
  const lam = lonDeg * D2R;
  const s = Math.sin(phi);
  const n = A / Math.sqrt(1 - E2 * s * s);
  return [n * Math.cos(phi) * Math.cos(lam), n * Math.cos(phi) * Math.sin(lam), n * (1 - E2) * s];
}

// Exact ENU (metres) of p in the tangent plane at o.
function enuOf(o, p) {
  const [ox, oy, oz] = ecef(e7(o[0]), e7(o[1]));
  const [px, py, pz] = ecef(e7(p[0]), e7(p[1]));
  const dx = px - ox, dy = py - oy, dz = pz - oz;
  const phi = e7(o[0]) * D2R, lam = e7(o[1]) * D2R;
  const sp = Math.sin(phi), cp = Math.cos(phi), sl = Math.sin(lam), cl = Math.cos(lam);
  return [-sl * dx + cl * dy, -sp * cl * dx - sp * sl * dy + cp * dz];
}

function tolFrame(d, latE7, rM) {
  const t = Math.abs(Math.tan(e7(latE7) * D2R));
  const q = rM / R_EARTH;
  return R_ABS + d * (1.25 * t * q + 2 * q + 1e-4 * (1 + t));
}
const tolTurf = (d, latE7, rM) => tolFrame(d, latE7, rM) + 0.006 * d;
function sagitta(verts, latE7) {
  let l = 0;
  for (let i = 0; i + 1 < verts.length; i++) {
    l = Math.max(l, Math.hypot(verts[i + 1][0] - verts[i][0], verts[i + 1][1] - verts[i][1]));
  }
  const t = Math.abs(Math.tan(e7(latE7) * D2R));
  return (l * l * (1 + t)) / (8 * R_EARTH);
}

function segFoot(p, a, b) {
  const vx = b[0] - a[0], vy = b[1] - a[1];
  const l2 = vx * vx + vy * vy;
  let t = l2 > 0 ? ((p[0] - a[0]) * vx + (p[1] - a[1]) * vy) / l2 : 0;
  t = Math.max(0, Math.min(1, t));
  const fx = a[0] + t * vx, fy = a[1] + t * vy;
  return { d: Math.hypot(p[0] - fx, p[1] - fy), t, len: Math.sqrt(l2) };
}

function projectPlanar(p, verts) {
  let best = null, cum = 0;
  for (let i = 0; i + 1 < verts.length; i++) {
    const f = segFoot(p, verts[i], verts[i + 1]);
    if (best === null || f.d < best.d) best = { d: f.d, along: cum + f.t * f.len };
    cum += f.len;
  }
  return best;
}

const stats = {};
function stat(name) {
  return (stats[name] ??= { n: 0, fail: 0, max_abs_err_m: 0, max_ratio: 0, worst: null });
}
function check(name, err, tol, c) {
  check1(name, err, tol, c);
  check1(`${name}@r${c.r}`, err, tol, c);
}
function check1(name, err, tol, c) {
  const s = stat(name);
  s.n++;
  const a = Math.abs(err);
  if (a > s.max_abs_err_m) s.max_abs_err_m = a;
  const ratio = a / tol;
  if (ratio > s.max_ratio) {
    s.max_ratio = ratio;
    s.worst = { err_m: err, tol_m: tol, case: c };
  }
  if (a > tol) s.fail++;
}

function perturb(c) {
  if (c.d_mm !== undefined) c.d_mm = Math.round(c.d_mm * 1.02);
  if (c.dist_mm !== undefined) c.dist_mm = Math.round(c.dist_mm * 1.02) + 2000;
  if (c.pip !== undefined && c.pip !== "boundary" && Math.abs(c.depth_mm) > 1000) {
    c.pip = c.pip === "inside" ? "outside" : "inside";
  }
  return c;
}

function handle(c) {
  const lat = c.o[0];
  const r = c.r;
  if (c.k === "dist") {
    const d = c.d_mm / 1000;
    const t = turf.distance(ll(c.a), ll(c.b), { units: "meters" });
    check("dist_vs_turf", d - t, tolTurf(t, lat, r), c);
    const ea = enuOf(c.o, c.a), eb = enuOf(c.o, c.b);
    const ref = Math.hypot(ea[0] - eb[0], ea[1] - eb[1]);
    check("dist_vs_wgs84_enu", d - ref, tolFrame(ref, lat, r), c);
  } else if (c.k === "pip") {
    const ring = c.poly.map(ll);
    ring.push(ring[0]);
    const inside = turf.booleanPointInPolygon(turf.point(ll(c.p)), turf.polygon([ring]));
    const s = stat("pip_vs_turf");
    s.n++;
    const agree =
      c.pip === "boundary" ||
      (c.pip === "inside") === inside ||
      Math.abs(c.depth_mm) / 1000 <= R_ABS;
    if (!agree) {
      s.fail++;
      s.worst ??= { case: c, turf_inside: inside };
    }
    if (c.pip !== "boundary" && (c.pip === "inside") !== inside) {
      s.disagree_within_tol = (s.disagree_within_tol ?? 0) + (agree ? 1 : 0);
    }
  } else if (c.k === "line") {
    const line = turf.lineString(c.line.map(ll));
    const pt = turf.point(ll(c.p));
    const d = c.dist_mm / 1000;
    const td = turf.pointToLineDistance(pt, line, { units: "meters" });
    const verts = c.line.map((v) => enuOf(c.o, v));
    const sag = sagitta(verts, lat);
    check("line_dist_vs_turf", d - td, tolTurf(td, lat, r) + sag, c);
    const ref = projectPlanar(enuOf(c.o, c.p), verts);
    check("line_dist_vs_wgs84_enu", d - ref.d, tolFrame(ref.d, lat, r) + sag, c);
    const along = c.along_mm / 1000;
    const gap = c.gap_mm / 1000;
    const tolA = tolTurf(along + d, lat, r) + sag + 0.0067 * d;
    if (gap > 2 * tolA + 1) {
      const np = turf.nearestPointOnLine(line, pt, { units: "meters" });
      check("line_along_vs_turf", along - np.properties.location, tolA, c);
      check("line_along_vs_wgs84_enu", along - ref.along, tolFrame(along + d, lat, r) + sag, c);
    } else {
      stat("line_along_skipped_ambiguous").n++;
    }
  } else {
    throw new Error(`unknown case kind ${c.k}`);
  }
}

const args = process.argv.slice(2);
const doPerturb = args.includes("--perturb");
const file = args.find((a) => !a.startsWith("--"));
const input = file ? createReadStream(file) : process.stdin;
const rl = createInterface({ input, crlfDelay: Infinity });
let cases = 0;
for await (const line of rl) {
  if (!line) continue;
  const c = JSON.parse(line);
  handle(doPerturb ? perturb(c) : c);
  cases++;
}
let failed = 0;
for (const s of Object.values(stats)) failed += s.fail;
const summary = { cases, failed, perturbed: doPerturb, checks: stats };
console.log(JSON.stringify(summary, null, 1));
if (cases === 0) {
  console.error("turf-compare: no cases read");
  process.exit(2);
}
process.exit(failed > 0 ? 1 : 0);
