use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    // Base Euclidean-ish hits on lane 0, then phase-shifted copies.
    let pulses = p.pulses.min(n).max(1);
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0xCA10_0000 + tr as u64);
        let shift = (tr * n / TRACKS) % n;
        for i in 0..n {
            let base = ((i * pulses) / n) != (((i + 1) * pulses) / n);
            let idx = (i + shift) % n;
            let mut hit = base && rng.unit() < p.probability.clamp(0.0, 1.0) * p.density.clamp(0.0, 1.0);
            if hit && tr > 0 && (0..tr).all(|k| out.hit[k][idx]) { hit = false; }
            let accent = if hit { 0.6 + 0.4 * rng.unit() } else { 0.0 };
            set_hit(&mut out, tr, idx, hit, accent);
        }
    }
    out
}