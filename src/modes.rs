//! The 40 generation modes in one list.
//!
//! `Mode` is the host-visible enum: `id` values are stable project state and `name` values are
//! shown by the host. Generators and panels match the same variants, so there is no second enum
//! or index mapping to maintain. Only this file names the plug-in enum machinery.

use crate::sequence::{MAX_STEPS, MIN_MAX_STEPS};
use nice_plug::prelude::Enum;

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Mode {
    #[id = "euclidean"] #[name = "Euclidean"] Euclidean,
    #[id = "eulerian"] #[name = "Eulerian Walk"] Eulerian,
    #[id = "phyllotaxis"] #[name = "Golden Ratio / Phyllotaxis"] Phyllotaxis,
    #[id = "cellular"] #[name = "Cellular Automaton"] Cellular,
    #[id = "oscillator"] #[name = "Oscillator / Phase"] Oscillator,
    #[id = "crystallographic"] #[name = "Crystallographic"] Crystallographic,
    #[id = "logistic"] #[name = "Logistic Map"] Logistic,
    #[id = "markov"] #[name = "Markov Walk"] Markov,
    #[id = "raised_cosine"] #[name = "Raised-Cos Threshold"] RaisedCosine,
    #[id = "slowcat"] #[name = "Slowcat Rhythm"] Slowcat,
    #[id = "toussaint_distance"] #[name = "Toussaint Distance"] ToussaintDistance,
    #[id = "well_formed"] #[name = "Well-Formed Rhythm"] WellFormed,
    #[id = "l_system"] #[name = "L-System Grammar"] LSystem,
    #[id = "phase_shift"] #[name = "Phase Shift / Reich"] PhaseShift,
    #[id = "polyrhythm"] #[name = "Polyrhythm Stack"] Polyrhythm,
    #[id = "syncopation"] #[name = "Weighted Syncopation"] Syncopation,
    #[id = "pebble_box"] #[name = "Pebble Box"] PebbleBox,
    #[id = "sieve"] #[name = "Xenakis Sieve"] Sieve,
    #[id = "kuramoto"] #[name = "Kuramoto Lanes"] Kuramoto,
    #[id = "euclidean_swing"] #[name = "Euclidean-Swing Hybrid"] EuclideanSwing,
    #[id = "bernoulli"] #[name = "Probabilistic (Bernoulli)"] Bernoulli,
    #[id = "african_bell"] #[name = "African Bell Patterns"] AfricanBell,
    #[id = "phase_canon"] #[name = "Polymetric Phase Canon"] PhaseCanon,
    #[id = "reaction_diffusion"] #[name = "Reaction-Diffusion"] ReactionDiffusion,
    #[id = "lorenz"] #[name = "Lorenz Attractor"] Lorenz,
    #[id = "cantor"] #[name = "Cantor Set Rhythm"] Cantor,
    #[id = "fibonacci"] #[name = "Fibonacci/Sturmian"] Fibonacci,
    #[id = "genetic"] #[name = "Genetic Mutation"] Genetic,
    #[id = "arnold"] #[name = "Arnold Circle Map"] Arnold,
    #[id = "sturmian"] #[name = "Sturmian Word"] Sturmian,
    #[id = "hilbert"] #[name = "Hilbert Curve"] Hilbert,
    #[id = "ulam"] #[name = "Ulam Spiral Primes"] Ulam,
    #[id = "vuza"] #[name = "Vuza Canon"] Vuza,
    #[id = "thue_morse"] #[name = "Thue-Morse"] ThueMorse,
    #[id = "rudin_shapiro"] #[name = "Rudin-Shapiro"] RudinShapiro,
    #[id = "brownian"] #[name = "Brownian Walk"] Brownian,
    #[id = "levy_flight"] #[name = "Levy Flight"] LevyFlight,
    #[id = "tent_map"] #[name = "Tent Map"] TentMap,
    #[id = "henon_map"] #[name = "Henon Map"] HenonMap,
    #[id = "life"] #[name = "Life-like CA"] Life,
}

/// The mode list as drawn: one grouped order shared by the dropdown and the wheel. Host indices
/// stay untouched. Adding a mode means appending it to the enum and to one group here.
///
/// Labels and order are shared with nelody seq's `ModeGroup`: Rhythm, Stochastic, Chaos / Physics,
/// Fractal, Cellular, Geometric, Number Theory, Music Theory, Acid. nrum seq omits the empty Acid
/// group. Within a group the host order is preserved.
pub const GROUPS: &[(&str, &[Mode])] = &[
    ("Rhythm", &[
        Mode::Euclidean, Mode::Oscillator, Mode::RaisedCosine, Mode::Slowcat,
        Mode::ToussaintDistance, Mode::WellFormed, Mode::LSystem, Mode::PhaseShift,
        Mode::Polyrhythm, Mode::Syncopation, Mode::EuclideanSwing,
    ]),
    ("Stochastic", &[
        Mode::Markov, Mode::Bernoulli, Mode::Genetic, Mode::Brownian, Mode::LevyFlight,
    ]),
    ("Chaos / Physics", &[
        Mode::Logistic, Mode::PebbleBox, Mode::Kuramoto, Mode::ReactionDiffusion, Mode::Lorenz,
        Mode::Arnold, Mode::TentMap, Mode::HenonMap,
    ]),
    ("Fractal", &[Mode::Cantor]),
    ("Cellular", &[Mode::Cellular, Mode::Life]),
    ("Geometric", &[
        Mode::Eulerian, Mode::Phyllotaxis, Mode::Crystallographic, Mode::Hilbert, Mode::Ulam,
    ]),
    ("Number Theory", &[
        Mode::Sieve, Mode::Fibonacci, Mode::Sturmian, Mode::Vuza, Mode::ThueMorse,
        Mode::RudinShapiro,
    ]),
    ("Music Theory", &[Mode::AfricanBell, Mode::PhaseCanon]),
];

impl Mode {
    /// Every mode in display order; the header wheel must step this, not the raw host index.
    pub fn display_order() -> Vec<Mode> {
        GROUPS.iter().flat_map(|(_, modes)| modes.iter().copied()).collect()
    }

    /// Next mode `delta` notches along [`Mode::display_order`], clamped at both ends.
    pub fn step_in_display_order(self, delta: i32) -> Mode {
        let order = Mode::display_order();
        let pos = order.iter().position(|m| *m == self).unwrap_or(0) as i32;
        order[(pos + delta).clamp(0, order.len() as i32 - 1) as usize]
    }

    /// Shortest loop that still shows the mode's defining structure, in steps. `None` means the
    /// mode is complete at any length and selecting it must not touch the loop.
    ///
    /// This is a functionality floor for the panel and the algorithm, not a taste preset:
    /// attractors need a dense path, word/staircase modes need more than one period, and the
    /// fixed-figure modes need a whole bar of their own grid. It never caps a longer loop.
    pub fn min_steps(self) -> Option<usize> {
        match self {
            // A 12-step bell figure twice, so the loop seam falls on a figure boundary.
            Self::AfricanBell => Some(24),
            // One full residue cycle, a readable orbit/path, or a motif rotation.
            Self::Phyllotaxis | Self::Logistic | Self::Slowcat | Self::PhaseShift
            | Self::PebbleBox | Self::Sieve | Self::Bernoulli | Self::PhaseCanon | Self::Vuza => Some(32),
            // A long-horizon texture: attractor, walk, word window or population wave.
            Self::LSystem | Self::Kuramoto | Self::ReactionDiffusion | Self::Lorenz
            | Self::Fibonacci | Self::Sturmian | Self::Ulam | Self::ThueMorse
            | Self::RudinShapiro | Self::Brownian | Self::LevyFlight | Self::TentMap
            | Self::HenonMap | Self::Life => Some(64),
            // Two levels of middle-third removal (3^4) at the widest lane depth.
            Self::Cantor => Some(81),
            _ => None,
        }
    }

    /// Effective `(steps, max_steps)` after a mode change: raises the floor without ever lowering
    /// a longer loop or a higher ceiling. Both values stay inside the host bounds.
    pub fn steps_for_mode(self, steps: f32, max_steps: usize) -> (f32, usize) {
        let Some(floor) = self.min_steps() else {
            return (steps, max_steps);
        };
        let floor = floor.clamp(1, MAX_STEPS);
        let ceiling = max_steps.clamp(MIN_MAX_STEPS, MAX_STEPS).max(floor).min(MAX_STEPS);
        let steps = if steps.is_finite() { steps } else { 1.0 };
        (steps.max(floor as f32).min(ceiling as f32), ceiling)
    }

    /// Every mode, for test sweeps; production dispatch matches on the variant.
    #[cfg(test)]
    pub fn all() -> impl Iterator<Item = Mode> {
        (0..<Mode as Enum>::variants().len()).map(Mode::from_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_labels_follow_the_shared_taxonomy() {
        let labels: Vec<&str> = GROUPS.iter().map(|(label, _)| *label).collect();
        assert_eq!(
            labels,
            [
                "Rhythm",
                "Stochastic",
                "Chaos / Physics",
                "Fractal",
                "Cellular",
                "Geometric",
                "Number Theory",
                "Music Theory",
            ],
            "keep this order and these labels identical to nelody seq's ModeGroup::ALL; \
             nrum seq omits only the empty Acid group"
        );
    }

    #[test]
    fn display_order_covers_every_mode_exactly_once() {
        let order = Mode::display_order();
        assert_eq!(order.len(), <Mode as Enum>::variants().len());
        for kind in Mode::all() {
            assert_eq!(order.iter().filter(|&&m| m == kind).count(), 1, "{kind:?} missing or duplicated");
        }
    }

    #[test]
    fn only_modes_that_need_a_longer_loop_have_a_step_floor() {
        // Selecting these must not change the loop length.
        for mode in [
            Mode::Euclidean, Mode::Eulerian, Mode::Cellular, Mode::Oscillator,
            Mode::Crystallographic, Mode::Markov, Mode::RaisedCosine, Mode::ToussaintDistance,
            Mode::WellFormed, Mode::Polyrhythm, Mode::Syncopation, Mode::EuclideanSwing,
            Mode::Genetic, Mode::Arnold, Mode::Hilbert,
        ] {
            assert_eq!(mode.min_steps(), None, "{mode:?} must not force a longer loop");
        }
        for (mode, floor) in [
            (Mode::AfricanBell, 24),
            (Mode::Phyllotaxis, 32), (Mode::Logistic, 32), (Mode::Slowcat, 32),
            (Mode::PhaseShift, 32), (Mode::PebbleBox, 32), (Mode::Sieve, 32),
            (Mode::Bernoulli, 32), (Mode::PhaseCanon, 32), (Mode::Vuza, 32),
            (Mode::LSystem, 64), (Mode::Kuramoto, 64), (Mode::ReactionDiffusion, 64),
            (Mode::Lorenz, 64), (Mode::Fibonacci, 64), (Mode::Sturmian, 64),
            (Mode::Ulam, 64), (Mode::ThueMorse, 64), (Mode::RudinShapiro, 64),
            (Mode::Brownian, 64), (Mode::LevyFlight, 64), (Mode::TentMap, 64),
            (Mode::HenonMap, 64), (Mode::Life, 64),
            (Mode::Cantor, 81),
        ] {
            assert_eq!(mode.min_steps(), Some(floor), "{mode:?}");
            assert!(floor <= crate::sequence::DEFAULT_MAX_STEPS, "{mode:?} floor spends the whole ceiling");
        }
        // Every mode is in exactly one of the two lists above.
        assert_eq!(<Mode as Enum>::variants().len(), 40);
    }

    #[test]
    fn floors_raise_without_ever_lowering() {
        // A floor mode lifts a short loop and a low ceiling.
        assert_eq!(Mode::Lorenz.steps_for_mode(16.0, 256), (64.0, 256));
        assert_eq!(Mode::Lorenz.steps_for_mode(16.0, 64), (64.0, 64));
        // The longest floor raises a 64-step ceiling just enough.
        assert_eq!(Mode::Cantor.steps_for_mode(16.0, 64), (81.0, 81));
        // Longer loops and higher ceilings are untouched.
        assert_eq!(Mode::Kuramoto.steps_for_mode(128.0, 1024), (128.0, 1024));
        // Modes without a floor are identity, including fractional loops.
        assert_eq!(Mode::Euclidean.steps_for_mode(16.5, 256), (16.5, 256));
        assert_eq!(Mode::Euclidean.steps_for_mode(8.0, 64), (8.0, 64));
        // Broken values cannot escape the host bounds.
        let (steps, ceiling) = Mode::Cantor.steps_for_mode(f32::NAN, 4096);
        assert_eq!(ceiling, MAX_STEPS);
        assert_eq!(steps, 81.0);
    }

    #[test]
    fn the_wheel_walks_the_displayed_order_and_clamps() {
        let order = Mode::display_order();
        let mut at = order[0];
        let mut walked = vec![at];
        for _ in 1..order.len() {
            let next = at.step_in_display_order(1);
            assert_ne!(next, at);
            assert_eq!(next.step_in_display_order(-1), at);
            walked.push(next);
            at = next;
        }
        assert_eq!(walked, order);
        assert_eq!(at.step_in_display_order(1), at, "clamps at the end");
        assert_eq!(order[0].step_in_display_order(-1), order[0], "clamps at the start");
        // Every group boundary is crossed exactly once while walking the list.
        let changes = order.windows(2).filter(|w| {
            !GROUPS.iter().any(|(_, g)| g.contains(&w[0]) && g.contains(&w[1]))
        }).count();
        assert_eq!(changes, GROUPS.len() - 1);
    }
}
