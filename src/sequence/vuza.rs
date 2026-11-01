use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    // modulus from ca_rule, clamped to a sane range
    let modulus = (p.ca_rule as usize).clamp(8, 32);
    let prob = p.probability.clamp(0.0, 1.0);
    let density = p.density.clamp(0.0, 1.0);
    // Build a seeded subset A of Z_modulus (the canon's "attack" set).
    let mut a_set = [false; 64];
    let mut rng = Rng::new(p.seed, p.variation, 0x56_00_00_00);
    let a_size = ((modulus as f32) * density).round().max(1.0) as usize;
    let mut placed = 0usize;
    let mut candidate = 0usize;
    while placed < a_size && candidate < modulus {
        if rng.unit() < 0.5 || placed + (modulus - candidate) <= a_size {
            a_set[candidate] = true;
            placed += 1;
        }
        candidate += 1;
    }
    // Ensure at least one
    if placed == 0 { a_set[0] = true; }
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0x56_00_00_00 + tr as u64);
        let lane_shift = tr * (modulus / TRACKS.max(1)).max(1);
        for i in 0..n {
            let residue = (i + lane_shift) % modulus;
            let hit = a_set[residue] && rng.unit() < prob;
            let accent = if hit { 0.6 + 0.4 * rng.unit() as f32 } else { 0.0 };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}