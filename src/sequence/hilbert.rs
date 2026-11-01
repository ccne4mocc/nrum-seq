use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

// Convert a 1D Hilbert index to (x,y) on a 2^k x 2^k grid.
fn hilbert_xy(index: usize, n: usize) -> (usize, usize) {
    let mut x = 0usize;
    let mut y = 0usize;
    let mut t = index;
    let mut s = 1usize;
    while s < n {
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

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    // order from pulses (clamped)
    let order = (p.pulses.max(1)).min(6);
    let grid = 1usize << order;
    let prob = p.probability.clamp(0.0, 1.0);
    let density = p.density.clamp(0.0, 1.0);
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0x48_00_00_00 + tr as u64);
        let phase_off = (tr as usize) * (grid / TRACKS.max(1)).max(1);
        for i in 0..n {
            let idx = (i + phase_off) % (grid * grid).max(1);
            let (x, y) = hilbert_xy(idx, grid);
            if tr == 0 {
                out.orbit[i] = x as f32 / grid as f32;
                out.theta[i] = y as f32 / grid as f32;
            }
            // onset when the point lies on the curve's "corner" or passes threshold
            let on_curve = (x + y) % 2 == 0;
            let mut hit = on_curve && rng.unit() < density * prob;
            if !hit && density >= 1.0 { hit = rng.unit() < prob; }
            if hit && tr > 0 && (0..tr).all(|k| out.hit[k][i]) { hit = false; }
            let accent = if hit { 0.6 + 0.4 * rng.unit() as f32 } else { 0.0 };
            set_hit(&mut out, tr, i, hit, accent);
        }
    }
    out
}