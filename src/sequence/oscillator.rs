use super::{set_hit, GenParams, Pattern, TRACKS};

/// Number of waveform shapes the oscillator mode offers. The host enum in `lib.rs` mirrors this
/// count and order; the test in `lib.rs` pins the two together, and that test is also where the
/// stable host ids are listed.
pub(crate) const WAVE_COUNT: usize = 8;

/// Waveform value in 0..=1 at a normalized phase. Shared with the panel so the drawn curve is
/// the curve the generator ranked.
///
/// The shape decides *where* the onsets land: the generator keeps the highest-scoring steps, so a
/// different curve moves the hits to a different part of the cycle.
pub(crate) fn value(wave: usize, phase: f32) -> f32 {
    let phase = (phase as f64).rem_euclid(1.0) as f32;
    match wave % WAVE_COUNT {
        0 => 0.5 + 0.5 * (std::f64::consts::TAU as f32 * phase).sin(),
        1 => 1.0 - (2.0 * phase - 1.0).abs(),
        2 => phase,
        3 => if phase < 0.5 { 1.0 } else { 0.0 },
        // Narrow gate: the onsets bunch into the first quarter of every cycle.
        4 => if phase < 0.25 { 1.0 } else { 0.0 },
        // Exponential fall: the onsets bunch at the start of every cycle.
        5 => (-3.2 * phase as f64).exp() as f32,
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

/// Trigger cutoff for a lane, on the same 0..=1 scale as [`value`]. Shared with the panel so the
/// drawn threshold line is the threshold the generator applied.
pub(crate) fn threshold(p: &GenParams) -> f32 {
    let pulse_ratio = p.pulses as f32 / p.steps as f32;
    (pulse_ratio * (0.25 + 0.75 * p.density.clamp(0.0, 1.0)) * (0.5 + super::DENSITY_PROFILE[0])).clamp(0.01, 0.99)
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
        // Rank selection, same rule as the chaos generator: a step hits when fewer than `wanted`
        // steps outrank it, with earlier steps winning ties. Exact count, no sort, no alloc.
        if wanted > 0 {
            for i in 0..p.steps {
                let mut rank = 0usize;
                for j in 0..p.steps {
                    if wave[j] > wave[i] || (wave[j] == wave[i] && j < i) { rank += 1; }
                }
                let hit = rank < wanted;
                set_hit(&mut out, tr, i, hit, 0.55 + 0.45 * wave[i]);
            }
        }
    }
    out
}
