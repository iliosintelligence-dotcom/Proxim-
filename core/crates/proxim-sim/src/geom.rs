//! Local-frame geometry for the simulator: scenario coordinates are integer millimetres east and
//! north of a scenario origin, converted to and from e7 degrees with the engine's own frame
//! (`proxim_geo::frame_for`, spec 1.2). Integer only.

use alloc::string::{String, ToString};
use core::fmt;
use proxim_geo::{Frame, frame_for, unit_from_course};
use proxim_types::{EngineError, Enu, Position};

/// A simulator error: bad scenario file, unreadable trace, or an engine error. Never a panic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimError(pub String);

impl SimError {
    #[must_use]
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<EngineError> for SimError {
    fn from(e: EngineError) -> Self {
        Self(alloc::format!("engine error: {e:?}"))
    }
}

impl From<serde_json::Error> for SimError {
    fn from(e: serde_json::Error) -> Self {
        Self(alloc::format!("json: {e}"))
    }
}

pub type SimResult<T> = Result<T, SimError>;

/// Insert `key` into a JSON object (no-op on a non-object; every caller passes an object).
pub fn put(v: &mut serde_json::Value, key: &str, x: serde_json::Value) {
    if let Some(m) = v.as_object_mut() {
        m.insert(key.to_string(), x);
    }
}

/// The scenario's local plane.
#[derive(Clone, Debug)]
pub struct Local {
    pub origin: Position,
    pub frame: Frame,
}

impl Local {
    /// # Errors
    /// The origin is invalid or above 80 degrees.
    pub fn new(origin: Position) -> SimResult<Self> {
        Ok(Self {
            origin,
            frame: frame_for(origin)?,
        })
    }

    /// e7 position of a point `e_mm` east and `n_mm` north of the origin.
    ///
    /// # Errors
    /// Outside the 150 km frame.
    pub fn pos(&self, e_mm: i64, n_mm: i64) -> SimResult<Position> {
        let e = i32::try_from(e_mm).map_err(|_| SimError::new("east offset out of range"))?;
        let n = i32::try_from(n_mm).map_err(|_| SimError::new("north offset out of range"))?;
        Ok(self.frame.unproject(Enu { e_mm: e, n_mm: n })?)
    }

    /// Local mm of an e7 position.
    ///
    /// # Errors
    /// Outside the 150 km frame.
    pub fn enu(&self, p: Position) -> SimResult<Enu> {
        Ok(self.frame.project(p)?)
    }
}

/// Course in 0.1 degree (clockwise from north) of the direction `(de, dn)`: the table entry of
/// `proxim_geo::trig::SINCOS` with the largest dot product, so it is exact to the table and needs
/// no `atan2`. `None` for a zero vector.
///
/// # Errors
/// Only if the engine's table lookup fails (it cannot for 0..3600).
pub fn course_ddeg(de: i64, dn: i64) -> SimResult<Option<u16>> {
    if de == 0 && dn == 0 {
        return Ok(None);
    }
    let mut best: Option<(i128, u16)> = None;
    for c in 0..3600_u16 {
        let u = unit_from_course(c)?;
        let [ue, un] = u;
        let dot = i128::from(de) * i128::from(ue) + i128::from(dn) * i128::from(un);
        if best.is_none_or(|(b, _)| dot > b) {
            best = Some((dot, c));
        }
    }
    Ok(best.map(|(_, c)| c))
}

/// Point `dist_mm` from `(e, n)` in the direction of `course_ddeg`, rounded to mm.
///
/// # Errors
/// `course_ddeg >= 3600`.
pub fn step_along_course(e: i64, n: i64, course: u16, dist_mm: i64) -> SimResult<(i64, i64)> {
    let [ue, un] = unit_from_course(course)?;
    let de = (i128::from(dist_mm) * i128::from(ue) + (1 << 29)) >> 30;
    let dn = (i128::from(dist_mm) * i128::from(un) + (1 << 29)) >> 30;
    Ok((
        e.saturating_add(i64::try_from(de).unwrap_or(0)),
        n.saturating_add(i64::try_from(dn).unwrap_or(0)),
    ))
}

/// Parse a decimal literal (as written in GeoJSON/GPX, or by `serde_json` for a float, which may
/// use an exponent such as `1e-7`) into units of 1e-7, rounding half away from zero. Integer
/// arithmetic on the text only: no float is ever formed from it.
///
/// # Errors
/// Not a decimal literal, more than 30 significant digits, or out of `i32` range.
pub fn parse_decimal_e7(s: &str) -> SimResult<i32> {
    let s = s.trim();
    let bad = || SimError::new(alloc::format!("not a decimal degree value: {s:?}"));
    let (neg, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (mant, exp) = match body.find(['e', 'E']) {
        Some(i) => {
            let (m, e) = body.split_at(i);
            let e = e.get(1..).ok_or_else(bad)?;
            let e: i64 = e.parse().map_err(|_| bad())?;
            if e.abs() > 40 {
                return Err(bad());
            }
            (m, e)
        }
        None => (body, 0),
    };
    let (int_part, frac_part) = match mant.split_once('.') {
        Some((a, b)) => (a, b),
        None => (mant, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return Err(bad());
    }
    if !int_part.bytes().all(|b| b.is_ascii_digit()) || !frac_part.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(bad());
    }
    // value = digits x 10^(exp - frac_len); we want round(value x 1e7).
    let mut digits: u128 = 0;
    let mut sig = 0_u32;
    for b in int_part.bytes().chain(frac_part.bytes()) {
        if digits > 0 || b != b'0' {
            sig += 1;
        }
        if sig > 30 {
            return Err(bad());
        }
        digits = digits * 10 + u128::from(b - b'0');
    }
    let frac_len = i64::try_from(frac_part.len()).map_err(|_| bad())?;
    let k = exp + 7 - frac_len;
    let scaled: u128 = if k >= 0 {
        let mut v = digits;
        for _ in 0..k {
            v = v.checked_mul(10).ok_or_else(bad)?;
        }
        v
    } else {
        let mut den: u128 = 1;
        for _ in 0..(-k).min(38) {
            den = den.saturating_mul(10);
        }
        (digits + den / 2) / den
    };
    let v = i64::try_from(scaled).map_err(|_| bad())?;
    let v = if neg { -v } else { v };
    i32::try_from(v).map_err(|_| bad())
}

/// `"12.3456789"`-style rendering of an e7 value (used in GPX/GeoJSON writers and messages).
#[must_use]
pub fn e7_to_decimal(v: i32) -> String {
    let neg = v < 0;
    let a = v.unsigned_abs();
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    s.push_str(&(a / 10_000_000).to_string());
    s.push('.');
    s.push_str(&alloc::format!("{:07}", a % 10_000_000));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_parsing_rounds_half_away() {
        assert_eq!(parse_decimal_e7("45").unwrap(), 450_000_000);
        assert_eq!(parse_decimal_e7("-7.12345675").unwrap(), -71_234_568);
        assert_eq!(parse_decimal_e7("7.12345674").unwrap(), 71_234_567);
        assert_eq!(parse_decimal_e7("0.0000001").unwrap(), 1);
        assert_eq!(parse_decimal_e7(".5").unwrap(), 5_000_000);
        assert_eq!(parse_decimal_e7("1e-7").unwrap(), 1);
        assert_eq!(parse_decimal_e7("4.5E1").unwrap(), 450_000_000);
        assert!(parse_decimal_e7("1e5").is_err(), "out of i32 range");
        assert!(parse_decimal_e7("1.2.3").is_err());
        assert!(parse_decimal_e7("").is_err());
        assert!(parse_decimal_e7("abc").is_err());
        assert_eq!(e7_to_decimal(-71_234_568), "-7.1234568");
        assert_eq!(e7_to_decimal(5), "0.0000005");
    }

    #[test]
    fn course_of_cardinal_directions() {
        assert_eq!(course_ddeg(0, 10).unwrap(), Some(0));
        assert_eq!(course_ddeg(10, 0).unwrap(), Some(900));
        assert_eq!(course_ddeg(0, -10).unwrap(), Some(1800));
        assert_eq!(course_ddeg(-10, 0).unwrap(), Some(2700));
        assert_eq!(course_ddeg(0, 0).unwrap(), None);
        assert_eq!(step_along_course(0, 0, 900, 1000).unwrap(), (1000, 0));
    }

    #[test]
    fn local_roundtrip_is_within_a_few_mm() {
        let l = Local::new(Position {
            lat_e7: 450_000_000,
            lon_e7: 70_000_000,
        })
        .unwrap();
        let p = l.pos(1_234_567, -2_345_678).unwrap();
        let q = l.enu(p).unwrap();
        assert!((i64::from(q.e_mm) - 1_234_567).abs() <= 10);
        assert!((i64::from(q.n_mm) + 2_345_678).abs() <= 10);
    }
}
