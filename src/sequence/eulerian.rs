use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

/// Builds a balanced directed multigraph from a seeded permutation, then traverses its Euler
/// circuit with Hierholzer's algorithm. Vertex 0 is a rest; vertices 1..=8 are drum lanes.
pub(super) fn generate(p: &GenParams) -> Pattern {
    const V: usize = TRACKS + 1;
    const MAX_DEG: usize = 4;
    let degree = 1 + (p.pulses % MAX_DEG);
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let mut rng = Rng::new(p.seed, p.variation, 0xE013_1234);
    let mut perm = [0u8; V];
    for (i, x) in perm.iter_mut().enumerate() { *x = i as u8; }
    for i in (1..V).rev() { let j = rng.below(i + 1); perm.swap(i, j); }

    // The ring edge guarantees one connected component; every extra permutation shift is balanced too.
    let mut to = [[0u8; MAX_DEG]; V];
    for v in 0..V {
        to[v][0] = ((v + 1) % V) as u8;
        for edge in 1..degree { to[v][edge] = perm[(v + edge) % V]; }
    }
    let mut used = [0u8; V];
    let mut stack = [0u8; V * MAX_DEG + 1];
    let mut stack_len = 1usize;
    let mut reverse = [0u8; V * MAX_DEG + 1];
    let mut rev_len = 0usize;
    stack[0] = 0;
    while stack_len > 0 {
        let v = stack[stack_len - 1] as usize;
        if (used[v] as usize) < degree {
            let edge = used[v] as usize;
            used[v] += 1;
            stack[stack_len] = to[v][edge];
            stack_len += 1;
        } else {
            stack_len -= 1;
            reverse[rev_len] = stack[stack_len];
            rev_len += 1;
        }
    }
    // reverse[] contains the circuit backwards. Repeating the circuit handles step lengths > E.
    let mut circuit = [0u8; V * MAX_DEG + 1];
    for i in 0..rev_len { circuit[i] = reverse[rev_len - 1 - i]; }
    // The circuit starts and ends on the rest vertex, so the closing vertex duplicates the
    // opening one. Repeating the full length put two rests back to back at every seam.
    let circuit_len = rev_len.saturating_sub(1).max(1);
    for i in 0..p.steps {
        // A circuit always starts at the rest vertex, so a one-step pattern would be silent.
        // Start from the first state that is not rest instead.
        let mut pos = i % circuit_len;
        if p.steps == 1 {
            pos = (0..circuit_len).find(|&j| circuit[j] > 0).unwrap_or(pos);
        }
        let state = circuit[pos];
        out.state[i] = state;
        if state > 0 {
            let tr = state as usize - 1;
            let threshold = p.density.clamp(0.0, 1.0) * (0.4 + super::DENSITY_PROFILE[tr]);
            let mut local = Rng::new(p.seed, p.variation, 0xE013_9000 + i as u64 * 17 + tr as u64);
            // A single step has no room for the stochastic gate to fail.
            let hit = p.steps == 1 || local.unit() < threshold.clamp(0.0, 1.0);
            set_hit(&mut out, tr, i, hit, 0.7 + local.unit() * 0.3);
        }
    }
    out
}
