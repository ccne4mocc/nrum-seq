use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

fn is_prime(n: usize) -> bool {
    if n < 2 { return false; }
    if n < 4 { return true; }
    if n % 2 == 0 { return false; }
    let mut d = 3;
    while d * d <= n {
        if n % d == 0 { return false; }
        d += 2;
    }
    true
}

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    let prob = p.probability.clamp(0.0, 1.0);
    let density = p.density.clamp(0.0, 1.0);
    let offset = (p.phase.clamp(0.0, 1.0) * 100.0) as usize;
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0x55_00_00_00 + tr as u64);
        let lane_offset = tr * 7;
        for i in 0..n {
            let num = i + 1 + offset + lane_offset;
            let prime = is_prime(num);
            let hit = prime && rng.unit() < density * prob;
            let accent = if hit { 0.6 + 0.4 * rng.unit() as f32 } else { 0.0 };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}