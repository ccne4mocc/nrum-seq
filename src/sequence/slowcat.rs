use super::{set_hit, GenParams, Pattern, TRACKS};

/// Phrase length for one lane. Shared with the panel so the drawn phrase boundaries are the
/// ones the generator used.
pub(crate) fn motif_len(tr: usize, count: usize, steps: usize) -> usize {
    (2 + (tr + count) % 7).min(steps).max(1)
}

/// Slowcat-style rhythm: rotate through lane motifs over successive slow subdivisions.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let count = p.pulses.max(1);
    for tr in 0..TRACKS {
        let motif = motif_len(tr, count, p.steps);
        let shift = ((p.seed * 997.0) as usize + tr * 3 + p.variation as usize) % motif;
        for i in 0..p.steps {
            let slot = (i + shift) % motif;
            let phrase = (i / motif + tr) % (tr % 3 + 2);
            let hit = (slot * count + tr) % motif < count.min(motif) && phrase != 0 && ((i + tr) % 4 == 0 || p.density > 0.35);
            set_hit(&mut out, tr, i, hit && ((i + tr * 2) % 7 != 6 || p.density > 0.75), if i % 4 == 0 { 1.0 } else { 0.66 });
        }
    }
    out
}