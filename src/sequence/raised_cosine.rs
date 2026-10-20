use super::{set_hit, GenParams, Pattern, TRACKS};
use std::f64::consts::{PI, TAU};

/// Smooth periodic envelope at step `i`, in 0..=1. Shared with the panel so the drawn curve
/// is the curve the generator actually thresholded.
pub(crate) fn envelope(i: usize, n: usize, hits: usize, phase: f64) -> f32 {
    let env = ((i as f64 / n.max(1) as f64 * hits as f64 + phase) % 1.0).rem_euclid(1.0);
    (0.5 - 0.5 * (TAU * env).cos()) as f32
}

/// Envelope phase offset. Lane-independent, so the panel can redraw the same curve.
pub(crate) fn phase_offset(seed: f32, phase: f32) -> f64 {
    phase.clamp(0.0, 1.0) as f64 + seed.clamp(0.0, 1.0) as f64 * 0.4
}

/// Cutoff giving the envelope a duty cycle of `hits / n`: each envelope cycle then
/// contributes exactly one onset. Inverting duty = 1 - acos(1-2T)/pi.
pub(crate) fn cutoff(hits: usize, n: usize) -> f32 {
    let duty = (hits as f64 / n.max(1) as f64).clamp(0.0, 1.0);
    (0.5 * (1.0 + (PI * duty).cos())) as f32
}

/// Raised-cosine threshold: a smooth periodic envelope is compared against a cutoff derived from
/// `pulses * density`. The envelope runs `hits` cycles around the ring, so each cycle lights one
/// onset and the onsets stay evenly spread rather than collapsing into a single contiguous arc.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    let phase = phase_offset(p.seed, p.phase);
    for tr in 0..TRACKS {
        let hits = super::pulse_hits(p.pulses, p.density.clamp(0.0, 1.0), tr, n);
        // Lane 0's envelope is the panel's plotted curve.
        if tr == 0 {
            for i in 0..n { out.orbit[i] = envelope(i, n, hits, phase); }
        }
        if hits == 0 { continue; }
        let threshold = cutoff(hits, n);
        // Every lane thresholds the same envelope, so lanes that request the same `hits`
        // produced identical words and fired together (all eight drums on the same step in
        // every variation). Rotating each lane by its own step offset keeps the exact count
        // while making the lanes independent; lane 0 stays on the downbeat.
        let offset = tr % n;
        for i in 0..n {
            let raised = envelope(i, n, hits, phase);
            set_hit(&mut out, tr, (i + offset) % n, raised >= threshold, (0.55 + raised * 0.45).clamp(0.05, 1.0));
        }
    }
    out
}