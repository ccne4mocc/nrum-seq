use super::{frac, set_hit, GenParams, Pattern, TRACKS, PHI_INV};

/// Sturmian slope for a density: `density * 1/phi`, so density 1.0 yields the Fibonacci word.
pub(crate) fn alpha(density: f32) -> f64 {
    (density.clamp(0.05, 1.0) as f64 * PHI_INV).clamp(0.05, PHI_INV)
}

/// Sturmian (Fibonacci) word: `floor((i+1)*alpha) - floor(i*alpha)`. Every lane reads the same
/// word at a different circular phase, so related rhythms interlock instead of all eight lanes
/// playing the same onset.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    let base_alpha = alpha(p.density);
    let base = frac(p.seed as f64 + p.phase as f64 + p.variation as f64 * PHI_INV);
    for tr in 0..TRACKS {
        // Lane 0 keeps the exact golden slope; the others detune and shift phase so the kit
        // hears interlocking Sturmian words instead of one word on eight drums.
        let a = (base_alpha * (1.0 - 0.03 * tr as f64)).clamp(0.03, 0.95);
        // Phase is measured in steps, so space lanes by a fraction of the loop.
        let phase = base + tr as f64 * n as f64 / TRACKS as f64;
        let mut prev = (phase * a).floor() as i64;
        let mut any = false;
        for i in 0..n {
            let cur = (((i + 1) as f64 + phase) * a).floor() as i64;
            let hit = cur != prev;
            prev = cur;
            if hit { any = true; }
            set_hit(&mut out, tr, i, hit, if hit { 0.9 } else { 0.65 });
        }
        // A one-step loop can land in the gap before the first onset; keep it sounding.
        if !any { set_hit(&mut out, tr, 0, true, 0.9); }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Mode;

    fn params(steps: usize, density: f32) -> GenParams {
        GenParams { kind: Mode::Fibonacci, steps, density, probability: 1.0, seed: 0.0, phase: 0.0, ..GenParams::default() }
    }

    #[test]
    fn density_one_is_the_exact_fibonacci_word() {
        let a = alpha(1.0);
        assert!((a - PHI_INV).abs() < 1e-12);
        let n = 610; // a Fibonacci number
        let out = generate(&params(n, 1.0));
        let hits = (0..n).filter(|&i| out.hit[0][i]).count();
        assert!((hits as f64 - n as f64 * a).abs() <= 1.5, "{hits} onsets of {n}");
        let on: Vec<usize> = (0..n).filter(|&i| out.hit[0][i]).collect();
        assert!(on.windows(2).all(|w| (1..=2).contains(&(w[1] - w[0]))), "golden gaps are 1 or 2");
    }

    #[test]
    fn lanes_are_detuned_and_none_is_silent() {
        let out = generate(&params(64, 0.8));
        for tr in 0..TRACKS {
            assert!(out.hit[tr][..64].iter().any(|&h| h), "lane {tr} is silent");
        }
        let line = |tr: usize| out.hit[tr][..64].to_vec();
        assert!((1..TRACKS).any(|tr| line(tr) != line(0)), "all lanes read the same word");
    }

    #[test]
    fn slope_follows_density_and_never_reaches_a_straight_grid() {
        assert!(alpha(0.1) < alpha(0.5));
        assert!(alpha(0.5) < alpha(1.0));
        assert!(alpha(1.0) < 0.95);
    }
}
