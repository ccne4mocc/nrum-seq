use super::{select_spread, set_hit, GenParams, Pattern, MAX_STEPS, TRACKS};

/// Convert a 1D Hilbert index to `(x, y)` on a `2^order x 2^order` grid. Exported so the panel
/// draws the same curve the generator sampled.
pub(crate) fn point(index: usize, grid: usize) -> (usize, usize) {
    let mut x = 0usize;
    let mut y = 0usize;
    let mut t = index;
    let mut s = 1usize;
    while s < grid {
        let rx = (t >> 1) & 1;
        let ry = (t ^ rx) & 1;
        if ry == 0 {
            if rx == 1 {
                x = s - 1 - x;
                y = s - 1 - y;
            }
            let tmp = x;
            x = y;
            y = tmp;
        }
        x += rx * s;
        y += ry * s;
        t >>= 2;
        s <<= 1;
    }
    (x, y)
}

/// Quarter turns selected by the phase control (0..3).
pub(crate) fn turns(phase: f32) -> usize {
    ((phase.clamp(0.0, 0.999_999) * 4.0) as usize) % 4
}

/// Rotate a grid coordinate by `turns` quarter turns inside a `2^order` square.
pub(crate) fn rotate(order: u32, d: usize, turns: usize) -> (usize, usize) {
    let size = 1usize << order;
    let (mut x, mut y) = point(d, size);
    for _ in 0..turns {
        let nx = y;
        let ny = size - 1 - x;
        x = nx;
        y = ny;
    }
    (x, y)
}

/// nelody sequencer-style Hilbert sampling: `d = i·total/n`, so the loop walks the whole curve. `pulses`
/// is the order (1..5), `phase` turns the curve by quarter turns, and `seed` shifts the
/// curve-position-to-lane mapping. Density thins each lane's assigned cells evenly.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    if p.pulses == 0 || p.density <= 0.0 { return out; }
    let order = p.pulses.clamp(1, 5) as u32;
    let size = 1usize << order;
    let total = (size * size).max(1);
    let turns = turns(p.phase);
    let seed_offset = ((p.seed.clamp(0.0, 0.999_999) * TRACKS as f32).round() as usize) % TRACKS;
    let denom = (size.saturating_sub(1)).max(1) as f32;
    let mut primary = [[false; MAX_STEPS]; TRACKS];
    for i in 0..n {
        let d = (i as u64 * total as u64 / n as u64) as usize;
        let (x, y) = rotate(order, d, turns);
        // The curve position picks the lane; adding the step index keeps every lane reachable
        // even when a short loop samples only a sparse set of curve cells.
        let lane = (x + 2 * y + i + seed_offset) % TRACKS;
        primary[lane][i] = true;
        out.orbit[i] = x as f32 / denom;
        out.theta[i] = y as f32 / denom;
        out.u[i] = out.orbit[i];
        out.state[i] = (lane + 1) as u8;
    }
    let density = p.density.clamp(0.0, 1.0);
    for tr in 0..TRACKS {
        let m = primary[tr][..n].iter().filter(|&&b| b).count();
        if m == 0 { continue; }
        let wanted = ((m as f32 * density).round() as usize).max(1).min(m);
        let mut hits = [false; MAX_STEPS];
        select_spread(&primary[tr], n, wanted, &mut hits);
        for i in 0..n {
            if hits[i] {
                let accent = 0.55 + 0.45 * out.theta[i];
                set_hit(&mut out, tr, i, true, accent);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Mode;

    fn params(steps: usize, pulses: usize, density: f32, phase: f32, seed: f32) -> GenParams {
        GenParams {
            kind: Mode::Hilbert, steps, pulses, density, phase, seed, probability: 1.0,
            ..GenParams::default()
        }
    }

    #[test]
    fn pulses_select_the_order_and_sampled_cells_follow_the_curve() {
        for order in 1..=5u32 {
            let out = generate(&params(64, order as usize, 1.0, 0.0, 0.0));
            let size = 1usize << order;
            let total = size * size;
            for i in 0..64 {
                let d = (i * total / 64) as usize;
                let (x, y) = rotate(order, d, 0);
                let got = (
                    (out.orbit[i] * (size - 1) as f32).round() as usize,
                    (out.theta[i] * (size - 1) as f32).round() as usize,
                );
                assert_eq!(got, (x, y), "order {order} step {i}");
            }
        }
        // The curve itself is a chain of adjacent cells.
        for order in 1..=5u32 {
            let size = 1usize << order;
            for d in 1..size * size {
                let (a, b) = (point(d - 1, size), point(d, size));
                assert_eq!(a.0.abs_diff(b.0) + a.1.abs_diff(b.1), 1, "order {order} d={d}");
            }
        }
    }

    #[test]
    fn phase_rotates_and_seed_moves_the_lane_mapping() {
        let base = generate(&params(32, 5, 1.0, 0.0, 0.0));
        let turned = generate(&params(32, 5, 1.0, 0.25, 0.0));
        // One quarter turn maps (x,y) to (y, size-1-x).
        let size = 32usize;
        for i in 0..32 {
            let x = (base.orbit[i] * (size - 1) as f32).round() as usize;
            let y = (base.theta[i] * (size - 1) as f32).round() as usize;
            let want = ((y as f32) / (size - 1) as f32, ((size - 1 - x) as f32) / (size - 1) as f32);
            assert!((turned.orbit[i] - want.0).abs() < 1e-6 && (turned.theta[i] - want.1).abs() < 1e-6,
                "step {i}: {:?} vs {:?}", (turned.orbit[i], turned.theta[i]), want);
        }
        let moved = generate(&params(32, 5, 1.0, 0.0, 0.7));
        assert_ne!(base.hit, moved.hit);
    }

    #[test]
    fn every_lane_sounds_at_default_order_and_full_density() {
        for steps in [16usize, 32, 64] {
            let out = generate(&params(steps, 5, 1.0, 0.0, 0.0));
            for tr in 0..TRACKS {
                assert!(out.hit[tr][..steps].iter().any(|&h| h), "steps={steps} lane={tr}");
            }
        }
    }

    #[test]
    fn density_thins_each_lanes_cells() {
        let full = generate(&params(64, 5, 1.0, 0.0, 0.0));
        let sparse = generate(&params(64, 5, 0.3, 0.0, 0.0));
        let full_hits: usize = (0..TRACKS).map(|tr| full.hit[tr][..64].iter().filter(|&&h| h).count()).sum();
        let sparse_hits: usize = (0..TRACKS).map(|tr| sparse.hit[tr][..64].iter().filter(|&&h| h).count()).sum();
        assert!(sparse_hits < full_hits, "density did not thin {full_hits} -> {sparse_hits}");
    }
}
