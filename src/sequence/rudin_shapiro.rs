use super::{lane_wanted, select_spread, set_hit, GenParams, Pattern, MAX_STEPS, TRACKS};

/// Rudin-Shapiro bit: parity of the number of overlapping `11` pairs in the binary expansion
/// (OEIS A020985, with 0/1 encoding).
pub(crate) fn bit(n: usize) -> u8 {
    ((n & (n >> 1)).count_ones() & 1) as u8
}

/// Rudin-Shapiro word: onsets where the bit is 1, thinned evenly to the lane's target count.
/// Each lane reads a different window, and the seven-slot contour gives the word a colour
/// without turning it back into a binary pitch.
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    if p.pulses == 0 || p.density <= 0.0 { return out; }
    let offset = (p.phase * 16.0).round() as usize + (p.variation as usize % 1024);
    let stride = (n / TRACKS).max(1);
    for i in 0..n {
        let b = bit(i + offset);
        out.lsym[i] = b;
        out.orbit[i] = b as f32;
        out.u[i] = (((i + b as usize) % 7) as f32 + 0.5) / 7.0;
    }
    for tr in 0..TRACKS {
        let mut primary = [false; MAX_STEPS];
        let mut m = 0usize;
        for i in 0..n {
            if bit(i + offset + tr * stride) == 1 { primary[i] = true; m += 1; }
        }
        if m == 0 {
            set_hit(&mut out, tr, (tr * n / TRACKS) % n, true, 0.7);
            continue;
        }
        let wanted = lane_wanted(p.pulses, p.density, tr, n).max(1).min(m);
        let mut hits = [false; MAX_STEPS];
        select_spread(&primary, n, wanted, &mut hits);
        for i in 0..n {
            if hits[i] {
                let accent = 0.55 + 0.45 * (((i + tr) % 7) as f32) / 6.0;
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

    fn params(steps: usize, pulses: usize, density: f32, phase: f32, variation: u32) -> GenParams {
        GenParams {
            kind: Mode::RudinShapiro, steps, density, pulses, phase, variation, probability: 1.0,
            ..GenParams::default()
        }
    }

    /// Independent definition: s(0)=1, s(2n)=s(n), s(2n+1)=(-1)^n s(n); bit 1 where s=-1.
    fn recursive(n: usize) -> u8 {
        if n == 0 { 0 } else if n % 2 == 0 { recursive(n / 2) } else { recursive(n / 2) ^ ((n / 2) & 1) as u8 }
    }

    #[test]
    fn the_word_is_a020985() {
        let head: Vec<u8> = (0..16).map(bit).collect();
        assert_eq!(head, [0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 0, 1]);
        for n in 0..4096 { assert_eq!(bit(n), recursive(n), "n = {n}"); }
    }

    #[test]
    fn every_lane_sounds_at_full_density() {
        for steps in [1usize, 7, 16, 32, 64] {
            let out = generate(&params(steps, 8, 1.0, 0.0, 0));
            for tr in 0..TRACKS {
                assert!(out.hit[tr][..steps].iter().any(|&h| h), "steps={steps} lane={tr}");
            }
        }
    }

    #[test]
    fn variation_moves_the_window() {
        let word = |v: u32| (0..32).map(|i| generate(&params(32, 8, 1.0, 0.0, v)).lsym[i]).collect::<Vec<_>>();
        assert_ne!(word(0), word(1));
    }
}
