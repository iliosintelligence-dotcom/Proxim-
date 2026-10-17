//! proxim-wasm: binding surface for the conformance entry (engine-spec 9).
//!
//! Plain Rust for now: no UniFFI, no wasm-bindgen, no `unsafe` (the workspace forbids it). The
//! function is a thin pass-through to `proxim_decide::json::decide_json`, so every binding runs the
//! same code and CI `golden-vectors` can demand identical bytes. A generated shim will call this
//! function with strings and a `u64` and hand the returned string back unchanged.

/// `decide_json(state, fix, signposts, policy, consent, clock, seed) -> {output, state}`.
///
/// Canonical JSON in (whitespace is tolerated), canonical JSON out. It never panics. On any bad
/// input it returns a typed error object, `{"error":{"code":"E_PARSE","input":"fix"}}` (the
/// `input` member names the argument for parse and limit failures only); a shell MUST treat an
/// error object as "no decision, state unchanged". `signposts` is a JSON array; `seed` is carried
/// and unused in M1.
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
    proxim_decide::json::decide_json(state, fix, signposts, policy, consent, clock, seed)
}

/// Engine specification version this build implements (golden header `engine_spec`).
#[must_use]
pub const fn engine_spec() -> u32 {
    proxim_types::ENGINE_SPEC
}

#[cfg(test)]
mod tests {
    use super::*;
    use proxim_decide::golden::verify_json;

    /// The `core/` directory. `PROXIM_CORE_DIR` overrides the build machine's path
    /// when the tests run on a device (Android emulator) with the tree copied over.
    fn core_dir() -> std::path::PathBuf {
        std::env::var_os("PROXIM_CORE_DIR").map_or_else(
            || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            std::path::PathBuf::from,
        )
    }

    fn vectors() -> Vec<(String, Vec<u8>)> {
        let dir = core_dir().join("testdata/golden/decide");
        let mut out: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .map(|p| {
                (
                    p.file_stem().unwrap().to_string_lossy().into_owned(),
                    std::fs::read(&p).unwrap(),
                )
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn golden_vectors_are_byte_identical_through_this_binding() {
        let all = vectors();
        assert!(all.len() >= 11, "the vectors must be present");
        for (id, bytes) in all {
            verify_json(&bytes, &decide_json).unwrap_or_else(|e| panic!("{id}: {e}"));
        }
    }

    #[test]
    fn golden_verifier_rejects_a_changed_vector_through_this_binding() {
        let (_, bytes) = vectors()
            .into_iter()
            .find(|(id, _)| id == "drive_in_enter")
            .unwrap();
        let text = String::from_utf8(bytes)
            .unwrap()
            .replacen("\"NO_EVENT\"", "\"NOT_CLOSING\"", 1);
        assert!(verify_json(text.as_bytes(), &decide_json).is_err());
    }

    #[test]
    fn bad_input_is_a_typed_error_object_not_a_panic() {
        assert_eq!(
            decide_json("", "", "", "", "", "", 0),
            r#"{"error":{"code":"E_PARSE","input":"state"}}"#
        );
        assert_eq!(
            decide_json("{}", "{}", "[]", "{}", "{}", "{}", 7),
            r#"{"error":{"code":"E_PARSE","input":"state"}}"#
        );
    }

    /// Deterministic junk (no randomness crate): every slot, many shapes, never a panic and
    /// always an error object.
    #[test]
    fn arbitrary_arguments_never_panic_and_always_answer() {
        let pieces = [
            "",
            "{",
            "}",
            "[",
            "]",
            "null",
            "0",
            "-0",
            "1.5",
            "\"x\"",
            "{\"a\":1}",
            "[1,2]",
            "\u{0}",
            "\u{feff}",
            "true",
            "{\"v\":1}",
            "9999999999999999999999",
        ];
        for a in pieces {
            for b in pieces {
                let r = decide_json(a, b, a, b, a, b, u64::from(a.len() as u32));
                assert!(
                    r.starts_with("{\"error\":{\"code\":\"E_"),
                    "{a:?} {b:?}: {r}"
                );
            }
        }
    }

    #[test]
    fn spec_version_is_one() {
        assert_eq!(engine_spec(), 2);
    }
}
