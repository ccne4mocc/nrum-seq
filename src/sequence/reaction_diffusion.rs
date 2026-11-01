use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0x3D00_0000 + tr as u64);
        for i in 0..n {
            // Simple Gray-Scott-like threshold on a sinusoidal activator.
            let a = (i as f32 / n as f32 * std::f32::consts::TAU * (1.0 + tr as f32 * 0.1) + tr as f32 * 0.7).sin();
            if tr == 0 { out.u[i] = (a + 1.0) * 0.5; }
            let mut hit = a >= (1.0 - p.density.clamp(0.0,1.0)) && rng.unit() < p.probability.clamp(0.0,1.0);
            if hit && tr > 0 && (0..tr).all(|k| out.hit[k][i]) { hit = false; }
            let accent = if hit { 0.6 + 0.4*rng.unit() } else { 0.0 };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}