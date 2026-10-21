//! Xenakis sieve. A step is a sieve member when it belongs to either of two residue classes
//! and to neither of a third: `M = (m₁,r₁) ∪ (m₂,r₂) \ (m₃,r₃)`.
//!
//! `ca_rule` picks the three moduli, `symmetry` and `phase` pick the residues, `pulses` says
//! how many union terms are active (0 silences, 1 = one class, ≥2 = both) and density decides
//! how many members survive. Each lane reads the same sieve shifted around the ring, which is
//! what makes the kit interlock instead of stacking.

use super::{set_hit, GenParams, Pattern, DENSITY_PROFILE, MAX_STEPS, TRACKS};

/// Residue-class moduli, small primes so 64 steps hold several periods of each.
pub(crate) const MODULI: [usize; 6] = [2, 3, 5, 7, 11, 13];

/// The three classes the sieve is built from: `[union, union, subtract]`, each `(modulus, residue)`.
pub(crate) fn classes(p: &GenParams) -> [(usize, usize); 3] {
    let rule = p.ca_rule as usize;
    let m1 = MODULI[rule % MODULI.len()];
    let m2 = MODULI[(rule / MODULI.len()) % MODULI.len()];
    let m3 = MODULI[(rule / (MODULI.len() * MODULI.len()) + p.symmetry) % MODULI.len()];
    // Variation re-rolls the residues, so Generate changes the sieve instead of only rotating it.
    let shift = (p.phase.clamp(0.0, 1.0) * MODULI.len() as f32).round() as usize
        + p.variation as usize;
    let r1 = (p.symmetry + shift) % m1;
    let r2 = (p.symmetry * 2 + shift) % m2;
    let r3 = (p.symmetry * 3 + shift + (p.seed.clamp(0.0, 1.0) * 97.0) as usize) % m3;
    [(m1, r1), (m2, r2), (m3, r3)]
}

/// Whether `step` belongs to a residue class.
pub(crate) fn class_contains(step: usize, class: (usize, usize)) -> bool {
    class.0 > 0 && step % class.0 == class.1
}

fn union_terms(p: &GenParams) -> usize {
    if p.pulses >= 2 { 2 } else { 1 }
}

fn is_member(step: usize, cls: &[(usize, usize); 3], terms: usize) -> bool {
    (0..terms).any(|k| class_contains(step, cls[k])) && !class_contains(step, cls[2])
}

fn lane_shift(n: usize, lane: usize) -> usize {
    lane * (n / TRACKS).max(1)
}

/// Lane 0's sieve over `n` steps, as a bitmask. The panel draws this exact mask.
pub(crate) fn mask(p: &GenParams, n: usize) -> u64 {
    if p.pulses == 0 { return 0; }
    let cls = classes(p);
    let terms = union_terms(p);
    let mut bits = 0u64;
    for i in 0..n.min(MAX_STEPS) {
        if is_member(i, &cls, terms) { bits |= 1 << i; }
    }
    bits
}

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    if p.pulses == 0 { return out; }
    let cls = classes(p);
    let terms = union_terms(p);
    let d = p.density.clamp(0.0, 1.0);

    for tr in 0..TRACKS {
        let shift = lane_shift(n, tr);
        let mut score = [0.0f32; MAX_STEPS];
        let mut members = 0usize;
        for i in 0..n {
            let idx = (i + shift) % n;
            if !is_member(idx, &cls, terms) { continue; }
            members += 1;
            let depth = (0..terms).filter(|&k| class_contains(idx, cls[k])).count();
            // Small deterministic lane jitter breaks ties between equally deep members, so a
            // partial-density lane drops different members than its neighbour instead of every
            // lane keeping the same first-k columns.
            let jitter = super::Rng::new(
                p.seed, p.variation, 0x51E7_0000 + (tr * MAX_STEPS + i) as u64,
            ).unit() * 0.15;
            score[i] = 1.0 + 0.25 * depth as f32 + jitter;
        }
        // A short pattern can miss a sparse sieve entirely. Fall back to the step grid, phased
        // per lane, so a one-step pattern still sounds and the lanes still differ.
        let candidates = if members > 0 { members } else { n };
        if members == 0 {
            for i in 0..n { score[i] = 1.0 - ((i + shift) % n) as f32 / n as f32; }
        }
        let wanted = (candidates as f32 * d * (0.35 + DENSITY_PROFILE[tr])).round() as usize;
        let wanted = wanted.min(candidates);
        if wanted == 0 { continue; }

        // Rank selection: exact count, no sort and no allocation on the audio path.
        for i in 0..n {
            let mut rank = 0usize;
            for j in 0..n {
                if score[j] > score[i] || (score[j] == score[i] && j < i) { rank += 1; }
            }
            if rank < wanted { set_hit(&mut out, tr, i, true, (0.55 + 0.15 * score[i]).min(1.0)); }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Mode;

    fn params(steps: usize, pulses: usize, density: f32) -> GenParams {
        GenParams { kind: Mode::Sieve, steps, pulses, density, probability: 1.0, ..GenParams::default() }
    }

    #[test]
    fn the_mask_is_the_classes_union_minus_the_subtraction() {
        for ca_rule in [0u8, 7, 30, 99, 200, 255] {
            for symmetry in [1usize, 4, 8] {
                let p = GenParams { ca_rule, symmetry, ..params(32, 2, 0.8) };
                let cls = classes(&p);
                let mask = mask(&p, 32);
                for i in 0..32 {
                    let wanted = class_contains(i, cls[0]) || class_contains(i, cls[1]);
                    let wanted = wanted && !class_contains(i, cls[2]);
                    assert_eq!(mask & (1 << i) != 0, wanted, "rule={ca_rule} symmetry={symmetry} step={i}");
                }
            }
        }
    }

    #[test]
    fn every_lane_keeps_exactly_the_density_scaled_member_count() {
        for steps in [7usize, 16, 32, 64] {
            for density in [0.25f32, 0.65, 1.0] {
                let p = params(steps, 2, density);
                let pat = generate(&p);
                for tr in 0..TRACKS {
                    let shift = lane_shift(steps, tr);
                    let cls = classes(&p);
                    let members = (0..steps)
                        .filter(|&i| is_member((i + shift) % steps, &cls, 2))
                        .count();
                    let want = ((members as f32 * density * (0.35 + DENSITY_PROFILE[tr])).round()
                        as usize).min(members);
                    let got = pat.hit[tr][..steps].iter().filter(|&&h| h).count();
                    assert_eq!(got, want, "steps={steps} density={density} lane={tr}");
                }
            }
        }
    }

    #[test]
    fn a_one_step_pattern_and_an_empty_sieve_still_sound() {
        for ca_rule in 0..=255u8 {
            let p = GenParams { ca_rule, ..params(1, 1, 1.0) };
            assert!((0..TRACKS).any(|tr| generate(&p).hit[tr][0]), "rule {ca_rule} silent at one step");
        }
    }

    #[test]
    fn terms_follow_the_pulse_control() {
        let one = mask(&params(64, 1, 1.0), 64);
        let two = mask(&params(64, 2, 1.0), 64);
        assert!(two.count_ones() > one.count_ones(), "the second union term must add members");
        assert_eq!(mask(&params(64, 0, 1.0), 64), 0, "zero terms is an empty sieve");
    }
}
