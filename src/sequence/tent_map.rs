use super::{frac, lane_wanted, set_hit, top_k, GenParams, Pattern, MAX_STEPS, TRACKS, PHI_INV};

/// Slope for a density: 1.0 (ordered) .. 1.99 (fully chaotic; exactly 2 collapses in floating
/// point).
pub(crate) fn mu_of(density: f32) -> f64 {
    1.0 + density.clamp(0.0, 1.0) as f64 * 0.99
}

/// Tent map `x' = mu * min(x, 1 - x)`. The orbit starts on the attractor after a transient, so a
/// loop no longer opens on the old exponential ramp, and `variation` moves it.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    if p.pulses == 0 || p.density <= 0.0 { return out; }
    let mu = mu_of(p.density);
    let (lo, hi) = (mu * (1.0 - mu / 2.0), mu / 2.0);
    let f = |x: f64| mu * x.min(1.0 - x);
    for tr in 0..TRACKS {
        let mut x = 0.05 + 0.9 * frac(
            p.seed as f64 + p.phase as f64 + p.variation as f64 * PHI_INV
                + tr as f64 / TRACKS as f64,
        );
        for _ in 0..32 { x = f(x); }
        let mut scores = [0.0f32; MAX_STEPS];
        let mut norm = [0.5f32; MAX_STEPS];
        for i in 0..n {
            if i > 0 { x = f(x); }
            scores[i] = x as f32;
            norm[i] = if hi - lo > 1e-3 { ((x - lo) / (hi - lo)).clamp(0.0, 1.0) as f32 } else { 0.5 };
            if tr == 0 {
                out.orbit[i] = x as f32;
                out.u[i] = norm[i];
                out.theta[i] = norm[i];
            }
        }
        let wanted = lane_wanted(p.pulses, p.density, tr, n).max(1).min(n);
        top_k(&scores, n, wanted, &mut out.hit[tr]);
        for i in 0..n {
            if out.hit[tr][i] {
                // Accent follows the normalized orbit value, not the raw map value, so the
                // whole attractor interval is audible at small mu too.
                let accent = 0.62 + 0.38 * norm[i];
                set_hit(&mut out, tr, i, true, accent);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Mode;

    fn params(steps: usize, density: f32, variation: u32) -> GenParams {
        GenParams {
            kind: Mode::TentMap, steps, density, variation, pulses: 32, probability: 1.0,
            ..GenParams::default()
        }
    }

    #[test]
    fn the_loop_does_not_open_on_an_exponential_ramp() {
        for density in [0.3f32, 0.65, 1.0] {
            let out = generate(&params(32, density, 0));
            let first: Vec<f32> = (0..8).map(|j| out.orbit[(j + out.rotation) % 32]).collect();
            assert!(first[0] > 0.02 || first.iter().any(|&u| u > 0.5), "{first:?}");
            let ups = first.windows(2).filter(|w| w[1] > w[0]).count();
            assert!(ups < 7, "d={density}: still a monotone ramp {first:?}");
        }
    }

    #[test]
    fn variation_changes_the_orbit_and_the_normalized_range_is_used() {
        let notes = |v: u32| {
            let mut out: Vec<i64> = generate(&params(32, 0.65, v)).orbit[..32].iter().map(|x| (*x * 1e5) as i64).collect();
            out.sort_unstable();
            out
        };
        assert_ne!(notes(0), notes(1));
        for density in [0.2f32, 0.65, 1.0] {
            let out = generate(&params(256, density, 0));
            let (lo, hi) = (0..256).fold((1.0f32, 0.0f32), |(l, h), i| (l.min(out.u[i]), h.max(out.u[i])));
            assert!(lo < 0.1 && hi > 0.9, "d={density}: pitch only covers {lo}..{hi}");
        }
    }

    #[test]
    fn orbit_obeys_the_map() {
        let out = generate(&params(32, 0.65, 0));
        let mu = mu_of(0.65);
        let s = out.rotation;
        for j in 1..32 {
            let (a, b) = (out.orbit[(j - 1 + s) % 32] as f64, out.orbit[(j + s) % 32] as f64);
            assert!((b - mu * a.min(1.0 - a)).abs() < 2e-5, "step {j}");
        }
    }
}
