use super::{set_hit, GenParams, Pattern, TRACKS, PHI, PHI_INV, GOLDEN_ANGLE};

/// Golden-ratio / phyllotaxis sequence. The golden-angle spiral defines each candidate's phase;
/// lane density clips the fractional golden-ratio orbit into trigger / rest decisions.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    for tr in 0..TRACKS {
        let pulse_ratio = (p.pulses.max(1) as f32 / p.steps as f32).clamp(0.02, 0.95);
        let threshold = (pulse_ratio * (0.35 + super::DENSITY_PROFILE[tr]) * (0.5 + p.density.clamp(0.0, 1.0)))
            .clamp(0.02, 0.98);
        for i in 0..p.steps {
            let angle = (i as f64 + tr as f64 * PHI_INV + p.phase as f64 * p.steps as f64) * GOLDEN_ANGLE;
            let radial = (angle / (std::f64::consts::TAU * PHI)).rem_euclid(1.0);
            let beat = (i as f64 * PHI_INV + p.seed as f64 + p.phase as f64 + tr as f64 * 0.137).rem_euclid(1.0);
            let hit = beat < threshold as f64 || radial < (threshold as f64 * 0.18);
            set_hit(&mut out, tr, i, hit, 0.65 + (1.0 - beat as f32) * 0.35);
        }
    }
    out
}
