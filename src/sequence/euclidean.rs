use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

/// Bjorklund-style bucket distribution: lane density scales the requested pulse count, then spaces the resulting hits around the ring.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    for tr in 0..TRACKS {
        let weight = super::DENSITY_PROFILE[tr];
        let pulses = ((p.pulses as f32 * p.density.clamp(0.0, 1.0) * (0.35 + weight)) + 0.5)
            .floor() as usize;
        let pulses = pulses.min(p.steps);
        let mut bucket = 0usize;
        let mut rng = Rng::new(p.seed, p.variation, 0xE0C1_1D00 + tr as u64);
        for i in 0..p.steps {
            bucket += pulses;
            let hit = bucket >= p.steps;
            if hit { bucket -= p.steps; }
            let accent = if i % 4 == 0 { 1.0 } else { 0.72 + 0.15 * rng.unit() };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}

