use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

/// Finite Fibonacci-style L-system. Symbols are expanded with a bounded fixed buffer, then
/// interpreted as a rhythm grammar: A favours low/downbeat voices, B favours upper/offbeat voices.
/// The rule family changes with seed/variation, so each regeneration can explore a related grammar.
pub(super) fn generate(p: &GenParams) -> Pattern {
    const CAP: usize = 1024;
    const A: u8 = 0;
    const B: u8 = 1;
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let mut symbols = [A; CAP];
    let mut len = 1usize;
    let family = ((p.seed.clamp(0.0, 0.999_999) * 4.0) as usize + p.variation as usize) % 4;

    // Parallel rewrites. Every production is finite and the maximum symbol buffer is fixed.
    for _ in 0..12 {
        if len >= p.steps { break; }
        let mut next = [A; CAP];
        let mut next_len = 0usize;
        for &symbol in &symbols[..len] {
            let (production, production_len): ([u8; 3], usize) = match (symbol, family) {
                (A, 0) => ([A, B, A], 2),
                (A, 1) => ([A, A, B], 3),
                (A, 2) => ([B, A, A], 2),
                (A, _) => ([A, B, A], 3),
                (B, 0) => ([A, A, A], 1),
                (B, 1) => ([B, A, A], 2),
                (B, 2) => ([A, B, A], 2),
                _ => ([A, A, A], 1),
            };
            for &token in &production[..production_len] {
                if next_len < CAP { next[next_len] = token; next_len += 1; }
            }
        }
        if next_len == 0 { break; }
        symbols = next;
        len = next_len;
    }

    let pulse_ratio = p.pulses as f32 / p.steps.max(1) as f32;
    for tr in 0..TRACKS {
        let threshold = (pulse_ratio * (0.35 + 0.65 * p.density.clamp(0.0, 1.0))
            * (0.45 + super::DENSITY_PROFILE[tr])).clamp(0.0, 1.0);
        let mut local = Rng::new(p.seed, p.variation, 0x1A57_0000 + tr as u64);
        for i in 0..p.steps {
            let symbol = symbols[i % len];
            let preferred = if tr <= 1 || tr == 6 { A } else { B };
            let grammar_weight = if symbol == preferred { 1.0 } else { 0.55 };
            let metrical = if i % 4 == 0 { 1.0 } else if i % 2 == 0 { 0.84 } else { 0.72 };
            let hit = local.unit() < (threshold * grammar_weight * metrical).clamp(0.0, 1.0);
            set_hit(&mut out, tr, i, hit, if symbol == A { 0.96 } else { 0.72 });
        }
    }
    out
}
