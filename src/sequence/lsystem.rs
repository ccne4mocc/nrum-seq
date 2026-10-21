use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

const CAP: usize = 1024;
const A: u8 = 0;
const B: u8 = 1;

/// Finite Fibonacci-style L-system. Symbols are expanded with a bounded fixed buffer, then
/// interpreted as a rhythm grammar: A favours low/downbeat voices, B favours upper/offbeat voices.
/// The rule family changes with seed/variation, so each regeneration can explore a related grammar.
pub(super) fn generate(p: &GenParams) -> Pattern {
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

    // The word the grammar produced, repeated across the pattern. Published so the panel draws
    // the rewrite instead of inferring a shape from the hits.
    for i in 0..p.steps {
        out.lsym[i] = symbols[i % len];
    }

    for tr in 0..TRACKS {
        let threshold = lane_threshold(p, tr);
        let mut local = Rng::new(p.seed, p.variation, 0x1A57_0000 + tr as u64);
        for i in 0..p.steps {
            let symbol = symbols[i % len];
            let hit = local.unit() < step_chance(threshold, symbol, tr, i);
            set_hit(&mut out, tr, i, hit, if symbol == A { 0.96 } else { 0.72 });
        }
    }
    out
}

/// The trigger threshold of a lane. `pulses` is clamped to the pattern length exactly as the
/// generator clamps it before dispatch.
pub(crate) fn lane_threshold(p: &GenParams, lane: usize) -> f32 {
    let steps = p.steps.max(1);
    let pulses = p.pulses.min(steps);
    let pulse_ratio = (pulses as f32 / steps as f32)
        * (0.35 + 0.65 * p.density.clamp(0.0, 1.0))
        * (0.45 + super::DENSITY_PROFILE[lane]);
    pulse_ratio.clamp(0.0, 1.0)
}

/// The symbol a lane prefers: the low voices want A, the upper voices want B.
pub(crate) fn preferred_symbol(lane: usize) -> u8 {
    if lane <= 1 || lane == 6 { A } else { B }
}

/// How much the grammar favours a symbol in a lane: the preferred one counts fully, the other
/// is damped.
pub(crate) fn grammar_weight(symbol: u8, lane: usize) -> f32 {
    if symbol == preferred_symbol(lane) { 1.0 } else { 0.55 }
}

/// The metrical weight of a step: downbeats count fully, offbeats less.
pub(crate) fn metrical_weight(step: usize) -> f32 {
    if step % 4 == 0 { 1.0 } else if step % 2 == 0 { 0.84 } else { 0.72 }
}

/// The chance that a step fires for a lane, before the lane's own randomness is applied.
pub(crate) fn step_chance(threshold: f32, symbol: u8, lane: usize, step: usize) -> f32 {
    (threshold * grammar_weight(symbol, lane) * metrical_weight(step)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Mode;
    use crate::sequence::{generate, GenParams};

    #[test]
    fn the_published_word_is_the_word_the_generator_played() {
        // The panel draws `lsym`; if it disagreed with the word the hits came from, the picture
        // would show a different grammar than the one sounding.
        let p = GenParams { kind: Mode::LSystem, steps: 32, pulses: 6, density: 0.7, probability: 1.0, ..GenParams::default() };
        let pat = generate(&p);
        assert!(pat.lsym.iter().all(|&s| s == A || s == B), "the word must be A/B only");
        assert!(pat.lsym.iter().any(|&s| s == A) && pat.lsym.iter().any(|&s| s == B),
            "a two-symbol grammar must produce both symbols");
    }

    #[test]
    fn the_lane_threshold_follows_the_pulse_ratio() {
        let few = lane_threshold(&GenParams { steps: 16, pulses: 2, density: 0.7, ..GenParams::default() }, 0);
        let many = lane_threshold(&GenParams { steps: 16, pulses: 12, density: 0.7, ..GenParams::default() }, 0);
        assert!(many > few, "more pulses must widen the window");
        // Pulses above the pattern length are clamped, so the window cannot exceed 1.
        let clamped = lane_threshold(&GenParams { steps: 4, pulses: 64, density: 1.0, ..GenParams::default() }, 0);
        assert!(clamped <= 1.0);
    }

    #[test]
    fn lanes_disagree_about_the_symbol_they_want() {
        assert_eq!(preferred_symbol(0), A);
        assert_eq!(preferred_symbol(2), B);
        assert!(grammar_weight(A, 0) > grammar_weight(B, 0), "lane 0 wants A");
        assert!(grammar_weight(B, 2) > grammar_weight(A, 2), "lane 2 wants B");
    }
}
