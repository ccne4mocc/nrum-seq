//! Kuramoto coupled lanes: eight phase oscillators on a ring. Each advances by its detuned rate
//! plus the mean pull of its neighbours; a step scores when a phase crosses a whole turn.
//!
//! `osc_rate` sets coupling, `phase` the frequency spread, `pulses` the onset count and `density`
//! the usual weighting. Low coupling drifts; high coupling locks into a rotating column.

use super::{set_hit, top_k, GenParams, Pattern, DENSITY_PROFILE, MAX_STEPS, TRACKS};
use std::f64::consts::TAU;

/// Coupling at the top of the `osc_rate` range, in radians per step.
const K_MAX: f64 = 2.0;
/// Slowest bank rate, so even a one-pulse pattern keeps moving.
const MIN_RATE: f64 = 0.02;

/// Coupling strength from `osc_rate` (0.125..8 maps to 0..K_MAX).
pub(crate) fn coupling(p: &GenParams) -> f64 {
    let r = p.osc_rate.clamp(0.125, 8.0) as f64;
    (r - 0.125) / (8.0 - 0.125) * K_MAX
}

/// Mean bank rate in turns per step: the requested pulse ratio.
pub(crate) fn rate(p: &GenParams, n: usize) -> f64 {
    if p.pulses == 0 { 0.0 } else { (p.pulses.min(n) as f64 / n as f64).max(MIN_RATE) }
}

/// Per-lane frequency multiplier, spread by `phase`.
fn detune(p: &GenParams, lane: usize) -> f64 {
    let spread = 0.05 + 0.45 * p.phase.clamp(0.0, 1.0) as f64;
    let x = lane as f64 / (TRACKS - 1) as f64 - 0.5;
    1.0 + spread * 2.0 * x
}

/// Order parameter: 1 = whole bank aligned, 0 = spread evenly. Shared with the panel's playhead.
pub(crate) fn order(phases: &[u8; TRACKS]) -> (f32, f32) {
    let (mut x, mut y) = (0.0f32, 0.0f32);
    for &phase in phases {
        let angle = TAU as f32 * phase as f32 / 255.0;
        x += angle.cos();
        y += angle.sin();
    }
    (x / TRACKS as f32, y / TRACKS as f32)
}

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    let k = coupling(p);
    let rate = rate(p, n);

    let mut theta = [0.0f64; TRACKS];
    for tr in 0..TRACKS {
        let init = (p.seed.clamp(0.0, 1.0) as f64
            + p.variation as f64 * 0.137
            + tr as f64 / TRACKS as f64)
            .rem_euclid(1.0);
        theta[tr] = TAU * init;
    }

    let mut score = [[0.0f32; MAX_STEPS]; TRACKS];
    for i in 0..n {
        for tr in 0..TRACKS {
            let prev = theta[tr];
            let left = theta[(tr + TRACKS - 1) % TRACKS];
            let right = theta[(tr + 1) % TRACKS];
            // Nearest-neighbour ring coupling: two `sin` calls per lane, bounded per block.
            let pull = (left - prev).sin() + (right - prev).sin();
            theta[tr] = prev + TAU * rate * detune(p, tr) + k / TRACKS as f64 * pull;
            let turns = theta[tr] / TAU;
            let before = prev / TAU;
            let phase = turns.rem_euclid(1.0) as f32;
            out.phases[i][tr] = (phase * 255.0).round().clamp(0.0, 255.0) as u8;
            // A crossing triggers; missed steps rank by proximity so the count stays exact.
            score[tr][i] = if turns.floor() > before.floor() { 1.0 } else { 0.5 + 0.45 * (1.0 - phase) };
        }
    }

    for tr in 0..TRACKS {
        let wanted = ((p.pulses.min(n) as f32 * p.density.clamp(0.0, 1.0)
            * (0.5 + DENSITY_PROFILE[tr])).round() as usize).min(n);
        if wanted == 0 { continue; }
        top_k(&score[tr], n, wanted, &mut out.hit[tr]);
        for i in 0..n {
            if out.hit[tr][i] {
                let accent = if score[tr][i] >= 1.0 { 1.0 } else { score[tr][i] + 0.05 };
                set_hit(&mut out, tr, i, true, accent);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Mode;

    fn params(steps: usize, pulses: usize, density: f32) -> GenParams {
        GenParams { kind: Mode::Kuramoto, steps, pulses, density, probability: 1.0, ..GenParams::default() }
    }

    #[test]
    fn hits_exactly_the_requested_count_per_lane() {
        for steps in [1usize, 2, 7, 16, 32, 64] {
            for pulses in [1usize, 2, 5, steps] {
                for density in [0.25f32, 0.65, 1.0] {
                    let pat = generate(&params(steps, pulses, density));
                    for tr in 0..TRACKS {
                        let want = ((pulses.min(steps) as f32 * density
                            * (0.5 + DENSITY_PROFILE[tr])).round() as usize).min(steps);
                        let got = pat.hit[tr][..steps].iter().filter(|&&h| h).count();
                        assert_eq!(got, want, "steps={steps} pulses={pulses} density={density} lane={tr}");
                    }
                }
            }
        }
    }

    #[test]
    fn strong_coupling_locks_the_bank() {
        let free = generate(&GenParams { osc_rate: 0.125, phase: 0.0, ..params(64, 64, 1.0) });
        let locked = generate(&GenParams { osc_rate: 8.0, phase: 0.0, ..params(64, 64, 1.0) });
        let strength_at = |pat: &Pattern, i: usize| {
            let (x, y) = order(&pat.phases[i]);
            (x * x + y * y).sqrt()
        };
        // Mid-pattern: an integer pulse ratio wraps back near the start by the last step.
        let (free_r, locked_r) = (strength_at(&free, 32), strength_at(&locked, 32));
        assert!(locked_r > free_r + 0.1,
            "coupling did not lock the bank: free={free_r} locked={locked_r}");
    }

    #[test]
    fn the_published_phases_rotate_with_the_pattern() {
        let p = params(17, 7, 0.82);
        let a = crate::sequence::generate(&p);
        let b = crate::sequence::generate(&GenParams { rotate: 3, ..p });
        for i in 0..17 {
            let src = (i + 17 - 3) % 17;
            assert_eq!(b.phases[i], a.phases[src], "step {i}");
        }
    }

    #[test]
    fn zero_pulses_stop_the_bank() {
        let pat = generate(&params(16, 0, 1.0));
        assert!(pat.hit.iter().all(|lane| lane[..16].iter().all(|h| !h)));
    }
}
