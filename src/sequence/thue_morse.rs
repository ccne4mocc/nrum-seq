use super::{lane_wanted, select_spread, set_hit, GenParams, Pattern, MAX_STEPS, TRACKS};

/// Thue-Morse bit: parity of the number of 1s in the binary expansion (OEIS A010060).
pub(crate) fn bit(n: usize) -> u8 {
    (n.count_ones() & 1) as u8
}

/// Norgard's infinity series: `a(0)=0`, `a(2n)=-a(n)`, `a(2n+1)=a(n)+1`
/// (0, 1, -1, 2, 1, 0, -2, 3, ...). It is odd exactly where `t(n)=1`, so it gives the
/// self-similar word a usable velocity contour instead of a second binary value.
pub(crate) fn infinity(mut n: usize) -> i32 {
    let (mut sum, mut mult) = (0i32, 1i32);
    while n > 0 {
        if n & 1 == 1 { sum += mult; } else { mult = -mult; }
        n >>= 1;
    }
    sum
}

fn contour(n: usize) -> f32 {
    ((infinity(n) + 5) as f32 / 10.0).clamp(0.0, 1.0)
}

/// Thue-Morse word: onsets where the bit is 1, thinned evenly to the lane's target count.
/// `phase` and `variation` slide the window along the infinite word; each lane reads a
/// different window, so the kit hears interlocking copies of the same self-similar sequence.
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
        out.u[i] = contour(i + offset);
    }
    for tr in 0..TRACKS {
        let mut primary = [false; MAX_STEPS];
        let mut m = 0usize;
        for i in 0..n {
            if bit(i + offset + tr * stride) == 1 { primary[i] = true; m += 1; }
        }
        if m == 0 {
            // A short window can contain only zeros; keep the lane audible on its own phase.
            set_hit(&mut out, tr, (tr * n / TRACKS) % n, true, 0.7);
            continue;
        }
        let wanted = lane_wanted(p.pulses, p.density, tr, n).max(1).min(m);
        let mut hits = [false; MAX_STEPS];
        select_spread(&primary, n, wanted, &mut hits);
        for i in 0..n {
            if hits[i] {
                let accent = 0.55 + 0.45 * contour(i + offset + tr * stride);
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
            kind: Mode::ThueMorse, steps, density, pulses, phase, variation, probability: 1.0,
            ..GenParams::default()
        }
    }

    #[test]
    fn the_word_is_a010060() {
        let want = "0110100110010110100101100110100110010110011010010110100110010110";
        let got: String = (0..64).map(|n| (b'0' + bit(n)) as char).collect();
        assert_eq!(got, want);
    }

    #[test]
    fn infinity_series_is_parity_linked_and_matches_its_definition() {
        fn recursive(n: usize) -> i32 {
            if n == 0 { 0 } else if n % 2 == 0 { -recursive(n / 2) } else { recursive(n / 2) + 1 }
        }
        let head: Vec<i32> = (0..16).map(infinity).collect();
        assert_eq!(head, [0, 1, -1, 2, 1, 0, -2, 3, -1, 2, 0, 1, 2, -1, -3, 4]);
        for n in 0..2048 {
            assert_eq!(infinity(n), recursive(n), "n = {n}");
            assert_eq!(infinity(n).rem_euclid(2) as u8, bit(n), "n = {n}");
        }
    }

    #[test]
    fn every_lane_sounds_and_hits_stay_on_the_word() {
        for steps in [1usize, 7, 16, 32, 64] {
            let out = generate(&params(steps, 8, 1.0, 0.0, 0));
            for tr in 0..TRACKS {
                assert!(out.hit[tr][..steps].iter().any(|&h| h), "steps={steps} lane={tr}");
            }
        }
        let out = generate(&params(64, 16, 1.0, 0.0, 0));
        for tr in 0..TRACKS {
            let stride = (64 / TRACKS).max(1);
            for i in 0..64 {
                if out.hit[tr][i] && bit(i + tr * stride) == 0 {
                    // Only the no-zero-window fallback may leave the 1s.
                    let zeros = (0..64).filter(|&j| bit(j + tr * stride) == 0).count();
                    assert_eq!(zeros, 64, "lane {tr} step {i} is not a Thue-Morse 1");
                }
            }
        }
    }

    #[test]
    fn phase_and_variation_move_the_window() {
        let key = |phase: f32, v: u32| {
            let out = generate(&params(32, 8, 1.0, phase, v));
            (0..32).map(|i| out.lsym[i]).collect::<Vec<_>>()
        };
        assert_ne!(key(0.0, 0), key(0.0, 1));
        assert_ne!(key(0.0, 0), key(0.5, 0));
    }
}
