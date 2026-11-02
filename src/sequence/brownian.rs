use super::{fold01, frac, lane_wanted, set_hit, top_k, GenParams, Pattern, Rng, MAX_STEPS, TRACKS};

/// Gaussian increment with unit variance: Irwin-Hall sum of four uniforms. A sum of two was
/// triangular (kurtosis 2.4) and its spread did not match the `sigma` the panel prints.
fn gaussian(rng: &mut Rng) -> f64 {
    let s = rng.unit() as f64 + rng.unit() as f64 + rng.unit() as f64 + rng.unit() as f64;
    (s - 2.0) * 3.0f64.sqrt()
}

/// Step standard deviation for a density (the panel prints it).
pub(crate) fn sigma(density: f32) -> f32 {
    0.01 + density.clamp(0.0, 1.0) * 0.20
}

/// Reflected Brownian walk. `phase` is the starting position (0 = the middle), not a drift; the
/// old drift term pulled the default walk to the floor. The largest steps become onsets and each
/// lane walks from its own phase.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    if p.pulses == 0 || p.density <= 0.0 { return out; }
    let sigma = sigma(p.density) as f64;
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0xB0B0_0001 + tr as u64);
        let mut x = frac(0.5 + p.phase as f64 + tr as f64 / TRACKS as f64);
        let mut scores = [0.0f32; MAX_STEPS];
        let mut walk = [0.0f32; MAX_STEPS];
        for i in 0..n {
            let mut step = 0.0f64;
            if i > 0 {
                step = gaussian(&mut rng) * sigma;
                x = fold01(x + step);
            }
            scores[i] = step.abs() as f32;
            walk[i] = x as f32;
            if tr == 0 {
                out.orbit[i] = x as f32;
                out.theta[i] = step.abs() as f32;
                out.u[i] = x as f32;
            }
        }
        let wanted = lane_wanted(p.pulses, p.density, tr, n).max(1).min(n);
        top_k(&scores, n, wanted, &mut out.hit[tr]);
        for i in 0..n {
            if out.hit[tr][i] {
                let accent = 0.62 + 0.38 * walk[i];
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
            kind: Mode::Brownian, steps, density, variation, pulses: 16, probability: 1.0,
            ..GenParams::default()
        }
    }

    #[test]
    fn increments_are_gaussian_with_unit_variance() {
        let mut rng = Rng::new(0.3, 0, 11);
        let xs: Vec<f64> = (0..200_000).map(|_| gaussian(&mut rng)).collect();
        let m2 = xs.iter().map(|x| x * x).sum::<f64>() / xs.len() as f64;
        let m4 = xs.iter().map(|x| x.powi(4)).sum::<f64>() / xs.len() as f64;
        assert!((m2 - 1.0).abs() < 0.02, "variance {m2}");
        assert!((m4 / (m2 * m2) - 2.75).abs() < 0.1, "kurtosis {}", m4 / (m2 * m2));
    }

    #[test]
    fn default_phase_does_not_pull_the_walk_to_the_floor() {
        let mut dec = [0usize; 10];
        for variation in 0..200u32 {
            let out = generate(&params(64, 0.65, variation));
            for i in 0..64 { dec[((out.u[i] * 10.0) as usize).min(9)] += 1; }
        }
        let total: usize = dec.iter().sum();
        assert!((dec[0] as f32 / total as f32) < 0.13, "lowest decile holds {dec:?}");
        assert!((dec[9] as f32 / total as f32) > 0.07, "{dec:?}");
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
