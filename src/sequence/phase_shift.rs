use super::{rotated_pos, set_hit, GenParams, Pattern, TRACKS};

/// Reich-inspired phase shifting: every lane repeats the same maximally-even pulse train at a
/// different circular offset, creating interlocking copies over one shared pattern length.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    let pulses = ((p.pulses as f32 * p.density.clamp(0.0, 1.0)).round() as usize).min(n);
    let mut base = [false; super::MAX_STEPS];
    let mut bucket = 0usize;
    for i in 0..n {
        bucket += pulses;
        if bucket >= n { base[i] = true; bucket -= n; }
    }
    let phase_span = (n / TRACKS).max(1);
    let user_phase = (p.phase.clamp(0.0, 1.0) * n as f32).round() as usize;
    let seed_phase = (p.seed.clamp(0.0, 1.0) * n as f32).round() as usize;
    for tr in 0..TRACKS {
        let shift = (tr * phase_span + user_phase + seed_phase) % n;
        for i in 0..n {
            let src = rotated_pos(i, shift, n);
            let hit = base[src];
            let accent = if src % 4 == 0 { 1.0 } else if src % 2 == 0 { 0.82 } else { 0.66 };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}
