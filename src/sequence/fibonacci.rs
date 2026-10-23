use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0xFB00_0000 + tr as u64);
        // Sturmian word: floor((i+1)*alpha) - floor(i*alpha)
        let alpha = (p.density.clamp(0.01,0.99) * (0.5 + tr as f32 * 0.05)) as f64;
        let mut prev = 0i64;
        let mut any = false;
        for i in 0..n {
            let cur = ((i+1) as f64 * alpha).floor() as i64;
            let mut hit = (cur != prev || i == 0) && rng.unit() < p.probability.clamp(0.0,1.0);
            if hit && tr > 0 && (0..tr).all(|k| out.hit[k][i]) { hit = false; }
            prev = cur;
            if hit { any = true; }
            let accent = if hit { 0.7 + 0.3*rng.unit() } else { 0.0 };
            set_hit(&mut out, tr, i, hit, accent);
        }
        if !any { set_hit(&mut out, tr, 0, true, 0.7); }
    }
    out
}