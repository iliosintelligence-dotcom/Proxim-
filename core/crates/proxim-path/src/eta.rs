//! Time to anchor (engine-spec 5.4) and the closed-form fire distance (6.2).

use proxim_types::EngineError;

/// Dead-reckoned remaining distance: `max(0, rem_along - floor(v_eff x age / 1000))` (5.4).
/// A negative `rem_along` (anchor passed) yields 0.
///
/// # Errors
/// [`EngineError::Parse`] for a negative `age_ms` (the caller clamps a fix from the near future to
/// age 0); [`EngineError::Overflow`] if the result does not fit `i64`.
pub fn d_rem_mm(rem_along_mm: i64, v_eff_mmps: u32, age_ms: i64) -> Result<i64, EngineError> {
    if age_ms < 0 {
        return Err(EngineError::Parse);
    }
    let travelled = i128::from(v_eff_mmps)
        .checked_mul(i128::from(age_ms))
        .ok_or(EngineError::Overflow)?
        / 1000;
    let rem = i128::from(rem_along_mm)
        .checked_sub(travelled)
        .ok_or(EngineError::Overflow)?
        .max(0);
    i64::try_from(rem).map_err(|_| EngineError::Overflow)
}

/// Time to anchor in ms: `floor(d_rem x 1000 / v_eff)` (5.4).
///
/// # Errors
/// [`EngineError::Parse`] if `d_rem_mm < 0` or `v_eff_mmps < 700` (the foot floor: the contract
/// excludes a smaller `v_eff`, so no division by zero or absurd ETA can occur).
pub fn t_hat_ms(d_rem_mm: i64, v_eff_mmps: u32) -> Result<i64, EngineError> {
    if d_rem_mm < 0 || v_eff_mmps < crate::params::FLOOR_FOOT_MMPS {
        return Err(EngineError::Parse);
    }
    let t = i128::from(d_rem_mm)
        .checked_mul(1000)
        .ok_or(EngineError::Overflow)?
        / i128::from(v_eff_mmps);
    i64::try_from(t).map_err(|_| EngineError::Overflow)
}

/// Steady-state fire distance `D_fire = clamp(v_eff x (lead_s + t_comp), d_lo, d_hi)` in mm (6.2).
/// The detector's rule is time based; this is its equivalent distance form, used by tests and by
/// the simulator to state the expected behaviour.
///
/// # Errors
/// [`EngineError::Parse`] if `d_lo_mm > d_hi_mm`; [`EngineError::Overflow`] on `i64` overflow.
pub fn fire_distance_mm(
    v_eff_mmps: u32,
    lead_s: u32,
    t_comp_ms: i64,
    d_lo_mm: i64,
    d_hi_mm: i64,
) -> Result<i64, EngineError> {
    if d_lo_mm > d_hi_mm {
        return Err(EngineError::Parse);
    }
    let lead_ms = i128::from(lead_s) * 1000 + i128::from(t_comp_ms);
    let d = i128::from(v_eff_mmps)
        .checked_mul(lead_ms)
        .ok_or(EngineError::Overflow)?
        / 1000;
    let d = i64::try_from(d).unwrap_or(i64::MAX);
    Ok(d.clamp(d_lo_mm, d_hi_mm))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d_rem_dead_reckons_and_floors_at_zero() {
        assert_eq!(d_rem_mm(1_000_000, 10_000, 0), Ok(1_000_000));
        // 10 m/s for 2.5 s = 25 m.
        assert_eq!(d_rem_mm(1_000_000, 10_000, 2500), Ok(975_000));
        // floor(7 mm/s x 1.5 s) = 10 mm.
        assert_eq!(d_rem_mm(100, 7, 1500), Ok(90));
        assert_eq!(d_rem_mm(1000, 10_000, 10_000), Ok(0));
        assert_eq!(d_rem_mm(-5000, 10_000, 0), Ok(0));
        assert_eq!(d_rem_mm(i64::MIN, u32::MAX, i64::MAX), Ok(0));
        assert_eq!(d_rem_mm(i64::MAX, 0, 0), Ok(i64::MAX));
        assert_eq!(d_rem_mm(1000, 1000, -1), Err(EngineError::Parse));
    }

    #[test]
    fn t_hat_is_floor_of_ratio() {
        assert_eq!(t_hat_ms(1_247_000, 29_000), Ok(43_000));
        assert_eq!(t_hat_ms(1000, 700), Ok(1428)); // 1428.57 floors
        assert_eq!(t_hat_ms(0, 700), Ok(0));
        assert_eq!(t_hat_ms(-1, 4000), Err(EngineError::Parse));
        assert_eq!(t_hat_ms(1000, 699), Err(EngineError::Parse));
        assert_eq!(t_hat_ms(1000, 0), Err(EngineError::Parse));
        assert_eq!(t_hat_ms(i64::MAX, 700), Err(EngineError::Overflow));
    }

    #[test]
    fn spec_fire_distances() {
        // Spec 11: lead 40 s + t_comp 3 s, d_lo 150 m, d_hi 3000 m.
        let f = |v| fire_distance_mm(v, 40, 3000, 150_000, 3_000_000).unwrap();
        assert_eq!(f(29_000), 1_247_000, "29 m/s");
        assert_eq!(f(15_000), 645_000, "15 m/s");
        assert_eq!(f(5_000), 215_000, "cyclist 5 m/s");
        assert_eq!(f(4_000), 172_000, "jam: 3 m/s is floored to 4 m/s");
        // Clamps at both ends.
        assert_eq!(f(1_000), 150_000);
        assert_eq!(f(100_000), 3_000_000);
        assert_eq!(fire_distance_mm(1, 40, 3000, 2, 1), Err(EngineError::Parse));
    }
}
