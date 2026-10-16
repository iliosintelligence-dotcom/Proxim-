//! Golden vectors for `decide` (engine-spec 9): types, builder, runners.
//!
//! File format, canonical JSON plus a trailing `\n`:
//! `{header, seed, policy, consent, signposts[], steps[{clock, fix, expect:{output, state_sha256}}],
//! trace_sha256}`. The runner starts at `State::empty()`, threads `state'`, compares the canonical
//! bytes of `output` and the SHA-256 of the canonical bytes of `state'`;
//! `trace_sha256 = sha256(concat(canon(output) + "\n" + state_sha256 + "\n"))`.

use crate::decide;
use crate::json::split_response;
use crate::sha256::sha256_hex;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::canon::to_canonical_json;
use proxim_types::strict::{JsonLimits, from_canonical_json_with};
use proxim_types::{
    Clock, Consent, ENGINE_SPEC, EngineError, Fix, Output, Policy, Signpost, State,
};
use serde::{Deserialize, Serialize};

/// `format` value of the header.
pub const FORMAT: &str = "proxim-golden/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    pub format: String,
    pub kind: String,
    pub id: String,
    pub engine_spec: u32,
    pub synthetic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    pub output: Output,
    pub state_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub clock: Clock,
    pub fix: Fix,
    pub expect: Expect,
}

/// A decide golden vector as stored on disk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vector {
    pub header: Header,
    pub seed: u64,
    pub policy: Policy,
    pub consent: Consent,
    pub signposts: Vec<Signpost>,
    pub steps: Vec<Step>,
    pub trace_sha256: String,
}

/// The inputs of one step (what a scenario author writes).
#[derive(Clone, Debug)]
pub struct StepInput {
    pub clock: Clock,
    pub fix: Fix,
}

/// A scenario before expectations exist.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub id: String,
    pub seed: u64,
    pub policy: Policy,
    pub consent: Consent,
    pub signposts: Vec<Signpost>,
    pub steps: Vec<StepInput>,
}

fn canon_state_hash(state: &State) -> Result<String, EngineError> {
    Ok(sha256_hex(to_canonical_json(state)?.as_bytes()))
}

/// `sha256(canon(output) + "\n" + state_sha256 + "\n" ...)` over all steps.
///
/// # Errors
/// [`EngineError::Parse`] if an output cannot be rendered canonically.
pub fn trace_hash(steps: &[Step]) -> Result<String, EngineError> {
    let mut buf = String::new();
    for s in steps {
        buf.push_str(&to_canonical_json(&s.expect.output)?);
        buf.push('\n');
        buf.push_str(&s.expect.state_sha256);
        buf.push('\n');
    }
    Ok(sha256_hex(buf.as_bytes()))
}

/// Run a scenario through the typed `decide` and record the expectations.
///
/// # Errors
/// Any [`EngineError`] returned by `decide` (a vector must be generated from valid inputs).
pub fn build(sc: &Scenario) -> Result<Vector, EngineError> {
    let mut state = State::empty();
    let mut steps = Vec::with_capacity(sc.steps.len());
    for si in &sc.steps {
        let (output, next) = decide(
            &state,
            &si.fix,
            &sc.signposts,
            &sc.policy,
            &sc.consent,
            &si.clock,
            sc.seed,
        )?;
        let state_sha256 = canon_state_hash(&next)?;
        state = next;
        steps.push(Step {
            clock: si.clock,
            fix: si.fix,
            expect: Expect {
                output,
                state_sha256,
            },
        });
    }
    let trace_sha256 = trace_hash(&steps)?;
    Ok(Vector {
        header: Header {
            format: String::from(FORMAT),
            kind: String::from("decide"),
            id: sc.id.clone(),
            engine_spec: ENGINE_SPEC,
            synthetic: true,
        },
        seed: sc.seed,
        policy: sc.policy.clone(),
        consent: sc.consent.clone(),
        signposts: sc.signposts.clone(),
        steps,
        trace_sha256,
    })
}

/// Canonical bytes of a vector: canonical JSON plus a trailing newline.
///
/// # Errors
/// [`EngineError::Parse`] if the vector cannot be rendered canonically.
pub fn render(v: &Vector) -> Result<String, EngineError> {
    let mut s = to_canonical_json(v)?;
    s.push('\n');
    Ok(s)
}

/// Parse a golden file; the bytes must be canonical plus the trailing newline.
///
/// # Errors
/// [`EngineError::Parse`] for non-canonical bytes, a missing newline, a wrong header or unknown
/// fields.
pub fn parse(bytes: &[u8]) -> Result<Vector, EngineError> {
    let body = bytes.strip_suffix(b"\n").ok_or(EngineError::Parse)?;
    let v: Vector = from_canonical_json_with(body, &JsonLimits::default())?;
    let h = &v.header;
    if h.format != FORMAT || h.kind != "decide" || h.engine_spec != ENGINE_SPEC || !h.synthetic {
        return Err(EngineError::Parse);
    }
    Ok(v)
}

fn mismatch(id: &str, step: usize, what: &str) -> String {
    format!("golden {id}: step {step}: {what}")
}

/// Verify a vector with the typed `decide`. `Err(message)` names the first difference.
///
/// # Errors
/// A human-readable description of the first mismatch.
pub fn verify_typed(bytes: &[u8]) -> Result<(), String> {
    let v = parse(bytes).map_err(|e| format!("golden: parse: {e}"))?;
    let id = v.header.id.clone();
    let mut state = State::empty();
    for (i, s) in v.steps.iter().enumerate() {
        let (output, next) = decide(
            &state,
            &s.fix,
            &v.signposts,
            &v.policy,
            &v.consent,
            &s.clock,
            v.seed,
        )
        .map_err(|e| mismatch(&id, i, &format!("decide failed: {e}")))?;
        compare(&id, i, s, &output, &next)?;
        state = next;
    }
    check_trace(&v)
}

fn compare(id: &str, i: usize, s: &Step, output: &Output, next: &State) -> Result<(), String> {
    let got = to_canonical_json(output).map_err(|e| mismatch(id, i, &format!("render: {e}")))?;
    let want = to_canonical_json(&s.expect.output)
        .map_err(|e| mismatch(id, i, &format!("render: {e}")))?;
    if got != want {
        return Err(mismatch(
            id,
            i,
            &format!("output bytes differ\n  got:  {got}\n  want: {want}"),
        ));
    }
    let h = canon_state_hash(next).map_err(|e| mismatch(id, i, &format!("render: {e}")))?;
    if h != s.expect.state_sha256 {
        return Err(mismatch(
            id,
            i,
            &format!(
                "state sha256 differs: got {h}, want {}",
                s.expect.state_sha256
            ),
        ));
    }
    Ok(())
}

fn check_trace(v: &Vector) -> Result<(), String> {
    let t = trace_hash(&v.steps).map_err(|e| format!("golden: trace: {e}"))?;
    if t != v.trace_sha256 {
        return Err(format!(
            "golden {}: trace_sha256 differs: got {t}, want {}",
            v.header.id, v.trace_sha256
        ));
    }
    Ok(())
}

/// Signature of a binding's `decide_json`.
pub type DecideJson<'a> = &'a dyn Fn(&str, &str, &str, &str, &str, &str, u64) -> String;

/// Verify a vector through a `decide_json` entry point (a binding): every argument travels as
/// canonical JSON text and the state is threaded as text, exactly as a port would do.
///
/// # Errors
/// A human-readable description of the first mismatch.
pub fn verify_json(bytes: &[u8], decide_json: DecideJson<'_>) -> Result<(), String> {
    let v = parse(bytes).map_err(|e| format!("golden: parse: {e}"))?;
    let id = v.header.id.clone();
    let render_err = |e: EngineError| format!("golden {id}: render: {e}");
    let policy = to_canonical_json(&v.policy).map_err(render_err)?;
    let consent = to_canonical_json(&v.consent).map_err(render_err)?;
    let signposts = to_canonical_json(&v.signposts).map_err(render_err)?;
    let mut state = to_canonical_json(&State::empty()).map_err(render_err)?;
    for (i, s) in v.steps.iter().enumerate() {
        let fix = to_canonical_json(&s.fix).map_err(render_err)?;
        let clock = to_canonical_json(&s.clock).map_err(render_err)?;
        let resp = decide_json(&state, &fix, &signposts, &policy, &consent, &clock, v.seed);
        let (out, next) = split_response(&resp)
            .map_err(|_| mismatch(&id, i, &format!("not a success response: {resp}")))?;
        let want = to_canonical_json(&s.expect.output).map_err(render_err)?;
        if out != want {
            return Err(mismatch(
                &id,
                i,
                &format!("output bytes differ\n  got:  {out}\n  want: {want}"),
            ));
        }
        let h = sha256_hex(next.as_bytes());
        if h != s.expect.state_sha256 {
            return Err(mismatch(
                &id,
                i,
                &format!(
                    "state sha256 differs: got {h}, want {}",
                    s.expect.state_sha256
                ),
            ));
        }
        state = next;
    }
    check_trace(&v)
}
