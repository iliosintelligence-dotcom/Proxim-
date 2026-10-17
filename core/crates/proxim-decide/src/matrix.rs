//! The policy matrix (engine spec 7.7; PLAN 4 `policy_rules`, 6.5; Addendum 01 1, 5.2): which
//! advert class may be delivered on which surface, in which mode, wake state and partner profile.
//!
//! The table is data: an ordered list of rows, each a set of allowed values per axis (empty = any)
//! and a verdict; the first matching row decides. The last row matches every cell and denies, so a
//! cell no other row names fails closed. `core/policy/matrix.json` is the published copy (rows plus
//! every cell expanded) for the server and console; a test keeps it byte-identical to this table.

use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::{Class, ContentMode, EngineError, Profile, ReasonCode, Surface, Wake};
use serde::Serialize;

/// Bumped on every change to [`RULES`]; recorded with the published copy.
pub const MATRIX_VERSION: u32 = 1;

/// One cell of the matrix: a value on every axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Cell {
    pub mode: ContentMode,
    pub surface: Surface,
    pub wake: Wake,
    pub class: Class,
    pub profile: Profile,
    pub health_tier: bool,
    pub child_directed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Deny(ReasonCode),
}

/// A row: every non-empty axis list must contain the cell's value.
#[derive(Debug)]
pub struct Rule {
    pub id: &'static str,
    pub mode: &'static [ContentMode],
    pub surface: &'static [Surface],
    pub wake: &'static [Wake],
    pub class: &'static [Class],
    pub profile: &'static [Profile],
    pub health_tier: &'static [bool],
    pub child_directed: &'static [bool],
    pub verdict: Verdict,
}

const ANY: Rule = Rule {
    id: "",
    mode: &[],
    surface: &[],
    wake: &[],
    class: &[],
    profile: &[],
    health_tier: &[],
    child_directed: &[],
    verdict: Verdict::Deny(ReasonCode::SurfaceNotAllowed),
};

const HEALTH: &[Class] = &[Class::HealthAdjacent, Class::HealthFacility];

/// The rows, in order. Each cites its source.
pub const RULES: &[Rule] = &[
    // COPPA (PLAN 6.6, I9): a child-directed app runs no matching at all (also F02).
    Rule {
        id: "child_directed",
        child_directed: &[true],
        verdict: Verdict::Deny(ReasonCode::EngineDisabled),
        ..ANY
    },
    // R4: a prohibited anchor is never served, anywhere.
    Rule {
        id: "prohibited_anchor",
        class: &[Class::ProhibitedAnchor],
        verdict: Verdict::Deny(ReasonCode::ProhibitedAnchor),
        ..ANY
    },
    // R6 (PLAN 5.8; Addendum 01 5.2): a health class is never an OS notification, in any mode.
    Rule {
        id: "health_never_notification",
        class: HEALTH,
        surface: &[Surface::OsNotification],
        verdict: Verdict::Deny(ReasonCode::SurfaceNotAllowed),
        ..ANY
    },
    // PLAN 6.5, Addendum 01 5.1 (3) and 5.2: a Sealed app has no notifications and no background,
    // in any mode, whatever later exception M4/M11 or M15 adds.
    Rule {
        id: "sealed_no_notification",
        profile: &[Profile::Sealed],
        surface: &[Surface::OsNotification],
        verdict: Verdict::Deny(ReasonCode::SurfaceNotAllowed),
        ..ANY
    },
    Rule {
        id: "sealed_no_background",
        profile: &[Profile::Sealed],
        wake: &[Wake::Background],
        verdict: Verdict::Deny(ReasonCode::HostedBackgroundOff),
        ..ANY
    },
    // Addendum 01 1 and 9: owner mode waits for the hosted pilot (M15).
    Rule {
        id: "owner_mode_not_built",
        mode: &[ContentMode::Owner],
        verdict: Verdict::Deny(ReasonCode::OwnerModeOff),
        ..ANY
    },
    // Addendum 01 5.1: hosted background matching is off (a named-partner exception is M4/M11).
    Rule {
        id: "hosted_background_off",
        mode: &[ContentMode::Hosted],
        wake: &[Wake::Background],
        verdict: Verdict::Deny(ReasonCode::HostedBackgroundOff),
        ..ANY
    },
    // Addendum 01 5.2: no hosted advert is an OS notification.
    Rule {
        id: "hosted_notification",
        mode: &[ContentMode::Hosted],
        surface: &[Surface::OsNotification],
        verdict: Verdict::Deny(ReasonCode::SurfaceNotAllowed),
        ..ANY
    },
    // PLAN 6.5 "who may receive": health classes only in a Sealed app with the health tier.
    Rule {
        id: "health_needs_sealed",
        class: HEALTH,
        profile: &[Profile::Standard],
        verdict: Verdict::Deny(ReasonCode::HealthPartnerNotEligible),
        ..ANY
    },
    Rule {
        id: "health_needs_tier",
        class: HEALTH,
        health_tier: &[false],
        verdict: Verdict::Deny(ReasonCode::HealthPartnerNotEligible),
        ..ANY
    },
    // The one allowed path in v1: a hosted advert in an on-screen app slot.
    Rule {
        id: "hosted_in_app",
        mode: &[ContentMode::Hosted],
        surface: &[Surface::InApp],
        wake: &[Wake::Foreground],
        class: &[Class::General, Class::HealthAdjacent, Class::HealthFacility],
        verdict: Verdict::Allow,
        ..ANY
    },
    // Fail closed: whatever no row above names is denied.
    Rule {
        id: "fail_closed",
        ..ANY
    },
];

fn has<T: PartialEq>(set: &[T], v: &T) -> bool {
    set.is_empty() || set.contains(v)
}

impl Rule {
    #[must_use]
    pub fn matches(&self, c: &Cell) -> bool {
        has(self.mode, &c.mode)
            && has(self.surface, &c.surface)
            && has(self.wake, &c.wake)
            && has(self.class, &canonical_class(c.class))
            && has(self.profile, &c.profile)
            && has(self.health_tier, &c.health_tier)
            && has(self.child_directed, &c.child_directed)
    }
}

/// The legacy engine_spec 1 `health` class reads as `health_facility`, the strictest (7.4).
const fn canonical_class(c: Class) -> Class {
    match c {
        Class::Health => Class::HealthFacility,
        other => other,
    }
}

/// The first matching row and its verdict. No match is impossible (the last row matches every
/// cell) and would deny.
#[must_use]
pub fn lookup(c: &Cell) -> (&'static str, Verdict) {
    RULES.iter().find(|r| r.matches(c)).map_or(
        ("fail_closed", Verdict::Deny(ReasonCode::SurfaceNotAllowed)),
        |r| (r.id, r.verdict),
    )
}

/// The classes the matrix ranges over (the legacy `health` is folded into `health_facility`).
pub const CLASSES: [Class; 4] = [
    Class::General,
    Class::HealthAdjacent,
    Class::HealthFacility,
    Class::ProhibitedAnchor,
];

/// Every cell, in a fixed order (axes as declared in [`Cell`], values as declared in their enums).
#[must_use]
pub fn cells() -> Vec<Cell> {
    let mut out = Vec::new();
    for mode in [ContentMode::Hosted, ContentMode::Owner] {
        for surface in [Surface::InApp, Surface::OsNotification] {
            for wake in [Wake::Foreground, Wake::Background] {
                for class in CLASSES {
                    for profile in [Profile::Standard, Profile::Sealed] {
                        for health_tier in [false, true] {
                            for child_directed in [false, true] {
                                out.push(Cell {
                                    mode,
                                    surface,
                                    wake,
                                    class,
                                    profile,
                                    health_tier,
                                    child_directed,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

#[derive(Serialize)]
struct RuleJson<'a> {
    id: &'a str,
    when: serde_json::Map<String, serde_json::Value>,
    verdict: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<ReasonCode>,
}

#[derive(Serialize)]
struct CellJson<'a> {
    #[serde(flatten)]
    cell: Cell,
    rule: &'a str,
    verdict: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<ReasonCode>,
}

fn verdict_parts(v: Verdict) -> (&'static str, Option<ReasonCode>) {
    match v {
        Verdict::Allow => ("allow", None),
        Verdict::Deny(r) => ("deny", Some(r)),
    }
}

fn axis<T: Serialize>(
    when: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
    set: &[T],
) -> Result<(), EngineError> {
    if !set.is_empty() {
        let v = serde_json::to_value(set).map_err(|_| EngineError::Parse)?;
        when.insert(name.into(), v);
    }
    Ok(())
}

/// The published copy (`core/policy/matrix.json`): the rows, then every cell with the row that
/// decides it, in canonical JSON.
///
/// # Errors
/// [`EngineError::Parse`] if serialisation fails (it cannot for these types).
pub fn published_json() -> Result<String, EngineError> {
    let mut rules = Vec::new();
    for r in RULES {
        let mut when = serde_json::Map::new();
        axis(&mut when, "mode", r.mode)?;
        axis(&mut when, "surface", r.surface)?;
        axis(&mut when, "wake", r.wake)?;
        axis(&mut when, "class", r.class)?;
        axis(&mut when, "profile", r.profile)?;
        axis(&mut when, "health_tier", r.health_tier)?;
        axis(&mut when, "child_directed", r.child_directed)?;
        let (verdict, reason) = verdict_parts(r.verdict);
        rules.push(RuleJson {
            id: r.id,
            when,
            verdict,
            reason,
        });
    }
    let all = cells();
    let mut expanded = Vec::with_capacity(all.len());
    for cell in all {
        let (rule, v) = lookup(&cell);
        let (verdict, reason) = verdict_parts(v);
        expanded.push(CellJson {
            cell,
            rule,
            verdict,
            reason,
        });
    }
    let doc = serde_json::json!({
        "format": "proxim-policy-matrix/1",
        "matrix_version": MATRIX_VERSION,
        "first_match_wins": true,
        "rules": rules,
        "cells": expanded,
    });
    proxim_types::canon::to_canonical_json(&doc)
}
