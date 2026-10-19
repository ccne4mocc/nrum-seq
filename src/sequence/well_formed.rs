use super::{set_hit, GenParams, Pattern, TRACKS};

/// Christoffel/Bresenham word: exactly k onsets with inter-onset spacings differing by at most one.
/// The onset test and the panel's slack plot both run on [`super::modular_distance`], so the
/// drawn curve is the curve that produced the hits.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let k = super::pulse_hits(p.pulses, p.density.clamp(0.0, 1.0), tr, n);
        let phase = (tr * n / TRACKS) % n;
        for i in 0..n {
            let pos = (i + n - phase) % n;
            // Anchored at the lane's phase origin (see euclidean.rs): onset j at floor(j*n/k).
            let d = super::modular_distance(pos, k, n);
            let hit = (n - d) % n < k;
            set_hit(&mut out, tr, i, hit, if hit { 1.0 } else { 0.72 });
        }
    }
    out
}