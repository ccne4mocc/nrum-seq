use super::{set_hit, top_k, GenParams, Pattern, TRACKS};

/// Number of waveform shapes; the host enum in `lib.rs` mirrors this count and order.
pub(crate) const WAVE_COUNT: usize = 8;

/// Waveform value in 0..=1 at a normalized phase, shared with the panel. The generator keeps the
/// highest-scoring steps, so the shape decides where onsets land.
pub(crate) fn value(wave: usize, phase: f32) -> f32 {
    let phase = (phase as f64).rem_euclid(1.0) as f32;
    match wave % WAVE_COUNT {
        0 => 0.5 + 0.5 * (std::f64::consts::TAU as f32 * phase).sin(),
        1 => 1.0 - (2.0 * phase - 1.0).abs(),
        2 => phase,
        3 => if phase < 0.5 { 1.0 } else { 0.0 },
        // Narrow gate: the onsets bunch into the first quarter of every cycle.
        4 => if phase < 0.25 { 1.0 } else { 0.0 },
        // Exponential fall: the onsets bunch at the start of every cycle. Normalized so the
        // wave reaches both 0 and 1 like the others, which keeps LFO depth ranges exact.
        5 => {
            const K: f32 = 3.2;
            let tail = (-K).exp();
            ((-K * phase).exp() - tail) / (1.0 - tail)
        }
        // Full-wave rectified: two humps per cycle instead of one.
        6 => (std::f64::consts::TAU as f32 * phase).sin().abs(),
        // Deterministic hash of the phase: the onsets scatter instead of clustering.
        _ => hash01(phase),
    }
    .clamp(0.0, 1.0)
}

/// A stable pseudo-random value in 0..=1 for a phase. The same phase always gives the same
/// value, so the drawn curve and the generator agree.
fn hash01(phase: f32) -> f32 {
    let x = (phase as f64 * 12.9898).sin() * 43758.5453;
    (x - x.floor()) as f32
}

/// Quantized oscillator: phase + rate are explicit parameters, wave selects the shape of the
/// curve the steps are ranked by. The waveform orders the steps and a density-derived rank
/// cutoff keeps exactly the requested number.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let rate = p.osc_rate.clamp(0.125, 8.0) as f64;
    for tr in 0..TRACKS {
        let wanted = ((p.pulses as f32 * p.density.clamp(0.0, 1.0)
            * (0.5 + super::DENSITY_PROFILE[tr])).round() as usize).min(p.steps);
        let mut wave = [0.0f32; super::MAX_STEPS];
        for i in 0..p.steps {
            wave[i] = value(p.osc_wave, (i as f64 / p.steps as f64 * rate + p.phase as f64 + tr as f64 * 0.071).rem_euclid(1.0) as f32);
        }
        // Publish lane 0's ranked curve so the panel draws the samples the generator ranked,
        // instead of re-deriving an unrotated waveform from the parameters. The shared
        // post-transform rotates `orbit` with `hit`, so the drawn curve tracks Generate.
        if tr == 0 {
            out.orbit[..p.steps].copy_from_slice(&wave[..p.steps]);
        }
        // In-place top-k selection, same tie rule as before: higher wave value first, earlier
        // step wins ties. Exact count, no allocation.
        if wanted > 0 {
            top_k(&wave, p.steps, wanted, &mut out.hit[tr]);
            for i in 0..p.steps {
                if out.hit[tr][i] { set_hit(&mut out, tr, i, true, 0.55 + 0.45 * wave[i]); }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LFO depth is a share of the target range, so every deterministic waveform must actually
    /// reach both endpoints; a wave that stops short would cap a 100% sweep below 100%.
    #[test]
    fn deterministic_waves_reach_both_endpoints() {
        for wave in 0..7 {
            let mut lo = f32::MAX;
            let mut hi = f32::MIN;
            for k in 0..10_000 {
                let v = value(wave, k as f32 / 10_000.0);
                lo = lo.min(v);
                hi = hi.max(v);
            }
            assert!(lo <= 1e-4, "wave {wave} bottoms at {lo}");
            assert!(hi >= 1.0 - 1e-4, "wave {wave} tops at {hi}");
        }
    }
}
