use super::{set_hit, GenParams, Pattern, Rng, TRACKS, MAX_STEPS};

/// Weighted syncopation selector. It assigns higher scores to off-beats and late subdivisions,
/// then selects a controlled number of the highest-scoring steps for each lane.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let target = ((p.pulses as f32 * p.density.clamp(0.0, 1.0)
            * (0.45 + super::DENSITY_PROFILE[tr])).round() as usize).min(n);
        let mut scores = [f32::NEG_INFINITY; MAX_STEPS];
        let mut rng = Rng::new(p.seed, p.variation, 0x5A1C_0000u64.wrapping_add(tr as u64));
        for i in 0..n {
            let beat_weight = match i % 4 { 0 => 0.58, 2 => 0.86, _ => 1.28 };
            let lane_shift = (tr * 3) % 4;
            let shifted_weight = match (i + lane_shift) % 4 { 0 => 0.60, 2 => 0.88, _ => 1.24 };
            let jitter = 0.85 + rng.unit() * 0.30;
            scores[i] = beat_weight * shifted_weight * jitter;
        }
        // Fixed-size top-k selection avoids allocations and sorting dependencies on the audio path.
        for _ in 0..target {
            let mut best = 0usize;
            for i in 1..n { if scores[i] > scores[best] { best = i; } }
            let accent = match best % 4 { 0 => 1.0, 2 => 0.84, _ => 0.62 };
            set_hit(&mut out, tr, best, true, accent);
            scores[best] = f32::NEG_INFINITY;
        }
    }
    out
}
