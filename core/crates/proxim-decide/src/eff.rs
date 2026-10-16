//! Effective policy: partner values clamped to platform ceilings (engine-spec 3.3, "partners only
//! narrow"). Pure.

use crate::params::{SLOTS_CEILING, STANDOFF_MAX_M};
use alloc::collections::BTreeSet;
use alloc::string::String;
use proxim_types::defaults::{
    APP_MIN_INTERVAL_S, CAP_SIGNPOST_DAY, CAP_SIGNPOST_WEEK, LEAD_S_MAX, LEAD_S_MIN, MAX_ACC_MM,
    MAX_STANDOFF_ZONES, SENSITIVITY_CEILING, STANDOFF_M,
};
use proxim_types::{EngineError, Policy};

/// A policy after clamping. Borrowed sets avoid copying the block lists.
pub(crate) struct Eff<'a> {
    pub explain: bool,
    pub simulator_mode: bool,
    pub child_directed: bool,
    pub sensitivity_ceiling: u8,
    pub lead_lo_s: u32,
    pub lead_hi_s: u32,
    pub cap_day: u32,
    pub cap_week: u32,
    pub app_interval_ms: i64,
    pub slots: usize,
    pub max_acc_mm: u32,
    pub categories_blocked: BTreeSet<&'a str>,
    pub senders_blocked: BTreeSet<&'a str>,
}

impl<'a> Eff<'a> {
    /// # Errors
    /// [`EngineError::Limit`] if the zone list exceeds 256 entries.
    pub fn new(p: &'a Policy) -> Result<Self, EngineError> {
        if p.standoff_zones.len() > MAX_STANDOFF_ZONES {
            return Err(EngineError::Limit);
        }
        // Narrowing only: a larger cap, longer-than-ceiling lead or a wider accuracy is clamped;
        // a longer app interval is kept (it narrows).
        let lead_lo = p
            .lead_s_min
            .unwrap_or(LEAD_S_MIN)
            .clamp(LEAD_S_MIN, LEAD_S_MAX);
        let lead_hi = p
            .lead_s_max
            .unwrap_or(LEAD_S_MAX)
            .clamp(lead_lo, LEAD_S_MAX);
        Ok(Self {
            explain: p.explain.unwrap_or(false),
            simulator_mode: p.simulator_mode.unwrap_or(false),
            child_directed: p.child_directed.unwrap_or(false),
            sensitivity_ceiling: p
                .sensitivity_ceiling
                .unwrap_or(SENSITIVITY_CEILING)
                .min(SENSITIVITY_CEILING),
            lead_lo_s: lead_lo,
            lead_hi_s: lead_hi,
            cap_day: p
                .cap_signpost_day
                .unwrap_or(CAP_SIGNPOST_DAY)
                .min(CAP_SIGNPOST_DAY),
            cap_week: p
                .cap_signpost_week
                .unwrap_or(CAP_SIGNPOST_WEEK)
                .min(CAP_SIGNPOST_WEEK),
            app_interval_ms: i64::from(
                p.app_min_interval_s
                    .unwrap_or(APP_MIN_INTERVAL_S)
                    .max(APP_MIN_INTERVAL_S),
            )
            .saturating_mul(1000),
            slots: usize::from(p.slots.unwrap_or(SLOTS_CEILING).min(SLOTS_CEILING)),
            max_acc_mm: p.max_acc_mm.unwrap_or(MAX_ACC_MM).min(MAX_ACC_MM),
            categories_blocked: p.categories_blocked.iter().map(String::as_str).collect(),
            senders_blocked: p.senders_blocked.iter().map(String::as_str).collect(),
        })
    }
}

/// Effective standoff of a zone in metres: never below the platform default, never above the
/// ceiling (errata: a zone may widen its standoff, not shrink it).
#[must_use]
pub(crate) fn standoff_m(given: Option<u32>) -> u32 {
    given
        .unwrap_or(STANDOFF_M)
        .clamp(STANDOFF_M, STANDOFF_MAX_M)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_platform_values() {
        let p = Policy::default();
        let e = Eff::new(&p).unwrap();
        assert_eq!(
            (
                e.cap_day,
                e.cap_week,
                e.slots,
                e.max_acc_mm,
                e.sensitivity_ceiling
            ),
            (2, 5, 1, 100_000, 2)
        );
        assert_eq!(
            (e.lead_lo_s, e.lead_hi_s, e.app_interval_ms),
            (15, 120, 1_800_000)
        );
        assert!(!e.explain && !e.simulator_mode && !e.child_directed);
    }

    #[test]
    fn partners_can_only_narrow() {
        let p = Policy {
            cap_signpost_day: Some(50),
            cap_signpost_week: Some(50),
            slots: Some(9),
            max_acc_mm: Some(u32::MAX),
            sensitivity_ceiling: Some(9),
            app_min_interval_s: Some(10),
            lead_s_min: Some(1),
            lead_s_max: Some(10_000),
            ..Policy::default()
        };
        let e = Eff::new(&p).unwrap();
        assert_eq!(
            (
                e.cap_day,
                e.cap_week,
                e.slots,
                e.max_acc_mm,
                e.sensitivity_ceiling
            ),
            (2, 5, 1, 100_000, 2)
        );
        assert_eq!(
            (e.lead_lo_s, e.lead_hi_s, e.app_interval_ms),
            (15, 120, 1_800_000)
        );
        let p = Policy {
            cap_signpost_day: Some(1),
            cap_signpost_week: Some(0),
            slots: Some(0),
            max_acc_mm: Some(30_000),
            sensitivity_ceiling: Some(0),
            app_min_interval_s: Some(7200),
            lead_s_min: Some(30),
            lead_s_max: Some(20),
            ..Policy::default()
        };
        let e = Eff::new(&p).unwrap();
        assert_eq!(
            (
                e.cap_day,
                e.cap_week,
                e.slots,
                e.max_acc_mm,
                e.sensitivity_ceiling
            ),
            (1, 0, 0, 30_000, 0)
        );
        assert_eq!(e.app_interval_ms, 7_200_000);
        // An inverted lead range collapses to the minimum.
        assert_eq!((e.lead_lo_s, e.lead_hi_s), (30, 30));
    }

    #[test]
    fn standoff_never_below_default() {
        assert_eq!(standoff_m(None), 1000);
        assert_eq!(standoff_m(Some(0)), 1000);
        assert_eq!(standoff_m(Some(999)), 1000);
        assert_eq!(standoff_m(Some(5000)), 5000);
        assert_eq!(standoff_m(Some(u32::MAX)), 100_000);
    }
}
