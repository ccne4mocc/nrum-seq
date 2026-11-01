use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    // irrational slope from phase (golden-ish), clamped away from 0/1
    let mut alpha = 0.3 + 0.4 * p.phase.clamp(0.0, 1.0) as f64;
    if alpha < 0.05 { alpha = 0.05; }
    if alpha > 0.95 { alpha = 0.95; }
    let prob = p.probability.clamp(0.0, 1.0);
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0x57_00_00_00 + tr as u64);
        let phase_off = (tr as f64) * 0.125;
        let mut prev = (phase_off * alpha).floor() as i64;
        let mut any = false;
        for i in 0..n {
            let cur = ((i as f64 + 1.0 + phase_off) * alpha).floor() as i64;
            let mut hit = if n == 1 {
                rng.unit() < p.density.clamp(0.0, 1.0) * prob
            } else {
                (cur != prev || i == 0) && rng.unit() < p.density.clamp(0.0, 1.0) * prob
            };
            if hit && tr > 0 && (0..tr).all(|k| out.hit[k][i]) { hit = false; }
            if hit { any = true; }
            prev = cur;
            let accent = if hit { 0.6 + 0.4 * rng.unit() as f32 } else { 0.0 };
            set_hit(&mut out, tr, i, hit, accent);
        }
        if !any && prob > 0.0 {
            set_hit(&mut out, tr, 0, true, 0.8);
        }
    }
    out
}