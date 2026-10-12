//! Canonical JSON writer (spec 3.2).
//!
//! Output rules, all enforced here and tested:
//! - UTF-8, no whitespace;
//! - object keys ascending by UTF-8 bytes (sorted explicitly, so the result does not depend on
//!   whether `serde_json` was built with `preserve_order`);
//! - integers only: a float anywhere is an error, and so is `null` (absent = omitted);
//! - strings escape only `"` and `\` (as `\"` and `\\`) and the controls below 0x20 (as lowercase
//!   `\u00xx`, including `\n` and `\t`); everything else, `/` and DEL included, is written raw.
//!
//! The writer works on a `serde_json::Value` tree. Because the same writer renders a tree that the
//! strict parser produced, `canonical(parse(x))` is a pure function of the document.

use crate::defaults::MAX_JSON_DEPTH;
use crate::error::EngineError;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;
use serde::Serialize;
use serde_json::{Number, Value};

/// Serialize to canonical JSON.
///
/// # Errors
/// [`EngineError::Parse`] if the value serializes to a float, a `null`, or something `serde_json`
/// cannot represent (for example a `u128`); [`EngineError::Limit`] if nesting exceeds
/// [`MAX_JSON_DEPTH`].
pub fn to_canonical_json<T: Serialize>(value: &T) -> Result<String, EngineError> {
    let tree = serde_json::to_value(value).map_err(|_| EngineError::Parse)?;
    value_to_canonical(&tree)
}

/// Render an already built tree as canonical JSON.
///
/// # Errors
/// As [`to_canonical_json`].
pub fn value_to_canonical(value: &Value) -> Result<String, EngineError> {
    let mut out = String::new();
    write_value(&mut out, value, 0)?;
    Ok(out)
}

fn write_value(out: &mut String, value: &Value, depth: usize) -> Result<(), EngineError> {
    match value {
        // spec 3.2: no null anywhere; an absent optional is omitted by the type, never nulled.
        Value::Null => return Err(EngineError::Parse),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => write_number(out, n)?,
        Value::String(s) => write_string(out, s),
        Value::Array(items) => {
            let inner = depth.checked_add(1).ok_or(EngineError::Limit)?;
            if inner > MAX_JSON_DEPTH {
                return Err(EngineError::Limit);
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(out, item, inner)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            let inner = depth.checked_add(1).ok_or(EngineError::Limit)?;
            if inner > MAX_JSON_DEPTH {
                return Err(EngineError::Limit);
            }
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            out.push('{');
            for (i, (key, item)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(out, key);
                out.push(':');
                write_value(out, item, inner)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

fn write_number(out: &mut String, n: &Number) -> Result<(), EngineError> {
    // `as_i64` / `as_u64` are `Some` only for integers: floats (even 1.0) fall through to the error.
    if let Some(v) = n.as_i64() {
        write!(out, "{v}").map_err(|_| EngineError::Limit)
    } else if let Some(v) = n.as_u64() {
        write!(out, "{v}").map_err(|_| EngineError::Limit)
    } else {
        Err(EngineError::Parse)
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                // Lowercase hex, four digits, for every control (\n and \t included).
                let b = u32::from(c);
                out.push_str("\\u00");
                out.push(hex_digit(b >> 4));
                out.push(hex_digit(b & 0xf));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn hex_digit(nibble: u32) -> char {
    let idx = usize::try_from(nibble & 0xf).unwrap_or(0);
    HEX.get(idx).map_or('0', |b| char::from(*b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use alloc::string::ToString;
    use alloc::vec;
    use serde_json::json;

    fn canon(v: &Value) -> String {
        value_to_canonical(v).unwrap()
    }

    #[test]
    fn scalars_and_containers() {
        assert_eq!(canon(&json!(true)), "true");
        assert_eq!(canon(&json!(false)), "false");
        assert_eq!(canon(&json!(0)), "0");
        assert_eq!(canon(&json!(-17)), "-17");
        assert_eq!(canon(&json!([])), "[]");
        assert_eq!(canon(&json!({})), "{}");
        assert_eq!(canon(&json!([1, [2, []], {}])), "[1,[2,[]],{}]");
    }

    #[test]
    fn integer_extremes_print_exactly() {
        assert_eq!(canon(&json!(i64::MIN)), "-9223372036854775808");
        assert_eq!(canon(&json!(i64::MAX)), "9223372036854775807");
        assert_eq!(canon(&json!(u64::MAX)), "18446744073709551615");
    }

    #[test]
    fn keys_ascending_by_utf8_bytes_at_every_level() {
        let mut inner = BTreeMap::new();
        inner.insert("b", 1_i64);
        inner.insert("a", 2);
        let mut outer = BTreeMap::new();
        outer.insert("z", 0_i64);
        outer.insert("é", 0); // 0xC3 0xA9 sorts after ASCII
        outer.insert("B", 0); // 0x42 sorts before 'a'
        outer.insert("aa", 0);
        outer.insert("a", 0);
        assert_eq!(
            to_canonical_json(&outer).unwrap(),
            r#"{"B":0,"a":0,"aa":0,"z":0,"é":0}"#
        );
        assert_eq!(to_canonical_json(&inner).unwrap(), r#"{"a":2,"b":1}"#);
        // Nested objects built in reverse order come out sorted too.
        let v = json!({"y": {"q": 1, "p": 2}, "x": [{"d": 1, "c": 2}]});
        assert_eq!(canon(&v), r#"{"x":[{"c":2,"d":1}],"y":{"p":2,"q":1}}"#);
    }

    #[test]
    fn writer_sorts_even_if_the_map_iterates_unsorted() {
        // Guards against `preserve_order` ever being enabled anywhere in the dependency graph:
        // that would make `Map` iterate in insertion order, and only the explicit sort saves us.
        let mut m = serde_json::Map::new();
        m.insert("b".to_string(), json!(1));
        m.insert("a".to_string(), json!(2));
        assert_eq!(canon(&Value::Object(m)), r#"{"a":2,"b":1}"#);
    }

    #[test]
    fn every_control_character_is_lowercase_u00xx() {
        for b in 0u32..0x20 {
            let c = char::from_u32(b).unwrap();
            let s = c.to_string();
            let got = canon(&Value::String(s));
            let want = alloc::format!("\"\\u00{b:02x}\"");
            assert_eq!(got, want, "control {b:#x}");
        }
        // Spot checks that nobody uses the short forms.
        assert_eq!(canon(&json!("\n")), "\"\\u000a\"");
        assert_eq!(canon(&json!("\t")), "\"\\u0009\"");
        assert_eq!(canon(&json!("\u{1f}")), "\"\\u001f\"");
        assert_eq!(canon(&json!("\u{0}")), "\"\\u0000\"");
    }

    #[test]
    fn only_quote_backslash_and_controls_are_escaped() {
        assert_eq!(canon(&json!("a\"b")), r#""a\"b""#);
        assert_eq!(canon(&json!("a\\b")), r#""a\\b""#);
        assert_eq!(canon(&json!("a/b")), r#""a/b""#);
        assert_eq!(canon(&json!("\u{7f}")), "\"\u{7f}\"");
        assert_eq!(canon(&json!("é€😀\u{2028}")), "\"é€😀\u{2028}\"");
        // Keys use the same escaping.
        assert_eq!(canon(&json!({"a\nb": 1})), "{\"a\\u000ab\":1}");
    }

    #[test]
    fn null_is_rejected_not_written() {
        assert_eq!(value_to_canonical(&Value::Null), Err(EngineError::Parse));
        assert_eq!(
            value_to_canonical(&json!({"a": null})),
            Err(EngineError::Parse)
        );
        assert_eq!(
            to_canonical_json(&Option::<u8>::None),
            Err(EngineError::Parse)
        );
        assert_eq!(
            to_canonical_json(&vec![Some(1_u8), None]),
            Err(EngineError::Parse)
        );
    }

    #[test]
    fn unrepresentable_numbers_are_a_typed_error() {
        assert_eq!(to_canonical_json(&u128::MAX), Err(EngineError::Parse));
    }

    #[test]
    fn nesting_is_bounded() {
        let mut v = json!(1);
        for _ in 0..MAX_JSON_DEPTH {
            v = Value::Array(vec![v]);
        }
        assert!(value_to_canonical(&v).is_ok(), "exactly the limit is fine");
        v = Value::Array(vec![v]);
        assert_eq!(value_to_canonical(&v), Err(EngineError::Limit));
    }

    #[test]
    fn skip_serializing_none_is_omitted_not_nulled() {
        let fix = crate::Fix {
            t_ms: 5,
            lat_e7: -3,
            lon_e7: 4,
            acc_mm: 7,
            speed_mmps: None,
            speed_acc_mmps: None,
            course_ddeg: Some(0),
            course_acc_ddeg: None,
            is_mock: false,
        };
        assert_eq!(
            to_canonical_json(&fix).unwrap(),
            r#"{"acc_mm":7,"course_ddeg":0,"lat_e7":-3,"lon_e7":4,"t_ms":5}"#
        );
    }
}
