use super::{frac, set_hit, GenParams, Pattern, TRACKS, PHI_INV};

/// Growth rate for the current density. Shared with the view's return-map curve so the drawn
/// parabola is the one the orbit actually follows.
pub(crate) fn r_of(density: f32) -> f64 {
    3.5 + density.clamp(0.0, 1.0) as f64 * 0.5
}

/// Initial condition the generator starts from. `variation` participates, so Generate changes
/// the orbit instead of only rotating the same one.
pub(crate) fn x0(p: &GenParams) -> f64 {
    let phase = frac(p.seed as f64 + p.phase as f64 + p.variation as f64 * PHI_INV);
    (0.05 + 0.9 * phase).clamp(0.001, 0.999)
}

/// The orbit may legitimately come within 1e-3 of 0 or 1 at r close to 4; only a true collapse
/// onto a fixed point counts as an escape.
const EDGE: f64 = 1e-9;

/// Logistic-map orbit x[n+1] = r*x[n]*(1-x[n]). A lane's density sets its trigger threshold.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let r = r_of(p.density);
    let mut x = x0(p);
    for _ in 0..24 {
        x = r * x * (1.0 - x);
        if !(EDGE..=1.0 - EDGE).contains(&x) { x = 0.37; }
    }
    // At r near 4 the burn-in can end next to 0, where the orbit needs several steps to climb
    // back and a short pattern is silent. Step on (bounded) until it is in the working range.
    for _ in 0..32 {
        if (0.1..=0.9).contains(&x) { break; }
        x = r * x * (1.0 - x);
        if !(EDGE..=1.0 - EDGE).contains(&x) { x = 0.37; }
    }
    for i in 0..p.steps {
        x = r * x * (1.0 - x);
        out.orbit[i] = x as f32;
    }
    // Each lane reads the orbit from a different phase, otherwise every lane triggers on the
    // same high samples and all eight drums fire together (measured: a third of all steps at
    // the default settings). The offset only moves *where* a lane looks, never the values.
    let stride = (p.steps / TRACKS).max(1);
    for tr in 0..TRACKS {
        let pulse_ratio = p.pulses as f32 / p.steps as f32;
        let threshold = (pulse_ratio * (0.3 + 0.7 * p.density.clamp(0.0, 1.0))
            * (0.5 + super::DENSITY_PROFILE[tr])).clamp(0.03, 0.97);
        // Hit at orbit peaks relative to a density/pulse quantile, not on every high value.
        let cutoff = 1.0 - threshold;
        let offset = (tr * stride) % p.steps;
        for i in 0..p.steps {
            let slot = (i + offset) % p.steps;
            let x = out.orbit[slot];
            // One sample cannot clear a cutoff tuned for a ring, and prev/next both wrap
            // onto this step. A single-step pattern is a plain trigger instead.
            if p.steps == 1 {
                set_hit(&mut out, tr, i, true, 0.55 + x * 0.45);
                continue;
            }
            // A two-step ring has the same neighbour on both sides, so no sample can be a
            // strict local peak; fall back to the cutoff alone there.
            let prev = out.orbit[(slot + p.steps - 1) % p.steps];
            let next = out.orbit[(slot + 1) % p.steps];
            let peak = p.steps <= 2 || (x >= prev && x > next);
            set_hit(&mut out, tr, i, x >= cutoff && peak, 0.55 + x * 0.45);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Mode;

    fn params(variation: u32) -> GenParams {
        GenParams { kind: Mode::Logistic, steps: 64, density: 0.8, probability: 1.0, variation, ..GenParams::default() }
    }

    #[test]
    fn variation_changes_the_orbit_not_only_its_rotation() {
        // Compare the sorted orbit values: a pure rotation leaves this multiset unchanged.
        let key = |v: u32| {
            let out = generate(&params(v));
            let mut values: Vec<i32> = out.orbit[..64].iter().map(|x| (x * 1e5).round() as i32).collect();
            values.sort_unstable();
            values
        };
        assert_ne!(key(0), key(1));
    }

    #[test]
    fn orbit_stays_finite_and_inside_the_unit_interval() {
        for variation in 0..32 {
            let out = generate(&params(variation));
            assert!(out.orbit[..64].iter().all(|x| x.is_finite() && (0.0..=1.0).contains(x)));
        }
    }
}
