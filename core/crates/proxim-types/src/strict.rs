//! Strict JSON parser for untrusted input (spec 3.2). Own implementation, because the stock
//! parser accepts things the contract forbids (duplicate keys, floats, `null`, `-0`) and has no
//! size bounds.
//!
//! Accepted: standard JSON restricted to integers. Insignificant whitespace (space, tab, LF, CR)
//! is tolerated by [`from_json`]; [`from_canonical_json`] additionally demands the exact canonical
//! bytes. Rejected with [`EngineError::Parse`]: `null`, any number with a fraction or exponent,
//! `-0`, leading zeros, `+`, integers outside `i64`/`u64`, duplicate object keys at any level,
//! invalid UTF-8, a UTF-8 BOM, raw control characters or bad escapes inside strings, lone
//! surrogate escapes, trailing bytes, truncated input. Rejected with [`EngineError::Limit`]:
//! input, nesting, string, array or object sizes beyond [`JsonLimits`].
//!
//! Typed decoding goes through `serde_json::from_value` so the types' `deny_unknown_fields`
//! applies: an unknown field is [`EngineError::Parse`] (reserved fields excepted, spec 10).

use crate::canon::value_to_canonical;
use crate::defaults::{
    MAX_JSON_ARRAY_LEN, MAX_JSON_BYTES, MAX_JSON_DEPTH, MAX_JSON_OBJECT_KEYS, MAX_JSON_STRING_BYTES,
};
use crate::error::EngineError;
use alloc::string::String;
use alloc::vec::Vec;
use serde::de::DeserializeOwned;
use serde_json::{Map, Number, Value};

/// Size bounds for one parse. `Default` is the engine's production bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsonLimits {
    pub max_bytes: usize,
    pub max_depth: usize,
    pub max_string_bytes: usize,
    pub max_array_len: usize,
    pub max_object_keys: usize,
}

impl Default for JsonLimits {
    fn default() -> Self {
        Self {
            max_bytes: MAX_JSON_BYTES,
            max_depth: MAX_JSON_DEPTH,
            max_string_bytes: MAX_JSON_STRING_BYTES,
            max_array_len: MAX_JSON_ARRAY_LEN,
            max_object_keys: MAX_JSON_OBJECT_KEYS,
        }
    }
}

/// Parse and decode with the default limits.
///
/// # Errors
/// [`EngineError::Parse`] for malformed or non-conforming input (including unknown fields),
/// [`EngineError::Limit`] for oversize input.
pub fn from_json<T: DeserializeOwned + serde::Serialize>(input: &[u8]) -> Result<T, EngineError> {
    from_json_with(input, &JsonLimits::default())
}

/// As [`from_json`] with explicit limits.
///
/// # Errors
/// As [`from_json`].
pub fn from_json_with<T: DeserializeOwned + serde::Serialize>(
    input: &[u8],
    limits: &JsonLimits,
) -> Result<T, EngineError> {
    let tree = parse_value(input, limits)?;
    decode_tree(tree)
}

/// As [`from_json`], and the input must already be byte-for-byte canonical (no whitespace, sorted
/// keys, canonical escapes). Used for golden files and anything that is re-hashed.
///
/// # Errors
/// [`EngineError::Parse`] if the input is valid but not canonical, otherwise as [`from_json`].
pub fn from_canonical_json<T: DeserializeOwned + serde::Serialize>(
    input: &[u8],
) -> Result<T, EngineError> {
    from_canonical_json_with(input, &JsonLimits::default())
}

/// As [`from_canonical_json`] with explicit limits (golden files with large tables need a larger
/// array bound than the production default).
///
/// # Errors
/// As [`from_canonical_json`].
pub fn from_canonical_json_with<T: DeserializeOwned + serde::Serialize>(
    input: &[u8],
    limits: &JsonLimits,
) -> Result<T, EngineError> {
    let tree = parse_value(input, limits)?;
    if value_to_canonical(&tree)?.as_bytes() != input {
        return Err(EngineError::Parse);
    }
    decode_tree(tree)
}

/// Decode a strict tree into `T`, then reject any positional encoding (errata E-D-POS).
///
/// Derived struct deserializers accept a JSON array as a positional struct, a spelling no other
/// binding accepts and that the canonical form never produces. After decoding, `T` is re-encoded:
/// wherever the input held an array, the re-encoding must hold an array too (or nothing, for
/// reserved fields that are never re-emitted). This covers every depth, including fields inside
/// internally tagged enums, which a wrapping deserializer would not reach.
///
/// # Errors
/// [`EngineError::Parse`] if decoding fails or a struct was given as an array.
pub fn decode_tree<T: DeserializeOwned + serde::Serialize>(tree: Value) -> Result<T, EngineError> {
    let out: T = serde_json::from_value(tree.clone()).map_err(|_| EngineError::Parse)?;
    let back = serde_json::to_value(&out).map_err(|_| EngineError::Parse)?;
    if arrays_preserved(&tree, &back) {
        Ok(out)
    } else {
        Err(EngineError::Parse)
    }
}

/// False if some array in `input` became an object at the same place in `back`.
fn arrays_preserved(input: &Value, back: &Value) -> bool {
    match (input, back) {
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| arrays_preserved(x, y))
        }
        // A struct given positionally decodes and re-encodes as an object: refuse it. An array that
        // a lenient custom type coerces to a scalar (e.g. a consent value parsed as `unknown`, spec
        // 3.2) is not a positional struct and stays allowed.
        (Value::Array(_), Value::Object(_)) => false,
        (Value::Array(_), _) => true,
        (Value::Object(a), Value::Object(b)) => a
            .iter()
            .all(|(k, x)| b.get(k).is_none_or(|y| arrays_preserved(x, y))),
        _ => true,
    }
}

/// Parse into a tree under the strict rules, without typed decoding.
///
/// # Errors
/// As [`from_json`].
pub fn parse_value(input: &[u8], limits: &JsonLimits) -> Result<Value, EngineError> {
    if input.len() > limits.max_bytes {
        return Err(EngineError::Limit);
    }
    // Whole-input UTF-8 validation: afterwards every byte sequence we copy is valid UTF-8.
    core::str::from_utf8(input).map_err(|_| EngineError::Parse)?;
    let mut p = Parser {
        b: input,
        pos: 0,
        limits,
    };
    p.skip_ws();
    let v = p.value(0)?;
    p.skip_ws();
    if p.pos != input.len() {
        return Err(EngineError::Parse);
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [u8],
    pos: usize,
    limits: &'a JsonLimits,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos = self.pos.checked_add(1)?;
        Some(c)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos = self.pos.saturating_add(1);
        }
    }

    fn eat(&mut self, c: u8) -> Result<(), EngineError> {
        if self.bump() == Some(c) {
            Ok(())
        } else {
            Err(EngineError::Parse)
        }
    }

    fn literal(&mut self, word: &[u8]) -> Result<(), EngineError> {
        let end = self.pos.checked_add(word.len()).ok_or(EngineError::Parse)?;
        if self.b.get(self.pos..end) == Some(word) {
            self.pos = end;
            Ok(())
        } else {
            Err(EngineError::Parse)
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, EngineError> {
        match self.peek().ok_or(EngineError::Parse)? {
            b'{' => self.object(depth),
            b'[' => self.array(depth),
            b'"' => self.string().map(Value::String),
            b't' => self.literal(b"true").map(|()| Value::Bool(true)),
            b'f' => self.literal(b"false").map(|()| Value::Bool(false)),
            b'-' | b'0'..=b'9' => self.number().map(Value::Number),
            // `null` is rejected on purpose (spec 3.2: absent = omitted), as is everything else.
            _ => Err(EngineError::Parse),
        }
    }

    fn enter(&self, depth: usize) -> Result<usize, EngineError> {
        let inner = depth.checked_add(1).ok_or(EngineError::Limit)?;
        if inner > self.limits.max_depth {
            Err(EngineError::Limit)
        } else {
            Ok(inner)
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, EngineError> {
        let inner = self.enter(depth)?;
        self.eat(b'[')?;
        let mut items: Vec<Value> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos = self.pos.saturating_add(1);
            return Ok(Value::Array(items));
        }
        loop {
            if items.len() >= self.limits.max_array_len {
                return Err(EngineError::Limit);
            }
            self.skip_ws();
            items.push(self.value(inner)?);
            self.skip_ws();
            match self.bump() {
                Some(b',') => {}
                Some(b']') => return Ok(Value::Array(items)),
                _ => return Err(EngineError::Parse),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, EngineError> {
        let inner = self.enter(depth)?;
        self.eat(b'{')?;
        let mut map: Map<String, Value> = Map::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos = self.pos.saturating_add(1);
            return Ok(Value::Object(map));
        }
        loop {
            if map.len() >= self.limits.max_object_keys {
                return Err(EngineError::Limit);
            }
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(EngineError::Parse);
            }
            let key = self.string()?;
            self.skip_ws();
            self.eat(b':')?;
            self.skip_ws();
            let v = self.value(inner)?;
            // Duplicate keys are ambiguous (which one wins?) and a known smuggling vector.
            if map.insert(key, v).is_some() {
                return Err(EngineError::Parse);
            }
            self.skip_ws();
            match self.bump() {
                Some(b',') => {}
                Some(b'}') => return Ok(Value::Object(map)),
                _ => return Err(EngineError::Parse),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, EngineError> {
        let mut v: u32 = 0;
        for _ in 0..4 {
            let d = match self.bump().ok_or(EngineError::Parse)? {
                c @ b'0'..=b'9' => c - b'0',
                c @ b'a'..=b'f' => c - b'a' + 10,
                c @ b'A'..=b'F' => c - b'A' + 10,
                _ => return Err(EngineError::Parse),
            };
            v = v * 16 + u32::from(d);
        }
        Ok(v)
    }

    fn string(&mut self) -> Result<String, EngineError> {
        self.eat(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            if out.len() > self.limits.max_string_bytes {
                return Err(EngineError::Limit);
            }
            let c = self.bump().ok_or(EngineError::Parse)?;
            match c {
                b'"' => break,
                0..=0x1f => return Err(EngineError::Parse),
                b'\\' => {
                    let e = self.bump().ok_or(EngineError::Parse)?;
                    let ch = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                // High surrogate: a low surrogate escape must follow.
                                self.eat(b'\\')?;
                                self.eat(b'u')?;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err(EngineError::Parse);
                                }
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else if (0xDC00..0xE000).contains(&hi) {
                                return Err(EngineError::Parse); // lone low surrogate
                            } else {
                                hi
                            };
                            char::from_u32(cp).ok_or(EngineError::Parse)?
                        }
                        _ => return Err(EngineError::Parse),
                    };
                    let mut buf = [0_u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                other => out.push(other),
            }
        }
        if out.len() > self.limits.max_string_bytes {
            return Err(EngineError::Limit);
        }
        String::from_utf8(out).map_err(|_| EngineError::Parse)
    }

    fn number(&mut self) -> Result<Number, EngineError> {
        let negative = self.peek() == Some(b'-');
        if negative {
            self.pos = self.pos.saturating_add(1);
        }
        let first = self.bump().ok_or(EngineError::Parse)?;
        if !first.is_ascii_digit() {
            return Err(EngineError::Parse);
        }
        let mut mag: u64 = u64::from(first - b'0');
        if first == b'0' {
            // No leading zeros: a zero must stand alone.
            if matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(EngineError::Parse);
            }
        } else {
            while let Some(d @ b'0'..=b'9') = self.peek() {
                self.pos = self.pos.saturating_add(1);
                mag = mag
                    .checked_mul(10)
                    .and_then(|m| m.checked_add(u64::from(d - b'0')))
                    .ok_or(EngineError::Parse)?;
            }
        }
        // Integers only: a fraction or exponent is a float.
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(EngineError::Parse);
        }
        if !negative {
            return Ok(Number::from(mag));
        }
        if mag == 0 {
            return Err(EngineError::Parse); // `-0`
        }
        let neg = i128::from(mag).checked_neg().ok_or(EngineError::Parse)?;
        let v = i64::try_from(neg).map_err(|_| EngineError::Parse)?;
        Ok(Number::from(v))
    }
}
