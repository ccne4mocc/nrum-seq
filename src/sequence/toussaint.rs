use super::{rotated_pos, set_hit, GenParams, Pattern, TRACKS};

/// Toussaint-inspired modular-distance rhythm. Integer boundary crossings select a maximally-even
/// onset word; the seed selects a circular origin, and modular distance shapes the onset accents.
/// The boundary crossings are just `modular_distance` wrapping, which the panel plots directly.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    let seed_offset = (p.seed.clamp(0.0, 1.0) * n as f32).floor() as usize % n;
    for tr in 0..TRACKS {
        let lane_offset = (seed_offset + tr * n / TRACKS) % n;
        let k = super::pulse_hits(p.pulses, p.density.clamp(0.0, 1.0), tr, n);
        for i in 0..n {
            let pos = rotated_pos(i, lane_offset, n);
            let before = pos * k / n;
            let after = (pos + 1) * k / n;
            let hit = after != before;
            let distance = super::modular_distance(pos, k, n);
            let accent = (0.62 + 0.38 * (1.0 - distance as f32 / n as f32)).clamp(0.05, 1.0);
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}