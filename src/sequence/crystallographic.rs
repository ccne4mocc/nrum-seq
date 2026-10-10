use super::{set_hit, GenParams, Pattern, Rng, TRACKS, MAX_STEPS};

/// Periodic unit cells with symmetry operations inspired by crystallographic space-group
/// families: translations, mirrors, glide/reflection, inversion, and alternating centering.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let order = p.symmetry.clamp(1, 8);
    let unit = (p.steps / order).max(1);
    for tr in 0..TRACKS {
        let pulse_ratio = (p.pulses.max(1) as f32 / p.steps as f32).clamp(0.02, 0.95);
        // Density scales the cutoff directly rather than multiplying into it. Multiplying
        // pushed the threshold onto its clamp floor, so the first non-zero density setting
        // released a dozen hits at once instead of a couple.
        let threshold = (pulse_ratio * (0.55 + 0.45 * p.density.clamp(0.0, 1.0))
            * (0.4 + 0.6 * super::DENSITY_PROFILE[tr])).clamp(0.0, 1.0);
        let mut rng = Rng::new(p.seed, p.variation, 0xC7A1_0000 + tr as u64);
        let mut motif = [false; MAX_STEPS];
        for i in 0..unit {
            motif[i] = rng.unit() < threshold;
        }
        for i in 0..p.steps {
            let cell = i / unit;
            let offset = i % unit;
            let mapped = match (p.symmetry - 1) % 8 {
                0 => offset,                                               // p1: translation
                1 => if cell % 2 == 0 { offset } else { unit - 1 - offset }, // mirror
                2 => if cell % 2 == 0 { offset } else { (unit / 2 + unit - offset) % unit }, // glide
                3 => (offset + unit - (cell % unit)) % unit,               // inversion centre
                4 => (offset + cell % unit) % unit,                       // screw-like phase
                5 => if cell % 2 == 0 { offset } else { (offset + unit / 2) % unit }, // centering
                6 => (unit - 1 - offset + cell % unit) % unit,             // alternating mirror
                _ => (offset + cell * (unit / 2 + 1)) % unit,              // staggered lattice
            };
            let mut hit = motif[mapped.min(unit - 1)];
            if p.symmetry == 8 && cell % 2 == 1 { hit = !hit && threshold > 0.35; }
            set_hit(&mut out, tr, i, hit, if cell % 2 == 0 { 0.95 } else { 0.72 });
        }
    }
    out
}
