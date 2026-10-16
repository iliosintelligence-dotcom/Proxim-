//! GNSS noise model: AR(1) position error per axis, occasional jumps (multipath) whose reported
//! accuracy does not reflect them, reported accuracy, Doppler speed and course noise, dropouts
//! (tunnels, GPS loss) with a degraded reacquisition, all from a seeded integer RNG.
//!
//! Profiles are [MODEL] parameters chosen to resemble published phone GNSS behaviour (open sky
//! a few metres, suburban several metres, urban canyon tens of metres with frequent multipath
//! jumps); they are inputs to test the engine's logic, not measurements. M4 field logs replace
//! them (PLAN M4).

use crate::geom::{Local, SimError, SimResult};
use crate::rng::Rng;
use crate::trace::Sample;
use alloc::vec::Vec;
use proxim_geo::{isqrt, unit_from_course};
use proxim_types::Fix;
use serde::Deserialize;

/// Noise parameters. Distances in mm, speeds in mm/s, angles in 0.1 degree, probabilities in
/// parts per million per fix, correlation in permille per second.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoiseParams {
    /// Stationary standard deviation of the AR(1) error, per axis.
    pub sigma_mm: i64,
    /// AR(1) coefficient for a 1 s step (applied as `rho^(period/1 s)`).
    pub rho_milli: i64,
    /// Reported 68% accuracy: mean and spread, floored at `acc_min_mm`.
    pub acc_mm: i64,
    pub acc_sigma_mm: i64,
    pub acc_min_mm: i64,
    /// Jump onset probability per fix.
    pub jump_ppm: u32,
    /// Jump offset magnitude range and duration in fixes (inclusive ranges).
    pub jump_mm: [i64; 2],
    pub jump_fixes: [i64; 2],
    /// Reported accuracy during a jump, permille of normal (1000 = the jump is not reported).
    pub jump_acc_permille: i64,
    /// Doppler speed noise; course noise; course only reported at or above this true speed.
    pub speed_sigma_mmps: i64,
    pub course_sigma_ddeg: i64,
    pub course_min_speed_mmps: u32,
    /// Fixes after a dropout with degraded accuracy and extra error.
    pub reacquire_fixes: u32,
    pub reacquire_acc_mm: i64,
    pub reacquire_sigma_mm: i64,
    /// Report Doppler speed and course at all.
    pub doppler: bool,
}

impl NoiseParams {
    /// Exact truth, reported accuracy 5 m (the engine floors it at 10 m), exact Doppler.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            sigma_mm: 0,
            rho_milli: 0,
            acc_mm: 5000,
            acc_sigma_mm: 0,
            acc_min_mm: 5000,
            jump_ppm: 0,
            jump_mm: [0, 0],
            jump_fixes: [0, 0],
            jump_acc_permille: 1000,
            speed_sigma_mmps: 0,
            course_sigma_ddeg: 0,
            course_min_speed_mmps: 800,
            reacquire_fixes: 0,
            reacquire_acc_mm: 5000,
            reacquire_sigma_mm: 0,
            doppler: true,
        }
    }

    /// Open sky [MODEL]: 3 m per axis, 0.9 correlation per second, about one jump of 15-40 m
    /// per 8 minutes lasting 1-3 fixes, accuracy reported as about 5 m.
    #[must_use]
    pub const fn open_sky() -> Self {
        Self {
            sigma_mm: 3000,
            rho_milli: 900,
            acc_mm: 5000,
            acc_sigma_mm: 1000,
            acc_min_mm: 3000,
            jump_ppm: 2000,
            jump_mm: [15_000, 40_000],
            jump_fixes: [1, 3],
            jump_acc_permille: 1000,
            speed_sigma_mmps: 300,
            course_sigma_ddeg: 30,
            course_min_speed_mmps: 800,
            reacquire_fixes: 5,
            reacquire_acc_mm: 30_000,
            reacquire_sigma_mm: 10_000,
            doppler: true,
        }
    }

    /// Suburban [MODEL]: 6 m, 0.93, jumps of 20-60 m for 1-5 fixes about once a minute and a
    /// half, accuracy about 8 m.
    #[must_use]
    pub const fn suburban() -> Self {
        Self {
            sigma_mm: 6000,
            rho_milli: 930,
            acc_mm: 8000,
            acc_sigma_mm: 2000,
            acc_min_mm: 4000,
            jump_ppm: 10_000,
            jump_mm: [20_000, 60_000],
            jump_fixes: [1, 5],
            jump_acc_permille: 1000,
            speed_sigma_mmps: 500,
            course_sigma_ddeg: 60,
            course_min_speed_mmps: 800,
            reacquire_fixes: 8,
            reacquire_acc_mm: 40_000,
            reacquire_sigma_mm: 15_000,
            doppler: true,
        }
    }

    /// Urban canyon [MODEL]: 12 m, 0.95, jumps of 30-120 m for 2-10 fixes every half minute,
    /// accuracy about 15 m (under-reported during jumps by half).
    #[must_use]
    pub const fn urban() -> Self {
        Self {
            sigma_mm: 12_000,
            rho_milli: 950,
            acc_mm: 15_000,
            acc_sigma_mm: 5000,
            acc_min_mm: 5000,
            jump_ppm: 30_000,
            jump_mm: [30_000, 120_000],
            jump_fixes: [2, 10],
            jump_acc_permille: 1500,
            speed_sigma_mmps: 1000,
            course_sigma_ddeg: 150,
            course_min_speed_mmps: 1000,
            reacquire_fixes: 10,
            reacquire_acc_mm: 60_000,
            reacquire_sigma_mm: 25_000,
            doppler: true,
        }
    }

    /// A named profile.
    ///
    /// # Errors
    /// Unknown name.
    pub fn profile(name: &str) -> SimResult<Self> {
        match name {
            "none" => Ok(Self::none()),
            "open_sky" => Ok(Self::open_sky()),
            "suburban" => Ok(Self::suburban()),
            "urban" => Ok(Self::urban()),
            _ => Err(SimError::new(alloc::format!("unknown noise profile {name:?}"))),
        }
    }

    /// True when every random term is zero, so one seed tells the whole story.
    #[must_use]
    pub const fn is_deterministic(&self) -> bool {
        self.sigma_mm == 0
            && self.acc_sigma_mm == 0
            && self.jump_ppm == 0
            && self.speed_sigma_mmps == 0
            && self.course_sigma_ddeg == 0
            && self.reacquire_sigma_mm == 0
    }
}

/// A fix produced from a truth sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoisyFix {
    /// Index into the sample list (the oracle's truth for this fix).
    pub sample: usize,
    pub fix: Fix,
    pub in_jump: bool,
}

/// AR(1) state for one axis.
#[derive(Clone, Copy, Debug)]
struct Ar1 {
    x: i64,
}

fn rho_for_period(rho_milli: i64, period_ms: i64) -> i64 {
    // rho^(period / 1 s), integer: whole seconds by repeated product, sub-second periods keep rho.
    let steps = (period_ms / 1000).max(1);
    let mut r: i64 = 1000;
    for _ in 0..steps.min(600) {
        r = r * rho_milli.clamp(0, 999) / 1000;
    }
    r
}

/// Turn truth samples into fixes. Samples that are unobserved (gaps) or inside `dropouts`
/// (inclusive `[from, to]` ranges of `t_rel_ms`) produce no fix; the AR(1) state keeps evolving
/// through them so a dropout does not change the noise after it.
///
/// # Errors
/// A sample outside the 150 km frame, or an engine table error.
#[allow(clippy::too_many_lines)]
pub fn apply(
    local: &Local,
    samples: &[Sample],
    params: &NoiseParams,
    dropouts: &[(i64, i64)],
    start_ms: i64,
    period_ms: i64,
    seed: u64,
) -> SimResult<Vec<NoisyFix>> {
    let mut rng = Rng::new(seed, 0x6E6F_6973);
    let rho = rho_for_period(params.rho_milli, period_ms);
    // innovation sd = sigma x sqrt(1 - rho^2)
    let innov_permille = i64::try_from(isqrt(
        u128::try_from(1_000_000 - rho * rho).unwrap_or(0),
    ))
    .unwrap_or(0);
    let innov_sigma = params.sigma_mm * innov_permille / 1000;
    let mut ax = Ar1 {
        x: rng.gauss(params.sigma_mm),
    };
    let mut ay = Ar1 {
        x: rng.gauss(params.sigma_mm),
    };
    let mut jump_left: i64 = 0;
    let mut jump_off = (0_i64, 0_i64);
    let mut reacq_left: u32 = 0;
    let mut was_dark = false;
    let mut out = Vec::new();
    for (i, s) in samples.iter().enumerate() {
        // Draw every random term for every sample, used or not, so streams stay aligned.
        ax.x = ax.x * rho / 1000 + rng.gauss(innov_sigma);
        ay.x = ay.x * rho / 1000 + rng.gauss(innov_sigma);
        let acc_draw = rng.gauss(params.acc_sigma_mm);
        let speed_draw = rng.gauss(params.speed_sigma_mmps);
        let course_draw = rng.gauss(params.course_sigma_ddeg);
        let reacq_draw = (
            rng.gauss(params.reacquire_sigma_mm),
            rng.gauss(params.reacquire_sigma_mm),
        );
        // `jump_left` = further samples that keep the current offset after this one.
        if jump_left > 0 {
            jump_left -= 1;
        } else {
            jump_off = (0, 0);
            if params.jump_fixes[1] > 0 && rng.chance_ppm(params.jump_ppm) {
                let mag = rng.range_i64(params.jump_mm[0], params.jump_mm[1]);
                let dir = u16::try_from(rng.below(3600)).unwrap_or(0);
                let [ue, un] = unit_from_course(dir)?;
                jump_off = (
                    i64::try_from((i128::from(mag) * i128::from(ue)) >> 30).unwrap_or(0),
                    i64::try_from((i128::from(mag) * i128::from(un)) >> 30).unwrap_or(0),
                );
                let dur = rng.range_i64(params.jump_fixes[0].max(1), params.jump_fixes[1]);
                jump_left = dur - 1;
            }
        }
        let in_jump = jump_off != (0, 0);

        let dark = !s.observed
            || dropouts
                .iter()
                .any(|(a, b)| s.t_rel_ms >= *a && s.t_rel_ms <= *b);
        if dark {
            was_dark = true;
            continue;
        }
        if was_dark {
            was_dark = false;
            reacq_left = params.reacquire_fixes;
        }
        let reacq = reacq_left > 0;
        reacq_left = reacq_left.saturating_sub(1);

        let mut e = s.e_mm + ax.x + jump_off.0;
        let mut n = s.n_mm + ay.x + jump_off.1;
        let mut acc = (params.acc_mm + acc_draw).max(params.acc_min_mm);
        if in_jump {
            acc = acc * params.jump_acc_permille / 1000;
        }
        if reacq {
            e += reacq_draw.0;
            n += reacq_draw.1;
            acc = acc.max(params.reacquire_acc_mm);
        }
        let pos = local.pos(e, n)?;
        let (speed, course) = if params.doppler {
            let v = (i64::from(s.speed_mmps) + speed_draw).max(0);
            let c = match s.course_ddeg {
                Some(c) if s.speed_mmps >= params.course_min_speed_mmps => {
                    Some(u16::try_from((i64::from(c) + course_draw).rem_euclid(3600)).unwrap_or(0))
                }
                _ => None,
            };
            (Some(u32::try_from(v).unwrap_or(u32::MAX)), c)
        } else {
            (None, None)
        };
        out.push(NoisyFix {
            sample: i,
            fix: Fix {
                t_ms: start_ms.saturating_add(s.t_rel_ms),
                lat_e7: pos.lat_e7,
                lon_e7: pos.lon_e7,
                acc_mm: u32::try_from(acc.max(0)).unwrap_or(u32::MAX),
                speed_mmps: speed,
                speed_acc_mmps: None,
                course_ddeg: course,
                course_acc_ddeg: None,
                is_mock: false,
            },
            in_jump,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::TruthBuilder;
    use proxim_types::Position;

    fn local() -> Local {
        Local::new(Position {
            lat_e7: 450_000_000,
            lon_e7: 70_000_000,
        })
        .unwrap()
    }

    #[test]
    fn none_profile_reproduces_the_truth() {
        let mut b = TruthBuilder::new((0, 0));
        b.go_to((0, 100_000), 10_000).unwrap();
        let s = b.build().samples(1000).unwrap();
        let l = local();
        let f = apply(&l, &s, &NoiseParams::none(), &[], 0, 1000, 1).unwrap();
        assert_eq!(f.len(), s.len());
        for nf in &f {
            let q = l.enu(nf.fix.position()).unwrap();
            let t = s[nf.sample];
            assert!((i64::from(q.n_mm) - t.n_mm).abs() <= 10, "e7 grid is about 11 mm");
            assert_eq!(nf.fix.acc_mm, 5000);
            assert_eq!(nf.fix.speed_mmps, Some(t.speed_mmps));
        }
    }

    #[test]
    fn ar1_spread_matches_sigma_and_dropouts_remove_fixes() {
        let mut b = TruthBuilder::new((0, 0));
        b.wait(2_000_000);
        let s = b.build().samples(1000).unwrap();
        let l = local();
        let mut p = NoiseParams::open_sky();
        p.jump_ppm = 0;
        let f = apply(&l, &s, &p, &[(100_000, 199_999)], 0, 1000, 9).unwrap();
        assert_eq!(f.len(), s.len() - 100);
        let mut sq = 0_i128;
        for nf in &f {
            let q = l.enu(nf.fix.position()).unwrap();
            sq += i128::from(q.e_mm) * i128::from(q.e_mm);
        }
        let var = sq / f.len() as i128;
        // sigma 3 m -> variance 9e6 mm^2; AR(1) with rho 0.9 over 2000 s: loose bounds
        assert!((5_000_000..=14_000_000).contains(&var), "var {var}");
        // reacquisition after the dropout degrades accuracy
        let after = f.iter().find(|nf| s[nf.sample].t_rel_ms == 200_000).unwrap();
        assert!(after.fix.acc_mm >= 30_000);
    }

    #[test]
    fn same_seed_same_fixes() {
        let mut b = TruthBuilder::new((0, 0));
        b.go_to((500_000, 0), 15_000).unwrap();
        let s = b.build().samples(1000).unwrap();
        let l = local();
        let a = apply(&l, &s, &NoiseParams::urban(), &[], 0, 1000, 5).unwrap();
        let c = apply(&l, &s, &NoiseParams::urban(), &[], 0, 1000, 5).unwrap();
        let d = apply(&l, &s, &NoiseParams::urban(), &[], 0, 1000, 6).unwrap();
        assert_eq!(a, c);
        assert_ne!(a, d);
        assert!(a.iter().any(|f| f.in_jump), "urban has jumps");
    }
}
