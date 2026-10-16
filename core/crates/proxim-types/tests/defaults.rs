//! The provisional constants of spec sections 5 and 6 live in `proxim_types::defaults` (spec 1:
//! one place for every [P] value). This pins the values they had in the per-crate `params`
//! modules they moved from, so the move cannot have changed a number.
#![allow(clippy::unwrap_used)]

use proxim_types::defaults as d;

#[test]
fn moved_constants_keep_their_values() {
    // from proxim-path::params (HEAD 0777139)
    assert_eq!(d::SPEED_TAU_MS, 8000);
    assert_eq!(d::HEADING_TAU_MS, 4000);
    assert_eq!(d::DT_CLAMP_MS, 60_000);
    assert_eq!(d::RESEED_GAP_MS, 5000);
    assert_eq!(d::STALE_HIDE_MS, 10_000);
    assert_eq!(d::SPEED_ACC_MAX_MMPS, 5000);
    assert_eq!(d::SPEED_DISP_MIN_AGE_MS, 3000);
    assert_eq!(d::SPEED_DISP_MAX_AGE_MS, 10_000);
    assert_eq!(d::HEADING_DISP_MIN_AGE_MS, 3000);
    assert_eq!(d::HEADING_DISP_MAX_AGE_MS, 15_000);
    assert_eq!(d::COURSE_ACC_MAX_DDEG, 450);
    assert_eq!(d::HEADING_MIN_SPEED_MMPS, 800);
    assert_eq!(d::HEADING_MIN_DISP_MM, 15_000);
    assert_eq!(d::FLOOR_VEHICLE_MMPS, 4000);
    assert_eq!(d::FLOOR_CYCLE_MMPS, 2000);
    assert_eq!(d::FLOOR_FOOT_MMPS, 700);
    assert_eq!(d::MIN_VEHICLE_MMPS, 2500);
    assert_eq!(d::MIN_CYCLE_MMPS, 1500);
    assert_eq!(d::MIN_FOOT_MMPS, 500);
    assert_eq!(d::MAX_FOOT_MMPS, 2500);
    assert_eq!(d::P1_ACC_EFF_MAX_MM, 65_000);
    assert_eq!(d::P1_MIN_FIXES, 3);
    assert_eq!(d::P1_WINDOW_MS, 60_000);
    assert_eq!(d::P1_HEADING_TOL_DDEG, 450);
    assert_eq!(d::P1_BONUS_HEADING_TOL_DDEG, 150);
    assert_eq!(d::P1_CONF_BASE, 600);
    assert_eq!(d::P1_CONF_BONUS, 100);
    assert_eq!(d::P1_CONF_PENALTY1, 100);
    assert_eq!(d::P1_CONF_PENALTY2, 200);
    assert_eq!(d::P1_ACC_PENALTY1_MM, 30_000);
    assert_eq!(d::P1_ACC_PENALTY2_MM, 50_000);
    assert_eq!(d::P1_CONF_FOOT_CAP, 600);
    assert_eq!(d::P0_CONF, 500);
    assert_eq!(d::P1_BONUS_FIXES, 5);
    assert_eq!(d::CLOSING_MIN_ADVANCE_MM, 5000);
    assert_eq!(d::WINDOW_BACK_MM, 100_000);
    assert_eq!(d::WINDOW_AHEAD_MM, 500_000);
    assert_eq!(d::WINDOW_AHEAD_S, 30);
    // from proxim-detect::params (HEAD 0777139)
    assert_eq!(d::ENTER_MIN_FIXES, 3);
    assert_eq!(d::ENTER_MIN_SPAN_MS, 3000);
    assert_eq!(d::ENTER_MIN_SPAN_SLOW_MS, 10_000);
    assert_eq!(d::ENTER_SLOW_MMPS, 1500);
    assert_eq!(d::ENTER_MAX_GAP_MS, 10_000);
    assert_eq!(d::CIRCLE_MAX_ACC_MM, 50_000);
    assert_eq!(d::EXIT_FIXES, 5);
    assert_eq!(d::EXIT_FEW_FIXES, 2);
    assert_eq!(d::EXIT_SPAN_MS, 10_000);
    assert_eq!(d::COOL_MS, 1_800_000);
    assert_eq!(d::IN_SURVIVE_GAP_MS, 120_000);
    assert_eq!(d::IN_MAX_GAP_MS, 900_000);
    assert_eq!(d::MAX_FIX_AGE_MS, 10_000);
    assert_eq!(d::MAX_ACC_EFF_MM, 65_000);
    assert_eq!(d::T_COMP_MS, 3000);
    assert_eq!(d::CONFIRM_SPACING_MS, 1000);
    assert_eq!(d::CONFIRM_MAX_GAP_MS, 5000);
    assert_eq!(d::L_MIN_MS_PER_LEAD_S, 400);
    assert_eq!(d::L_MIN_FLOOR_MS, 10_000);
    assert_eq!(d::D_PASS_MIN_MM, 60_000);
    assert_eq!(d::D_PASS_S, 5);
    assert_eq!(d::D_LO_VEHICLE_MM, 150_000);
    assert_eq!(d::D_LO_SLOW_MM, 100_000);
    assert_eq!(d::BAND_HYSTERESIS_MMPS, 1000);
    assert_eq!(d::REARM_NUM, 12);
    assert_eq!(d::REARM_DEN, 10);
    assert_eq!(d::REACH_MARGIN_MM, 100_000);
    assert_eq!(d::BUFFER_MIN_M, 30);
    assert_eq!(d::BUFFER_MAX_M, 500);
    assert_eq!(d::BEARING_TOL_MIN_DDEG, 150);
    assert_eq!(d::BEARING_TOL_MAX_DDEG, 450);
}

#[test]
fn new_closing_constants_are_the_spec_values() {
    assert_eq!(d::CLOSING_MIN_SPAN_MS, 4000);
    assert_eq!(d::CLOSING_MAX_SPAN_MS, 60_000);
    assert_eq!(d::CLOSING_MIN_ADVANCE_MM, 5000);
    // The span cap equals the P1 fix window.
    assert_eq!(d::CLOSING_MAX_SPAN_MS, d::P1_WINDOW_MS);
}

#[test]
fn the_per_crate_params_modules_are_gone() {
    // Spec 1: [P] values live in `proxim-types::defaults` and nowhere else.
    let root = core_dir().join("crates");
    for krate in ["proxim-path", "proxim-detect"] {
        let p = root.join(krate).join("src").join("params.rs");
        assert!(!p.exists(), "{} must not exist", p.display());
    }
}

/// The `core/` directory. `PROXIM_CORE_DIR` overrides the build machine's path
/// when the tests run on a device (Android emulator) with the tree copied over.
fn core_dir() -> std::path::PathBuf {
    std::env::var_os("PROXIM_CORE_DIR").map_or_else(
        || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    )
}
