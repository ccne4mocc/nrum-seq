use super::{set_hit, GenParams, Pattern, TRACKS};

/// Slowcat-style rhythm: rotate through lane motifs over successive slow subdivisions.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let count = p.pulses.max(1);
    for tr in 0..TRACKS {
        let motif_len = (2 + (tr + count) % 7).min(p.steps).max(1);
        let shift = ((p.seed * 997.0) as usize + tr * 3 + p.variation as usize) % motif_len;
        for i in 0..p.steps {
            let slot = (i + shift) % motif_len;
            let phrase = (i / motif_len + tr) % (tr % 3 + 2);
            let hit = (slot * count + tr) % motif_len < count.min(motif_len) && phrase != 0 && ((i + tr) % 4 == 0 || p.density > 0.35);
            set_hit(&mut out, tr, i, hit && ((i + tr * 2) % 7 != 6 || p.density > 0.75), if i % 4 == 0 { 1.0 } else { 0.66 });
        }
    }
    out
}
