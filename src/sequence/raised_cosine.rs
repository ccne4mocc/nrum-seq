use super::{set_hit, GenParams, Pattern, TRACKS};
use std::f64::consts::TAU;

/// Raised-cosine threshold: a smooth periodic envelope is compared against a cutoff derived from
/// `pulses * density`. The envelope runs `hits` cycles around the ring, so each cycle lights one
/// onset and the onsets stay evenly spread rather than collapsing into a single contiguous arc.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let requested = if p.pulses == 0 { 0 } else { (p.pulses + tr % 3).min(n) };
        let hits = ((requested as f32 * p.density.clamp(0.0, 1.0)).round() as usize).min(n);
        if hits == 0 { continue; }
        // Cutoff giving the raised cosine a duty cycle of `hits / n`: each envelope cycle
        // then contributes exactly one onset. Inverting duty = 1 - acos(1-2T)/pi.
        let duty = (hits as f64 / n as f64).clamp(0.0, 1.0);
        let threshold = 0.5 * (1.0 + (std::f64::consts::PI * duty).cos());
        let phase = p.phase as f64 + p.seed as f64 * 0.4;
        for i in 0..n {
            let env = ((i as f64 / n as f64 * hits as f64 + phase) % 1.0).rem_euclid(1.0);
            let raised = 0.5 - 0.5 * (TAU * env).cos();
            set_hit(&mut out, tr, i, raised >= threshold, (0.55 + raised as f32 * 0.45).clamp(0.05, 1.0));
        }
    }
    out
}