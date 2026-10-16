//! Independent canonical-JSON checker and writer (engine-spec 3.2).
//!
//! Canonical form: UTF-8, no whitespace, object keys strictly ascending by UTF-8 bytes (so no
//! duplicates), integers only (no `+`, leading zero, fraction, exponent or `-0`), no `null`, escapes
//! only for `"` and `\` and controls below 0x20 as lowercase `\u00xx`.

use serde_json::Value;

/// Check that `text` is exactly canonical JSON. Returns the reason otherwise.
pub fn check_canonical(text: &str) -> Result<(), String> {
    let mut p = P {
        b: text.as_bytes(),
        i: 0,
    };
    p.value(0)?;
    if p.i != p.b.len() {
        return Err(format!("trailing bytes at {}", p.i));
    }
    Ok(())
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn value(&mut self, depth: usize) -> Result<(), String> {
        if depth > 256 {
            return Err("too deep".into());
        }
        match self.peek() {
            Some(b'{') => {
                self.i += 1;
                if self.peek() == Some(b'}') {
                    self.i += 1;
                    return Ok(());
                }
                let mut prev: Option<Vec<u8>> = None;
                loop {
                    let k = self.string()?;
                    if let Some(p) = &prev
                        && k.as_slice() <= p.as_slice()
                    {
                        return Err(format!("keys not strictly ascending near {}", self.i));
                    }
                    prev = Some(k);
                    if self.peek() != Some(b':') {
                        return Err(format!("expected ':' at {}", self.i));
                    }
                    self.i += 1;
                    self.value(depth + 1)?;
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(());
                        }
                        _ => return Err(format!("expected ',' or '}}' at {}", self.i)),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                if self.peek() == Some(b']') {
                    self.i += 1;
                    return Ok(());
                }
                loop {
                    self.value(depth + 1)?;
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(());
                        }
                        _ => return Err(format!("expected ',' or ']' at {}", self.i)),
                    }
                }
            }
            Some(b'"') => self.string().map(|_| ()),
            Some(b't') => self.lit(b"true"),
            Some(b'f') => self.lit(b"false"),
            Some(b'-' | b'0'..=b'9') => self.number(),
            other => Err(format!("unexpected {other:?} at {}", self.i)),
        }
    }

    fn lit(&mut self, s: &[u8]) -> Result<(), String> {
        if self.b.get(self.i..self.i + s.len()) == Some(s) {
            self.i += s.len();
            Ok(())
        } else {
            Err(format!("bad literal at {}", self.i))
        }
    }

    fn number(&mut self) -> Result<(), String> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.i += 1;
                if self.b[start..self.i] == *b"-0" {
                    return Err("negative zero".into());
                }
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.i += 1;
                }
            }
            _ => return Err(format!("bad number at {start}")),
        }
        if matches!(self.peek(), Some(b'0'..=b'9' | b'.' | b'e' | b'E' | b'+')) {
            return Err(format!("non-integer or leading zero number at {start}"));
        }
        Ok(())
    }

    /// Returns the decoded key/string bytes.
    fn string(&mut self) -> Result<Vec<u8>, String> {
        if self.peek() != Some(b'"') {
            return Err(format!("expected string at {}", self.i));
        }
        self.i += 1;
        let mut out = Vec::new();
        loop {
            let Some(c) = self.peek() else {
                return Err("unterminated string".into());
            };
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(e) = self.peek() else {
                        return Err("dangling escape".into());
                    };
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'u' => {
                            let h = self.b.get(self.i..self.i + 4).ok_or("short \\u")?;
                            if &h[..2] != b"00" {
                                return Err("\\u escape outside the control range".into());
                            }
                            let hex = |d: u8| match d {
                                b'0'..=b'9' => Ok(d - b'0'),
                                b'a'..=b'f' => Ok(d - b'a' + 10),
                                _ => Err("\\u escape must be lowercase hex".to_string()),
                            };
                            let v = hex(h[2])? * 16 + hex(h[3])?;
                            if v >= 0x20 {
                                return Err(format!("\\u00{v:02x} must not be escaped"));
                            }
                            self.i += 4;
                            out.push(v);
                        }
                        other => return Err(format!("forbidden escape \\{}", other as char)),
                    }
                }
                0..=0x1f => return Err("raw control character in string".into()),
                _ => out.push(c),
            }
        }
    }
}

/// Canonical writer for a `serde_json::Value` (keys sorted explicitly by bytes). Panics on a float
/// or `null`: test inputs must be valid by construction.
pub fn render(v: &Value) -> String {
    let mut s = String::new();
    write(v, &mut s);
    s
}

fn write(v: &Value, out: &mut String) {
    match v {
        Value::Null => panic!("null is not canonical"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            assert!(n.is_i64() || n.is_u64(), "float in canonical JSON: {n}");
            out.push_str(&n.to_string());
        }
        Value::String(s) => write_str(s, out),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(x, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_str(k, out);
                out.push(':');
                write(&m[*k], out);
            }
            out.push('}');
        }
    }
}

fn write_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical() {
        for ok in [
            "{}",
            "[]",
            r#"{"a":1,"b":[true,false,-5,0],"c":"x\"y\\z\u000a"}"#,
            "9223372036854775807",
            "-9223372036854775808",
            "\"é€😀\"",
        ] {
            check_canonical(ok).unwrap_or_else(|e| panic!("{ok}: {e}"));
        }
    }

    #[test]
    fn rejects_every_non_canonical_form() {
        for bad in [
            " {}",
            "{} ",
            "{\"a\": 1}",
            "{\"a\" :1}",
            "{\"b\":1,\"a\":2}",
            "{\"a\":1,\"a\":2}",
            "+1",
            "01",
            "-0",
            "1.0",
            "1e3",
            "1E3",
            "null",
            "[null]",
            "\"\\/\"",
            "\"\\n\"",
            "\"\\u0041\"",
            "\"\\u001F\"",
            "\"\\u0022\"",
            "\"a\nb\"",
            "\"\\ud83d\\ude00\"",
            "[1,]",
            "{\"a\":1,}",
            "",
            "[",
            "\"abc",
        ] {
            assert!(check_canonical(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn writer_output_is_canonical_and_sorts_by_bytes() {
        let v = serde_json::json!({"b":1,"a":{"z":[1,2],"B":"q\u{1}\"\\é"},"é":true});
        let t = render(&v);
        check_canonical(&t).unwrap();
        assert_eq!(
            t,
            "{\"a\":{\"B\":\"q\\u0001\\\"\\\\é\",\"z\":[1,2]},\"b\":1,\"é\":true}"
        );
    }
}
