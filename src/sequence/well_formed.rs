use super::{set_hit, GenParams, Pattern, TRACKS};

/// Christoffel/Bresenham word: exactly k onsets with inter-onset spacings differing by at most one.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let requested = if p.pulses == 0 { 0 } else { (p.pulses + tr % 3).min(n) };
        let k = ((requested as f32 * p.density.clamp(0.0, 1.0)).round() as usize).min(n);
        let phase = (tr * n / TRACKS) % n;
        for i in 0..n {
            let pos = (i + n - phase) % n;
            let hit = ((pos + 1) * k / n) != (pos * k / n);
            let distance = (pos * k) % n;
            set_hit(&mut out, tr, i, hit, if distance < k.max(1) { 1.0 } else { 0.72 });
        }
    }
    out
}
