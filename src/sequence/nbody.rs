//! Three-body chaos. A soft-gravity N-body orbit produces a chaotic score; rank selection
//! turns the strongest steps into onsets, and rotating-line crossings accent them.
//!
//! The bodies are a tight binary (masses 1.0 and 0.6) plus a lighter third body on a wider
//! orbit. That hierarchy is mildly chaotic through Kozai-Lidov exchange: the third body's
//! eccentricity breathes, so close encounters arrive irregularly instead of on a schedule.

use super::{set_hit, GenParams, Pattern, DENSITY_PROFILE, MAX_STEPS, TRACKS};
use std::f64::consts::{PI, TAU};

/// Flat state layout: `[x0, y0, x1, y1, x2, y2]`, and the same six slots for velocity.
const W: usize = 6;
const MASS: [f64; 3] = [1.0, 0.6, 0.4];
const G: f64 = 1.0;
/// Softening squared. Without it a close encounter sends 1/r^2 towards infinity and the
/// integrator diverges part-way through the pattern.
const SOFT2: f64 = 0.02;
const DT: f64 = 0.004;
/// ponytail: hard radius/speed clamps are a crude stand-in for adaptive step control. They
/// keep every value finite and bounded; a real integrator would shorten the step on an
/// encounter instead of rescaling the state. Revisit if encounters start sounding clipped.
const R_MAX: f64 = 3.2;
const V_MAX: f64 = 6.0;

fn accel(r: &[f64; W], a: &mut [f64; W]) {
    for k in 0..W { a[k] = 0.0; }
    for i in 0..3 {
        for j in 0..3 {
            if i == j { continue; }
            let dx = r[2 * j] - r[2 * i];
            let dy = r[2 * j + 1] - r[2 * i + 1];
            let k = G * MASS[j] / (dx * dx + dy * dy + SOFT2).powf(1.5);
            a[2 * i] += dx * k;
            a[2 * i + 1] += dy * k;
        }
    }
}

fn clamp_state(r: &mut [f64; W], v: &mut [f64; W]) {
    for i in 0..3 {
        let (x, y) = (r[2 * i], r[2 * i + 1]);
        let d = (x * x + y * y).sqrt();
        if d > R_MAX { let s = R_MAX / d; r[2 * i] = x * s; r[2 * i + 1] = y * s; }
        let (vx, vy) = (v[2 * i], v[2 * i + 1]);
        let s = (vx * vx + vy * vy).sqrt();
        if s > V_MAX { let f = V_MAX / s; v[2 * i] = vx * f; v[2 * i + 1] = vy * f; }
    }
}

/// Bounded substeps for one generation: resolution follows `osc_rate`, total work is capped.
/// Shared with the help text so the displayed budget matches the integrator's.
pub(crate) fn substep_budget(steps: usize, osc_rate: f32) -> usize {
    (steps * (8 + 32 * osc_rate.clamp(0.125, 8.0) as usize)).clamp(64, 2048)
}

/// Initial state: circular binary plus a wider third orbit, then perturbed so neighbouring
/// seeds diverge. Deterministic for a given `(seed, variation)` snapshot.
fn initial_state(p: &GenParams) -> ([f64; W], [f64; W]) {
    let jitter = |k: u64| super::Rng::new(p.seed, p.variation, 0x7B0D_0000 + k).unit() as f64 - 0.5;
    let binary_sep = 0.7;
    let light_radius = 1.05 + 0.95 * p.phase.clamp(0.0, 1.0) as f64;
    let binary_mass = MASS[0] + MASS[1];
    let total_mass: f64 = MASS.iter().sum();
    let alpha = jitter(1) * 0.7;
    let (ca, sa) = (alpha.cos(), alpha.sin());

    let mut r = [0.0f64; W];
    let mut v = [0.0f64; W];
    // Binary, centred on its own barycentre, which is the frame's origin.
    let d0 = -binary_sep * MASS[1] / binary_mass;
    let d1 = binary_sep * MASS[0] / binary_mass;
    let w_bin = (G * binary_mass / (binary_sep * binary_sep * binary_sep)).sqrt();
    r[0] = d0 * ca; r[1] = d0 * sa;
    r[2] = d1 * ca; r[3] = d1 * sa;
    v[0] = -w_bin * r[1]; v[1] = w_bin * r[0];
    v[2] = -w_bin * r[3]; v[3] = w_bin * r[2];
    // Third body, counter-phased on the wider orbit.
    let beta = alpha + 2.1 + 0.9 * jitter(2);
    let w_light = (G * total_mass / (light_radius * light_radius * light_radius)).sqrt();
    r[4] = light_radius * beta.cos(); r[5] = light_radius * beta.sin();
    v[4] = -w_light * r[5]; v[5] = w_light * r[4];
    for k in 0..W {
        r[k] += jitter(16 + k as u64) * 0.03;
        v[k] *= 1.0 + jitter(32 + k as u64) * 0.03;
    }
    (r, v)
}

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    let substeps = substep_budget(n, p.osc_rate);
    let per_step = (substeps / n).max(1);
    let total_time = substeps as f64 * DT;
    // The rotating line completes `pulses` turns across the pattern, so the number of
    // crossings the third body makes against it scales with the requested onset density.
    let line_rate = TAU * p.pulses.clamp(1, 64) as f64 / total_time;

    let (mut r, mut v) = initial_state(p);
    let mut a = [0.0f64; W];
    accel(&r, &mut a);

    // Per-step observables, in revolutions of orbital sweep and in 1/(1 + nearest approach).
    let mut sweep = [0.0f32; MAX_STEPS];
    let mut close = [0.0f32; MAX_STEPS];
    let mut crossed = [false; MAX_STEPS];
    let (mut theta, mut last, mut line) = (0.0f64, 0.0f64, 0.0f64);

    for i in 0..n {
        // View snapshot: bodies 1 and 2 relative to body 0, so the panel can draw the orbit
        // without caring where the pair happens to be in its own orbit.
        out.traj[i] = [r[2] as f32 - r[0] as f32, r[3] as f32 - r[1] as f32, r[4] as f32 - r[0] as f32, r[5] as f32 - r[1] as f32];
        let mut swept = 0.0f64;
        let mut nearest = f64::INFINITY;
        let mut crossing = false;
        for _ in 0..per_step {
            for k in 0..W { v[k] += a[k] * DT * 0.5; }
            for k in 0..W { r[k] += v[k] * DT; }
            clamp_state(&mut r, &mut v);
            accel(&r, &mut a);
            for k in 0..W { v[k] += a[k] * DT * 0.5; }

            // Barycentre of all three, then the third body's unwrapped angle about it.
            let bx = (MASS[0] * r[0] + MASS[1] * r[2] + MASS[2] * r[4]) / 2.0;
            let by = (MASS[0] * r[1] + MASS[1] * r[3] + MASS[2] * r[5]) / 2.0;
            let angle = (r[5] - by).atan2(r[4] - bx);
            let mut delta = angle - last;
            if delta > PI { delta -= TAU; } else if delta < -PI { delta += TAU; }
            theta += delta;
            last = angle;
            swept += delta.abs();
            for j in 0..2 {
                let dx = r[4] - r[2 * j];
                let dy = r[5] - r[2 * j + 1];
                nearest = nearest.min((dx * dx + dy * dy).sqrt());
            }
            let before = ((theta - line) / TAU).floor();
            line += line_rate * DT;
            if ((theta - line) / TAU).floor() != before { crossing = true; }
        }
        sweep[i] = (swept / TAU) as f32;
        close[i] = (1.0 / (1.0 + nearest)) as f32;
        crossed[i] = crossing;
    }

    for tr in 0..TRACKS {
        let profile = DENSITY_PROFILE[tr];
        let wanted = ((p.pulses as f32 * p.density.clamp(0.0, 1.0) * (0.45 + profile)).round() as usize).min(n);
        if wanted == 0 { continue; }
        // Quiet lanes ride the close encounters (ghost-note material) while loud lanes follow
        // the third body's orbital sweep, so the eight lanes are not nested subsets.
        let w_sweep = 1.0 - 0.55 * profile;
        let w_close = 0.25 + 1.75 * profile;
        // Every lane reads the same physical events, so without a per-lane phase the highest
        // ranked steps are shared and all eight drums land together (measured: 10% of steps).
        // Offsetting only moves where a lane looks, never the events themselves.
        let offset = (tr * (n / TRACKS).max(1)) % n;
        let mut score = [0.0f32; MAX_STEPS];
        for i in 0..n {
            let slot = (i + offset) % n;
            score[i] = w_sweep * sweep[slot] + w_close * close[slot];
        }
        // Rank selection, same rule as the oscillator: a step hits when fewer than `wanted`
        // steps outrank it, with earlier steps winning ties. Exact count, no sort, no alloc.
        for i in 0..n {
            let mut rank = 0usize;
            for j in 0..n {
                if score[j] > score[i] || (score[j] == score[i] && j < i) { rank += 1; }
            }
            let hit = rank < wanted;
            let accent = if crossed[i] { 1.0 } else { (0.5 + 0.4 * close[i].min(1.0)).clamp(0.05, 1.0) };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::{generate, GenKind, GenParams};

    fn params(steps: usize, pulses: usize, density: f32, seed: f32) -> GenParams {
        GenParams { kind: GenKind::ThreeBody, steps, length: steps as f32, pulses, density, probability: 1.0, seed, ..GenParams::default() }
    }

    #[test]
    fn the_orbit_stays_finite_for_every_seed_and_phase() {
        for seed in [0.0f32, 0.05, 0.271, 0.5, 0.99, 1.0] {
            for phase in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
                for steps in [1usize, 2, 7, 16, 33, MAX_STEPS] {
                    for rate in [0.125f32, 1.0, 8.0] {
                        let p = GenParams { osc_rate: rate, phase, ..params(steps, 3, 0.7, seed) };
                        let pat = generate(&p);
                        for i in 0..steps {
                            for v in pat.traj[i] {
                                assert!(v.is_finite() && v.abs() < 16.0, "seed={seed} phase={phase} steps={steps} traj={:?}", pat.traj[i]);
                            }
                            assert!(pat.accent[0][i].is_finite());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn hit_counts_follow_the_density_scaled_target() {
        for steps in [1usize, 2, 7, 16, 31, 63, 64] {
            for pulses in [0usize, 1, 2, 5, 16, 64] {
                for density in [0.0f32, 0.13, 0.45, 0.8, 1.0] {
                    let pat = generate(&params(steps, pulses, density, 0.271));
                    // `generate` clamps pulses to the step count before dispatch, so the
                    // expectation has to use the clamped value or it measures the wrong thing.
                    let pulses = pulses.min(steps);
                    for tr in 0..TRACKS {
                        let expected = ((pulses as f32 * density * (0.45 + DENSITY_PROFILE[tr])).round() as usize).min(steps);
                        let actual = pat.hit[tr][..steps].iter().filter(|&&h| h).count();
                        assert_eq!(actual, expected, "steps={steps} pulses={pulses} density={density} lane={tr}");
                    }
                }
            }
        }
    }

    #[test]
    fn different_seeds_give_different_orbits_and_the_same_seed_repeats() {
        let base = params(16, 5, 0.8, 0.271);
        let a = generate(&base);
        let again = generate(&base);
        assert_eq!(a.traj, again.traj, "same snapshot must repeat");
        assert_eq!(a.hit, again.hit);
        for seed in [0.05f32, 0.5, 0.77, 0.95] {
            let other = generate(&GenParams { seed, ..base });
            assert_ne!(a.traj, other.traj, "seed={seed} produced an identical orbit");
            assert_ne!(a.hit, other.hit, "seed={seed} produced an identical pattern");
        }
    }

    #[test]
    fn the_third_body_actually_moves_relative_to_the_pair() {
        // A frozen orbit would make every pattern identical apart from ties; require real
        // travel so the mode stays a chaos source and not a metronome.
        let pat = generate(&params(32, 8, 1.0, 0.271));
        let mut lo = [f32::INFINITY; 2];
        let mut hi = [f32::NEG_INFINITY; 2];
        for i in 0..32 {
            lo[0] = lo[0].min(pat.traj[i][2]); hi[0] = hi[0].max(pat.traj[i][2]);
            lo[1] = lo[1].min(pat.traj[i][3]); hi[1] = hi[1].max(pat.traj[i][3]);
        }
        assert!(hi[0] - lo[0] > 0.2 && hi[1] - lo[1] > 0.2, "orbit is static: {:?}", (lo, hi));
    }
}