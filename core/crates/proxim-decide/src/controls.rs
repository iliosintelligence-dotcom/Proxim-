//! The person's own controls (PLAN 5.6 step 5; engine spec 7.2): dismiss rests a family for
//! 7 days, hide is permanent for a signpost, report is permanent for a sender on this device.
//! They live in `State.controls`, so they are local to the install and lost on reinstall.

use alloc::vec::Vec;
use proxim_types::defaults::{DISMISS_COOLDOWN_MS, MAX_DISMISSED, MAX_HIDDEN, MAX_REPORTED};
use proxim_types::{Clock, Control, Controls, Dismissal, EngineError, State};

/// Apply one user action. Pure: the result is the new state, `state` is untouched. Expired
/// dismissals are pruned first; time is `max(clock.now_ms, caps.hwm_ms)`, as in `decide`.
///
/// # Errors
/// [`EngineError::Parse`] for an invalid state or clock or an empty sender;
/// [`EngineError::Limit`] when the list is full (a full dismissal list first drops the entry that
/// ends soonest).
pub fn apply(state: &State, control: &Control, clock: &Clock) -> Result<State, EngineError> {
    crate::validate::state(state)?;
    crate::validate::clock(clock)?;
    let now = clock.now_ms.max(state.caps.hwm_ms);
    let mut next = state.clone();
    next.controls = prune(&state.controls, now);
    let c = &mut next.controls;
    match control {
        Control::Dismiss { id } => {
            let until_ms = now.saturating_add(DISMISS_COOLDOWN_MS);
            match c.dismissed.binary_search_by(|d| d.id.cmp(id)) {
                Ok(i) => {
                    if let Some(d) = c.dismissed.get_mut(i) {
                        d.until_ms = until_ms;
                    }
                }
                Err(i) => {
                    c.dismissed.insert(
                        i,
                        Dismissal {
                            id: id.clone(),
                            until_ms,
                        },
                    );
                    if c.dismissed.len() > MAX_DISMISSED {
                        let soonest = c
                            .dismissed
                            .iter()
                            .enumerate()
                            .filter(|(_, d)| &d.id != id)
                            .min_by_key(|(_, d)| (d.until_ms, d.id.clone()))
                            .map(|(k, _)| k);
                        if let Some(k) = soonest {
                            c.dismissed.remove(k);
                        }
                    }
                }
            }
        }
        Control::Hide { id } => {
            if let Err(i) = c.hidden.binary_search(id) {
                if c.hidden.len() >= MAX_HIDDEN {
                    return Err(EngineError::Limit);
                }
                c.hidden.insert(i, id.clone());
            }
        }
        Control::Report { sender } => {
            if sender.is_empty() {
                return Err(EngineError::Parse);
            }
            if let Err(i) = c.reported.binary_search(sender) {
                if c.reported.len() >= MAX_REPORTED {
                    return Err(EngineError::Limit);
                }
                c.reported.insert(i, sender.clone());
            }
        }
    }
    Ok(next)
}

/// Drop dismissals whose rest is over at `now`.
pub(crate) fn prune(c: &Controls, now: i64) -> Controls {
    Controls {
        dismissed: c
            .dismissed
            .iter()
            .filter(|d| now < d.until_ms)
            .cloned()
            .collect::<Vec<_>>(),
        hidden: c.hidden.clone(),
        reported: c.reported.clone(),
    }
}

/// The user-control filter for one signpost, in spec order (F17a to F17c), or `None`.
pub(crate) fn blocks(
    c: &Controls,
    id: &str,
    sender: &str,
    now: i64,
) -> Option<proxim_types::ReasonCode> {
    use proxim_types::ReasonCode;
    if c.reported
        .binary_search_by(|s| s.as_str().cmp(sender))
        .is_ok()
    {
        return Some(ReasonCode::SenderReported);
    }
    if c.hidden.binary_search_by(|h| h.as_str().cmp(id)).is_ok() {
        return Some(ReasonCode::HiddenByUser);
    }
    let resting = c
        .dismissed
        .binary_search_by(|d| d.id.as_str().cmp(id))
        .ok()
        .and_then(|i| c.dismissed.get(i))
        .is_some_and(|d| now < d.until_ms);
    resting.then_some(ReasonCode::CooldownDismissed)
}

/// Structure of `State.controls`: sorted, unique, within bounds.
pub(crate) fn validate(c: &Controls) -> Result<(), EngineError> {
    if c.dismissed.len() > MAX_DISMISSED
        || c.hidden.len() > MAX_HIDDEN
        || c.reported.len() > MAX_REPORTED
    {
        return Err(EngineError::Limit);
    }
    let sorted = c
        .dismissed
        .windows(2)
        .all(|w| matches!(w, [a, b] if a.id < b.id))
        && c.hidden.windows(2).all(|w| matches!(w, [a, b] if a < b))
        && c.reported.windows(2).all(|w| matches!(w, [a, b] if a < b))
        && c.reported.iter().all(|s| !s.is_empty());
    if sorted {
        Ok(())
    } else {
        Err(EngineError::Parse)
    }
}
