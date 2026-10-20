//! Pebble box: eight pebbles (one per lane) bounce in a tilted box. A shaker impulse kicks a
//! rotating third of them each pulse; a pebble fires its lane when it strikes a wall harder than
//! the density-derived impact threshold. The per-step positions are recorded so the panel can
//! animate the same simulation the audio thread schedules.
use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

/// Gravity in box-heights per step². Small next to a shaker kick: the impulses drive the motion,
/// gravity only curves it, so the impact threshold tracks the impulses instead of the falls.
const GRAVITY: f32 = 0.35;
/// Kick speed upward, in box-heights per step. The lowest value clears the density-0.65
/// threshold (1.47) even after gravity decelerates the climb.
const KICK_MIN: f32 = 2.4;
const KICK_RANGE: f32 = 1.8;

pub(super) fn generate(p: &GenParams) -> Pattern {
    let n = p.steps;
    let mut out = Pattern { steps: n, ..Pattern::default() };
    let mut rng = Rng::new(p.seed, p.variation, 0x9EBB_1E00);

    // Normalized box position and velocity per pebble, plus its restitution and layout.
    let mut pos = [(0.5f32, 0.5f32); TRACKS];
    let mut vel = [(0.0f32, 0.0f32); TRACKS];
    let mut bounce = [0.75f32; TRACKS];
    for tr in 0..TRACKS {
        pos[tr] = (0.12 + rng.unit() * 0.76, 0.12 + rng.unit() * 0.76);
        vel[tr] = ((rng.unit() - 0.5) * 0.8, -(0.3 + rng.unit() * 0.9));
        // Heavily damped: a secondary bounce must fall under the lowest threshold, otherwise
        // every pebble keeps firing for several steps and occasional all-eight columns appear.
        bounce[tr] = 0.40 + rng.unit() * 0.15;
    }

    let tilt = p.phase.clamp(0.0, 1.0) * std::f32::consts::TAU;
    let (gx, gy) = (tilt.sin() * GRAVITY * 0.7, tilt.cos() * GRAVITY);
    let substeps = ((p.osc_rate * 8.0).round() as usize).clamp(1, 16);
    let dt = 1.0 / substeps as f32;
    let threshold = (1.0 - p.density.clamp(0.0, 1.0)) * 4.2;
    let shaker = p.pulses.clamp(1, n.max(1));
    let stride = (n / shaker).max(1);

    for i in 0..n {
        if i % stride == 0 {
            let impulse = i / stride;
            for tr in 0..TRACKS {
                // Rotating third of the pebbles per impulse: all eight never leave the floor
                // together, so a shaker hit cannot stack every lane on one step.
                if tr % 3 != impulse % 3 { continue; }
                // Kick up, never down: a pebble resting on the floor must leave it, not be
                // hammered into the wall it is already touching.
                let speed = KICK_MIN + rng.unit() * KICK_RANGE;
                vel[tr].1 = -speed;
                vel[tr].0 += (rng.unit() - 0.5) * 1.6;
            }
        }

        let mut best = [0.0f32; TRACKS];
        for _ in 0..substeps {
            for tr in 0..TRACKS {
                vel[tr].0 += gx * dt;
                vel[tr].1 += gy * dt;
                pos[tr].0 += vel[tr].0 * dt;
                pos[tr].1 += vel[tr].1 * dt;
                let mut impact = 0.0f32;
                if pos[tr].0 < 0.0 {
                    impact = impact.max(-vel[tr].0);
                    pos[tr].0 = 0.0;
                    vel[tr].0 = (-vel[tr].0 * bounce[tr]).abs();
                } else if pos[tr].0 > 1.0 {
                    impact = impact.max(vel[tr].0);
                    pos[tr].0 = 1.0;
                    vel[tr].0 = -(vel[tr].0.abs() * bounce[tr]);
                }
                if pos[tr].1 < 0.0 {
                    impact = impact.max(-vel[tr].1);
                    pos[tr].1 = 0.0;
                    vel[tr].1 = (-vel[tr].1 * bounce[tr]).abs();
                } else if pos[tr].1 > 1.0 {
                    impact = impact.max(vel[tr].1);
                    pos[tr].1 = 1.0;
                    vel[tr].1 = -(vel[tr].1.abs() * bounce[tr]);
                }
                if impact >= threshold && impact >= 0.6 {
                    best[tr] = best[tr].max((impact / (KICK_MIN + KICK_RANGE)).clamp(0.15, 1.0));
                }
            }
        }

        for tr in 0..TRACKS {
            if best[tr] > 0.0 {
                set_hit(&mut out, tr, i, true, best[tr]);
            }
            out.pebbles[i][tr * 2] = (pos[tr].0.clamp(0.0, 1.0) * 255.0).round() as u8;
            out.pebbles[i][tr * 2 + 1] = (pos[tr].1.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::generate;

    #[test]
    fn same_snapshot_gives_the_same_bounces_and_frames() {
        let p = GenParams { kind: super::super::GenKind::PebbleBox, steps: 16, pulses: 5, density: 0.7, seed: 0.42, ..GenParams::default() };
        let a = generate(&p);
        let b = generate(&p);
        assert_eq!(a.hit, b.hit);
        assert_eq!(a.pebbles, b.pebbles);
    }

    #[test]
    fn frames_are_recorded_for_every_step_and_pebble() {
        let p = GenParams { kind: super::super::GenKind::PebbleBox, steps: 32, pulses: 5, density: 0.8, ..GenParams::default() };
        let pat = generate(&p);
        for i in 0..32 {
            assert_ne!(pat.pebbles[i], [0; TRACKS * 2], "frame {i} was never recorded");
        }
        // Neighbouring frames differ, or the panel's tween would move nothing.
        assert!((0..31).filter(|&i| pat.pebbles[i] != pat.pebbles[i + 1]).count() > 24);
    }

    #[test]
    fn density_raises_the_impact_threshold() {
        let base = GenParams { kind: super::super::GenKind::PebbleBox, steps: 32, pulses: 8, seed: 0.271, ..GenParams::default() };
        let count = |density| {
            let pat = generate(&GenParams { density, ..base });
            (0..32).map(|i| (0..TRACKS).filter(|&tr| pat.hit[tr][i]).count()).sum::<usize>()
        };
        assert!(count(1.0) > count(0.4), "density does not gate impacts");
    }
}
