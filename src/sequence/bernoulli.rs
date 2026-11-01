use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0xBE30_0000 + tr as u64);
        let prob = (p.density.clamp(0.0, 1.0) * p.probability.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        for i in 0..n {
            let draw = rng.unit();
            let mut hit = draw < prob;
            // Avoid every lane firing on the same step (structural column).
            if hit && tr > 0 && (0..tr).all(|k| out.hit[k][i]) {
                hit = false;
            }
            if tr == 0 { out.u[i] = draw; }
            let accent = if hit { 0.6 + 0.4 * rng.unit() } else { 0.0 };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}