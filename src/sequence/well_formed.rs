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
            // Anchored at the lane's phase origin (see euclidean.rs): onset j at floor(j*n/k).
            let distance = (pos * k) % n;
            let hit = (n - distance) % n < k;
            set_hit(&mut out, tr, i, hit, if hit { 1.0 } else { 0.72 });
        }
    }
    out
}
