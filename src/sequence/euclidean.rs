use super::{anchored_onset, rotated_pos, set_hit, GenParams, Pattern, Rng, TRACKS};

/// Onset count for E(k, n): the pulse count scaled by density and the lane's density profile.
/// Shared with the help text so the displayed `k` is the one the generator builds.
pub(crate) fn lane_pulses(pulses: usize, density: f32, tr: usize, n: usize) -> usize {
    let weight = super::DENSITY_PROFILE[tr];
    let k = ((pulses as f32 * density.clamp(0.0, 1.0) * (0.35 + weight)) + 0.5).floor() as usize;
    k.min(n)
}

/// Bjorklund-style bucket distribution: lane density scales the requested pulse count, then spaces the resulting hits around the ring.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let pulses = lane_pulses(p.pulses, p.density, tr, n);
        let mut rng = Rng::new(p.seed, p.variation, 0xE0C1_1D00 + tr as u64);
        // Each lane anchors its word on its own phase origin; lane 0 keeps the downbeat.
        let phase = (tr * n / TRACKS) % n;
        for i in 0..n {
            let pos = rotated_pos(i, phase, n);
            // Anchored E(k,n): the first onset sits on the lane's phase origin, the rest follow
            // at floor(j*n/k).
            let hit = anchored_onset(pos, pulses, n);
            let accent = if i % 4 == 0 { 1.0 } else { 0.72 + 0.15 * rng.unit() };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}
