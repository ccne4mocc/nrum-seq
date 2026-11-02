use super::{frac, lane_wanted, set_hit, top_k, GenParams, Pattern, MAX_STEPS, TRACKS, PHI_INV};

/// Henon `a` for a density: 1.0 (periodic orbits) .. 1.4 (the classic chaotic attractor);
/// `b = 0.3`.
pub(crate) fn a_of(density: f32) -> f64 {
    1.0 + density.clamp(0.0, 1.0) as f64 * 0.4
}

/// Henon attractor: `x' = 1 - a*x^2 + y`, `y' = b*x`. The old start `(1.8*phase-0.9,
/// 1.8*seed-0.9)` was often outside the basin, escaped the clamp and played one note forever.
/// Start near the origin, discard a transient and scale x by the real attractor extent (+-1.3).
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    if p.pulses == 0 || p.density <= 0.0 { return out; }
    let a = a_of(p.density);
    let b = 0.3f64;
    let step = |x: f64, y: f64| (1.0 - a * x * x + y, b * x);
    for tr in 0..TRACKS {
        let (seed, phase, var) = (p.seed as f64, p.phase as f64, p.variation as f64);
        // A lane-specific offset keeps the eight voices on different parts of the attractor.
        let lane = tr as f64 / TRACKS as f64;
        let mut x = (frac(seed * 0.7 + phase + var * PHI_INV + lane) - 0.5) * 0.4;
        let mut y = (frac(seed + var * (1.0 - PHI_INV) + phase * 0.7 + lane) - 0.5) * 0.2;
        for _ in 0..64 {
            (x, y) = step(x, y);
            if !(x.abs() < 3.0 && y.abs() < 3.0) { (x, y) = (0.1, 0.05); }
        }
        let mut scores = [0.0f32; MAX_STEPS];
        let mut uxs = [0.5f32; MAX_STEPS];
        let mut prev = x;
        for i in 0..n {
            if i > 0 {
                (x, y) = step(x, y);
                if !(x.abs() < 3.0 && y.abs() < 3.0) { (x, y) = (0.1, 0.05); }
            }
            let ux = ((x + 1.3) / 2.6).clamp(0.0, 1.0) as f32;
            let uy = ((y + 0.4) / 0.8).clamp(0.0, 1.0) as f32;
            scores[i] = if i == 0 { 0.0 } else { (x - prev).abs() as f32 };
            prev = x;
            uxs[i] = ux;
            if tr == 0 {
                out.orbit[i] = ux;
                out.theta[i] = uy;
                out.u[i] = ux;
            }
        }
        let wanted = lane_wanted(p.pulses, p.density, tr, n).max(1).min(n);
        top_k(&scores, n, wanted, &mut out.hit[tr]);
        for i in 0..n {
            if out.hit[tr][i] {
                let accent = 0.58 + 0.42 * uxs[i];
                set_hit(&mut out, tr, i, true, accent);
            }
        }
        // At short loop lengths eight lanes × several peaks can land on one step. Keep the
        // column out of the pattern, but never silence a lane while doing so.
        if n > 1 && tr > 0 {
            for i in 0..n {
                if out.hit[tr][i] && (0..tr).all(|k| out.hit[k][i]) {
                    out.hit[tr][i] = false;
                    out.impact[i][tr] = 0;
                }
            }
            if !out.hit[tr][..n].iter().any(|&h| h) {
                let best = (0..n).min_by_key(|&i| (0..tr).filter(|&k| out.hit[k][i]).count()).unwrap_or(0);
                let accent = 0.58 + 0.42 * uxs[best];
                set_hit(&mut out, tr, best, true, accent);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Mode;

    fn params(steps: usize, density: f32, phase: f32, seed: f32, variation: u32) -> GenParams {
        GenParams {
            kind: Mode::HenonMap, steps, density, phase, seed, variation, pulses: 64,
            probability: 1.0, ..GenParams::default()
        }
    }

    #[test]
    fn no_start_diverges_or_goes_constant() {
        for pi in 0..=10 {
            for si in 0..=10 {
                for density in [0.05f32, 0.5, 1.0] {
                    for variation in [0u32, 3] {
                        let out = generate(&params(64, density, pi as f32 / 10.0, si as f32 / 10.0, variation));
                        let tail: Vec<f32> = (24..64).map(|j| out.orbit[(j + out.rotation) % 64]).collect();
                        let (lo, hi) = tail.iter().fold((1.0f32, 0.0f32), |(l, h), &x| (l.min(x), h.max(x)));
                        assert!(hi - lo > 0.01, "phase {pi} seed {si} d {density}: constant {lo}");
                        assert!(lo > 0.0 && hi < 1.0, "pinned at the scale edge");
                    }
                }
            }
        }
    }

    #[test]
    fn orbit_obeys_the_map_in_unrotated_order() {
        let out = generate(&params(64, 1.0, 0.0, 0.0, 2));
        let a = a_of(1.0);
        let s = out.rotation;
        let at = |j: usize| (
            (out.orbit[(j + s) % 64] as f64 * 2.6 - 1.3),
            (out.theta[(j + s) % 64] as f64 * 0.8 - 0.4),
        );
        for j in 1..64 {
            let ((x0, y0), (x1, y1)) = (at(j - 1), at(j));
            assert!((x1 - (1.0 - a * x0 * x0 + y0)).abs() < 2e-3, "x at {j}");
            assert!((y1 - 0.3 * x0).abs() < 2e-3, "y at {j}");
        }
    }

    #[test]
    fn variation_changes_the_line_and_every_lane_sounds() {
        let key = |v: u32| {
            let out = generate(&params(32, 1.0, 0.0, 0.0, v));
            let mut values: Vec<i32> = out.orbit[..32].iter().map(|x| (*x * 1e4) as i32).collect();
            values.sort_unstable();
            values
        };
        assert_ne!(key(0), key(1));
        for steps in [1usize, 16, 64] {
            let out = generate(&params(steps, 1.0, 0.0, 0.0, 0));
            for tr in 0..TRACKS {
                assert!(out.hit[tr][..steps].iter().any(|&h| h), "steps={steps} lane={tr}");
            }
        }
    }
}
