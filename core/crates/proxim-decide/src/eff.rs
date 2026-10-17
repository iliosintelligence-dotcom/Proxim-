//! Effective policy: partner values clamped to platform ceilings (engine-spec 3.3, "partners only
//! narrow"). Pure.

use crate::params::{SLOTS_CEILING, STANDOFF_MAX_M};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use proxim_types::defaults::{
    AFFINITY_MAX_MILLI, AFFINITY_NEUTRAL_MILLI, APP_MIN_INTERVAL_S, CAP_APP_DAY, CAP_APP_DAY_MAX,
    CAP_SENDER_DAY, CAP_SIGNPOST_DAY, CAP_SIGNPOST_WEEK, LEAD_S_MAX, LEAD_S_MIN, MAX_ACC_MM,
    MAX_CATEGORY_RULES, MAX_STANDOFF_ZONES, SENSITIVITY_CEILING, SLOTS, STANDOFF_M,
};
use proxim_types::taxonomy::{is_valid_code, prefixes};
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
    pub cap_sender_day: u32,
    pub cap_app_day: u32,
    pub rot_lane: u32,
    pub rot_lanes: u32,
    pub driving: proxim_types::Driving,
    pub surface: proxim_types::Surface,
    pub profile: proxim_types::Profile,
    pub health_tier: bool,
    pub quiet_hours: &'a [proxim_types::Window],
    pub slots: usize,
    pub max_acc_mm: u32,
    pub categories_blocked: BTreeSet<&'a str>,
    /// Empty: every category is accepted (spec 7.1).
    pub categories_accepted: BTreeSet<&'a str>,
    pub affinity: BTreeMap<&'a str, u16>,
    pub senders_blocked: BTreeSet<&'a str>,
}

impl<'a> Eff<'a> {
    /// # Errors
    /// [`EngineError::Limit`] if the zone list exceeds 256 entries, or either category rule list
    /// does; [`EngineError::Parse`] for a malformed category code in `categories_accepted` or
    /// `category_affinity`, or the same affinity code twice (spec 7.1).
    pub fn new(p: &'a Policy) -> Result<Self, EngineError> {
        if p.standoff_zones.len() > MAX_STANDOFF_ZONES
            || p.categories_accepted.len() > MAX_CATEGORY_RULES
            || p.category_affinity.len() > MAX_CATEGORY_RULES
        {
            return Err(EngineError::Limit);
        }
        // Rotation lane (Addendum 02 5.1): lanes >= 1 and lane < lanes, else the policy is invalid.
        let rot_lanes = p.rot_lanes.unwrap_or(1);
        let rot_lane = p.rot_lane.unwrap_or(0);
        if rot_lanes == 0 || rot_lane >= rot_lanes {
            return Err(EngineError::Parse);
        }
        if !crate::timewin::valid(&p.quiet_hours) {
            return Err(EngineError::Parse);
        }
        let mut categories_accepted = BTreeSet::new();
        for c in &p.categories_accepted {
            if !is_valid_code(c) {
                return Err(EngineError::Parse);
            }
            categories_accepted.insert(c.as_str());
        }
        let mut affinity = BTreeMap::new();
        for a in &p.category_affinity {
            let w = a.weight_milli.min(AFFINITY_MAX_MILLI);
            if !is_valid_code(&a.category) || affinity.insert(a.category.as_str(), w).is_some() {
                return Err(EngineError::Parse);
            }
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
            cap_sender_day: p
                .cap_sender_day
                .unwrap_or(CAP_SENDER_DAY)
                .min(CAP_SENDER_DAY),
            cap_app_day: p.cap_app_day.unwrap_or(CAP_APP_DAY).min(CAP_APP_DAY_MAX),
            rot_lane,
            rot_lanes,
            surface: p.surface.unwrap_or_default(),
            profile: p.profile.unwrap_or_default(),
            health_tier: p.health_tier.unwrap_or(false),
            driving: p.driving.unwrap_or_default(),
            quiet_hours: &p.quiet_hours,
            slots: usize::from(p.slots.unwrap_or(SLOTS).min(SLOTS_CEILING)),
            max_acc_mm: p.max_acc_mm.unwrap_or(MAX_ACC_MM).min(MAX_ACC_MM),
            categories_blocked: p.categories_blocked.iter().map(String::as_str).collect(),
            categories_accepted,
            affinity,
            senders_blocked: p.senders_blocked.iter().map(String::as_str).collect(),
        })
    }
}

impl Eff<'_> {
    /// F16: a blocked entry equals the category or is a whole-segment prefix of it.
    pub fn category_blocked(&self, category: &str) -> bool {
        prefixes(category).any(|p| self.categories_blocked.contains(p))
    }

    /// F16 (spec 7.1): with an accept list, some entry must match.
    pub fn category_accepted(&self, category: &str) -> bool {
        self.categories_accepted.is_empty()
            || prefixes(category).any(|p| self.categories_accepted.contains(p))
    }

    /// The weight of the longest matching affinity entry, else neutral (1000).
    pub fn affinity_milli(&self, category: &str) -> u16 {
        prefixes(category)
            .find_map(|p| self.affinity.get(p).copied())
            .unwrap_or(AFFINITY_NEUTRAL_MILLI)
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
            (2, 5, 3, 100_000, 2)
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
    fn category_rules() {
        use proxim_types::CategoryAffinity;
        let aff = |c: &str, w| CategoryAffinity {
            category: c.into(),
            weight_milli: w,
        };
        let p = Policy {
            categories_blocked: alloc::vec!["alcohol".into(), "food.fast_food".into()],
            categories_accepted: alloc::vec!["food".into(), "health.pharmacy".into()],
            category_affinity: alloc::vec![
                aff("food", 1500),
                aff("food.cafe", 2500),
                aff("auto", 9000)
            ],
            ..Policy::default()
        };
        let e = Eff::new(&p).unwrap();
        assert!(e.category_blocked("alcohol") && e.category_blocked("alcohol.bar"));
        assert!(e.category_blocked("food.fast_food") && !e.category_blocked("food.cafe"));
        assert!(!e.category_blocked("alcoholfree"));
        assert!(e.category_accepted("food.cafe") && e.category_accepted("health.pharmacy"));
        assert!(!e.category_accepted("health") && !e.category_accepted("health.hospital"));
        assert_eq!(e.affinity_milli("food.cafe"), 2500, "longest match wins");
        assert_eq!(e.affinity_milli("food.cafe.x"), 2500);
        assert_eq!(e.affinity_milli("food.bakery"), 1500);
        assert_eq!(e.affinity_milli("retail"), 1000, "neutral");
        assert_eq!(e.affinity_milli("auto.fuel"), 3000, "clamped");
        assert!(
            Eff::new(&Policy::default())
                .unwrap()
                .category_accepted("anything")
        );
    }

    #[test]
    fn malformed_category_rules_are_rejected() {
        use proxim_types::CategoryAffinity;
        for bad in ["", "Food", "food.", "a.b.c.d.e"] {
            let p = Policy {
                categories_accepted: alloc::vec![bad.into()],
                ..Policy::default()
            };
            assert_eq!(Eff::new(&p).err(), Some(EngineError::Parse), "{bad}");
        }
        let dup = Policy {
            category_affinity: alloc::vec![
                CategoryAffinity {
                    category: "food".into(),
                    weight_milli: 1
                },
                CategoryAffinity {
                    category: "food".into(),
                    weight_milli: 2
                },
            ],
            ..Policy::default()
        };
        assert_eq!(Eff::new(&dup).err(), Some(EngineError::Parse));
        let many = Policy {
            categories_accepted: (0..=MAX_CATEGORY_RULES)
                .map(|i| alloc::format!("c{i}"))
                .collect(),
            ..Policy::default()
        };
        assert_eq!(Eff::new(&many).err(), Some(EngineError::Limit));
        // the block list keeps its old tolerance: any string, matched exactly or as a prefix
        let odd = Policy {
            categories_blocked: alloc::vec!["Not A Code".into()],
            ..Policy::default()
        };
        assert!(Eff::new(&odd).unwrap().category_blocked("Not A Code"));
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
