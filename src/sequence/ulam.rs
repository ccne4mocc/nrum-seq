use super::{lane_wanted, select_spread, set_hit, GenParams, Pattern, MAX_STEPS, TRACKS};

fn is_prime(n: usize) -> bool {
    if n < 2 { return false; }
    if n < 4 { return true; }
    if n % 2 == 0 { return false; }
    let mut d = 3;
    while d * d <= n {
        if n % d == 0 { return false; }
        d += 2;
    }
    true
}

/// Ulam spiral primes: walk the square spiral once, mark cells whose number is prime, and assign
/// each prime cell to a lane from its diagonal (`x - y`). The spiral starts at
/// `1 + round(64*phase) + 5*variation`, so phase and Generate move the primes onto other
/// diagonals, as in Ulam's own spirals started at 17 or 41.
///
/// The old version was not a spiral at all: it tested the primality of the step index and the
/// panel drew a different spiral of its own. The coordinates below are the ones the hits use.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    if p.pulses == 0 || p.density <= 0.0 { return out; }
    // Bound the start integer: primality is trial division, so an unbounded variation would make
    // the per-cell work grow without limit. The window repeats after 1024 variations.
    let offset = (p.phase * 64.0).round() as usize + 5 * (p.variation as usize % 1024);
    let mut primary = [[false; MAX_STEPS]; TRACKS];
    let mut xs = [0i32; MAX_STEPS];
    let mut ys = [0i32; MAX_STEPS];
    // Legs 1,1,2,2,3,3, ...; the standard square-spiral walk.
    let (mut x, mut y) = (0i32, 0i32);
    let (mut dx, mut dy) = (1i32, 0i32);
    let (mut leg, mut leg_pos, mut turns) = (1i32, 0i32, 0i32);
    for i in 0..n {
        let cell = 1 + offset + i;
        let prime = cell == 1 || is_prime(cell);
        xs[i] = x;
        ys[i] = y;
        out.state[i] = prime as u8;
        if prime {
            let lane = (x - y).rem_euclid(TRACKS as i32) as usize;
            primary[lane][i] = true;
        }
        x += dx;
        y += dy;
        leg_pos += 1;
        if leg_pos == leg {
            leg_pos = 0;
            let (ndx, ndy) = (-dy, dx);
            dx = ndx;
            dy = ndy;
            turns += 1;
            if turns % 2 == 0 { leg += 1; }
        }
    }
    let (x0, x1) = (xs[..n].iter().copied().min().unwrap_or(0), xs[..n].iter().copied().max().unwrap_or(0));
    let (y0, y1) = (ys[..n].iter().copied().min().unwrap_or(0), ys[..n].iter().copied().max().unwrap_or(0));
    let norm = |v: i32, lo: i32, hi: i32| if hi > lo { (v - lo) as f32 / (hi - lo) as f32 } else { 0.5 };
    for i in 0..n {
        out.orbit[i] = norm(xs[i], x0, x1);
        out.theta[i] = norm(ys[i], y0, y1);
        out.u[i] = ((out.orbit[i] + out.theta[i]) * 0.5).clamp(0.0, 1.0);
    }
    for tr in 0..TRACKS {
        let wanted = lane_wanted(p.pulses, p.density, tr, n);
        if wanted == 0 { continue; }
        let m = primary[tr][..n].iter().filter(|&&b| b).count();
        if m == 0 {
            // No prime on this diagonal yet (short loops). Keep the lane audible on its own
            // phase instead of silently dropping it.
            set_hit(&mut out, tr, (tr * n / TRACKS) % n, true, 0.6);
            continue;
        }
        let k = wanted.min(m).max(1);
        let mut hits = [false; MAX_STEPS];
        select_spread(&primary[tr], n, k, &mut hits);
        for i in 0..n {
            if hits[i] { set_hit(&mut out, tr, i, true, 0.95); }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Mode;

    fn params(steps: usize, density: f32, phase: f32, variation: u32) -> GenParams {
        GenParams {
            kind: Mode::Ulam, steps, density, phase, variation, pulses: 8, probability: 1.0,
            ..GenParams::default()
        }
    }

    #[test]
    fn spiral_consecutive_cells_are_one_manhattan_step_apart() {
        // The published coordinates must describe one continuous square spiral.
        let out = generate(&params(64, 1.0, 0.0, 0));
        let s = out.rotation;
        for j in 1..64 {
            let a = (j - 1 + s) % 64;
            let b = (j + s) % 64;
            let d = (out.orbit[a] - out.orbit[b]).abs() + (out.theta[a] - out.theta[b]).abs();
            assert!(d > 0.0, "step {j} did not move");
        }
    }

    #[test]
    fn onsets_are_primes_and_phase_or_variation_moves_them() {
        // Replay the spiral: lane `(x-y) mod 8`, cell `1 + offset + i`.
        // Every prime on a lane must be selectable; fallback hits are only allowed on a lane
        // that has no prime cells at all in this window.
        let out = generate(&params(64, 1.0, 0.0, 0));
        let offset = 0usize;
        let mut x = 0i32; let mut y = 0i32;
        let mut dx = 1i32; let mut dy = 0i32;
        let mut leg = 1i32; let mut leg_pos = 0i32; let mut turns = 0i32;
        let mut primes = [[false; 64]; TRACKS];
        for i in 0..64 {
            let cell = 1 + offset + i;
            if cell == 1 || is_prime(cell) {
                let lane = (x - y).rem_euclid(TRACKS as i32) as usize;
                primes[lane][i] = true;
            }
            x += dx; y += dy; leg_pos += 1;
            if leg_pos == leg {
                leg_pos = 0;
                let (ndx, ndy) = (-dy, dx);
                dx = ndx; dy = ndy; turns += 1;
                if turns % 2 == 0 { leg += 1; }
            }
        }
        for tr in 0..TRACKS {
            let m = primes[tr].iter().filter(|&&b| b).count();
            if m == 0 { continue; }
            for i in 0..64 {
                if out.hit[tr][i] {
                    assert!(primes[tr][i], "lane {tr} step {i} is not a prime on its diagonal");
                }
            }
        }

        let key = |phase: f32, v: u32| {
            let out = generate(&params(64, 1.0, phase, v));
            (0..TRACKS).map(|tr| (0..64).filter(|&i| out.hit[tr][i]).collect::<Vec<_>>()).collect::<Vec<_>>()
        };
        assert_ne!(key(0.0, 0), key(0.0, 1));
        assert_ne!(key(0.0, 0), key(0.3, 0));
    }

    #[test]
    fn every_lane_sounds_at_full_density() {
        for steps in [1usize, 7, 16, 32, 64] {
            let out = generate(&params(steps, 1.0, 0.0, 0));
            for tr in 0..TRACKS {
                assert!(out.hit[tr][..steps].iter().any(|&h| h), "steps={steps} lane={tr}");
            }
        }
    }
}
