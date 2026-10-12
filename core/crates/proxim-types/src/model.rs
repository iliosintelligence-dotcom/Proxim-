//! Engine inputs, outputs and state (spec 3.3). Integers only; absent = omitted.

use crate::error::EngineError;
use crate::reason::ReasonCode;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use serde::de::IgnoredAny;
use serde::{Deserialize, Deserializer, Serialize};

/// ASCII `[A-Za-z0-9._:-]{1,64}` identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Id(String);

impl Id {
    /// # Errors
    /// [`EngineError::Parse`] if the string is not a valid id.
    pub fn new(s: &str) -> Result<Self, EngineError> {
        let ok = !s.is_empty()
            && s.len() <= 64
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'));
        if ok {
            Ok(Self(String::from(s)))
        } else {
            Err(EngineError::Parse)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Id {
    type Error = EngineError;
    fn try_from(s: String) -> Result<Self, EngineError> {
        Self::new(&s)
    }
}

impl From<Id> for String {
    fn from(id: Id) -> Self {
        id.0
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Degrees x 1e7 (about 11 mm).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub lat_e7: i32,
    pub lon_e7: i32,
}

/// Millimetres east/north of a frame origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Enu {
    pub e_mm: i32,
    pub n_mm: i32,
}

/// One location fix. `speed_mmps` absent = invalid (never -1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fix {
    pub t_ms: i64,
    pub lat_e7: i32,
    pub lon_e7: i32,
    pub acc_mm: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_mmps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_acc_mmps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub course_ddeg: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub course_acc_ddeg: Option<u16>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_mock: bool,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
}

impl Fix {
    #[must_use]
    pub const fn position(&self) -> Position {
        Position {
            lat_e7: self.lat_e7,
            lon_e7: self.lon_e7,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clock {
    pub now_ms: i64,
    pub tz_offset_min: i16,
}

/// Consent state. Any wire value other than exactly `"granted"` / `"denied"` parses as `Unknown` (I4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentValue {
    Granted,
    Denied,
    #[default]
    Unknown,
}

impl<'de> Deserialize<'de> for ConsentValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // Fail closed (I4): a value of any type other than the exact strings is `Unknown`,
        // never a parse failure that a caller might handle by guessing.
        Ok(match serde_json::Value::deserialize(d)? {
            serde_json::Value::String(s) if s == "granted" => Self::Granted,
            serde_json::Value::String(s) if s == "denied" => Self::Denied,
            _ => Self::Unknown,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consent {
    #[serde(default)]
    pub location_matching: ConsentValue,
    #[serde(default)]
    pub nearby_health_events: ConsentValue,
    #[serde(default)]
    pub aggregate_reporting: ConsentValue,
    #[serde(default)]
    pub local_notifications: ConsentValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_receipt: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StandoffZone {
    pub id: Id,
    pub center: Position,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standoff_m: Option<u32>,
}

/// Opaque reserved field: accepted on input, ignored, never re-emitted (spec 10).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reserved(bool);

impl<'de> Deserialize<'de> for Reserved {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        IgnoredAny::deserialize(d).map(|_| Self(true))
    }
}

impl Serialize for Reserved {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_unit()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simulator_mode: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_directed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explain: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories_blocked: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub senders_blocked: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitivity_ceiling: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead_s_min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead_s_max: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_signpost_day: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_signpost_week: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_min_interval_s: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slots: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_acc_mm: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub standoff_zones: Vec<StandoffZone>,
    #[serde(default, skip_serializing)]
    pub rot_lane: Reserved,
    #[serde(default, skip_serializing)]
    pub rot_lanes: Reserved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    General,
    Health,
}

/// `exit` is internal and not authorable; `cross dwell pass sequence` are reserved (spec 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Enter,
    Approach,
    Exit,
    Cross,
    Dwell,
    Pass,
    Sequence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Vehicle,
    Cycle,
    Foot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Shape {
    Circle {
        center: Position,
        radius_m: u32,
    },
    Polygon {
        vertices: Vec<Position>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        inradius_m: Option<u32>,
    },
    Route {
        vertices: Vec<Position>,
        buffer_m: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor_along_m: Option<u32>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trigger {
    pub origin: Position,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reach_m: Option<u32>,
    pub shape: Shape,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_event_age_s: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_event_dist_m: Option<u32>,
    // approach parameters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead_s: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d_lo_m: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d_hi_m: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Mode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_min_mmps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_max_mmps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearing_tol_ddeg: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signpost {
    pub id: Id,
    pub rev: u32,
    pub class: Class,
    pub category: String,
    pub sender: String,
    pub sensitivity: u8,
    pub event: EventKind,
    pub valid_from_ms: i64,
    pub valid_until_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_path_tier: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_path_conf_milli: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank_milli: Option<u32>,
    pub trigger: Trigger,
    // Reserved, accepted and ignored (spec 10).
    #[serde(default, skip_serializing)]
    pub intent: Reserved,
    #[serde(default, skip_serializing)]
    pub content_mode: Reserved,
    #[serde(default, skip_serializing)]
    pub precision_class: Reserved,
    #[serde(default, skip_serializing)]
    pub min_mode: Reserved,
    #[serde(default, skip_serializing)]
    pub max_acc_m: Reserved,
    #[serde(default, skip_serializing)]
    pub arm_scope: Reserved,
    #[serde(default, skip_serializing)]
    pub relation: Reserved,
    #[serde(default, skip_serializing)]
    pub selector: Reserved,
    #[serde(default, skip_serializing)]
    pub live_at_ms: Reserved,
    #[serde(default, skip_serializing)]
    pub fresh_required_s: Reserved,
    #[serde(default, skip_serializing)]
    pub cooldown_s: Reserved,
    #[serde(default, skip_serializing)]
    pub creatives: Reserved,
    #[serde(default, skip_serializing)]
    pub rot_days: Reserved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Fire,
    Drop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Copy {
    NearYou,
    Ahead,
    OnRoute,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub signpost: Id,
    pub rev: u32,
    pub outcome: Outcome,
    pub reason: ReasonCode,
    pub event: EventKind,
    pub t_ms: i64,
    pub path_tier: u8,
    pub path_conf_milli: u16,
    pub copy: Copy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d_rem_mm: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t_hat_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixStatus {
    pub ok: bool,
    pub reason: ReasonCode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathInfo {
    pub tier: u8,
    pub conf_milli: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_mmps: Option<u32>,
    pub heading_valid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub signpost: Id,
    pub reason: ReasonCode,
}

/// Engine output. Carries no position (I2, I6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub v: u8,
    pub t_ms: i64,
    pub fix: FixStatus,
    pub path: PathInfo,
    pub decisions: Vec<Decision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<Note>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignpostCaps {
    pub id: Id,
    pub day_idx: i64,
    pub day_n: u32,
    pub week_idx: i64,
    pub week_n: u32,
}

/// Persistable part of the state (encrypted, no backup, I3).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Caps {
    pub hwm_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_fire_ms: Option<i64>,
    pub signposts: Vec<SignpostCaps>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceFix {
    pub t_ms: i64,
    pub lat_e7: i32,
    pub lon_e7: i32,
    pub acc_mm: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_mmps: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LatchWhy {
    Fired,
    Late,
}

/// Per-signpost detector state (spec 6, tag `s`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "s", rename_all = "snake_case", deny_unknown_fields)]
pub enum DetState {
    Cand {
        n: u32,
        first_t: i64,
        last_t: i64,
    },
    In {
        xn: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x_first_t: Option<i64>,
        last_t: i64,
    },
    Cool {
        until_ms: i64,
    },
    Armed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        confirm_t: Option<i64>,
        last_t: i64,
    },
    Latched {
        why: LatchWhy,
        last_t: i64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetEntry {
    pub id: Id,
    pub rev: u32,
    pub st: DetState,
}

/// Memory-only part of the state. MUST NOT be persisted, logged or sent (I3).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_t_ms: Option<i64>,
    pub jump_rejects: u8,
    pub trace: Vec<TraceFix>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vf_mmps: Option<u32>,
    /// Heading unit vector (east, north), Q30.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<[i32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_t_ms: Option<i64>,
    pub det: Vec<DetEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub v: u8,
    pub caps: Caps,
    pub mem: Mem,
}

impl State {
    /// A new process starts from this.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            v: 1,
            caps: Caps::default(),
            mem: Mem::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_validation() {
        assert!(Id::new("sp-1.a:b_c").is_ok());
        assert_eq!(Id::new(""), Err(EngineError::Parse));
        assert_eq!(Id::new("a b"), Err(EngineError::Parse));
        let long = "a".repeat(65);
        assert_eq!(Id::new(&long), Err(EngineError::Parse));
    }

    #[test]
    fn consent_non_granted_is_unknown() {
        let c: Consent = serde_json::from_str(r#"{"location_matching":"Granted"}"#).unwrap();
        assert_eq!(c.location_matching, ConsentValue::Unknown);
        let c: Consent = serde_json::from_str(r#"{"location_matching":"granted"}"#).unwrap();
        assert_eq!(c.location_matching, ConsentValue::Granted);
        let c: Consent = serde_json::from_str("{}").unwrap();
        assert_eq!(c.location_matching, ConsentValue::Unknown);
    }

    #[test]
    fn unknown_fields_rejected_but_reserved_accepted() {
        assert!(
            serde_json::from_str::<Fix>(r#"{"t_ms":1,"lat_e7":1,"lon_e7":1,"acc_mm":1,"bogus":1}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<Policy>(r#"{"rot_lane":{"x":[1,2]}}"#).is_ok());
    }

    #[test]
    fn empty_state_roundtrips() {
        let s = State::empty();
        let j = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<State>(&j).unwrap(), s);
    }
}
