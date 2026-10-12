//! Integer primitives (spec 1.3). All checked: overflow is a typed error, never a wrap or panic.

use proxim_types::{EngineError, Enu};

/// `floor((2a + b) / (2b))` for `b > 0`: division rounding half toward +infinity.
/// `div_round(7, 2) == 4`, `div_round(-7, 2) == -3`, `div_round(5, 2) == 3`, `div_round(-5, 2) == -2`.
///
/// # Errors
/// [`EngineError::Overflow`] if `b <= 0` (a non-positive divisor is outside the contract).
pub fn div_round(a: i64, b: i64) -> Result<i64, EngineError> {
    let q = div_round_i128(i128::from(a), i128::from(b))?;
    i64::try_from(q).map_err(|_| EngineError::Overflow)
}

/// [`div_round`] in `i128`.
///
/// # Errors
/// [`EngineError::Overflow`] if `b <= 0` or `2a + b` overflows.
pub fn div_round_i128(a: i128, b: i128) -> Result<i128, EngineError> {
    if b <= 0 {
        return Err(EngineError::Overflow);
    }
    let num = a
        .checked_mul(2)
        .and_then(|x| x.checked_add(b))
        .ok_or(EngineError::Overflow)?;
    let den = b.checked_mul(2).ok_or(EngineError::Overflow)?;
    // `div_euclid` with a positive divisor is exactly floor division.
    Ok(num.div_euclid(den))
}

/// Round-shift-right `rsr(x, s) = (x + 2^(s-1)) >> s` with an arithmetic shift (half rounds up).
///
/// # Errors
/// [`EngineError::Overflow`] if `s > 126` or the addition overflows. `s == 0` returns `x`.
pub fn rsr(x: i128, shift: u32) -> Result<i128, EngineError> {
    if shift == 0 {
        return Ok(x);
    }
    if shift > 126 {
        return Err(EngineError::Overflow);
    }
    let half = 1_i128 << (shift - 1);
    Ok(x.checked_add(half).ok_or(EngineError::Overflow)? >> shift)
}

/// Exact floor square root.
#[must_use]
pub fn isqrt(n: u128) -> u64 {
    // Digit-by-digit (base 4) method: exact for every u128, no floats, no division.
    let mut rem = n;
    let mut res: u128 = 0;
    let mut bit: u128 = 1 << 126;
    while bit > rem {
        bit >>= 2;
    }
    while bit != 0 {
        // `res + bit` cannot overflow: res < 2^64 and bit <= 2^126.
        let trial = res + bit;
        if rem >= trial {
            rem -= trial;
            res = (res >> 1) + bit;
        } else {
            res >>= 1;
        }
        bit >>= 2;
    }
    u64::try_from(res).unwrap_or(u64::MAX)
}

/// `orient2d(a, b, c)`: sign of the cross product `(b - a) x (c - a)`:
/// `1` if `c` is left of `a -> b`, `-1` if right, `0` if collinear. Exact.
///
/// # Errors
/// [`EngineError::Overflow`] when the exact cross product does not fit `i128` (only possible for
/// coordinates near the `i64` limits; engine coordinates are within 150 km of a frame origin).
pub fn orient2d(a: [i64; 2], b: [i64; 2], c: [i64; 2]) -> Result<i8, EngineError> {
    let [ax, ay] = a.map(i128::from);
    let [bx, by] = b.map(i128::from);
    let [cx, cy] = c.map(i128::from);
    let abx = bx - ax;
    let aby = by - ay;
    let acx = cx - ax;
    let acy = cy - ay;
    let l = abx.checked_mul(acy).ok_or(EngineError::Overflow)?;
    let r = aby.checked_mul(acx).ok_or(EngineError::Overflow)?;
    let cross = l.checked_sub(r).ok_or(EngineError::Overflow)?;
    Ok(match cross.signum() {
        1 => 1,
        -1 => -1,
        _ => 0,
    })
}

/// Euclidean distance in mm: `isqrt(dx^2 + dy^2)` (floor).
///
/// # Errors
/// [`EngineError::Overflow`] only if the result does not fit `i64` (unreachable for `i32` inputs;
/// kept so the signature matches the rest of the checked API).
pub fn dist_mm(a: Enu, b: Enu) -> Result<i64, EngineError> {
    let d2 = dist2(a, b);
    i64::try_from(isqrt(d2)).map_err(|_| EngineError::Overflow)
}

/// Squared distance, exact (`<= 2 * 2^64`, always fits `u128`).
pub(crate) fn dist2(a: Enu, b: Enu) -> u128 {
    let dx = i128::from(a.e_mm) - i128::from(b.e_mm);
    let dy = i128::from(a.n_mm) - i128::from(b.n_mm);
    // Squares of values below 2^33 are below 2^66: no overflow, and always non-negative.
    (dx * dx + dy * dy).unsigned_abs()
}

/// Exact orientation sign for `Enu` points. Differences fit 33 bits so the products fit `i128`.
pub(crate) fn orient_enu(a: Enu, b: Enu, c: Enu) -> i8 {
    let abx = i128::from(b.e_mm) - i128::from(a.e_mm);
    let aby = i128::from(b.n_mm) - i128::from(a.n_mm);
    let acx = i128::from(c.e_mm) - i128::from(a.e_mm);
    let acy = i128::from(c.n_mm) - i128::from(a.n_mm);
    match (abx * acy - aby * acx).signum() {
        1 => 1,
        -1 => -1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn div_round_rounding_edges() {
        // Half rounds toward +infinity (floor((2a+b)/(2b))).
        assert_eq!(div_round(7, 2), Ok(4));
        assert_eq!(div_round(5, 2), Ok(3));
        assert_eq!(div_round(3, 2), Ok(2));
        assert_eq!(div_round(1, 2), Ok(1));
        assert_eq!(div_round(0, 2), Ok(0));
        assert_eq!(div_round(-1, 2), Ok(0));
        assert_eq!(div_round(-3, 2), Ok(-1));
        assert_eq!(div_round(-5, 2), Ok(-2));
        assert_eq!(div_round(-7, 2), Ok(-3));
        // Just below and above a half.
        assert_eq!(div_round(1, 3), Ok(0));
        assert_eq!(div_round(2, 3), Ok(1));
        assert_eq!(div_round(-1, 3), Ok(0));
        assert_eq!(div_round(-2, 3), Ok(-1));
        // b == 1 is the identity, including at the extremes.
        assert_eq!(div_round(i64::MAX, 1), Ok(i64::MAX));
        assert_eq!(div_round(i64::MIN, 1), Ok(i64::MIN));
        assert_eq!(div_round(i64::MIN, i64::MAX), Ok(-1));
        assert_eq!(div_round(i64::MAX, i64::MAX), Ok(1));
        // Latitude grid: 0.01 degree = 100_000 e7 units; exactly half a cell rounds up.
        assert_eq!(div_round(449_950_000, 100_000), Ok(4500));
        assert_eq!(div_round(449_949_999, 100_000), Ok(4499));
        assert_eq!(div_round(-449_950_000, 100_000), Ok(-4499));
        assert_eq!(div_round(-449_950_001, 100_000), Ok(-4500));
    }

    #[test]
    fn div_round_rejects_non_positive_divisor() {
        assert_eq!(div_round(1, 0), Err(EngineError::Overflow));
        assert_eq!(div_round(1, -1), Err(EngineError::Overflow));
        assert_eq!(div_round(i64::MIN, i64::MIN), Err(EngineError::Overflow));
        assert_eq!(div_round_i128(1, 0), Err(EngineError::Overflow));
        assert_eq!(div_round_i128(i128::MAX, 1), Err(EngineError::Overflow));
    }

    #[test]
    fn rsr_semantics() {
        assert_eq!(rsr(5, 1), Ok(3)); // 2.5 -> 3
        assert_eq!(rsr(-5, 1), Ok(-2)); // -2.5 -> -2 (half up)
        assert_eq!(rsr(3, 1), Ok(2));
        assert_eq!(rsr(-3, 1), Ok(-1));
        assert_eq!(rsr(1 << 32, 32), Ok(1));
        assert_eq!(rsr((1 << 31) - 1, 32), Ok(0));
        assert_eq!(rsr(1 << 31, 32), Ok(1));
        assert_eq!(rsr(-(1 << 31), 32), Ok(0));
        assert_eq!(rsr(-(1 << 31) - 1, 32), Ok(-1));
        assert_eq!(rsr(7, 0), Ok(7));
        assert_eq!(rsr(1, 127), Err(EngineError::Overflow));
        assert_eq!(rsr(i128::MAX, 1), Err(EngineError::Overflow));
    }

    #[test]
    fn isqrt_exact_values() {
        for (n, r) in [
            (0_u128, 0_u64),
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 2),
            (8, 2),
            (9, 3),
            (15, 3),
            (16, 4),
            (99, 9),
            (100, 10),
            (101, 10),
        ] {
            assert_eq!(isqrt(n), r, "isqrt({n})");
        }
        let m = u128::from(u64::MAX);
        assert_eq!(isqrt(m * m), u64::MAX);
        assert_eq!(isqrt(m * m - 1), u64::MAX - 1);
        assert_eq!(isqrt(u128::MAX), u64::MAX);
        let two63 = 1_u128 << 63;
        assert_eq!(isqrt(two63 * two63), 1 << 63);
        assert_eq!(isqrt(two63 * two63 - 1), (1 << 63) - 1);
    }

    #[test]
    fn isqrt_matches_naive_for_the_first_100k() {
        let mut r: u128 = 0;
        for n in 0_u128..100_000 {
            while (r + 1) * (r + 1) <= n {
                r += 1;
            }
            assert_eq!(u128::from(isqrt(n)), r, "n = {n}");
        }
    }

    #[test]
    fn orient2d_signs() {
        assert_eq!(orient2d([0, 0], [10, 0], [5, 5]), Ok(1)); // left of an eastward line
        assert_eq!(orient2d([0, 0], [10, 0], [5, -5]), Ok(-1));
        assert_eq!(orient2d([0, 0], [10, 0], [20, 0]), Ok(0));
        assert_eq!(orient2d([0, 0], [10, 10], [-3, -3]), Ok(0));
        // Exact where floating point would round: huge near-collinear triple.
        let big = 1_i64 << 40;
        assert_eq!(
            orient2d([0, 0], [big, big + 1], [2 * big, 2 * big + 2]),
            Ok(0)
        );
        assert_eq!(
            orient2d([0, 0], [big, big + 1], [2 * big, 2 * big + 3]),
            Ok(1)
        );
        assert_eq!(
            orient2d([0, 0], [big, big + 1], [2 * big, 2 * big + 1]),
            Ok(-1)
        );
    }

    #[test]
    fn orient2d_overflow_is_typed() {
        let r = orient2d(
            [i64::MIN, i64::MIN],
            [i64::MAX, i64::MIN],
            [i64::MIN, i64::MAX],
        );
        assert_eq!(r, Err(EngineError::Overflow));
        // Within the 150 km frame (1.5e8 mm) it cannot overflow.
        let f = 150_000_000;
        assert_eq!(orient2d([-f, -f], [f, -f], [-f, f]), Ok(1));
    }

    #[test]
    fn dist_matches_pythagoras() {
        let o = Enu { e_mm: 0, n_mm: 0 };
        assert_eq!(
            dist_mm(
                o,
                Enu {
                    e_mm: 3000,
                    n_mm: 4000
                }
            ),
            Ok(5000)
        );
        assert_eq!(
            dist_mm(
                o,
                Enu {
                    e_mm: -3000,
                    n_mm: -4000
                }
            ),
            Ok(5000)
        );
        assert_eq!(dist_mm(o, Enu { e_mm: 1, n_mm: 1 }), Ok(1)); // floor(1.414)
        let lo = Enu {
            e_mm: i32::MIN,
            n_mm: i32::MIN,
        };
        let hi = Enu {
            e_mm: i32::MAX,
            n_mm: i32::MAX,
        };
        // sqrt(2) * (2^32 - 1) = 6_074_000_998.x
        assert_eq!(dist_mm(lo, hi), Ok(6_074_000_998));
    }
}
