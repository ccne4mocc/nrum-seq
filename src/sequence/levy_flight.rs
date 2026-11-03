use super::{fold01, frac, lane_wanted, set_hit, top_k, GenParams, Pattern, Rng, MAX_STEPS, TRACKS};

/// Pareto tail index: 2.0 (Gaussian-like) at 0 down to 0.5 (infinite mean) at 1.
pub(crate) fn tail_alpha(density: f32) -> f32 {
    2.0 - 1.5 * density.clamp(0.0, 1.0)
}

/// Minimum step (the Pareto scale) for a density.
pub(crate) fn step_min(density: f32) -> f32 {
    0.02 + 0.06 * density.clamp(0.0, 1.0)
}

/// One Levy step length: Pareto `s_min * U^(-1/alpha)`, so `P(S > s) = (s_min/s)^alpha`.
/// Capped at eight ranges to keep f32 precision.
pub(crate) fn step_len(u: f32, density: f32) -> f32 {
    let alpha = tail_alpha(density) as f64;
    ((step_min(density) as f64) * (u.max(1e-6) as f64).powf(-1.0 / alpha)).min(8.0) as f32
}

/// Heavy-tailed walk: most steps are small, occasional jumps dominate. Jumps become onsets;
/// `phase` is the starting position, not a drift.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    if p.pulses == 0 || p.density <= 0.0 { return out; }
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0x1E7E_0001 + tr as u64);
        let mut x = frac(0.5 + p.phase as f64 + tr as f64 / TRACKS as f64);
        let mut scores = [0.0f32; MAX_STEPS];
        let mut walk = [0.0f32; MAX_STEPS];
        for i in 0..n {
            let mut len = 0.0f32;
            if i > 0 {
                let draw = rng.unit();
                let sign = if rng.unit() < 0.5 { -1.0 } else { 1.0 };
                len = step_len(draw, p.density);
                x = fold01(x + sign as f64 * len as f64);
            }
            scores[i] = len;
            walk[i] = x as f32;
            if tr == 0 {
                out.orbit[i] = x as f32;
                out.theta[i] = len;
                out.u[i] = x as f32;
            }
        }
        let wanted = lane_wanted(p.pulses, p.density, tr, n).max(1).min(n);
        top_k(&scores, n, wanted, &mut out.hit[tr]);
        for i in 0..n {
            if out.hit[tr][i] {
                let accent = 0.58 + 0.42 * walk[i];
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
            kind: Mode::LevyFlight, steps, density, variation, pulses: 16, probability: 1.0,
            ..GenParams::default()
        }
    }

    #[test]
    fn step_lengths_follow_a_pareto_tail_and_are_bounded() {
        for density in [0.2f32, 0.65, 1.0] {
            let (alpha, s_min) = (tail_alpha(density) as f64, step_min(density) as f64);
            let mut rng = Rng::new(0.3, 0, 5);
            let mut v: Vec<f64> = (0..50_000).map(|_| step_len(rng.unit(), density) as f64).collect();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            assert!(v[0] >= s_min * 0.999, "never below the minimum step");
            assert!(*v.last().unwrap() <= 8.0 + 1e-6, "capped at 8 ranges");
            let median = v[v.len() / 2];
            assert!((median / (s_min * 2f64.powf(1.0 / alpha)) - 1.0).abs() < 0.05, "median {median} d={density}");
            for k in [10.0f64, 30.0] {
                if s_min * k < 8.0 {
                    let got = v.iter().filter(|&&s| s > s_min * k).count() as f64 / v.len() as f64;
                    let want = k.powf(-alpha);
                    assert!((got / want - 1.0).abs() < 0.12, "d={density} k={k}: {got} vs {want}");
                }
            }
        }
    }

    #[test]
    fn heavy_tail_means_big_jumps_dwarf_the_typical_step() {
        let mut rng = Rng::new(0.3, 0, 6);
        let mut v: Vec<f32> = (0..20_000).map(|_| step_len(rng.unit(), 0.65)).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(v[v.len() - 1] / v[v.len() / 2] > 20.0, "max/median only {}", v[v.len() - 1] / v[v.len() / 2]);
    }

    #[test]
    fn variation_changes_the_walk_and_every_lane_sounds() {
        let key = |v: u32| {
            let out = generate(&params(64, 0.8, v));
            let mut values: Vec<i32> = out.orbit[..64].iter().map(|x| (x * 1e5) as i32).collect();
            values.sort_unstable();
            values
        };
        assert_ne!(key(0), key(1));
        for steps in [1usize, 16, 64] {
            let out = generate(&params(steps, 1.0, 0));
            for tr in 0..TRACKS {
                assert!(out.hit[tr][..steps].iter().any(|&h| h), "steps={steps} lane={tr}");
            }
        }
    }
}
