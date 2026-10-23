use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

// A 12-step bell pattern (standard African bell). Rotated per lane/variation.
const BELL: [bool; 12] = [true,false,false,true,false,false,true,false,false,true,false,false];

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0xAF31_0000 + tr as u64);
        let rot = rng.below(n);
        for i in 0..n {
            let src = BELL[i % BELL.len()];
            let idx = (i + rot) % n;
            let mut hit = src && rng.unit() < p.probability.clamp(0.0, 1.0);
            if hit && tr > 0 && (0..tr).all(|k| out.hit[k][idx]) { hit = false; }
            let accent = if hit { 0.75 + 0.25 * rng.unit() } else { 0.0 };
            set_hit(&mut out, tr, idx, hit, accent);
        }
    }
    out
}