//! The 39 generation modes, in one list.
//!
//! `Mode` is the host-visible enum: the stable `id` values are what a saved project stores and
//! the `name` values are what the host shows. The generators and the panels match on the same
//! variants, so there is one list and one order — no second enum and no index mapping to keep in
//! sync. `src/sequence/` and `src/view.rs` use the plain data type; only this file names the
//! plug-in enum machinery.

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
}

/// The mode list as it is drawn: one grouped order shared by the dropdown and the mode wheel.
/// Keep host indices alone — this only changes presentation. Adding a mode means appending it to
/// the enum and to exactly one group here.
///
/// Labels and order are shared with nelody seq's `ModeGroup`. Keep them identical in both projects:
/// Rhythm, Stochastic, Chaos / Physics, Fractal, Cellular, Geometric, Number Theory,
/// Music Theory, Acid. nrum seq has no acid modes, so that group is omitted rather than rendered
/// empty. Within a group the host order is preserved.
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
    ("Cellular", &[Mode::Cellular]),
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
    /// Every mode in the order the list is drawn. The header wheel must step this order, not the
    /// raw host index, or one notch from a mode lands on a different group far away in the
    /// visible list.
    pub fn display_order() -> Vec<Mode> {
        GROUPS.iter().flat_map(|(_, modes)| modes.iter().copied()).collect()
    }

    /// Next mode `delta` notches along [`Mode::display_order`], clamped at both ends.
    pub fn step_in_display_order(self, delta: i32) -> Mode {
        let order = Mode::display_order();
        let pos = order.iter().position(|m| *m == self).unwrap_or(0) as i32;
        order[(pos + delta).clamp(0, order.len() as i32 - 1) as usize]
    }

    /// Every mode, for tests that sweep all of them. Production dispatch matches on the variant,
    /// so this stays compiled out of the audio path.
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
