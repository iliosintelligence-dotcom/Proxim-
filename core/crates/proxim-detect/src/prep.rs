//! Trigger preparation: validate a signpost's trigger once and project its geometry into the
//! trigger's own frame (engine-spec 1.2, 3.3). A defect is a [`ReasonCode`], never an `Err`.

use crate::params::{
    BEARING_TOL_MAX_DDEG, BEARING_TOL_MIN_DDEG, BUFFER_MAX_M, BUFFER_MIN_M, D_LO_SLOW_MM,
    D_LO_VEHICLE_MM, REACH_MARGIN_MM,
};
use alloc::vec::Vec;
use proxim_geo::{Frame, Polyline, cos_tol_q30, dist_mm, frame_for};
use proxim_path::{min_speed_mmps, params::MAX_FOOT_MMPS};
use proxim_types::defaults::{
    BEARING_TOL_DDEG, D_HI_M, D_HI_MAX_M, LEAD_S_DEFAULT, LEAD_S_MAX, LEAD_S_MIN,
    MAX_POLYGON_VERTICES, MAX_ROUTE_VERTICES, MIN_CIRCLE_RADIUS_M,
};
use proxim_types::{EngineError, Enu, EventKind, Mode, ReasonCode, Shape, Signpost};

/// Parameters of an APPROACH trigger after validation.
#[derive(Clone, Debug)]
pub(crate) struct RoutePrep {
    pub route: Polyline,
    pub buffer_mm: i64,
    pub anchor_mm: i64,
    pub anchor_pt: Enu,
    pub lead_s: u32,
    pub d_lo_base_mm: i64,
    pub d_hi_mm: i64,
    pub mode: Mode,
    pub band_min_mmps: u32,
    pub band_max_mmps: u32,
    pub cos_tol_q30: i64,
}

#[derive(Clone, Debug)]
pub(crate) enum Kind {
    Circle {
        center: Enu,
        radius_mm: i64,
    },
    Polygon {
        vertices: Vec<Enu>,
        inradius_mm: Option<i64>,
    },
    Route(RoutePrep),
}

/// A validated trigger: frame, projected geometry, derived parameters. Build once per signpost
/// revision with [`Prepared::new`] and reuse it for every fix; `detect` builds one per call.
#[derive(Clone, Debug)]
pub struct Prepared {
    pub(crate) frame: Frame,
    pub(crate) kind: Kind,
    pub(crate) reach_mm: i64,
    pub(crate) min_tier: u8,
    pub(crate) min_conf_milli: u16,
}

fn map_err(_: EngineError) -> ReasonCode {
    ReasonCode::TriggerInvalid
}

/// Point at `along_mm` on the polyline (round to nearest mm, ties up).
fn point_at(pl: &Polyline, along_mm: i64) -> Result<Enu, EngineError> {
    let last = pl
        .vertices
        .len()
        .checked_sub(1)
        .ok_or(EngineError::DegenerateGeometry)?;
    for i in 0..last {
        let c0 = *pl.cum_mm.get(i).ok_or(EngineError::DegenerateGeometry)?;
        let c1 = *pl
            .cum_mm
            .get(i + 1)
            .ok_or(EngineError::DegenerateGeometry)?;
        if along_mm > c1 {
            continue;
        }
        let a = *pl.vertices.get(i).ok_or(EngineError::DegenerateGeometry)?;
        let b = *pl
            .vertices
            .get(i + 1)
            .ok_or(EngineError::DegenerateGeometry)?;
        let len = i128::from(c1 - c0);
        let t = i128::from(along_mm.saturating_sub(c0).max(0));
        let lerp = |p: i32, q: i32| -> Result<i32, EngineError> {
            let d = (i128::from(q) - i128::from(p)) * t;
            let r = proxim_geo::prim::div_round_i128(d, len)?;
            i32::try_from(i128::from(p) + r).map_err(|_| EngineError::Overflow)
        };
        return Ok(Enu {
            e_mm: lerp(a.e_mm, b.e_mm)?,
            n_mm: lerp(a.n_mm, b.n_mm)?,
        });
    }
    pl.vertices
        .last()
        .copied()
        .ok_or(EngineError::DegenerateGeometry)
}

impl Prepared {
    /// Validate and project a signpost's trigger.
    ///
    /// # Errors
    /// [`ReasonCode::UnsupportedEvent`] for `exit cross dwell pass sequence` (reserved or
    /// internal, spec 10); [`ReasonCode::TriggerInvalid`] for a shape/event mismatch, a limit,
    /// an out-of-range parameter, geometry outside the 150 km frame or degenerate geometry.
    pub fn new(sp: &Signpost) -> Result<Self, ReasonCode> {
        match sp.event {
            EventKind::Enter | EventKind::Approach => {}
            _ => return Err(ReasonCode::UnsupportedEvent),
        }
        let trig = &sp.trigger;
        let frame = frame_for(trig.origin).map_err(map_err)?;
        let kind;
        let extent_mm;
        let margin_mm;
        match (&sp.event, &trig.shape) {
            (EventKind::Enter, Shape::Circle { center, radius_m }) => {
                if *radius_m < MIN_CIRCLE_RADIUS_M {
                    return Err(ReasonCode::TriggerInvalid);
                }
                let c = frame.project(*center).map_err(map_err)?;
                let radius_mm = i64::from(*radius_m) * 1000;
                extent_mm = dist_mm(Enu { e_mm: 0, n_mm: 0 }, c).map_err(map_err)? + radius_mm;
                margin_mm = REACH_MARGIN_MM;
                kind = Kind::Circle {
                    center: c,
                    radius_mm,
                };
            }
            (
                EventKind::Enter,
                Shape::Polygon {
                    vertices,
                    inradius_m,
                },
            ) => {
                if vertices.len() < 3 || vertices.len() > MAX_POLYGON_VERTICES {
                    return Err(ReasonCode::TriggerInvalid);
                }
                let v = frame.project_all(vertices).map_err(map_err)?;
                extent_mm = max_dist(&v)?;
                margin_mm = REACH_MARGIN_MM;
                kind = Kind::Polygon {
                    vertices: v,
                    inradius_mm: inradius_m.map(|m| i64::from(m) * 1000),
                };
            }
            (
                EventKind::Approach,
                Shape::Route {
                    vertices,
                    buffer_m,
                    anchor_along_m,
                },
            ) => {
                if vertices.len() < 2 || vertices.len() > MAX_ROUTE_VERTICES {
                    return Err(ReasonCode::TriggerInvalid);
                }
                if !(BUFFER_MIN_M..=BUFFER_MAX_M).contains(buffer_m) {
                    return Err(ReasonCode::TriggerInvalid);
                }
                let v = frame.project_all(vertices).map_err(map_err)?;
                let route = Polyline::new(&v).map_err(map_err)?;
                extent_mm = max_dist(&v)?;
                let anchor_mm = match anchor_along_m {
                    Some(m) => i64::from(*m) * 1000,
                    None => route.length_mm(),
                };
                if anchor_mm > route.length_mm() {
                    return Err(ReasonCode::TriggerInvalid);
                }
                let lead_s = trig.lead_s.unwrap_or(LEAD_S_DEFAULT);
                if !(LEAD_S_MIN..=LEAD_S_MAX).contains(&lead_s) {
                    return Err(ReasonCode::TriggerInvalid);
                }
                let d_hi_m = trig.d_hi_m.unwrap_or(D_HI_M);
                if d_hi_m == 0 || d_hi_m > D_HI_MAX_M {
                    return Err(ReasonCode::TriggerInvalid);
                }
                let mode = trig.mode.unwrap_or(Mode::Vehicle);
                let d_lo_base_mm = match (trig.d_lo_m, mode) {
                    (Some(m), _) => i64::from(m) * 1000,
                    (None, Mode::Vehicle) => D_LO_VEHICLE_MM,
                    (None, _) => D_LO_SLOW_MM,
                };
                let d_hi_mm = i64::from(d_hi_m) * 1000;
                if d_lo_base_mm > d_hi_mm {
                    return Err(ReasonCode::TriggerInvalid);
                }
                let tol = trig.bearing_tol_ddeg.unwrap_or(BEARING_TOL_DDEG);
                if !(BEARING_TOL_MIN_DDEG..=BEARING_TOL_MAX_DDEG).contains(&tol) {
                    return Err(ReasonCode::TriggerInvalid);
                }
                // Partners only narrow: the band never reaches below the mode minimum or above the
                // mode's default maximum (foot 2.5 m/s).
                let band_min = trig.speed_min_mmps.unwrap_or(0).max(min_speed_mmps(mode));
                let mode_max = if mode == Mode::Foot {
                    MAX_FOOT_MMPS
                } else {
                    u32::MAX
                };
                let band_max = trig.speed_max_mmps.unwrap_or(mode_max).min(mode_max);
                if band_min > band_max {
                    return Err(ReasonCode::TriggerInvalid);
                }
                let anchor_pt = point_at(&route, anchor_mm).map_err(map_err)?;
                margin_mm = d_hi_mm.max(i64::from(*buffer_m) * 1000) + REACH_MARGIN_MM;
                kind = Kind::Route(RoutePrep {
                    route,
                    buffer_mm: i64::from(*buffer_m) * 1000,
                    anchor_mm,
                    anchor_pt,
                    lead_s,
                    d_lo_base_mm,
                    d_hi_mm,
                    mode,
                    band_min_mmps: band_min,
                    band_max_mmps: band_max,
                    cos_tol_q30: i64::from(cos_tol_q30(tol).map_err(map_err)?),
                });
            }
            _ => return Err(ReasonCode::TriggerInvalid),
        }
        let reach_mm = match trig.reach_m {
            Some(m) => i64::from(m) * 1000,
            None => extent_mm.saturating_add(margin_mm),
        };
        Ok(Self {
            frame,
            kind,
            reach_mm,
            min_tier: sp.min_path_tier.unwrap_or(0),
            min_conf_milli: sp.min_path_conf_milli.unwrap_or(500),
        })
    }

    /// Distance a fix may be from the trigger origin and still be a candidate (spec 1.2), mm.
    #[must_use]
    pub const fn reach_mm(&self) -> i64 {
        self.reach_mm
    }

    /// `true` when the fix is beyond `reach_m` of the origin or outside the 150 km frame, so the
    /// trigger cannot be affected by it (a quick reject for callers).
    #[must_use]
    pub fn beyond_reach(&self, p: proxim_types::Position) -> bool {
        match self.frame.project(p) {
            Ok(q) => dist_mm(Enu { e_mm: 0, n_mm: 0 }, q).map_or(true, |d| d > self.reach_mm),
            Err(_) => true,
        }
    }
}

fn max_dist(v: &[Enu]) -> Result<i64, ReasonCode> {
    let mut m = 0_i64;
    for p in v {
        m = m.max(dist_mm(Enu { e_mm: 0, n_mm: 0 }, *p).map_err(map_err)?);
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enu(e: i32, n: i32) -> Enu {
        Enu { e_mm: e, n_mm: n }
    }

    #[test]
    fn point_at_interpolates_exactly_and_clamps_to_the_end() {
        // East 1000 mm, then north 2000 mm.
        let pl = Polyline::new(&[enu(0, 0), enu(1000, 0), enu(1000, 2000)]).unwrap();
        assert_eq!(point_at(&pl, 0).unwrap(), enu(0, 0));
        assert_eq!(point_at(&pl, 250).unwrap(), enu(250, 0));
        assert_eq!(
            point_at(&pl, 1000).unwrap(),
            enu(1000, 0),
            "a vertex belongs to its first segment"
        );
        assert_eq!(point_at(&pl, 1001).unwrap(), enu(1000, 1));
        assert_eq!(point_at(&pl, 3000).unwrap(), enu(1000, 2000), "the end");
        assert_eq!(
            point_at(&pl, 9999).unwrap(),
            enu(1000, 2000),
            "beyond the end clamps"
        );
        assert_eq!(
            point_at(&pl, -5).unwrap(),
            enu(0, 0),
            "negative clamps to the start"
        );
        // Rounding: a 3-4-5 segment at 1/5 of its length is (0.6, 0.8) -> (1, 1).
        let d = Polyline::new(&[enu(0, 0), enu(3, 4)]).unwrap();
        assert_eq!(point_at(&d, 1).unwrap(), enu(1, 1));
        assert_eq!(point_at(&d, 2).unwrap(), enu(1, 2)); // (1.2, 1.6) -> (1, 2)
    }

    #[test]
    fn max_dist_is_the_farthest_vertex() {
        assert_eq!(
            max_dist(&[enu(0, 0), enu(3000, 4000), enu(-1000, 0)]),
            Ok(5000)
        );
        assert_eq!(max_dist(&[]), Ok(0));
    }

    #[test]
    fn map_err_is_trigger_invalid() {
        assert_eq!(map_err(EngineError::Overflow), ReasonCode::TriggerInvalid);
    }
}
