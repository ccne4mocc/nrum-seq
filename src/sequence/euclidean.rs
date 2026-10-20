use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

/// Bjorklund-style bucket distribution: lane density scales the requested pulse count, then spaces the resulting hits around the ring.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let weight = super::DENSITY_PROFILE[tr];
        let pulses = ((p.pulses as f32 * p.density.clamp(0.0, 1.0) * (0.35 + weight)) + 0.5)
            .floor() as usize;
        let pulses = pulses.min(n);
        let mut rng = Rng::new(p.seed, p.variation, 0xE0C1_1D00 + tr as u64);
        // Each lane anchors its word on its own phase origin, the rule `well_formed` already
        // uses. Anchoring every lane on step 0 put all eight lanes on the same step in every
        // variation: `variation` only rotates the whole pattern, so the column moved but never
        // went away. Lane 0 keeps the downbeat anchor so the kick still plays on beat one.
        let phase = (tr * n / TRACKS) % n;
        for i in 0..n {
            let pos = (i + n - phase) % n;
            // Anchored: the first onset sits on the lane's phase origin and the rest follow at
            // floor(j*n/k), the usual E(k,n) necklace. The old bucket form placed the first
            // onset at the end of its bucket, so e.g. E(4,16) landed on 3,7,11,15.
            let hit = (n - (pos * pulses) % n) % n < pulses;
            let accent = if i % 4 == 0 { 1.0 } else { 0.72 + 0.15 * rng.unit() };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}
