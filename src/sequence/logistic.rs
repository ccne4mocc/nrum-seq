use super::{set_hit, GenParams, Pattern, TRACKS};

/// Logistic-map orbit x[n+1] = r*x[n]*(1-x[n]). A lane's density sets its trigger threshold.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let r = 3.5 + p.density.clamp(0.0, 1.0) as f64 * 0.5;
    let mut x = (0.13 + p.seed as f64 * 0.73 + p.phase as f64 * 0.11).clamp(0.001, 0.999);
    for _ in 0..24 { x = r * x * (1.0 - x); }
    // At r near 4 the burn-in can end next to 0, where the orbit needs several steps to climb
    // back and a short pattern is silent. Step on (bounded) until it is in the working range.
    for _ in 0..32 { if (0.1..=0.9).contains(&x) { break; } x = r * x * (1.0 - x); }
    for i in 0..p.steps {
        x = r * x * (1.0 - x);
        out.orbit[i] = x as f32;
    }
    for tr in 0..TRACKS {
        let pulse_ratio = p.pulses as f32 / p.steps as f32;
        let threshold = (pulse_ratio * (0.3 + 0.7 * p.density.clamp(0.0, 1.0))
            * (0.5 + super::DENSITY_PROFILE[tr])).clamp(0.03, 0.97);
        // Hit at orbit peaks relative to a density/pulse quantile, not on every high value.
        let cutoff = 1.0 - threshold;
        for i in 0..p.steps {
            let x = out.orbit[i];
            // One sample cannot clear a cutoff tuned for a ring, and prev/next both wrap
            // onto this step. A single-step pattern is a plain trigger instead.
            if p.steps == 1 {
                set_hit(&mut out, tr, i, true, 0.55 + x * 0.45);
                continue;
            }
            let prev = out.orbit[(i + p.steps - 1) % p.steps];
            let local_peak = x >= prev && x > out.orbit[(i + 1) % p.steps];
            set_hit(&mut out, tr, i, x >= cutoff && (local_peak || x > 0.75), 0.55 + x * 0.45);
        }
    }
    out
}
