//! Random geometry cases with Proxim's answers, for the turf.js comparison
//! (`tools/turf-compare/`). Each line of output is one JSON object:
//!
//! - `{"k":"dist","o":[lat,lon],"a":[..],"b":[..],"d_mm":..}`: `dist_mm` of `a` and `b`
//!   projected into the frame of the origin `o` (how the engine measures inside a signpost).
//! - `{"k":"pip","o":..,"poly":[[lat,lon],..],"p":..,"pip":"inside|outside|boundary","depth_mm":..}`
//! - `{"k":"line","o":..,"line":[..],"p":..,"dist_mm":..,"along_mm":..,"gap_mm":..}` where
//!   `gap_mm` is the distance of the second-nearest segment minus the nearest (an along
//!   comparison is only meaningful when the nearest segment is unambiguous).
//!
//! Coordinates are e7 integers. Origins span latitudes -70..70 degrees; geometry lies within
//! `r_max_m` of the origin (`r` field), drawn from 1, 5 and 20 km.

use crate::geom::{SimError, SimResult};
use crate::rng::Rng;
use alloc::vec::Vec;
use proxim_geo::{
    Frame, PipResult, Polyline, dist_mm, dist_point_segment_mm, frame_for, point_in_polygon,
    project_to_polyline, signed_depth_polygon_mm, unit_from_course,
};
use proxim_types::{Enu, Position};
use serde_json::{Value, json};
use std::io::Write;

const RADII_M: [i64; 3] = [1000, 5000, 20_000];

fn pj(p: Position) -> Value {
    json!([p.lat_e7, p.lon_e7])
}

fn random_origin(rng: &mut Rng) -> SimResult<(Position, Frame)> {
    let lat = i32::try_from(rng.range_i64(-700_000_000, 700_000_000)).unwrap_or(0);
    let lon = i32::try_from(rng.range_i64(-1_790_000_000, 1_790_000_000)).unwrap_or(0);
    let o = Position {
        lat_e7: lat,
        lon_e7: lon,
    };
    Ok((o, frame_for(o)?))
}

fn polar(rng: &mut Rng, r_mm: i64) -> SimResult<(i64, i64)> {
    let c = u16::try_from(rng.below(3600)).unwrap_or(0);
    let d = rng.range_i64(0, r_mm);
    offset((0, 0), c, d)
}

fn offset(p: (i64, i64), course: u16, d: i64) -> SimResult<(i64, i64)> {
    let [ue, un] = unit_from_course(course)?;
    Ok((
        p.0 + i64::try_from((i128::from(d) * i128::from(ue)) >> 30).unwrap_or(0),
        p.1 + i64::try_from((i128::from(d) * i128::from(un)) >> 30).unwrap_or(0),
    ))
}

fn to_pos(f: &Frame, p: (i64, i64)) -> SimResult<Position> {
    let e = Enu {
        e_mm: i32::try_from(p.0).map_err(|_| SimError::new("offset"))?,
        n_mm: i32::try_from(p.1).map_err(|_| SimError::new("offset"))?,
    };
    Ok(f.unproject(e)?)
}

/// One case line, or `None` when the random geometry was degenerate (skipped, not counted).
fn case(rng: &mut Rng, kind: u64) -> SimResult<Option<Value>> {
    let (o, f) = random_origin(rng)?;
    let r_m = *RADII_M
        .get(usize::try_from(rng.below(3)).unwrap_or(0))
        .unwrap_or(&1000);
    let r_mm = r_m * 1000;
    match kind {
        0 => {
            let a = to_pos(&f, polar(rng, r_mm)?)?;
            let b = to_pos(&f, polar(rng, r_mm)?)?;
            let d = dist_mm(f.project(a)?, f.project(b)?)?;
            Ok(Some(
                json!({"k":"dist","o":pj(o),"r":r_m,"a":pj(a),"b":pj(b),"d_mm":d}),
            ))
        }
        1 => {
            let n = rng.range_i64(3, 12);
            let mut courses: Vec<u16> = (0..n)
                .map(|_| u16::try_from(rng.below(3600)).unwrap_or(0))
                .collect();
            courses.sort_unstable();
            courses.dedup();
            if courses.len() < 3 {
                return Ok(None);
            }
            let mut verts = Vec::new();
            for c in &courses {
                let d = rng.range_i64(200_000, r_mm);
                verts.push(to_pos(&f, offset((0, 0), *c, d)?)?);
            }
            let poly: Vec<Enu> = verts
                .iter()
                .map(|v| f.project(*v))
                .collect::<Result<_, _>>()?;
            // half the points near an edge (within 5 m), half anywhere
            let p_local = if rng.below(2) == 0 {
                let i =
                    usize::try_from(rng.below(u64::try_from(poly.len()).unwrap_or(1))).unwrap_or(0);
                let a = poly.get(i).copied().unwrap_or(Enu { e_mm: 0, n_mm: 0 });
                let b = poly
                    .get((i + 1) % poly.len())
                    .copied()
                    .unwrap_or(Enu { e_mm: 0, n_mm: 0 });
                let t = rng.range_i64(0, 1000);
                let base = (
                    i64::from(a.e_mm) + (i64::from(b.e_mm) - i64::from(a.e_mm)) * t / 1000,
                    i64::from(a.n_mm) + (i64::from(b.n_mm) - i64::from(a.n_mm)) * t / 1000,
                );
                offset(
                    base,
                    u16::try_from(rng.below(3600)).unwrap_or(0),
                    rng.range_i64(0, 5000),
                )?
            } else {
                polar(rng, r_mm * 12 / 10)?
            };
            let p = to_pos(&f, p_local)?;
            let q = f.project(p)?;
            let Ok(pip) = point_in_polygon(&poly, q) else {
                return Ok(None);
            };
            let depth = signed_depth_polygon_mm(&poly, q)?;
            let s = match pip {
                PipResult::Inside => "inside",
                PipResult::Outside => "outside",
                PipResult::Boundary => "boundary",
            };
            let vj: Vec<Value> = verts.iter().map(|v| pj(*v)).collect();
            Ok(Some(
                json!({"k":"pip","o":pj(o),"r":r_m,"poly":vj,"p":pj(p),"pip":s,"depth_mm":depth}),
            ))
        }
        _ => {
            let n = rng.range_i64(2, 8);
            let mut cur = polar(rng, r_mm / 2)?;
            let mut pts = Vec::new();
            for _ in 0..n {
                pts.push(to_pos(&f, cur)?);
                let c = u16::try_from(rng.below(3600)).unwrap_or(0);
                cur = offset(cur, c, rng.range_i64(200_000, (r_mm / 3).max(200_001)))?;
                if cur.0.abs() > r_mm || cur.1.abs() > r_mm {
                    cur = polar(rng, r_mm / 2)?;
                }
            }
            let enu: Vec<Enu> = pts
                .iter()
                .map(|v| f.project(*v))
                .collect::<Result<_, _>>()?;
            let Ok(line) = Polyline::new(&enu) else {
                return Ok(None);
            };
            let p = to_pos(&f, polar(rng, r_mm)?)?;
            let q = f.project(p)?;
            let pr = project_to_polyline(&line, q, None)?;
            let mut second = i64::MAX;
            for (i, w) in enu.windows(2).enumerate() {
                if u32::try_from(i).ok() == Some(pr.seg) {
                    continue;
                }
                if let (Some(a), Some(b)) = (w.first(), w.get(1)) {
                    second = second.min(dist_point_segment_mm(q, *a, *b)?);
                }
            }
            let gap = if second == i64::MAX {
                i64::MAX / 2
            } else {
                second - pr.dist_mm
            };
            let lj: Vec<Value> = pts.iter().map(|v| pj(*v)).collect();
            Ok(Some(
                json!({"k":"line","o":pj(o),"r":r_m,"line":lj,"p":pj(p),
                "dist_mm":pr.dist_mm,"along_mm":pr.along_mm,"gap_mm":gap,
                "len_mm":line.length_mm()}),
            ))
        }
    }
}

/// Write `count` cases (a third of each kind, round robin) as JSON lines.
///
/// # Errors
/// Write failure.
pub fn write_cases(count: u64, seed: u64, out: &mut dyn Write) -> SimResult<u64> {
    let mut rng = Rng::new(seed, 0x7475_7266);
    let mut written = 0_u64;
    let mut k = 0_u64;
    while written < count {
        if let Some(v) = case(&mut rng, k % 3)? {
            let line = proxim_types::canon::value_to_canonical(&v)?;
            out.write_all(line.as_bytes())
                .and_then(|()| out.write_all(b"\n"))
                .map_err(|e| SimError::new(alloc::format!("write: {e}")))?;
            written += 1;
            k += 1;
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cases_are_deterministic_and_well_formed() {
        let mut a = Vec::new();
        let mut b = Vec::new();
        write_cases(60, 3, &mut a).unwrap();
        write_cases(60, 3, &mut b).unwrap();
        assert_eq!(a, b);
        let text = String::from_utf8(a).unwrap();
        let kinds: Vec<String> = text
            .lines()
            .map(|l| {
                let v: Value = serde_json::from_str(l).unwrap();
                v["k"].as_str().unwrap().to_string()
            })
            .collect();
        assert_eq!(kinds.len(), 60);
        for k in ["dist", "pip", "line"] {
            assert_eq!(kinds.iter().filter(|x| *x == k).count(), 20);
        }
    }
}
