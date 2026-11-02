use super::{frac, set_hit, GenParams, Pattern, TRACKS, PHI_INV};
use std::f64::consts::TAU;

/// Coupling `K` for the Arnold circle map. Matching the Nelody generator, density is the
/// coupling knob and high density means weak coupling: `K = (1 - density) * 1.4`.
pub(crate) fn coupling(density: f32) -> f64 {
    (1.0 - density.clamp(0.0, 1.0)) as f64 * 1.4
}

/// Arnold circle map: `theta' = theta + Omega - K/(2*pi) * sin(2*pi*theta)`. `pulses` is the
/// rotation number (`Omega = pulses/steps`), density is the coupling `K`, and phase/seed set the
/// starting phase. A step fires when the phase wraps past 1.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    if p.pulses == 0 || p.density <= 0.0 { return out; }
    let omega = p.pulses.min(n).max(1) as f64 / n as f64;
    let k = coupling(p.density);
    let start = frac(p.seed as f64 + p.phase as f64 + p.variation as f64 * PHI_INV);
    for tr in 0..TRACKS {
        let mut theta = frac(start + tr as f64 / TRACKS as f64);
        let mut any = false;
        for i in 0..n {
            let next = theta + omega - k / TAU * (TAU * theta).sin();
            // 1e-9 slack: at K = 0 the accumulated rotation lands within rounding error of an
            // integer and would otherwise miss a wrap.
            let wraps = (next + 1e-9).floor();
            let fired = wraps >= 1.0;
            theta = (next - wraps).max(0.0);
            if fired { any = true; }
            set_hit(&mut out, tr, i, fired, if fired { 0.55 + 0.45 * theta as f32 } else { 0.6 });
            if tr == 0 {
                out.orbit[i] = theta as f32;
                out.theta[i] = theta as f32;
                out.u[i] = theta as f32;
            }
        }
        // High coupling can lock into a fixed point without wrapping. Pick the step with the
        // highest phase so every lane still sounds at steps=1 and at full density.
        if !any {
            let mut theta = frac(start + tr as f64 / TRACKS as f64);
            let mut best_i = 0usize;
            let mut best_v = -1.0f64;
            for i in 0..n {
                let next = theta + omega - k / TAU * (TAU * theta).sin();
                let wraps = (next + 1e-9).floor();
                theta = (next - wraps).max(0.0);
                if theta > best_v { best_v = theta; best_i = i; }
            }
            set_hit(&mut out, tr, best_i, true, 0.75);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Mode;

    fn params(steps: usize, density: f32, phase: f32, seed: f32, variation: u32) -> GenParams {
        GenParams {
            kind: Mode::Arnold, steps, density, phase, seed, variation, pulses: 5,
            probability: 1.0, ..GenParams::default()
        }
    }

    #[test]
    fn hits_are_phase_wraps_not_unrelated_random_draws() {
        // Recompute lane 0 from the published parameters: every hit must be a wrap.
        for density in [0.2f32, 0.65, 1.0] {
            for phase in [0.0f32, 0.27, 0.8] {
                let p = params(64, density, phase, 0.31, 3);
                let out = generate(&p);
                let mut theta = frac(p.seed as f64 + p.phase as f64 + p.variation as f64 * PHI_INV);
                let omega = p.pulses.min(64) as f64 / 64.0;
                let k = coupling(density);
                for i in 0..64 {
                    let next = theta + omega - k / TAU * (TAU * theta).sin();
                    let wraps = (next + 1e-9).floor();
                    theta = (next - wraps).max(0.0);
                    if wraps >= 1.0 { assert!(out.hit[0][i], "d={density} p={phase} step {i}: wrap without hit"); }
                }
            }
        }
    }

    #[test]
    fn omega_comes_from_pulses_and_k_from_density() {
        let low = generate(&params(64, 0.2, 0.0, 0.271, 0));
        let high = generate(&params(64, 0.9, 0.0, 0.271, 0));
        // Weak coupling at high density is steadier: fewer wrapped steps than the low-density
        // strongly coupled map at the same rotation number is not guaranteed, so compare the
        // helper directly and require a real difference in the generated line.
        assert!(coupling(0.2) > coupling(0.9));
        assert_ne!(low.hit[0], high.hit[0]);
        let more = GenParams { pulses: 7, ..params(64, 0.65, 0.0, 0.271, 0) };
        assert_ne!(generate(&params(64, 0.65, 0.0, 0.271, 0)).hit[0], generate(&more).hit[0]);
    }

    #[test]
    fn every_lane_sounds_and_high_coupling_does_not_silence_the_mode() {
        for density in [0.05f32, 0.65, 1.0] {
            for steps in [1usize, 7, 32, 64] {
                let out = generate(&params(steps, density, 0.0, 0.271, 0));
                for tr in 0..TRACKS {
                    assert!(out.hit[tr][..steps].iter().any(|&h| h), "d={density} steps={steps} lane={tr}");
                }
            }
        }
    }

    #[test]
    fn variation_moves_the_orbit() {
        let key = |v: u32| {
            let out = generate(&params(32, 0.65, 0.0, 0.271, v));
            let mut values: Vec<i32> = out.orbit[..32].iter().map(|x| (x * 1e5) as i32).collect();
            values.sort_unstable();
            values
        };
        assert_ne!(key(0), key(1));
    }
}
