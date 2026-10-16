//! Minimal sensitive-zone standoff (engine-spec 7, F10 and F19; invariant I6).
//!
//! Each zone has its own frame centred on the zone, so the zone centre is the plane origin and the
//! distance to the zone is the distance to `(0, 0)`.

use crate::eff::standoff_m;
use alloc::vec::Vec;
use proxim_geo::{Frame, PipResult, dist_mm, dist_point_segment_mm, frame_for, point_in_polygon};
use proxim_types::defaults::MAX_FRAME_MM;
use proxim_types::{EngineError, Enu, Fix, Policy, Position, Shape, Signpost};

const ORIGIN: Enu = Enu { e_mm: 0, n_mm: 0 };

pub(crate) struct Zone {
    frame: Frame,
    standoff_mm: i64,
}

/// Build the zone frames once per call.
///
/// # Errors
/// [`EngineError::Parse`] for an invalid centre coordinate, [`EngineError::UnsupportedLatitude`]
/// for a centre above 80 degrees (fail closed: a zone that cannot be evaluated is never ignored,
/// the whole call is "no decision").
pub(crate) fn build(policy: &Policy) -> Result<Vec<Zone>, EngineError> {
    policy
        .standoff_zones
        .iter()
        .map(|z| {
            if z.center.lat_e7.unsigned_abs() > 900_000_000
                || z.center.lon_e7.unsigned_abs() > 1_800_000_000
            {
                return Err(EngineError::Parse);
            }
            Ok(Zone {
                frame: frame_for(z.center)?,
                standoff_mm: i64::from(standoff_m(z.standoff_m)) * 1000,
            })
        })
        .collect()
}

/// F10: some zone has `dist(fix, centre) <= standoff + acc_eff`.
///
/// A fix outside a zone's 150 km frame is far from it unless the reach (`standoff + acc_eff`)
/// itself exceeds the frame, in which case the answer is "inside" (fail closed).
///
/// # Errors
/// Arithmetic errors only (unreachable for validated input).
pub(crate) fn fix_in_standoff(
    zones: &[Zone],
    fix: &Fix,
    acc_eff_mm: i64,
) -> Result<bool, EngineError> {
    for z in zones {
        let reach = z.standoff_mm.saturating_add(acc_eff_mm);
        match z.frame.project(fix.position()) {
            Ok(q) => {
                if dist_mm(ORIGIN, q)? <= reach {
                    return Ok(true);
                }
            }
            Err(EngineError::OutOfFrame) => {
                if reach > MAX_FRAME_MM {
                    return Ok(true);
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(false)
}

/// Distance in mm from the zone centre to the trigger geometry (0 when the centre is inside it),
/// `None` when no part of the geometry lies inside the zone's frame (so it is far).
///
/// Vertices beyond the frame are ignored; the engine guarantees triggers within 150 km of their
/// own origin, and a shape larger than that is not meaningful for a 100 km ceiling standoff
/// (errata).
fn distance_to_geometry(frame: &Frame, sp: &Signpost) -> Result<Option<i64>, EngineError> {
    let project_all = |vs: &[Position]| -> Vec<Option<Enu>> {
        vs.iter().map(|p| frame.project(*p).ok()).collect()
    };
    match &sp.trigger.shape {
        Shape::Circle { center, radius_m } => {
            let Ok(c) = frame.project(*center) else {
                // Centre beyond the frame: far, unless the radius reaches into the frame.
                let r = i64::from(*radius_m) * 1000;
                return Ok(if r > MAX_FRAME_MM { Some(0) } else { None });
            };
            let d = dist_mm(ORIGIN, c)?;
            Ok(Some((d - i64::from(*radius_m) * 1000).max(0)))
        }
        Shape::Polygon { vertices, .. } => {
            let pts = project_all(vertices);
            let flat: Vec<Enu> = pts.iter().flatten().copied().collect();
            if flat.len() == pts.len() && point_in_polygon(&flat, ORIGIN)? != PipResult::Outside {
                return Ok(Some(0));
            }
            min_edge_distance(&pts, true)
        }
        Shape::Route { vertices, .. } => min_edge_distance(&project_all(vertices), false),
    }
}

/// Minimum distance from the origin to the edges whose both ends are known.
fn min_edge_distance(pts: &[Option<Enu>], closed: bool) -> Result<Option<i64>, EngineError> {
    let mut best: Option<i64> = None;
    let n = pts.len();
    let edges = if closed { n } else { n.saturating_sub(1) };
    for i in 0..edges {
        let a = pts.get(i).copied().flatten();
        let b = pts.get((i + 1) % n.max(1)).copied().flatten();
        let d = match (a, b) {
            (Some(a), Some(b)) => dist_point_segment_mm(ORIGIN, a, b)?,
            (Some(p), None) | (None, Some(p)) => dist_mm(ORIGIN, p)?,
            (None, None) => continue,
        };
        best = Some(best.map_or(d, |x| x.min(d)));
    }
    Ok(best)
}

/// F19: a health signpost whose geometry lies within some zone's standoff. Other classes are not
/// tested here (F10 handles the user's own position for every class).
///
/// # Errors
/// Arithmetic errors only.
pub(crate) fn trigger_in_standoff(zones: &[Zone], sp: &Signpost) -> Result<bool, EngineError> {
    for z in zones {
        if let Some(d) = distance_to_geometry(&z.frame, sp)?
            && d <= z.standoff_mm
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proxim_types::{Id, StandoffZone};

    fn fix_at_lat(lat_e7: i32, acc_mm: u32) -> Fix {
        Fix {
            t_ms: 0,
            lat_e7,
            lon_e7: 70_000_000,
            acc_mm,
            speed_mmps: None,
            speed_acc_mmps: None,
            course_ddeg: None,
            course_acc_ddeg: None,
            is_mock: false,
        }
    }

    fn zone_at_origin(standoff_mm: i64) -> Zone {
        Zone {
            frame: frame_for(Position {
                lat_e7: 450_000_000,
                lon_e7: 70_000_000,
            })
            .unwrap(),
            standoff_mm,
        }
    }

    #[test]
    fn outside_the_frame_is_far_unless_the_reach_exceeds_the_frame() {
        // 3 degrees of latitude (333 km) from the zone: not projectable.
        let f = fix_at_lat(480_000_000, 5000);
        let z = zone_at_origin(100_000_000);
        assert_eq!(fix_in_standoff(&[z], &f, 10_000), Ok(false));
        // A reach beyond the 150 km frame cannot be decided by distance: fail closed.
        let z = zone_at_origin(149_000_000);
        assert_eq!(fix_in_standoff(&[z], &f, 2_000_000), Ok(true));
        let z = zone_at_origin(149_000_000);
        assert_eq!(
            fix_in_standoff(&[z], &f, 1_000_000),
            Ok(false),
            "a reach of exactly the frame size is decided by distance, which is far"
        );
    }

    #[test]
    fn build_clamps_and_validates() {
        let mk = |lat, lon, st| Policy {
            standoff_zones: alloc::vec![StandoffZone {
                id: Id::new("z").unwrap(),
                center: Position {
                    lat_e7: lat,
                    lon_e7: lon,
                },
                standoff_m: st,
            }],
            ..Policy::default()
        };
        assert_eq!(build(&mk(0, 0, Some(5))).unwrap()[0].standoff_mm, 1_000_000);
        assert_eq!(build(&mk(0, 0, None)).unwrap()[0].standoff_mm, 1_000_000);
        assert_eq!(
            build(&mk(0, 0, Some(u32::MAX))).unwrap()[0].standoff_mm,
            100_000_000
        );
        assert!(matches!(
            build(&mk(0, 1_900_000_000, None)),
            Err(EngineError::Parse)
        ));
        assert!(matches!(
            build(&mk(850_000_000, 0, None)),
            Err(EngineError::UnsupportedLatitude)
        ));
    }

    #[test]
    fn geometry_far_from_a_zone_is_none_and_a_circle_reaching_into_the_frame_is_not() {
        let z = zone_at_origin(1_000_000);
        let far = Position {
            lat_e7: 480_000_000,
            lon_e7: 70_000_000,
        };
        let mk = |shape| {
            let mut sp = crate::tests_support::general_signpost();
            sp.trigger.shape = shape;
            sp
        };
        let circle = |r| Shape::Circle {
            center: far,
            radius_m: r,
        };
        assert_eq!(distance_to_geometry(&z.frame, &mk(circle(200))), Ok(None));
        // A circle so large that its edge is inside the frame: treated as touching the zone.
        assert_eq!(
            distance_to_geometry(&z.frame, &mk(circle(200_000))),
            Ok(Some(0))
        );
        let edge = |a: Option<(i32, i32)>, b: Option<(i32, i32)>| {
            let to_enu = |p: (i32, i32)| Enu {
                e_mm: p.0,
                n_mm: p.1,
            };
            [a.map(to_enu), b.map(to_enu)]
        };
        assert_eq!(min_edge_distance(&edge(None, None), false), Ok(None));
        assert_eq!(
            min_edge_distance(&edge(Some((3000, 4000)), None), false),
            Ok(Some(5000)),
            "an edge with one far end is bounded by its near vertex"
        );
        assert_eq!(
            min_edge_distance(&edge(Some((0, 10)), Some((100, 10))), false),
            Ok(Some(10))
        );
        assert_eq!(min_edge_distance(&[], true), Ok(None));
    }
}
