use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    let omega = p.phase.clamp(0.0, 1.0) as f64;
    let k = (p.density.clamp(0.0, 1.0) as f64) * 4.0;
    let prob = p.probability.clamp(0.0, 1.0);
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0xA7_00_00_00 + tr as u64);
        let detune = (rng.unit() * 0.1) as f64;
        let mut theta = (tr as f64) / (TRACKS as f64);
        for i in 0..n {
            theta = theta + omega + detune + (k / (2.0 * std::f64::consts::PI)) * (2.0 * std::f64::consts::PI * theta).sin();
            // keep in [0,1)
            theta = theta - theta.floor();
            let mut hit = rng.unit() < (p.density.clamp(0.0, 1.0) * prob).clamp(0.0, 1.0);
            if hit && tr > 0 && (0..tr).all(|k| out.hit[k][i]) { hit = false; }
            let accent = if hit { 0.6 + 0.4 * rng.unit() as f32 } else { 0.0 };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    // record orbit
    let mut theta = 0.0f64;
    for i in 0..n {
        theta = theta + omega + (k / (2.0 * std::f64::consts::PI)) * (2.0 * std::f64::consts::PI * theta).sin();
        theta = theta - theta.floor();
        out.orbit[i] = theta as f32;
    }
    out
}