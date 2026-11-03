use super::{set_hit, GenParams, Pattern, Rng, CA_ROWS, TRACKS};

/// Fixed 16 × 8 torus: sixteen time positions across, one row per drum lane. Keeping the grid
/// this small bounds the worst case to `steps × 128` cell updates.
pub(crate) const WIDTH: usize = 16;
pub(crate) const HEIGHT: usize = 8;

const LIFE_BIRTH: u16 = 1 << 3;
const LIFE_SURVIVE: u16 = (1 << 2) | (1 << 3);

fn rule_masks(rule: u8) -> (u16, u16) {
    match rule % 5 {
        0 => (LIFE_BIRTH, LIFE_SURVIVE),
        // HighLife: B36/S23.
        1 => ((1 << 3) | (1 << 6), LIFE_SURVIVE),
        // Seeds: B2/S.
        2 => (1 << 2, 0),
        // Day & Night: B3678/S34678.
        3 => ((1 << 3) | (1 << 6) | (1 << 7) | (1 << 8), (1 << 3) | (1 << 4) | (1 << 6) | (1 << 7) | (1 << 8)),
        // A dense, high-survival rule that keeps the eight rows active.
        _ => (1 << 3, (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4) | (1 << 5)),
    }
}

type Grid = [[bool; WIDTH]; HEIGHT];

fn seed(p: &GenParams, rng: &mut Rng) -> Grid {
    let mut grid = [[false; WIDTH]; HEIGHT];
    match p.symmetry {
        1 => {
            // Centre block.
            for row in 2..6 {
                for col in 6..10 { grid[row][col] = true; }
            }
        }
        2 => {
            // Vertical stripes.
            for row in 0..HEIGHT {
                for col in (0..WIDTH).step_by(2) { grid[row][col] = true; }
            }
        }
        3 => {
            // Sparse random with a guaranteed live cell in every row.
            for row in 0..HEIGHT {
                for col in 0..WIDTH {
                    grid[row][col] = rng.unit() < p.density.clamp(0.0, 1.0) * 0.35;
                }
            }
        }
        _ => {
            for row in 0..HEIGHT {
                for col in 0..WIDTH {
                    grid[row][col] = rng.unit() < p.density.clamp(0.0, 1.0) * 0.6;
                }
            }
        }
    }
    // A diagonal seed guarantees every lane has something to read on the first generation.
    for row in 0..HEIGHT { grid[row][row % WIDTH] = true; }
    grid
}

fn step(grid: &Grid, birth: u16, survive: u16) -> Grid {
    let mut next = [[false; WIDTH]; HEIGHT];
    for row in 0..HEIGHT {
        for col in 0..WIDTH {
            let mut neighbours = 0u16;
            for dr in [-1i32, 0, 1] {
                for dc in [-1i32, 0, 1] {
                    if dr == 0 && dc == 0 { continue; }
                    let r = (row as i32 + dr).rem_euclid(HEIGHT as i32) as usize;
                    let c = (col as i32 + dc).rem_euclid(WIDTH as i32) as usize;
                    neighbours += grid[r][c] as u16;
                }
            }
            let mask = 1u16 << neighbours;
            next[row][col] = if grid[row][col] { survive & mask != 0 } else { birth & mask != 0 };
        }
    }
    // A dead torus is a dead lane bank; inject one cell so the next generation can recover.
    if !next.iter().any(|row| row.iter().any(|cell| *cell)) { next[0][0] = true; }
    next
}

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let mut rng = Rng::new(p.seed, p.variation, 0x11FE_0000);
    let (birth, survive) = rule_masks(p.ca_rule);
    let mut grid = seed(p, &mut rng);
    // `pulses` is the warm-up generation count; it gives the automaton time to develop before
    // the first audible step without changing the step count.
    let warmup = p.pulses.min(64);
    for _ in 0..warmup { grid = step(&grid, birth, survive); }
    for i in 0..p.steps {
        let live: [usize; HEIGHT] = std::array::from_fn(|row| grid[row].iter().filter(|cell| **cell).count());
        let mut any = false;
        for tr in 0..TRACKS {
            let count = if tr < HEIGHT { live[tr] } else { 0 };
            let mut local = Rng::new(p.seed, p.variation, 0x11FE_1000 + (i * TRACKS + tr) as u64);
            let hit = count > 0
                && (i == 0 || local.unit() < (0.55 + p.density.clamp(0.0, 1.0) * 0.45))
                && !(tr > 0 && (0..tr).all(|previous| out.hit[previous][i]));
            any |= hit;
            set_hit(&mut out, tr, i, hit, (0.35 + count as f32 / WIDTH as f32 * 0.65).clamp(0.35, 1.0));
        }
        // A generation with live cells must be audible somewhere.
        if !any {
            let lane = (0..TRACKS).max_by_key(|tr| if *tr < HEIGHT { live[*tr] } else { 0 }).unwrap_or(0);
            set_hit(&mut out, lane, i, true, 0.7);
        }
        // Publish the current snapshot for the panel, then advance one generation per step.
        for row in 0..HEIGHT { out.ca[row][..WIDTH].copy_from_slice(&grid[row]); }
        grid = step(&grid, birth, survive);
    }
    // Keep `CA_ROWS` bounded even if the panel reads past the live rows.
    let _ = CA_ROWS;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Mode;
    use crate::sequence::generate;

    #[test]
    fn every_rule_family_produces_hits_and_a_bounded_grid() {
        for rule in 0..5u8 {
            for symmetry in 1..=4usize {
                let p = GenParams {
                    kind: Mode::Life, steps: 64, pulses: 4, density: 0.5,
                    ca_rule: rule, symmetry, probability: 1.0, ..GenParams::default()
                };
                let out = generate(&p);
                assert!(out.hit.iter().any(|lane| lane[..64].iter().any(|hit| *hit)), "rule {rule} symmetry {symmetry}");
                assert!(out.ca[..HEIGHT].iter().any(|row| row[..WIDTH].iter().any(|cell| *cell)),
                    "published grid is empty for rule {rule} symmetry {symmetry}");
            }
        }
    }

    #[test]
    fn one_step_patterns_make_a_sound() {
        let out = generate(&GenParams { kind: Mode::Life, steps: 1, pulses: 0, density: 0.25, probability: 1.0, ..GenParams::default() });
        assert!(out.hit.iter().any(|lane| lane[0]));
    }
}
