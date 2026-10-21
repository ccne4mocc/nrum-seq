use super::{set_hit, GenParams, Pattern, TRACKS, PHI, PHI_INV, GOLDEN_ANGLE};

/// Phase offset between lanes, in turns. Every lane runs the same rotation, one lane apart.
const LANE_OFFSET: f64 = 0.137;

/// Steps the generator ran with. `generate` clamps the snapshot before dispatch, so the panel
/// has to clamp the same way to draw the same arithmetic.
pub(crate) fn steps_of(p: &GenParams) -> usize {
    p.steps.clamp(1, super::MAX_STEPS)
}

/// Pulses the generator ran with. `generate` clamps the pulse count to the pattern length for
/// every mode but the cellular automaton, and the phyllotaxis rule is one of those.
fn pulses_of(p: &GenParams) -> usize {
    p.pulses.min(steps_of(p)).max(1)
}

/// The primary rule's phase for a step, in turns: the golden rotation advanced by 1/φ per step,
/// plus the lane's own offset. `primary_phase` and `primary_angle` differ only in the wrap, so
/// the panel can draw the unwrapped spiral and still test the wrapped value.
pub(crate) fn primary_angle(i: usize, lane: usize, p: &GenParams) -> f64 {
    i as f64 * PHI_INV + p.seed as f64 + p.phase as f64 + lane as f64 * LANE_OFFSET
}

/// The primary phase folded into one turn: the value the generator tests.
pub(crate) fn primary_phase(i: usize, lane: usize, p: &GenParams) -> f64 {
    primary_angle(i, lane, p).rem_euclid(1.0)
}

/// The secondary rule's phase for a step, in turns: a second, slower rotation by 1/φ³. The lane
/// term sits inside the bracket, so it is scaled with it.
pub(crate) fn secondary_angle(i: usize, lane: usize, p: &GenParams) -> f64 {
    (i as f64 + lane as f64 * PHI_INV + p.phase as f64 * steps_of(p) as f64) * GOLDEN_ANGLE
        / (std::f64::consts::TAU * PHI)
}

/// The secondary phase folded into one turn: the value the generator tests.
pub(crate) fn secondary_phase(i: usize, lane: usize, p: &GenParams) -> f64 {
    secondary_angle(i, lane, p).rem_euclid(1.0)
}

/// Trigger window of a lane, on the same 0..1 scale as the phases. This is the generator's own
/// threshold, so the panel's lit wedge is the wedge the generator used.
pub(crate) fn lane_threshold(p: &GenParams, lane: usize) -> f64 {
    let steps = steps_of(p);
    let pulse_ratio = (pulses_of(p) as f32 / steps as f32).clamp(0.02, 0.95);
    (pulse_ratio * (0.35 + super::DENSITY_PROFILE[lane]) * (0.5 + p.density.clamp(0.0, 1.0)))
        .clamp(0.02, 0.98) as f64
}

/// Which rules fired a step: bit 0 = primary rotation, bit 1 = secondary rotation. The panel
/// lights the band of the spiral that actually triggered.
pub(crate) fn rules_fired(i: usize, lane: usize, p: &GenParams) -> u8 {
    let threshold = lane_threshold(p, lane);
    let mut rules = 0u8;
    if primary_phase(i, lane, p) < threshold { rules |= 1; }
    if secondary_phase(i, lane, p) < threshold * 0.18 { rules |= 2; }
    rules
}

/// Golden-ratio / phyllotaxis sequence. Two irrational rotations decide each step: the primary
/// golden rotation by 1/φ, and a second, slower one by 1/φ³ that adds a sparse family of onsets.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    for tr in 0..TRACKS {
        for i in 0..p.steps {
            let rules = rules_fired(i, tr, p);
            let beat = primary_phase(i, tr, p);
            set_hit(&mut out, tr, i, rules != 0, 0.65 + (1.0 - beat as f32) * 0.35);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Mode;
    use crate::sequence::{generate, GenParams, DENSITY_PROFILE, MAX_STEPS};

    fn params(steps: usize, pulses: usize, density: f32) -> GenParams {
        GenParams { kind: Mode::Phyllotaxis, steps, length: steps as f32, pulses, density, probability: 1.0, ..GenParams::default() }
    }

    #[test]
    fn the_panel_helpers_reproduce_the_generators_arithmetic() {
        // The panel draws these phases, so they must be the ones the hit test uses. Recompute the
        // generator's original expressions and compare.
        let p = params(16, 5, 0.65);
        for lane in 0..TRACKS {
            for i in 0..16 {
                let beat = (i as f64 * PHI_INV + p.seed as f64 + p.phase as f64 + lane as f64 * 0.137).rem_euclid(1.0);
                let angle = (i as f64 + lane as f64 * PHI_INV + p.phase as f64 * p.steps as f64) * GOLDEN_ANGLE;
                let radial = (angle / (std::f64::consts::TAU * PHI)).rem_euclid(1.0);
                assert!((primary_phase(i, lane, &p) - beat).abs() < 1e-12, "primary phase drifted");
                assert!((secondary_phase(i, lane, &p) - radial).abs() < 1e-12, "secondary phase drifted");
            }
        }
    }

    #[test]
    fn the_threshold_matches_the_generator_window() {
        let p = params(16, 5, 0.65);
        for lane in 0..TRACKS {
            let steps = p.steps.clamp(1, MAX_STEPS);
            let pulses = p.pulses.min(steps).max(1);
            let ratio = (pulses as f32 / steps as f32).clamp(0.02, 0.95);
            let want = (ratio * (0.35 + DENSITY_PROFILE[lane]) * (0.5 + p.density.clamp(0.0, 1.0)))
                .clamp(0.02, 0.98) as f64;
            assert_eq!(lane_threshold(&p, lane), want, "lane {lane}");
        }
    }

    #[test]
    fn the_helper_predicates_match_the_generated_hits() {
        let p = params(32, 6, 0.7);
        let pat = generate(&p);
        for lane in 0..TRACKS {
            for i in 0..32 {
                let expected = rules_fired(i, lane, &p) != 0;
                assert_eq!(pat.hit[lane][i], expected, "lane {lane} step {i}");
            }
        }
    }

    #[test]
    fn the_lanes_do_not_share_a_phase_origin() {
        let p = params(16, 5, 0.65);
        // Each lane is the previous one shifted by exactly one lane offset, so all eight
        // origins are distinct even though the offsets wrap around the circle.
        for lane in 1..TRACKS {
            let d = primary_angle(0, lane, &p) - primary_angle(0, lane - 1, &p);
            assert!((d - 0.137).abs() < 1e-12, "primary lane {lane} offset {d}");
            let s = secondary_angle(0, lane, &p) - secondary_angle(0, lane - 1, &p);
            assert!((s - PHI_INV.powi(4)).abs() < 1e-12, "secondary lane {lane} offset {s}");
        }
    }

    #[test]
    fn the_window_grows_with_the_pulse_ratio() {
        let few = lane_threshold(&params(16, 2, 0.65), 0);
        let many = lane_threshold(&params(16, 12, 0.65), 0);
        assert!(many > few, "more pulses must widen the window");
        // Pulses above the pattern length are clamped, so the window cannot exceed the f32
        // clamp (0.98 widened to f64 is not exactly the decimal literal).
        assert!(lane_threshold(&params(4, 64, 1.0), 0) <= 0.98f32 as f64);
    }

    #[test]
    fn the_generator_still_fires_at_one_step_and_at_full_density() {
        let one = generate(&params(1, 1, 1.0));
        assert!((0..TRACKS).any(|tr| one.hit[tr][0]), "silent at one step");
        let full = generate(&params(16, 16, 1.0));
        assert!((0..TRACKS).any(|tr| full.hit[tr].iter().any(|&h| h)), "silent at full density");
    }
}
