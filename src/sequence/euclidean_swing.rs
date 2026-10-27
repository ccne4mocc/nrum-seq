use super::{GenParams, Pattern, Rng, TRACKS};

pub(super) fn generate(p: &GenParams) -> Pattern {
    // Euclidean base plus a per-lane swing feel handled at scheduling time.
    let mut out = super::euclidean::generate(p);
    // Mark swing accents on off-beats.
    let n = p.steps.max(1);
    for tr in 0..TRACKS {
        let mut rng = Rng::new(p.seed, p.variation, 0x5E10_0000 + tr as u64);
        for i in 0..n {
            if out.hit[tr][i] && i % 2 == 1 {
                out.accent[tr][i] = (0.55 + 0.25 * rng.unit()).min(1.0);
            }
        }
    }
    out
}