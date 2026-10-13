use super::{frac, set_hit, GenParams, Pattern, TRACKS};
use std::f64::consts::TAU;

/// Quantized oscillator: phase + rate are explicit parameters, wave selects sine/triangle/saw/square.
/// The waveform picks *which* steps are strongest; the cutoff then keeps exactly the
/// requested share of them, so density is a ramp instead of a burst.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let rate = p.osc_rate.clamp(0.125, 8.0) as f64;
    for tr in 0..TRACKS {
        let wanted = ((p.pulses as f32 * p.density.clamp(0.0, 1.0)
            * (0.5 + super::DENSITY_PROFILE[tr])).round() as usize).min(p.steps);
        let mut wave = [0.0f32; super::MAX_STEPS];
        for i in 0..p.steps {
            let phase = frac(i as f64 / p.steps as f64 * rate + p.phase as f64 + tr as f64 * 0.071);
            wave[i] = match p.osc_wave % 4 {
                0 => (0.5 + 0.5 * (TAU * phase).sin()) as f32,
                1 => (1.0 - (2.0 * phase - 1.0).abs()) as f32,
                2 => phase as f32,
                _ => if phase < 0.5 { 1.0 } else { 0.0 },
            };
        }
        // Rank selection: a step hits when fewer than `wanted` steps outrank it, where
        // "outranks" means a higher value, or an equal value at an earlier step. That gives
        // exactly `wanted` hits even for square waves and symmetric sines (whose ties a plain
        // `>= cutoff` test lets through in bulk), needs no sort, and cannot allocate.
        // Cost is O(steps^2) = 4096 comparisons per lane at MAX_STEPS.
        if wanted > 0 {
            for i in 0..p.steps {
                let mut rank = 0usize;
                for j in 0..p.steps {
                    if wave[j] > wave[i] || (wave[j] == wave[i] && j < i) { rank += 1; }
                }
                set_hit(&mut out, tr, i, rank < wanted, 0.55 + 0.45 * wave[i]);
            }
        }
    }
    out
}
