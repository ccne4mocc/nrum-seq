use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

fn in_cantor(x: f32, depth: usize) -> bool {
    let mut x = x;
    for _ in 0..depth {
        let t = (x*3.0).fract();
        if t > 1.0/3.0 && t < 2.0/3.0 { return false; }
        x = x*3.0 % 1.0;
    }
    true
}

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0xCA10_0000 + tr as u64);
        let depth = 2 + tr.min(3);
        for i in 0..n {
            let x = i as f32 / n as f32;
            let mut hit = in_cantor(x, depth) && rng.unit() < p.probability.clamp(0.0,1.0)*p.density.clamp(0.0,1.0);
            if hit && tr > 0 && (0..tr).all(|k| out.hit[k][i]) { hit = false; }
            let accent = if hit { 0.7 + 0.3*rng.unit() } else { 0.0 };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}