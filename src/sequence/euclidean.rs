use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

/// Bjorklund-style bucket distribution: lane density scales the requested pulse count, then spaces the resulting hits around the ring.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    for tr in 0..TRACKS {
        let weight = super::DENSITY_PROFILE[tr];
        let pulses = ((p.pulses as f32 * p.density.clamp(0.0, 1.0) * (0.35 + weight)) + 0.5)
            .floor() as usize;
        let pulses = pulses.min(p.steps);
        let mut rng = Rng::new(p.seed, p.variation, 0xE0C1_1D00 + tr as u64);
        for i in 0..p.steps {
            // Anchored: the first onset sits on step 0 and the rest follow at floor(j*n/k),
            // the usual E(k,n) necklace. The old bucket form placed the first onset at the
            // end of its bucket, so e.g. E(4,16) landed on 3,7,11,15 instead of 0,4,8,12.
            let hit = (p.steps - (i * pulses) % p.steps) % p.steps < pulses;
            let accent = if i % 4 == 0 { 1.0 } else { 0.72 + 0.15 * rng.unit() };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}

