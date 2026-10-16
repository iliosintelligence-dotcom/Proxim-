//! Noise-free truth paths ("where the phone really is") and their importers.
//!
//! A [`Truth`] is a list of [`Piece`]s in the scenario's local plane (mm): constant-speed straight
//! legs, waits (stationary) and gaps (time passes with no fixes, for example the app was not
//! running, optionally moving the phone elsewhere). Sampling a truth at a fixed period gives the
//! [`Sample`]s that [`crate::noise`] turns into fixes and that [`crate::oracle`] judges against.
//!
//! Sources: synthetic legs ([`TruthBuilder`]), a GeoJSON `LineString` ([`geojson_points`]) or a
//! GPX track ([`gpx_points`]); both importers are minimal, integer-only and dependency-free.

use crate::geom::{Local, SimError, SimResult, course_ddeg, parse_decimal_e7};
use alloc::string::String;
use alloc::vec::Vec;
use proxim_geo::isqrt;
use proxim_types::Position;

/// One constant-velocity stretch of the truth, times relative to the scenario start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    pub t0_ms: i64,
    pub t1_ms: i64,
    pub from: (i64, i64),
    pub to: (i64, i64),
    /// Distance travelled before this piece (mm).
    pub along0_mm: i64,
    pub speed_mmps: u32,
    pub course_ddeg: Option<u16>,
    /// `false` for a gap: no fixes are produced while it lasts.
    pub observed: bool,
}

/// Where the phone truly is at a sample time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample {
    pub t_rel_ms: i64,
    pub e_mm: i64,
    pub n_mm: i64,
    pub along_mm: i64,
    pub speed_mmps: u32,
    pub course_ddeg: Option<u16>,
    pub observed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Truth {
    pub pieces: Vec<Piece>,
}

fn len_mm(a: (i64, i64), b: (i64, i64)) -> i64 {
    let de = i128::from(b.0) - i128::from(a.0);
    let dn = i128::from(b.1) - i128::from(a.1);
    let sq = u128::try_from(de * de + dn * dn).unwrap_or(u128::MAX);
    i64::try_from(isqrt(sq)).unwrap_or(i64::MAX)
}

fn lerp(a: i64, b: i64, num: i64, den: i64) -> i64 {
    if den <= 0 {
        return b;
    }
    let v = i128::from(a) + (i128::from(b) - i128::from(a)) * i128::from(num) / i128::from(den);
    i64::try_from(v).unwrap_or(b)
}

/// Builds a [`Truth`] leg by leg.
#[derive(Clone, Debug)]
pub struct TruthBuilder {
    pos: (i64, i64),
    t_ms: i64,
    along_mm: i64,
    pieces: Vec<Piece>,
}

impl TruthBuilder {
    #[must_use]
    pub const fn new(start_mm: (i64, i64)) -> Self {
        Self {
            pos: start_mm,
            t_ms: 0,
            along_mm: 0,
            pieces: Vec::new(),
        }
    }

    /// Drive or walk in a straight line to `to_mm` at `speed_mmps`.
    ///
    /// # Errors
    /// Zero speed.
    pub fn go_to(&mut self, to_mm: (i64, i64), speed_mmps: u32) -> SimResult<&mut Self> {
        if speed_mmps == 0 {
            return Err(SimError::new("a move needs a positive speed"));
        }
        let len = len_mm(self.pos, to_mm);
        if len == 0 {
            return Ok(self);
        }
        let v = i128::from(speed_mmps);
        let dur = (i128::from(len) * 1000 + v - 1) / v;
        let dur = i64::try_from(dur).map_err(|_| SimError::new("move too long"))?;
        let course = course_ddeg(to_mm.0 - self.pos.0, to_mm.1 - self.pos.1)?;
        self.push(to_mm, dur, len, speed_mmps, course, true);
        Ok(self)
    }

    /// Stand still for `ms`.
    pub fn wait(&mut self, ms: i64) -> &mut Self {
        if ms > 0 {
            self.push(self.pos, ms, 0, 0, None, true);
        }
        self
    }

    /// Let `ms` pass with no fixes, then continue from `to_mm` (or from here).
    pub fn gap(&mut self, ms: i64, to_mm: Option<(i64, i64)>) -> &mut Self {
        if ms > 0 {
            let to = to_mm.unwrap_or(self.pos);
            self.push(to, ms, 0, 0, None, false);
        } else if let Some(to) = to_mm {
            self.pos = to;
        }
        self
    }

    fn push(
        &mut self,
        to: (i64, i64),
        dur_ms: i64,
        len: i64,
        speed: u32,
        course: Option<u16>,
        observed: bool,
    ) {
        self.pieces.push(Piece {
            t0_ms: self.t_ms,
            t1_ms: self.t_ms.saturating_add(dur_ms),
            from: self.pos,
            to,
            along0_mm: self.along_mm,
            speed_mmps: speed,
            course_ddeg: course,
            observed,
        });
        self.t_ms = self.t_ms.saturating_add(dur_ms);
        self.along_mm = self.along_mm.saturating_add(len);
        self.pos = to;
    }

    #[must_use]
    pub fn build(&self) -> Truth {
        Truth {
            pieces: self.pieces.clone(),
        }
    }
}

impl Truth {
    #[must_use]
    pub fn duration_ms(&self) -> i64 {
        self.pieces.last().map_or(0, |p| p.t1_ms)
    }

    /// The truth at `t_rel_ms` (clamped to the path). A sample exactly at a piece boundary belongs
    /// to the later piece, so a gap's first instant is unobserved.
    #[must_use]
    pub fn at(&self, t_rel_ms: i64) -> Option<Sample> {
        let first = self.pieces.first()?;
        let t = t_rel_ms.max(first.t0_ms);
        let piece = self
            .pieces
            .iter()
            .find(|p| t >= p.t0_ms && t < p.t1_ms)
            .or_else(|| self.pieces.last())?;
        let dt = (t - piece.t0_ms).min(piece.t1_ms - piece.t0_ms);
        let span = piece.t1_ms - piece.t0_ms;
        let (e, n) = if piece.observed {
            (
                lerp(piece.from.0, piece.to.0, dt, span),
                lerp(piece.from.1, piece.to.1, dt, span),
            )
        } else {
            piece.to
        };
        let moved = len_mm(piece.from, (e, n));
        let along = if piece.observed {
            piece.along0_mm.saturating_add(moved)
        } else {
            piece.along0_mm
        };
        Some(Sample {
            t_rel_ms: t,
            e_mm: e,
            n_mm: n,
            along_mm: along,
            speed_mmps: piece.speed_mmps,
            course_ddeg: piece.course_ddeg,
            observed: piece.observed,
        })
    }

    /// Samples every `period_ms` from 0 to the end of the path inclusive.
    ///
    /// # Errors
    /// Non-positive period, or more than 2,000,000 samples.
    pub fn samples(&self, period_ms: i64) -> SimResult<Vec<Sample>> {
        if period_ms <= 0 {
            return Err(SimError::new("period_ms must be positive"));
        }
        let n = self.duration_ms() / period_ms;
        if n > 2_000_000 {
            return Err(SimError::new("trace too long (over 2,000,000 samples)"));
        }
        let mut out = Vec::new();
        for k in 0..=n {
            if let Some(s) = self.at(k * period_ms) {
                out.push(s);
            }
        }
        Ok(out)
    }
}

/// A truth from imported points: timed (each leg takes the time between its two timestamps) when
/// every point has a time and `speed_mmps` is `None`, else at constant `speed_mmps`.
///
/// # Errors
/// Fewer than two points, untimed points without a speed, non-increasing times, or a point
/// outside the 150 km frame.
pub fn truth_from_points(
    local: &Local,
    pts: &[(Position, Option<i64>)],
    speed_mmps: Option<u32>,
) -> SimResult<Truth> {
    let first = pts
        .first()
        .ok_or_else(|| SimError::new("track has no points"))?;
    if pts.len() < 2 {
        return Err(SimError::new("track needs at least two points"));
    }
    let to_mm = |p: Position| -> SimResult<(i64, i64)> {
        let q = local.enu(p)?;
        Ok((i64::from(q.e_mm), i64::from(q.n_mm)))
    };
    let mut b = TruthBuilder::new(to_mm(first.0)?);
    let timed = speed_mmps.is_none();
    let mut prev_t = first.1;
    for (p, t) in pts.iter().skip(1) {
        let to = to_mm(*p)?;
        if timed {
            let (Some(t0), Some(t1)) = (prev_t, *t) else {
                return Err(SimError::new("track points have no times: give speed_mmps"));
            };
            let dt = t1 - t0;
            if dt <= 0 {
                return Err(SimError::new("track times must increase"));
            }
            let len = len_mm(b.pos, to);
            if len == 0 {
                b.wait(dt);
            } else {
                let v = (i128::from(len) * 1000) / i128::from(dt);
                let v = u32::try_from(v.max(1)).map_err(|_| SimError::new("speed out of range"))?;
                let course = course_ddeg(to.0 - b.pos.0, to.1 - b.pos.1)?;
                b.push(to, dt, len, v, course, true);
            }
            prev_t = *t;
        } else if let Some(v) = speed_mmps {
            b.go_to(to, v)?;
        }
    }
    Ok(b.build())
}

/// The `[lon, lat]` vertices of the first `LineString` in a GeoJSON document (a bare geometry, a
/// `Feature` or a `FeatureCollection`). Coordinates are parsed from their decimal text, never via
/// float arithmetic; a third (altitude) member is ignored.
///
/// # Errors
/// Not JSON, no `LineString`, or malformed coordinates.
pub fn geojson_points(text: &str) -> SimResult<Vec<Position>> {
    let v: serde_json::Value = serde_json::from_str(text)?;
    let line = find_linestring(&v).ok_or_else(|| SimError::new("no LineString in GeoJSON"))?;
    let coords = line
        .get("coordinates")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| SimError::new("LineString without coordinates"))?;
    let mut out = Vec::new();
    for c in coords {
        let arr = c
            .as_array()
            .ok_or_else(|| SimError::new("coordinate is not an array"))?;
        let (Some(lon), Some(lat)) = (arr.first(), arr.get(1)) else {
            return Err(SimError::new("coordinate needs [lon, lat]"));
        };
        let num = |x: &serde_json::Value| -> SimResult<i32> {
            match x {
                serde_json::Value::Number(n) => parse_decimal_e7(&n.to_string()),
                _ => Err(SimError::new("coordinate is not a number")),
            }
        };
        out.push(Position {
            lat_e7: num(lat)?,
            lon_e7: num(lon)?,
        });
    }
    Ok(out)
}

fn find_linestring(v: &serde_json::Value) -> Option<&serde_json::Value> {
    match v.get("type").and_then(serde_json::Value::as_str)? {
        "LineString" => Some(v),
        "Feature" => v.get("geometry").and_then(find_linestring),
        "FeatureCollection" => v
            .get("features")?
            .as_array()?
            .iter()
            .find_map(find_linestring),
        _ => None,
    }
}

/// `<trkpt lat=".." lon="..">` points of a GPX document in order, each with its `<time>` (UTC
/// `YYYY-MM-DDTHH:MM:SS[.fff]Z`) if present. A minimal scanner, not an XML parser: enough for
/// GPX 1.1 tracks as written by common tools; elevation and extensions are ignored.
///
/// # Errors
/// No track points, a point without both coordinates, or an unparseable time.
pub fn gpx_points(text: &str) -> SimResult<Vec<(Position, Option<i64>)>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("<trkpt") {
        let after = rest.get(i..).unwrap_or("");
        let tag_end = after
            .find('>')
            .ok_or_else(|| SimError::new("unterminated <trkpt"))?;
        let tag = after.get(..tag_end).unwrap_or("");
        let lat = attr(tag, "lat").ok_or_else(|| SimError::new("trkpt without lat"))?;
        let lon = attr(tag, "lon").ok_or_else(|| SimError::new("trkpt without lon"))?;
        let self_closing = tag.ends_with('/');
        let body_start = tag_end + 1;
        let (body, next) = if self_closing {
            ("", body_start)
        } else {
            let close = after
                .find("</trkpt>")
                .ok_or_else(|| SimError::new("unterminated trkpt"))?;
            (after.get(body_start..close).unwrap_or(""), close + 8)
        };
        let time = match between(body, "<time>", "</time>") {
            Some(t) => Some(parse_iso8601_ms(t)?),
            None => None,
        };
        out.push((
            Position {
                lat_e7: parse_decimal_e7(lat)?,
                lon_e7: parse_decimal_e7(lon)?,
            },
            time,
        ));
        rest = after.get(next..).unwrap_or("");
    }
    if out.is_empty() {
        return Err(SimError::new("no <trkpt> in GPX"));
    }
    Ok(out)
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    for q in ['"', '\''] {
        let pat = alloc::format!(" {name}={q}");
        if let Some(i) = tag.find(&pat) {
            let v = tag.get(i + pat.len()..)?;
            let end = v.find(q)?;
            return v.get(..end);
        }
    }
    None
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let i = s.find(open)? + open.len();
    let rest = s.get(i..)?;
    let j = rest.find(close)?;
    rest.get(..j).map(str::trim)
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's algorithm).
#[must_use]
pub const fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `YYYY-MM-DDTHH:MM:SS[.fff]Z` to ms since the Unix epoch. Only UTC (`Z`) is accepted.
///
/// # Errors
/// Any other format.
pub fn parse_iso8601_ms(s: &str) -> SimResult<i64> {
    let bad = || SimError::new(alloc::format!("unsupported GPX time {s:?} (need UTC ...Z)"));
    let s = s.strip_suffix('Z').ok_or_else(bad)?;
    let (date, time) = s.split_once('T').ok_or_else(bad)?;
    let mut d = date.split('-');
    let num = |x: Option<&str>| -> SimResult<i64> {
        x.ok_or_else(bad)?.parse::<i64>().map_err(|_| bad())
    };
    let (y, mo, da) = (num(d.next())?, num(d.next())?, num(d.next())?);
    let (hms, frac) = match time.split_once('.') {
        Some((a, b)) => (a, b),
        None => (time, ""),
    };
    let mut t = hms.split(':');
    let (h, mi, se) = (num(t.next())?, num(t.next())?, num(t.next())?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&da) || h > 23 || mi > 59 || se > 60 {
        return Err(bad());
    }
    let mut ms = 0_i64;
    let mut scale = 100_i64;
    for b in frac.bytes().take(3) {
        if !b.is_ascii_digit() {
            return Err(bad());
        }
        ms += i64::from(b - b'0') * scale;
        scale /= 10;
    }
    let days = days_from_civil(y, mo, da);
    Ok((((days * 24 + h) * 60 + mi) * 60 + se) * 1000 + ms)
}

/// A synthetic GPX document for a list of points (used to write the fixture routes).
#[must_use]
pub fn write_gpx(name: &str, pts: &[(Position, Option<i64>)]) -> String {
    use crate::geom::e7_to_decimal;
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<gpx version=\"1.1\" creator=\"proxim-sim (synthetic)\" xmlns=\"http://www.topografix.com/GPX/1/1\">\n",
    );
    s.push_str(&alloc::format!("<trk><name>{name}</name><trkseg>\n"));
    for (p, t) in pts {
        s.push_str(&alloc::format!(
            "<trkpt lat=\"{}\" lon=\"{}\">",
            e7_to_decimal(p.lat_e7),
            e7_to_decimal(p.lon_e7)
        ));
        if let Some(t) = t {
            s.push_str("<time>");
            s.push_str(&iso8601(*t));
            s.push_str("</time>");
        }
        s.push_str("</trkpt>\n");
    }
    s.push_str("</trkseg></trk>\n</gpx>\n");
    s
}

/// Inverse of [`parse_iso8601_ms`] (whole seconds when `ms % 1000 == 0`).
#[must_use]
pub fn iso8601(t_ms: i64) -> String {
    let days = t_ms.div_euclid(86_400_000);
    let rem = t_ms.rem_euclid(86_400_000);
    // civil_from_days
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let (h, mi, s, ms) = (
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1000 % 60,
        rem % 1000,
    );
    if ms == 0 {
        alloc::format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
    } else {
        alloc::format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{ms:03}Z")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_timing_and_sampling() {
        let mut b = TruthBuilder::new((0, 0));
        b.go_to((0, 100_000), 10_000).unwrap(); // 100 m at 10 m/s: 10 s
        b.wait(5000);
        b.gap(10_000, Some((0, 500_000)));
        b.go_to((0, 600_000), 10_000).unwrap();
        let t = b.build();
        assert_eq!(t.duration_ms(), 35_000);
        let s = t.at(5000).unwrap();
        assert_eq!(
            (s.e_mm, s.n_mm, s.along_mm, s.speed_mmps),
            (0, 50_000, 50_000, 10_000)
        );
        assert_eq!(s.course_ddeg, Some(0));
        let s = t.at(12_000).unwrap();
        assert_eq!((s.n_mm, s.speed_mmps, s.course_ddeg), (100_000, 0, None));
        assert!(!t.at(16_000).unwrap().observed);
        let s = t.at(30_000).unwrap();
        assert!(s.observed);
        assert_eq!(s.n_mm, 550_000);
        assert_eq!(t.samples(1000).unwrap().len(), 36);
    }

    #[test]
    fn iso_times_roundtrip() {
        let t = 1_791_763_200_000 + 8 * 3_600_000 + 1234;
        assert_eq!(iso8601(t), "2026-10-12T08:00:01.234Z");
        assert_eq!(parse_iso8601_ms(&iso8601(t)).unwrap(), t);
        assert_eq!(parse_iso8601_ms("1970-01-01T00:00:00Z").unwrap(), 0);
        assert!(parse_iso8601_ms("2026-10-12T08:00:00+02:00").is_err());
    }

    #[test]
    fn gpx_and_geojson_parse() {
        let pts = [
            (
                Position {
                    lat_e7: 450_000_000,
                    lon_e7: 70_000_000,
                },
                Some(1000),
            ),
            (
                Position {
                    lat_e7: 450_010_000,
                    lon_e7: -70_000_001,
                },
                Some(2000),
            ),
        ];
        let g = write_gpx("t", &pts);
        assert_eq!(gpx_points(&g).unwrap(), pts.to_vec());
        let gj = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},
            "geometry":{"type":"LineString","coordinates":[[7.0,45.0,12.5],[7.0001234,45.00001]]}}]}"#;
        let v = geojson_points(gj).unwrap();
        assert_eq!(
            v,
            [
                Position {
                    lat_e7: 450_000_000,
                    lon_e7: 70_000_000
                },
                Position {
                    lat_e7: 450_000_100,
                    lon_e7: 70_001_234
                }
            ]
        );
        assert!(geojson_points(r#"{"type":"Point","coordinates":[1,2]}"#).is_err());
        assert!(gpx_points("<gpx></gpx>").is_err());
    }
}
