use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    // Start from Euclidean-ish base then mutate.
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0x6E00_0000 + tr as u64);
        for i in 0..n {
            let base = (i * (tr+1)) % (n.max(1)) < n/2;
            let mutate = rng.unit() < (1.0 - p.density.clamp(0.0,1.0));
            let mut hit = if mutate { !base } else { base };
            if n == 1 { hit = true; }
            hit = hit && rng.unit() < p.probability.clamp(0.0,1.0);
            if hit && tr > 0 && (0..tr).all(|k| out.hit[k][i]) { hit = false; }
            let accent = if hit { 0.6 + 0.4*rng.unit() } else { 0.0 };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}