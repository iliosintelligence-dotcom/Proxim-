//! Reason codes (spec 8). Ids are `repr(u16)`, stable, never reused.

use serde::{Deserialize, Serialize};

/// Why a call, fix or signpost was dropped (or `Ok`). Wire form SCREAMING_SNAKE_CASE.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(u16)]
pub enum ReasonCode {
    Ok = 0,
    Suppressed = 1,
    ConsentDenied = 100,
    EngineDisabled = 101,
    FixInvalid = 110,
    FixMock = 111,
    FixAccuracyTooCoarse = 112,
    FixFromFuture = 113,
    FixStale = 114,
    FixOutOfOrder = 115,
    FixJump = 116,
    FixOutOfFrame = 117,
    AccTooCoarse = 118,
    SensitiveStandoff = 120,
    UnsupportedEvent = 200,
    TriggerInvalid = 201,
    NotYetLive = 202,
    Expired = 203,
    HealthConsentMissing = 204,
    PartnerCategoryBlocked = 205,
    SenderBlocked = 206,
    SensitivityAboveCeiling = 207,
    TriggerInStandoff = 208,
    OutOfRange = 209,
    OffCorridor = 300,
    SpeedOutOfBand = 301,
    BearingMismatch = 302,
    NotClosing = 303,
    BeyondWindow = 304,
    PathConfidenceLow = 305,
    TooLateForLead = 306,
    PathTierLow = 307,
    EventTooOld = 400,
    EventTooFar = 401,
    CapSignpostDay = 402,
    CapSignpostWeek = 403,
    CapAppInterval = 404,
    LostSlot = 405,
    NoEvent = 406,
}

impl ReasonCode {
    /// Stable numeric id.
    #[must_use]
    pub const fn id(self) -> u16 {
        self as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_match_spec_rows() {
        assert_eq!(ReasonCode::Ok.id(), 0);
        assert_eq!(ReasonCode::ConsentDenied.id(), 100);
        assert_eq!(ReasonCode::SensitiveStandoff.id(), 120);
        assert_eq!(ReasonCode::OutOfRange.id(), 209);
        assert_eq!(ReasonCode::PathTierLow.id(), 307);
        assert_eq!(ReasonCode::NoEvent.id(), 406);
    }

    #[test]
    fn wire_form_is_screaming_snake() {
        let s = serde_json::to_string(&ReasonCode::FixAccuracyTooCoarse).unwrap();
        assert_eq!(s, "\"FIX_ACCURACY_TOO_COARSE\"");
        let r: ReasonCode = serde_json::from_str("\"UNSUPPORTED_EVENT\"").unwrap();
        assert_eq!(r, ReasonCode::UnsupportedEvent);
    }
}
