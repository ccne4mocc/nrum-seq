use super::{rotated_pos, set_hit, GenParams, Pattern, TRACKS};

/// Simultaneous integer-ratio pulse trains. Every lane spans the same pattern duration, but uses a
/// different rational pulse ratio (for example 3:2, 5:4, or 2:3) to create true polyrhythmic layers.
pub(super) fn generate(p: &GenParams) -> Pattern {
    const RATIOS: [(usize, usize); TRACKS] = [
        (1, 1), (3, 2), (2, 1), (5, 2), (4, 3), (2, 3), (5, 4), (3, 4),
    ];
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    let seed_phase = (p.seed.clamp(0.0, 1.0) * n as f32).round() as usize % n;
    for tr in 0..TRACKS {
        let (num, den) = RATIOS[tr];
        let scaled = (p.pulses.saturating_mul(num) + den / 2) / den;
        let k = ((scaled as f32 * p.density.clamp(0.0, 1.0)).round() as usize).min(n);
        let phase = (seed_phase + tr * n / TRACKS) % n;
        for i in 0..n {
            let pos = rotated_pos(i, phase, n);
            let hit = ((pos + 1) * k / n) != (pos * k / n);
            let accent = if pos % 4 == 0 { 1.0 } else if pos % 2 == 0 { 0.82 } else { 0.64 };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}
