//! The 28 generation modes, in one list.
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
}

impl Mode {
    /// Every mode, for tests that sweep all of them. Production dispatch matches on the variant,
    /// so this stays compiled out of the audio path.
    #[cfg(test)]
    pub fn all() -> impl Iterator<Item = Mode> {
        (0..<Mode as Enum>::variants().len()).map(Mode::from_index)
    }
}
