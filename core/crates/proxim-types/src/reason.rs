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
    CategoryNotAccepted = 210,
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
    // Pack verification (M2, engine spec 8 row 500; `proxim-pack`). Never emitted by `decide`.
    Revoked = 500,
    SignatureInvalid = 501,
    SeqRollback = 502,
    KidRevoked = 503,
    PackExpired = 504,
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

    /// The whole spec section 8 table (rows 0..400 and the M2 pack codes of row 500): id and wire string.
    const SPEC_TABLE: &[(ReasonCode, u16, &str)] = &[
        (ReasonCode::Ok, 0, "OK"),
        (ReasonCode::Suppressed, 1, "SUPPRESSED"),
        (ReasonCode::ConsentDenied, 100, "CONSENT_DENIED"),
        (ReasonCode::EngineDisabled, 101, "ENGINE_DISABLED"),
        (ReasonCode::FixInvalid, 110, "FIX_INVALID"),
        (ReasonCode::FixMock, 111, "FIX_MOCK"),
        (
            ReasonCode::FixAccuracyTooCoarse,
            112,
            "FIX_ACCURACY_TOO_COARSE",
        ),
        (ReasonCode::FixFromFuture, 113, "FIX_FROM_FUTURE"),
        (ReasonCode::FixStale, 114, "FIX_STALE"),
        (ReasonCode::FixOutOfOrder, 115, "FIX_OUT_OF_ORDER"),
        (ReasonCode::FixJump, 116, "FIX_JUMP"),
        (ReasonCode::FixOutOfFrame, 117, "FIX_OUT_OF_FRAME"),
        (ReasonCode::AccTooCoarse, 118, "ACC_TOO_COARSE"),
        (ReasonCode::SensitiveStandoff, 120, "SENSITIVE_STANDOFF"),
        (ReasonCode::UnsupportedEvent, 200, "UNSUPPORTED_EVENT"),
        (ReasonCode::TriggerInvalid, 201, "TRIGGER_INVALID"),
        (ReasonCode::NotYetLive, 202, "NOT_YET_LIVE"),
        (ReasonCode::Expired, 203, "EXPIRED"),
        (
            ReasonCode::HealthConsentMissing,
            204,
            "HEALTH_CONSENT_MISSING",
        ),
        (
            ReasonCode::PartnerCategoryBlocked,
            205,
            "PARTNER_CATEGORY_BLOCKED",
        ),
        (ReasonCode::SenderBlocked, 206, "SENDER_BLOCKED"),
        (
            ReasonCode::SensitivityAboveCeiling,
            207,
            "SENSITIVITY_ABOVE_CEILING",
        ),
        (ReasonCode::TriggerInStandoff, 208, "TRIGGER_IN_STANDOFF"),
        (ReasonCode::OutOfRange, 209, "OUT_OF_RANGE"),
        (
            ReasonCode::CategoryNotAccepted,
            210,
            "CATEGORY_NOT_ACCEPTED",
        ),
        (ReasonCode::OffCorridor, 300, "OFF_CORRIDOR"),
        (ReasonCode::SpeedOutOfBand, 301, "SPEED_OUT_OF_BAND"),
        (ReasonCode::BearingMismatch, 302, "BEARING_MISMATCH"),
        (ReasonCode::NotClosing, 303, "NOT_CLOSING"),
        (ReasonCode::BeyondWindow, 304, "BEYOND_WINDOW"),
        (ReasonCode::PathConfidenceLow, 305, "PATH_CONFIDENCE_LOW"),
        (ReasonCode::TooLateForLead, 306, "TOO_LATE_FOR_LEAD"),
        (ReasonCode::PathTierLow, 307, "PATH_TIER_LOW"),
        (ReasonCode::EventTooOld, 400, "EVENT_TOO_OLD"),
        (ReasonCode::EventTooFar, 401, "EVENT_TOO_FAR"),
        (ReasonCode::CapSignpostDay, 402, "CAP_SIGNPOST_DAY"),
        (ReasonCode::CapSignpostWeek, 403, "CAP_SIGNPOST_WEEK"),
        (ReasonCode::CapAppInterval, 404, "CAP_APP_INTERVAL"),
        (ReasonCode::LostSlot, 405, "LOST_SLOT"),
        (ReasonCode::NoEvent, 406, "NO_EVENT"),
        (ReasonCode::Revoked, 500, "REVOKED"),
        (ReasonCode::SignatureInvalid, 501, "SIGNATURE_INVALID"),
        (ReasonCode::SeqRollback, 502, "SEQ_ROLLBACK"),
        (ReasonCode::KidRevoked, 503, "KID_REVOKED"),
        (ReasonCode::PackExpired, 504, "PACK_EXPIRED"),
    ];

    #[test]
    fn every_code_matches_the_spec_table_and_round_trips() {
        for (code, id, name) in SPEC_TABLE {
            assert_eq!(code.id(), *id, "{name}");
            let wire = serde_json::to_string(code).unwrap();
            assert_eq!(wire, alloc::format!("\"{name}\""));
            assert_eq!(serde_json::from_str::<ReasonCode>(&wire).unwrap(), *code);
        }
        // Ids are unique and the table is strictly ascending.
        for pair in SPEC_TABLE.windows(2) {
            assert!(pair[0].1 < pair[1].1);
        }
    }

    #[test]
    fn unknown_or_reserved_names_do_not_parse() {
        for bad in ["\"CAP_SENDER_DAY\"", "\"ok\"", "\"\"", "7", "null"] {
            assert!(serde_json::from_str::<ReasonCode>(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn wire_form_is_screaming_snake() {
        let s = serde_json::to_string(&ReasonCode::FixAccuracyTooCoarse).unwrap();
        assert_eq!(s, "\"FIX_ACCURACY_TOO_COARSE\"");
        let r: ReasonCode = serde_json::from_str("\"UNSUPPORTED_EVENT\"").unwrap();
        assert_eq!(r, ReasonCode::UnsupportedEvent);
    }
}
