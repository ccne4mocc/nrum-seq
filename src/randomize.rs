//! Mode-aware vocabularies for Randomize: instead of drawing every control uniformly, values
//! are snapped to small musical sets. Editor thread only; pure and deterministic for a given RNG.
//!
//! Palettes intentionally replace the old uniform draw, so Randomize always lands on a
//! musically legible pattern. Anything that depends on the mode is keyed on the mode Randomize
//! just chose (or kept), so the value and the generator can never disagree.

use crate::modes::Mode;
use crate::sequence::Rng;

/// Loop lengths in grid cells. Values above the selected ceiling are filtered out.
pub(crate) const LENGTHS: &[usize] = &[8, 12, 16, 20, 24, 32, 48, 64, 96, 128, 192, 256];
pub(crate) const DENSITIES: &[f32] = &[0.35, 0.5, 0.65, 0.8, 1.0];
pub(crate) const PROBABILITIES: &[f32] = &[0.5, 0.7, 0.85, 0.95, 1.0];
pub(crate) const VELOCITIES: &[f32] = &[0.6, 0.7, 0.8, 0.9, 1.0];

/// One uniform draw from a palette.
pub(crate) fn pick<T: Copy>(palette: &[T], rng: &mut Rng) -> T {
    debug_assert!(!palette.is_empty());
    let index = rng.below(palette.len()).min(palette.len().saturating_sub(1));
    palette[index]
}

/// The mode-specific second control: pulses, generations, warm-up steps, motif length, …
///
/// Values are kept small and odd where the algorithm benefits from co-prime gaps. The caller
/// still clamps the draw to `pulse_limit`, so a vocabulary entry larger than the grid is
/// filtered out rather than clamped into a duplicate.
fn pulse_vocabulary(mode: Mode) -> &'static [usize] {
    match mode {
        // Closed-form rhythms read best on a small odd count.
        Mode::Euclidean | Mode::EuclideanSwing | Mode::WellFormed | Mode::ToussaintDistance => {
            &[3, 5, 7]
        }
        Mode::Eulerian => &[2, 3, 4],
        Mode::Phyllotaxis => &[4, 8, 12, 16],
        Mode::Cellular => &[4, 8, 12, 15],
        // CA generations and Life warm-up steps.
        Mode::Life => &[2, 4, 8, 12],
        Mode::Oscillator => &[3, 5, 7, 8],
        Mode::Crystallographic => &[3, 4, 6, 8],
        Mode::Logistic | Mode::ReactionDiffusion | Mode::Lorenz | Mode::Brownian
        | Mode::LevyFlight | Mode::TentMap | Mode::HenonMap | Mode::Ulam => &[4, 8, 12, 16],
        Mode::Markov => &[4, 8, 16],
        Mode::RaisedCosine | Mode::PhaseCanon | Mode::Kuramoto => &[4, 8, 12],
        Mode::Slowcat | Mode::PhaseShift => &[3, 5, 7],
        Mode::LSystem | Mode::ThueMorse | Mode::RudinShapiro | Mode::Bernoulli => &[4, 8, 12, 16],
        Mode::Polyrhythm => &[3, 4, 5, 6, 7, 8],
        Mode::Syncopation | Mode::AfricanBell | Mode::Genetic | Mode::Vuza => &[4, 6, 8],
        // Structural limits are enforced by `pulse_limit`; the vocabulary stays inside them.
        Mode::Sieve => &[1, 2],
        Mode::Hilbert => &[2, 3, 4, 5],
        Mode::Fibonacci | Mode::Sturmian => &[5, 8, 13],
        Mode::PebbleBox => &[3, 4, 5, 6],
        Mode::Arnold => &[3, 5, 7],
        Mode::Cantor => &[3, 5, 7],
    }
}

/// Longest loop the ceiling allows, chosen from the length palette.
pub(crate) fn length(ceiling: usize, rng: &mut Rng) -> usize {
    let ceiling = ceiling.max(1);
    let usable = LENGTHS.iter().filter(|&&l| l <= ceiling).count();
    if usable == 0 {
        return ceiling;
    }
    let mut index = rng.below(usable);
    for &l in LENGTHS.iter().filter(|&&l| l <= ceiling) {
        if index == 0 {
            return l;
        }
        index -= 1;
    }
    ceiling
}

/// Pulse count inside the mode's own vocabulary, clamped to the mode's limit and the grid.
pub(crate) fn pulses(mode: Mode, cells: usize, limit: usize, rng: &mut Rng) -> usize {
    let max = limit.min(cells).max(1);
    let vocab = pulse_vocabulary(mode);
    let usable = vocab.iter().filter(|&&p| (1..=max).contains(&p)).count();
    if usable == 0 {
        return max;
    }
    let mut index = rng.below(usable);
    for &p in vocab.iter().filter(|&&p| (1..=max).contains(&p)) {
        if index == 0 {
            return p;
        }
        index -= 1;
    }
    max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pulse_vocabulary_fits_the_smallest_grid() {
        for mode in Mode::all() {
            let vocab = pulse_vocabulary(mode);
            assert!(!vocab.is_empty(), "{mode:?} has no pulse vocabulary");
            for &p in vocab {
                assert!((1..=64).contains(&p), "{mode:?} draws {p}, outside the smallest ceiling");
            }
        }
    }

    #[test]
    fn length_never_exceeds_the_ceiling() {
        for ceiling in [8usize, 16, 32, 64, 128, 256, 1024] {
            let mut rng = Rng::new(0.3, 1, 0x1234);
            for _ in 0..200 {
                let picked = length(ceiling, &mut rng);
                assert!(picked <= ceiling && picked >= 1, "picked {picked} from ceiling {ceiling}");
            }
        }
    }

    #[test]
    fn draws_are_deterministic_and_inside_the_limit() {
        for mode in Mode::all() {
            let mut a = Rng::new(0.42, 7, 0xABCD);
            let mut b = Rng::new(0.42, 7, 0xABCD);
            for _ in 0..32 {
                let pa = pulses(mode, 16, 15, &mut a);
                let pb = pulses(mode, 16, 15, &mut b);
                assert_eq!(pa, pb, "{mode:?} is not deterministic");
                assert!((1..=15).contains(&pa), "{mode:?} drew {pa} outside 1..=15");
            }
        }
    }

    #[test]
    fn a_zero_limit_still_produces_a_usable_count() {
        let mut rng = Rng::new(0.1, 0, 1);
        assert_eq!(pulses(Mode::Sieve, 16, 0, &mut rng), 1);
    }
}
