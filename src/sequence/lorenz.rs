use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0x1032_0000 + tr as u64);
        // Lorenz orbit sampled; onset on local z peaks.
        let mut x = 0.1 + tr as f32 * 0.05;
        let mut y = 0.0;
        let mut z = 0.0;
        let sigma = 10.0;
        let rho = 28.0;
        let beta = 8.0/3.0;
        let dt = 0.01;
        let mut prev_z = z;
        for i in 0..n {
            for _ in 0..10 {
                let dx = sigma*(y-x);
                let dy = x*(rho-z)-y;
                let dz = x*y - beta*z;
                x += dx*dt; y += dy*dt; z += dz*dt;
            }
            let peak = z > prev_z && z > 1.0 + tr as f32 * 0.5;
            prev_z = z;
            let mut hit = (peak || i == 0) && rng.unit() < p.probability.clamp(0.0,1.0)*p.density.clamp(0.0,1.0);
            if hit && tr > 0 && (0..tr).all(|k| out.hit[k][i]) { hit = false; }
            let accent = if hit { (z.abs()/30.0).min(1.0) } else { 0.0 };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}