//! Pebble box: eight pebbles (one per lane) bounce in a box tilted by `phase`. A shaker impulse
//! kicks a rotating third of them each pulse; a pebble fires its lane when it strikes a wall harder
//! than the density-derived impact threshold, and the wall is recorded for the panel. Gravity is
//! the mode-scoped `ca_rule` control. The per-step positions are recorded so the panel can animate
//! the same simulation the audio thread schedules.
use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

/// Gravity range in box-heights per step², mapped from the mode-scoped Gravity control
/// (`ca_rule`, 0-255). Small next to a shaker kick: the impulses drive the motion, gravity only
/// curves it, so the impact threshold tracks the impulses instead of the falls.
const GRAVITY_MIN: f32 = 0.05;
const GRAVITY_RANGE: f32 = 1.15;
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
    // `ca_rule` is the mode-scoped Gravity knob in the editor; 0-255 maps to 0.05-1.2.
    let gravity = GRAVITY_MIN + (p.ca_rule as f32 / 255.0) * GRAVITY_RANGE;
    let (gx, gy) = (tilt.sin() * gravity * 0.7, tilt.cos() * gravity);
    let substeps = (p.osc_rate.round() as i32).clamp(1, 16) as usize;
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
        let mut best_wall = [0u8; TRACKS];
        for _ in 0..substeps {
            for tr in 0..TRACKS {
                vel[tr].0 += gx * dt;
                vel[tr].1 += gy * dt;
                pos[tr].0 += vel[tr].0 * dt;
                pos[tr].1 += vel[tr].1 * dt;
                let mut impact = 0.0f32;
                let mut wall = 0u8;
                if pos[tr].0 < 0.0 {
                    let v = -vel[tr].0;
                    if v > impact { impact = v; wall = 1; }
                    pos[tr].0 = 0.0;
                    vel[tr].0 = (-vel[tr].0 * bounce[tr]).abs();
                } else if pos[tr].0 > 1.0 {
                    let v = vel[tr].0;
                    if v > impact { impact = v; wall = 2; }
                    pos[tr].0 = 1.0;
                    vel[tr].0 = -(vel[tr].0.abs() * bounce[tr]);
                }
                if pos[tr].1 < 0.0 {
                    let v = -vel[tr].1;
                    if v > impact { impact = v; wall = 3; }
                    pos[tr].1 = 0.0;
                    vel[tr].1 = (-vel[tr].1 * bounce[tr]).abs();
                } else if pos[tr].1 > 1.0 {
                    let v = vel[tr].1;
                    if v > impact { impact = v; wall = 4; }
                    pos[tr].1 = 1.0;
                    vel[tr].1 = -(vel[tr].1.abs() * bounce[tr]);
                }
                if impact >= threshold && impact >= 0.6 {
                    let e = (impact / (KICK_MIN + KICK_RANGE)).clamp(0.15, 1.0);
                    if e > best[tr] { best[tr] = e; best_wall[tr] = wall; }
                }
            }
        }

        for tr in 0..TRACKS {
            if best[tr] > 0.0 {
                set_hit(&mut out, tr, i, true, best[tr]);
                out.impact[i][tr] = best_wall[tr];
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

    #[test]
    fn every_hit_records_the_wall_it_struck() {
        let p = GenParams { kind: super::super::GenKind::PebbleBox, steps: 64, pulses: 8, density: 1.0, ca_rule: 255, ..GenParams::default() };
        let pat = generate(&p);
        for i in 0..64 {
            for tr in 0..TRACKS {
                if pat.hit[tr][i] {
                    assert!((1..=4).contains(&pat.impact[i][tr]), "step {i} lane {tr} hit without a wall");
                } else {
                    assert_eq!(pat.impact[i][tr], 0, "step {i} lane {tr} recorded a wall without a hit");
                }
            }
        }
    }

    #[test]
    fn gravity_and_substeps_change_the_bounce() {
        let base = GenParams { kind: super::super::GenKind::PebbleBox, steps: 32, pulses: 6, seed: 0.314, ..GenParams::default() };
        let a = generate(&base);
        let heavy = generate(&GenParams { ca_rule: 255, ..base });
        let fine = generate(&GenParams { osc_rate: 16.0, ..base });
        assert_ne!(a.pebbles, heavy.pebbles, "gravity did not change the frames");
        assert_ne!(a.pebbles, fine.pebbles, "substeps did not change the frames");
    }
}
