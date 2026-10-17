//! `decide_json`: the cross-binding conformance entry (engine-spec 9).
//!
//! Canonical JSON in, canonical JSON out. It never panics and never returns a bare error: a failure
//! is a typed error object `{"error":{"code":"E_PARSE","input":"fix"}}` (`input` only for parse
//! and limit failures of one argument). A shell MUST treat an error object as "no decision, state
//! unchanged". Inputs are parsed with the strict parser (no `null`, floats, duplicate keys or
//! unknown fields; bounded sizes), whitespace is tolerated, output is always canonical.

use crate::decide;
use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::canon::{to_canonical_json, value_to_canonical};
use proxim_types::strict::{JsonLimits, decode_tree, parse_value};
use proxim_types::{Clock, Consent, EngineError, Fix, Policy, Signpost, State};

/// Names of the arguments, used in parse errors.
const ARGS: [&str; 6] = ["state", "fix", "signposts", "policy", "consent", "clock"];

/// `decide_json(state, fix, signposts, policy, consent, clock, seed) -> {output, state}`.
///
/// `signposts` is a JSON array. The success form is exactly `{"output":O,"state":S}` in canonical
/// JSON, the failure form `{"error":{"code":C}}` or `{"error":{"code":C,"input":NAME}}`.
#[must_use]
pub fn decide_json(
    state: &str,
    fix: &str,
    signposts: &str,
    policy: &str,
    consent: &str,
    clock: &str,
    seed: u64,
) -> String {
    match run(state, fix, signposts, policy, consent, clock, seed) {
        Ok(s) => s,
        Err((code, input)) => error_object(code, input),
    }
}

fn error_object(code: EngineError, input: Option<&str>) -> String {
    let mut s = String::from("{\"error\":{\"code\":\"");
    s.push_str(code.code());
    s.push('"');
    if let Some(i) = input {
        s.push_str(",\"input\":\"");
        s.push_str(i);
        s.push('"');
    }
    s.push_str("}}");
    s
}

type Failure = (EngineError, Option<&'static str>);

/// Parse one argument. The top-level JSON kind must be the one the canonical form has (object for
/// every struct, array for the signpost list): derived struct deserializers also accept a
/// positional array, which no other binding would, so it is rejected here.
fn arg<T: serde::de::DeserializeOwned + serde::Serialize>(
    text: &str,
    name: &'static str,
    want_array: bool,
) -> Result<T, Failure> {
    let fail = |e: EngineError| (e, Some(name));
    let tree = parse_value(text.as_bytes(), &JsonLimits::default()).map_err(fail)?;
    if tree.is_array() != want_array || !(tree.is_array() || tree.is_object()) {
        return Err(fail(EngineError::Parse));
    }
    decode_tree(tree).map_err(fail)
}

fn run(
    state: &str,
    fix: &str,
    signposts: &str,
    policy: &str,
    consent: &str,
    clock: &str,
    seed: u64,
) -> Result<String, Failure> {
    let [n_state, n_fix, n_signposts, n_policy, n_consent, n_clock] = ARGS;
    let state: State = arg(state, n_state, false)?;
    let fix: Fix = arg(fix, n_fix, false)?;
    let signposts: Vec<Signpost> = arg(signposts, n_signposts, true)?;
    let policy: Policy = arg(policy, n_policy, false)?;
    let consent: Consent = arg(consent, n_consent, false)?;
    let clock: Clock = arg(clock, n_clock, false)?;
    let (output, next) =
        decide(&state, &fix, &signposts, &policy, &consent, &clock, seed).map_err(|e| (e, None))?;
    let out = to_canonical_json(&output).map_err(|e| (e, None))?;
    let st = to_canonical_json(&next).map_err(|e| (e, None))?;
    let mut s = String::with_capacity(out.len() + st.len() + 24);
    s.push_str("{\"output\":");
    s.push_str(&out);
    s.push_str(",\"state\":");
    s.push_str(&st);
    s.push('}');
    Ok(s)
}

/// `apply_control_json(state, control, clock) -> {state}` (engine spec 7.2): a dismiss, hide or
/// report from the person, e.g. `{"action":"dismiss","id":"sp-1"}`. Same error form as
/// [`decide_json`]; the success form is exactly `{"state":S}` in canonical JSON.
#[must_use]
pub fn apply_control_json(state: &str, control: &str, clock: &str) -> String {
    let run = || -> Result<String, Failure> {
        let st: State = arg(state, "state", false)?;
        let c: proxim_types::Control = arg(control, "control", false)?;
        let clock: Clock = arg(clock, "clock", false)?;
        let next = crate::controls::apply(&st, &c, &clock).map_err(|e| (e, None))?;
        let st = to_canonical_json(&next).map_err(|e| (e, None))?;
        let mut s = String::with_capacity(st.len() + 10);
        s.push_str("{\"state\":");
        s.push_str(&st);
        s.push('}');
        Ok(s)
    };
    run().unwrap_or_else(|(code, input)| error_object(code, input))
}

/// Split a `decide_json` success string into its canonical `output` and `state` texts.
///
/// # Errors
/// [`EngineError::Parse`] if the text is an error object or not the success form.
pub fn split_response(resp: &str) -> Result<(String, String), EngineError> {
    let v = parse_value(resp.as_bytes(), &JsonLimits::default())?;
    let obj = v.as_object().ok_or(EngineError::Parse)?;
    if obj.len() != 2 {
        return Err(EngineError::Parse);
    }
    let o = obj.get("output").ok_or(EngineError::Parse)?;
    let s = obj.get("state").ok_or(EngineError::Parse)?;
    Ok((value_to_canonical(o)?, value_to_canonical(s)?))
}
