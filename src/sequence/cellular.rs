use super::{enabled_threshold, set_hit, GenParams, Pattern, Rng, CA_ROWS, TRACKS};

/// Generation a lane reads; the panel marks the same rows the hits came from.
pub(crate) fn selected_row(p: &GenParams, tr: usize) -> usize {
    if p.steps <= 1 {
        0
    } else {
        (p.pulses + (p.phase * CA_ROWS as f32).round() as usize + tr * 2) % CA_ROWS
    }
}

/// Elementary 1D cellular automaton on a ring; `ca_rule` is the Wolfram rule number.
/// The selected generation is `pulses % CA_ROWS`; phase offsets the selected row per lane.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let mut rng = Rng::new(p.seed, p.variation, 0xCA30_0001);
    // Seed at `density`, then force a gap unless the ring is too short: a solid row maps to 0
    // under rule 30 and would silence every lane.
    let mut live = 0usize;
    for i in 0..p.steps {
        out.ca[0][i] = rng.unit() < p.density.clamp(0.0, 1.0);
        live += out.ca[0][i] as usize;
    }
    if live == p.steps && p.steps > 1 { out.ca[0][rng.below(p.steps)] = false; }
    for row in 1..CA_ROWS {
        for i in 0..p.steps {
            let left = out.ca[row - 1][(i + p.steps - 1) % p.steps] as u8;
            let mid = out.ca[row - 1][i] as u8;
            let right = out.ca[row - 1][(i + 1) % p.steps] as u8;
            let neighborhood = (left << 2) | (mid << 1) | right;
            out.ca[row][i] = ((p.ca_rule >> neighborhood) & 1) != 0;
        }
    }
    for tr in 0..TRACKS {
        // With one step generations 1..CA_ROWS collapse; `selected_row` clamps to row 0.
        let selected = selected_row(p, tr);
        let threshold = enabled_threshold(p, tr);
        let mut local = Rng::new(p.seed, p.variation, 0xCA80_0000 + tr as u64);
        for i in 0..p.steps {
            let cell = out.ca[selected][i];
            // Generations 1..CA_ROWS are copies of row 0; take the row as-is rather than gate it.
            let hit = if p.steps == 1 { cell } else { cell && local.unit() < (0.45 + threshold * 0.55) };
            set_hit(&mut out, tr, i, hit, if cell { 1.0 } else { 0.6 });
        }
    }
    out
}
