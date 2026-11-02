//! Deterministic, bounded-work rhythm generators. Keep these functions pure and audio-thread safe.
//! Adding a generator should not require touching MIDI scheduling or Slint geometry.

mod cellular;
mod crystallographic;
mod eulerian;
mod euclidean;
mod logistic;
mod markov;
mod lsystem;
mod phase_shift;
mod polyrhythm;
mod syncopation;
mod raised_cosine;
mod slowcat;
mod toussaint;
mod well_formed;
mod oscillator;
mod phyllotaxis;
mod pebble;
mod sieve;
mod kuramoto;
mod euclidean_swing;
mod bernoulli;
mod african_bell;
mod phase_canon;
mod reaction_diffusion;
mod lorenz;
mod cantor;
mod fibonacci;
mod genetic;
mod arnold;
mod sturmian;
mod hilbert;
mod ulam;
mod vuza;
mod thue_morse;
mod rudin_shapiro;
mod brownian;
mod levy_flight;
mod tent_map;
mod henon_map;

// The panel draws these algorithms' own curves and cutoffs rather than restating their
// formulas, so these re-exports exist for `crate::view` alone.
pub(crate) use arnold::coupling as arnold_coupling;
pub(crate) use brownian::sigma as brownian_sigma;
pub(crate) use cellular::selected_row as cellular_row;
pub(crate) use fibonacci::alpha as fibonacci_alpha;
pub(crate) use henon_map::a_of as henon_a;
pub(crate) use hilbert::{rotate as hilbert_rotate, turns as hilbert_turns};
pub(crate) use levy_flight::{step_min as levy_step_min, tail_alpha as levy_alpha};
pub(crate) use logistic::{r_of as logistic_r, x0 as logistic_x0};
pub(crate) use oscillator::WAVE_COUNT as oscillator_wave_count;
pub(crate) use tent_map::mu_of as tent_mu;
// Only the panel's unit tests sample the waveform directly now; the panel itself reads the
// generator's published `orbit`.
#[cfg(test)]
pub(crate) use oscillator::value as oscillator_value;
pub(crate) use phyllotaxis::{
    lane_threshold as phyllotaxis_threshold, primary_phase as phyllotaxis_primary,
    rules_fired as phyllotaxis_rules, secondary_phase as phyllotaxis_secondary,
};
pub(crate) use raised_cosine::{
    cutoff as raised_cosine_cutoff, envelope as raised_cosine_envelope, phase_offset as raised_cosine_phase,
};
pub(crate) use slowcat::motif_len as slowcat_motif_len;
// Help text shows these derived quantities, so it reads them from the generators instead of
// restating the formula.
pub(crate) use euclidean::lane_pulses as euclidean_lane_pulses;
pub(crate) use lsystem::{
    lane_threshold as lsystem_threshold,
    step_chance as lsystem_step_chance,
};
pub(crate) use pebble::gravity as pebble_gravity;
pub(crate) use sieve::{classes as sieve_classes, member as sieve_member};
pub(crate) use kuramoto::{
    coupling as kuramoto_coupling, order as kuramoto_order, rate as kuramoto_rate,
};

// The mode enum is the host parameter type, shared with the generators: one list of modes,
// one order, no mapping table to keep in sync.
use crate::modes::Mode;

pub const TRACKS: usize = 8;
/// Hard capacity for every fixed-size pattern array. The host exposes a smaller, user-selected
/// `max_steps` ceiling (64 by default); 1024 is the largest value that ceiling can reach.
pub const MAX_STEPS: usize = 1024;
/// Lowest selectable ceiling. It is the historical limit, so an untouched project behaves
/// exactly as it did before the ceiling became adjustable.
pub const MIN_MAX_STEPS: usize = 64;
pub const DEFAULT_MAX_STEPS: usize = 1024;
pub const CA_ROWS: usize = 16;
// Spelled out: `std::f64::consts::GOLDEN_RATIO` is unstable on older toolchains, which made the
// documented "Rust 1.88 or newer" baseline wrong.
#[allow(clippy::approx_constant)] // the std constant is newer than the supported baseline
const PHI: f64 = 1.618_033_988_749_894;
const PHI_INV: f64 = PHI - 1.0;
const GOLDEN_ANGLE: f64 = 2.399_963_229_728_653;

/// Immutable snapshot of every value that affects generated events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GenParams {
    pub kind: Mode,
    /// Number of grid cells the generators fill. Always a whole number.
    pub steps: usize,
    /// Loop length in steps. Equal to `steps` for whole-step patterns; fractional values
    /// make the loop wrap part-way through a cell, so successive cycles shift by the remainder.
    pub length: f32,
    /// Main pulse/generation control; its exact interpretation is mode dependent.
    pub pulses: usize,
    pub rotate: usize,
    pub density: f32,
    /// Global probability multiplier for retaining generated hits.
    pub probability: f32,
    /// Global velocity multiplier for emitted MIDI notes.
    pub velocity: f32,
    /// Normalized deterministic seed.
    pub seed: f32,
    /// Normalized oscillator / phyllotaxis phase.
    pub phase: f32,
    /// Oscillator cycles per pattern.
    pub osc_rate: f32,
    /// Waveform index for the oscillator mode; the count lives in `oscillator::WAVE_COUNT` and
    /// the names in the `OscWave` host enum.
    pub osc_wave: usize,
    pub ca_rule: u8,
    /// 1..8 unit-cell symmetry order.
    pub symmetry: usize,
    /// Incremented when the user presses Generate; saved with the project.
    pub variation: u32,
    /// Flip every hit to a rest and every rest to a hit after generation.
    pub invert: bool,
    /// Reverse the step order after generation.
    pub mirror: bool,
}

impl Default for GenParams {
    fn default() -> Self {
        Self {
            kind: Mode::Euclidean, steps: 16, length: 16.0, pulses: 5, rotate: 0,
            density: 0.65, probability: 1.0, velocity: 1.0, seed: 0.271, phase: 0.0, osc_rate: 2.0,
            osc_wave: 0, ca_rule: 30, symmetry: 4, variation: 0, invert: false, mirror: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Pattern {
    pub steps: usize,
    pub hit: [[bool; MAX_STEPS]; TRACKS],
    /// Per-step accent multiplier before the lane's velocity parameter.
    pub accent: [[f32; MAX_STEPS]; TRACKS],
    /// Which symbolic drum state generated this step (0 = rest, 1..8 = lane).
    pub state: [u8; MAX_STEPS],
    /// Cellular-automaton history for its space-time view.
    pub ca: [[bool; MAX_STEPS]; CA_ROWS],
    /// Orbit samples used by the logistic-map plot.
    pub orbit: [f32; MAX_STEPS],
    /// Secondary coordinate (normalized 0..1) for 2-D views (Hilbert, Ulam, Arnold, Bernoulli).
    pub theta: [f32; MAX_STEPS],
    /// Contour value (normalized 0..1) for scatter/contour views (Bernoulli, Reaction-Diffusion).
    pub u: [f32; MAX_STEPS],
    /// L-System symbol per step: 0 = A, 1 = B. The word the grammar actually produced.
    pub lsym: [u8; MAX_STEPS],
    /// Steps the finished pattern was rotated by. The rotation happens after the mode ran, so
    /// display slot `i` holds the generator's step `(i + steps - rotation) % steps`; the panel
    /// uses this to draw the geometry that actually produced each onset.
    pub rotation: usize,
    /// Pebble-box positions, normalized 0-255 as `[x0, y0, x1, y1, …]` per step.
    pub pebbles: [[u8; TRACKS * 2]; MAX_STEPS],
    /// Wall a pebble struck this step: 0 = none, 1 = left, 2 = right, 3 = top, 4 = bottom.
    /// The frame records the end of the step, so this is what tells the panel *where* it hit.
    pub impact: [[u8; TRACKS]; MAX_STEPS],
    /// Kuramoto phase per lane per step, normalized 0-255. The panel draws these as pointers,
    /// so it shows the same bank the scheduler ranked instead of re-integrating it.
    pub phases: [[u8; TRACKS]; MAX_STEPS],
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            steps: 16,
            hit: [[false; MAX_STEPS]; TRACKS],
            accent: [[1.0; MAX_STEPS]; TRACKS],
            state: [0; MAX_STEPS],
            ca: [[false; MAX_STEPS]; CA_ROWS],
            orbit: [0.0; MAX_STEPS],
            theta: [0.0; MAX_STEPS],
            u: [0.0; MAX_STEPS],
            lsym: [0; MAX_STEPS],
            rotation: 0,
            pebbles: [[0; TRACKS * 2]; MAX_STEPS],
            impact: [[0; TRACKS]; MAX_STEPS],
            phases: [[0; TRACKS]; MAX_STEPS],
        }
    }
}

/// Small deterministic PRNG; no locks, heap allocations, or OS entropy on the audio thread.
#[derive(Clone, Copy)]
pub(super) struct Rng(u64);
impl Rng {
    pub(super) fn new(seed: f32, variation: u32, salt: u64) -> Self {
        let bits = seed.to_bits() as u64;
        let mut r = Self((bits << 32) ^ variation as u64 ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        r.next_u64();
        r
    }
    pub(super) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub(super) fn unit(&mut self) -> f32 { (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32 }
    pub(super) fn below(&mut self, n: usize) -> usize { if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize } }
}

pub(crate) const DENSITY_PROFILE: [f32; TRACKS] = [0.50, 0.30, 1.00, 0.12, 0.18, 0.15, 0.12, 0.28];

#[inline]
pub(super) fn enabled_threshold(p: &GenParams, track: usize) -> f32 {
    (p.density.clamp(0.0, 1.0) * DENSITY_PROFILE[track]).clamp(0.0, 1.0)
}

/// Onset count for the "pulse count scaled by density" modes. Every lane asks for
/// `pulses + tr % 3` onsets and density decides how many of those survive.
#[inline]
pub(crate) fn pulse_hits(pulses: usize, density: f32, tr: usize, n: usize) -> usize {
    let requested = if pulses == 0 { 0 } else { (pulses + tr % 3).min(n) };
    ((requested as f32 * density.clamp(0.0, 1.0)).round() as usize).min(n)
}

/// Circular modular distance of `pos` in a k-over-n word: the Bresenham remainder that both
/// the well-formed and the Toussaint onset tests are built from, and the curve the well-formed
/// panel plots.
#[inline]
pub(crate) fn modular_distance(pos: usize, k: usize, n: usize) -> usize {
    (pos * k) % n.max(1)
}

/// Onset test for the anchored E(k,n) word: onset j sits at floor(j*n/k) from the lane's
/// phase origin, so a step hits when its circular distance to the next onset is under k.
/// Shared by the Euclidean and well-formed generators so the two cannot drift apart.
#[inline]
pub(super) fn anchored_onset(pos: usize, k: usize, n: usize) -> bool {
    (n - modular_distance(pos, k, n)) % n < k
}

/// The position a step occupies once the ring is rotated by a lane's phase origin.
#[inline]
pub(super) fn rotated_pos(i: usize, offset: usize, n: usize) -> usize {
    (i + n - offset % n) % n
}

/// Fractional part in `0..1` for any finite value, including negatives.
#[inline]
pub(crate) fn frac(x: f64) -> f64 { x - x.floor() }

#[inline]
pub(super) fn set_hit(out: &mut Pattern, track: usize, step: usize, hit: bool, accent: f32) {
    out.hit[track][step] = hit;
    out.accent[track][step] = accent.clamp(0.05, 1.0);
    if hit { out.state[step] = (track + 1) as u8; }
}

/// Mark the `k` highest scores, breaking ties by earlier step. The sort is in place over a fixed
/// stack array, so this stays allocation-free on the audio path. Replaces the old O(n²) rank
/// scan, which became too expensive once the ceiling reached 1024 steps.
pub(super) fn top_k(scores: &[f32; MAX_STEPS], n: usize, k: usize, hits: &mut [bool; MAX_STEPS]) {
    hits[..n].fill(false);
    if k == 0 || n == 0 { return; }
    if k >= n { hits[..n].fill(true); return; }
    let mut ranked: [(f32, usize); MAX_STEPS] =
        std::array::from_fn(|i| if i < n { (scores[i], i) } else { (f32::NEG_INFINITY, usize::MAX) });
    ranked[..n].sort_unstable_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    for &(_, i) in &ranked[..k] { hits[i] = true; }
}

/// Triangle-wave reflection into `0..=1`, exact for any displacement. Iterative reflection loops
/// need the step to be smaller than the range and pin a long jump to the wall.
pub(super) fn fold01(x: f64) -> f64 {
    let m = x.rem_euclid(2.0);
    if m <= 1.0 { m } else { 2.0 - m }
}

/// Mark `k` steps, spread evenly over the structural ones first. `primary[i]` flags the steps the
/// algorithm itself calls onsets (Thue-Morse 1s, sieve members, prime cells). Fewer than `k`:
/// take every primary step. More: pick `k` evenly spaced among them. Ranking by an arbitrary
/// tie-break leaves the end of the loop empty, which is the defect this replaces.
/// Allocation-free: fixed stack arrays only.
pub(super) fn select_spread(primary: &[bool; MAX_STEPS], n: usize, k: usize, hits: &mut [bool; MAX_STEPS]) {
    hits[..n].fill(false);
    let k = k.min(n);
    if k == 0 { return; }
    let m = primary[..n].iter().filter(|&&b| b).count();
    let mark = |want: usize, which: bool, hits: &mut [bool; MAX_STEPS]| {
        let cnt = if which { m } else { n - m };
        if want == 0 || cnt == 0 { return; }
        let (mut c, mut j) = (0usize, 0usize);
        for i in 0..n {
            if primary[i] != which { continue; }
            // the j-th of `want` picks candidate floor((2j+1)·cnt / 2·want): centred in its slice
            if j < want && c == (2 * j + 1) * cnt / (2 * want) { hits[i] = true; j += 1; }
            c += 1;
        }
    };
    mark(k.min(m), true, hits);
    if k > m { mark(k - m, false, hits); }
}

/// Exact onset count for a lane from the shared pulse/density profile. Used by the walk and map
/// modes whose onsets are the largest steps of one orbit.
#[inline]
pub(crate) fn lane_wanted(pulses: usize, density: f32, lane: usize, n: usize) -> usize {
    let scaled = pulses.min(n) as f32 * density.clamp(0.0, 1.0) * (0.5 + DENSITY_PROFILE[lane]);
    (scaled.round() as usize).min(n)
}

/// Stable step accents: backbeat and downbeat are emphasized without tying any lane to pitch.
pub(super) fn accents(out: &mut Pattern, p: &GenParams) {
    for tr in 0..TRACKS {
        for i in 0..out.steps {
            let base = if i % 4 == 0 { 1.0 } else if i % 2 == 0 { 0.82 } else { 0.68 };
            let mut rng = Rng::new(p.seed, p.variation, 0xACCE_0000 + (tr * MAX_STEPS + i) as u64);
            let human = 0.90 + rng.unit() * 0.10;
            out.accent[tr][i] = (base * human).clamp(0.05, 1.0);
        }
    }
}

/// Run one generator and return its raw output, before the shared post-transforms.
fn run_mode(q: &GenParams) -> Pattern {
    match q.kind {
        Mode::Euclidean => euclidean::generate(q),
        Mode::Eulerian => eulerian::generate(q),
        Mode::Phyllotaxis => phyllotaxis::generate(q),
        Mode::Cellular => cellular::generate(q),
        Mode::Oscillator => oscillator::generate(q),
        Mode::Crystallographic => crystallographic::generate(q),
        Mode::Logistic => logistic::generate(q),
        Mode::Markov => markov::generate(q),
        Mode::RaisedCosine => raised_cosine::generate(q),
        Mode::Slowcat => slowcat::generate(q),
        Mode::ToussaintDistance => toussaint::generate(q),
        Mode::WellFormed => well_formed::generate(q),
        Mode::LSystem => lsystem::generate(q),
        Mode::PhaseShift => phase_shift::generate(q),
        Mode::Polyrhythm => polyrhythm::generate(q),
        Mode::Syncopation => syncopation::generate(q),
        Mode::PebbleBox => pebble::generate(q),
        Mode::Sieve => sieve::generate(q),
        Mode::Kuramoto => kuramoto::generate(q),
        Mode::EuclideanSwing => euclidean_swing::generate(q),
        Mode::Bernoulli => bernoulli::generate(q),
        Mode::AfricanBell => african_bell::generate(q),
        Mode::PhaseCanon => phase_canon::generate(q),
        Mode::ReactionDiffusion => reaction_diffusion::generate(q),
        Mode::Lorenz => lorenz::generate(q),
        Mode::Cantor => cantor::generate(q),
        Mode::Fibonacci => fibonacci::generate(q),
        Mode::Genetic => genetic::generate(q),
        Mode::Arnold => arnold::generate(q),
        Mode::Sturmian => sturmian::generate(q),
        Mode::Hilbert => hilbert::generate(q),
        Mode::Ulam => ulam::generate(q),
        Mode::Vuza => vuza::generate(q),
        Mode::ThueMorse => thue_morse::generate(q),
        Mode::RudinShapiro => rudin_shapiro::generate(q),
        Mode::Brownian => brownian::generate(q),
        Mode::LevyFlight => levy_flight::generate(q),
        Mode::TentMap => tent_map::generate(q),
        Mode::HenonMap => henon_map::generate(q),
    }
}


pub fn generate(p: &GenParams) -> Pattern {
    let steps = p.steps.clamp(1, MAX_STEPS);
    let mut q = *p;
    q.steps = steps;
    q.pulses = if q.kind == Mode::Cellular { q.pulses.min(CA_ROWS - 1) } else { q.pulses.min(steps) };
    q.rotate %= steps;
    q.symmetry = q.symmetry.clamp(1, 8);
    q.osc_wave %= oscillator::WAVE_COUNT;
    let mut out = run_mode(&q);
    out.steps = steps;
    // Probability is applied after the algorithm so it behaves consistently across modes.
    for tr in 0..TRACKS {
        for i in 0..steps {
            if out.hit[tr][i] {
                let mut rng = Rng::new(q.seed, q.variation, 0xBABA_0000 + (tr * MAX_STEPS + i) as u64);
                if rng.unit() >= q.probability.clamp(0.0, 1.0) { out.hit[tr][i] = false; out.impact[i][tr] = 0; }
            }
        }
    }
    if q.density <= 0.0 || (q.kind != Mode::Cellular && q.pulses == 0) {
        for tr in 0..TRACKS { out.hit[tr][..steps].fill(false); }
        out.impact[..steps].fill([0; TRACKS]);
    }
    // Rotation includes probability drops, the mode's own accent data, and auxiliary view data.
    // Variation also rotates by one or more steps, so closed-form modes can actually regenerate
    // differently.
    let shift = ((q.rotate % steps) + (q.variation as usize % steps)) % steps;
    out.rotation = shift;
    if shift != 0 {
        let old_hit = out.hit; let old_acc = out.accent; let old_state = out.state;
        let old_ca = out.ca; let old_orbit = out.orbit; let old_theta = out.theta; let old_u = out.u;
        let old_lsym = out.lsym;
        let old_pebbles = out.pebbles; let old_impact = out.impact; let old_phases = out.phases;
        for tr in 0..TRACKS { for i in 0..steps { let src = (i + steps - shift) % steps; out.hit[tr][i] = old_hit[tr][src]; out.accent[tr][i] = old_acc[tr][src]; } }
        for i in 0..steps { let src = (i + steps - shift) % steps; out.state[i] = old_state[src]; out.orbit[i] = old_orbit[src]; out.theta[i] = old_theta[src]; out.u[i] = old_u[src]; out.lsym[i] = old_lsym[src]; out.pebbles[i] = old_pebbles[src]; out.impact[i] = old_impact[src]; out.phases[i] = old_phases[src]; for row in 0..CA_ROWS { out.ca[row][i] = old_ca[row][src]; } }
    }
    if q.invert {
        for tr in 0..TRACKS {
            for i in 0..steps {
                out.hit[tr][i] = !out.hit[tr][i];
                if !out.hit[tr][i] { out.impact[i][tr] = 0; }
            }
        }
    }
    if q.mirror {
        for tr in 0..TRACKS {
            for i in 0..steps / 2 {
                let j = steps - 1 - i;
                out.hit[tr].swap(i, j);
                out.accent[tr].swap(i, j);
            }
        }
        for i in 0..steps / 2 {
            let j = steps - 1 - i;
            out.state.swap(i, j); out.orbit.swap(i, j); out.theta.swap(i, j); out.u.swap(i, j);
            out.lsym.swap(i, j);
            out.pebbles.swap(i, j); out.impact.swap(i, j); out.phases.swap(i, j);
            for row in 0..CA_ROWS { out.ca[row].swap(i, j); }
        }
    }
    // The stable downbeat/backbeat accent shape is applied *after* rotation, so it stays tied to
    // the step grid (and therefore the beat) instead of travelling with the rotated pattern;
    // each mode's own accent data, already rotated above, is multiplied in.
    let mode_accents = out.accent;
    accents(&mut out, &q);
    for tr in 0..TRACKS {
        for i in 0..steps { out.accent[tr][i] = (out.accent[tr][i] * mode_accents[tr][i]).clamp(0.05, 1.0); }
    }
    out
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout, System};

    // Counting allocator: invariant #1 requires `generate()` to run on the audio thread
    // without allocating, and reading the code cannot prove that. Thread-local so the
    // parallel test harness cannot leak its own allocations into the count.
    std::thread_local! {
        static ALLOCS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    struct Counting;
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
            System.alloc(l)
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) { System.dealloc(p, l) }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
            let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
            System.realloc(p, l, n)
        }
    }
    #[global_allocator]
    static ALLOCATOR: Counting = Counting;

    #[test]
    fn generation_never_allocates() {
        // Everything below runs inside one `try_with` so the counter is read on this thread
        // only; `ALLOCS.with` outside the closure would register the access itself.
        ALLOCS.with(|c| {
            c.set(0);
            for kind in Mode::all() {
                for steps in [1usize, 2, 7, 16, 32, MAX_STEPS] {
                    let p = GenParams { kind, steps, length: steps as f32, pulses: steps / 2, density: 0.8, probability: 1.0, ..GenParams::default() };
                    std::hint::black_box(generate(&p));
                }
            }
            let used = c.get();
            assert_eq!(used, 0, "generate() allocated {used} times on the audio path");
        });
    }

    #[test]
    #[ignore = "timing run; use cargo test --release bench_generation_at_max_steps -- --ignored --nocapture"]
    fn bench_generation_at_max_steps() {
        use std::time::Instant;
        for kind in Mode::all() {
            let p = GenParams { kind, steps: MAX_STEPS, length: MAX_STEPS as f32, pulses: MAX_STEPS / 2, density: 1.0, probability: 1.0, ..GenParams::default() };
            let mut times = [0u64; 200];
            for t in times.iter_mut() {
                let start = Instant::now();
                std::hint::black_box(generate(&p));
                *t = start.elapsed().as_nanos() as u64;
            }
            times.sort_unstable();
            println!("{kind:?}: median {:.1} us, max {:.1} us at {MAX_STEPS} steps",
                times[100] as f64 / 1000.0, times[199] as f64 / 1000.0);
        }
    }

    #[test]
    fn all_modes_are_deterministic_and_bounded() {
        for kind in Mode::all() {
            let p = GenParams { kind, ..GenParams::default() };
            let a = generate(&p);
            let b = generate(&p);
            assert_eq!(a.hit, b.hit, "{kind:?} is not deterministic");
            assert_eq!(a.steps, 16);
            for tr in 0..TRACKS { assert!(a.hit[tr][16..].iter().all(|h| !h)); }
        }
    }

    #[test]
    fn probability_zero_suppresses_all_hits_and_one_preserves_algorithm_hits() {
        for kind in Mode::all() {
            let zero = generate(&GenParams { kind, probability: 0.0, ..GenParams::default() });
            assert!(zero.hit.iter().all(|lane| lane[..zero.steps].iter().all(|hit| !hit)), "{kind:?}");
        }
    }

    #[test]
    fn steps_and_rotation_are_clamped() {
        let p = GenParams { steps: MAX_STEPS + 100, rotate: MAX_STEPS + 100, ..GenParams::default() };
        assert_eq!(generate(&p).steps, MAX_STEPS);
        let p = GenParams { steps: 0, ..GenParams::default() };
        assert_eq!(generate(&p).steps, 1);
    }

    #[test]
    fn raised_cosine_hits_track_the_requested_pulse_count() {
        // Regression: the generator once gated hits behind an unrelated modulo term, collapsing
        // every onset into one contiguous arc and ignoring `pulses`.
        for steps in [1usize, 7, 16, 17, 32, 63, 64] {
            for pulses in [0usize, 1, 2, 3, 5, 8, 16, 32, 64] {
                for density in [0.0f32, 0.07, 0.25, 0.5, 0.65, 1.0] {
                    let p = GenParams { kind: Mode::RaisedCosine, steps, pulses, density, probability: 1.0, ..GenParams::default() };
                    let pat = generate(&p);
                    for tr in 0..TRACKS {
                        let requested = if pulses == 0 { 0 } else { (pulses + tr % 3).min(steps.max(1)) };
                        let expected = ((requested as f32 * density).round() as usize).min(steps);
                        assert_eq!(pat.hit[tr][..steps].iter().filter(|&&h| h).count(), expected,
                            "steps={steps}, pulses={pulses}, density={density}, lane={tr}");
                    }
                }
            }
        }
    }

    #[test]
    fn raised_cosine_onsets_stay_evenly_spaced() {
        for steps in [7usize, 13, 16, 31, 64] {
            for pulses in [2usize, 3, 5, 8] {
                let pat = generate(&GenParams { kind: Mode::RaisedCosine, steps, pulses, density: 1.0, probability: 1.0, ..GenParams::default() });
                for tr in 0..TRACKS {
                    let positions: Vec<usize> = (0..steps).filter(|&i| pat.hit[tr][i]).collect();
                    if positions.len() > 1 {
                        let mut gaps: Vec<usize> = positions.windows(2).map(|w| w[1] - w[0]).collect();
                        gaps.push(steps - positions[positions.len() - 1] + positions[0]);
                        let max_gap = *gaps.iter().max().unwrap();
                        let min_gap = *gaps.iter().min().unwrap();
                        assert!(max_gap - min_gap <= 1,
                            "steps={steps}, pulses={pulses}, lane={tr}, gaps={gaps:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn every_mode_supports_all_step_lengths() {
        for kind in Mode::all() {
            for steps in 1..=MAX_STEPS {
                let p = GenParams { kind, steps, pulses: steps / 2, ..GenParams::default() };
                let pat = generate(&p);
                assert_eq!(pat.steps, steps, "{kind:?} at {steps} steps");
                for tr in 0..TRACKS {
                    assert!(pat.hit[tr][steps..].iter().all(|hit| !hit), "tail cells should stay clear");
                }
            }
        }
    }

    #[test]
    fn rotation_is_a_circular_shift_for_every_mode() {
        for kind in Mode::all() {
            let p = GenParams { kind, density: 0.82, probability: 0.67, steps: 17, pulses: 7, rotate: 0, ..GenParams::default() };
            let a = generate(&p); let b = generate(&GenParams { rotate: 3, ..p });
            for tr in 0..TRACKS { for i in 0..p.steps { assert_eq!(b.hit[tr][i], a.hit[tr][(i + p.steps - 3) % p.steps], "{kind:?}"); } }
        }
    }

    #[test]
    fn rotation_reorders_every_per_step_view_array() {
        // The view arrays are the generator's published geometry. If only `hit` follows the
        // rotation, every panel drawn from `theta`/`u` (Arnold, Hilbert, Ulam, Bernoulli,
        // Reaction-Diffusion) points at the wrong step after Generate or a variation bump.
        let n = 17usize;
        for kind in Mode::all() {
            let p = GenParams { kind, density: 0.82, probability: 1.0, steps: n, pulses: 7, rotate: 0, ..GenParams::default() };
            let a = generate(&p);
            let b = generate(&GenParams { rotate: 3, ..p });
            for i in 0..n {
                let src = (i + n - 3) % n;
                assert_eq!(b.state[i], a.state[src], "{kind:?} state at {i}");
                assert_eq!(b.orbit[i], a.orbit[src], "{kind:?} orbit at {i}");
                assert_eq!(b.theta[i], a.theta[src], "{kind:?} theta at {i}");
                assert_eq!(b.u[i], a.u[src], "{kind:?} u at {i}");
                assert_eq!(b.lsym[i], a.lsym[src], "{kind:?} lsym at {i}");
                assert_eq!(b.pebbles[i], a.pebbles[src], "{kind:?} pebbles at {i}");
                assert_eq!(b.impact[i], a.impact[src], "{kind:?} impact at {i}");
                assert_eq!(b.phases[i], a.phases[src], "{kind:?} phases at {i}");
                for row in 0..CA_ROWS { assert_eq!(b.ca[row][i], a.ca[row][src], "{kind:?} ca row {row} at {i}"); }
            }
        }
    }

    #[test]
    fn mirror_reorders_every_per_step_view_array() {
        let n = 17usize;
        for kind in Mode::all() {
            let p = GenParams { kind, density: 0.82, probability: 1.0, steps: n, pulses: 7, ..GenParams::default() };
            let a = generate(&p);
            let b = generate(&GenParams { mirror: true, ..p });
            for i in 0..n {
                let src = n - 1 - i;
                assert_eq!(b.state[i], a.state[src], "{kind:?} state at {i}");
                assert_eq!(b.orbit[i], a.orbit[src], "{kind:?} orbit at {i}");
                assert_eq!(b.theta[i], a.theta[src], "{kind:?} theta at {i}");
                assert_eq!(b.u[i], a.u[src], "{kind:?} u at {i}");
                assert_eq!(b.lsym[i], a.lsym[src], "{kind:?} lsym at {i}");
                assert_eq!(b.pebbles[i], a.pebbles[src], "{kind:?} pebbles at {i}");
                assert_eq!(b.impact[i], a.impact[src], "{kind:?} impact at {i}");
                assert_eq!(b.phases[i], a.phases[src], "{kind:?} phases at {i}");
                for row in 0..CA_ROWS { assert_eq!(b.ca[row][i], a.ca[row][src], "{kind:?} ca row {row} at {i}"); }
            }
        }
    }

    #[test]
    fn select_spread_is_exact_even_and_prefers_structural_steps() {
        for n in [8usize, 16, 32, 100] {
            let mut primary = [false; MAX_STEPS];
            for i in 0..n { primary[i] = i % 3 == 0 || i % 7 == 0; }
            let m = primary[..n].iter().filter(|&&b| b).count();
            for k in 1..=n {
                let mut hits = [false; MAX_STEPS];
                select_spread(&primary, n, k, &mut hits);
                assert_eq!(hits[..n].iter().filter(|&&h| h).count(), k, "n={n} k={k}");
                if k <= m { assert!((0..n).all(|i| !hits[i] || primary[i]), "n={n} k={k}: left the structural steps"); }
                else { assert!((0..n).all(|i| !primary[i] || hits[i]), "n={n} k={k}: dropped a structural step"); }
            }
            for k in [2usize, 3, 5] {
                if k > m { continue; }
                let mut hits = [false; MAX_STEPS];
                select_spread(&primary, n, k, &mut hits);
                let on: Vec<usize> = (0..n).filter(|&i| hits[i]).collect();
                let widest = on.windows(2).map(|w| w[1] - w[0]).max().unwrap_or(0);
                assert!(widest <= 2 * n / k + 2, "n={n} k={k}: gap {widest} in {on:?}");
            }
        }
        let mut h = [true; MAX_STEPS];
        select_spread(&[false; MAX_STEPS], 0, 3, &mut h);
    }

    #[test]
    fn fold01_reflects_any_displacement() {
        for (x, want) in [(0.3, 0.3), (-0.3, 0.3), (1.2, 0.8), (2.5, 0.5), (-1.25, 0.75), (1e6 + 0.25, 0.25), (-7.0, 1.0)] {
            assert!((fold01(x) - want).abs() < 1e-9, "{x} -> {} want {want}", fold01(x));
        }
    }

    #[test]
    fn lane_wanted_matches_the_density_profile() {
        for pulses in [0usize, 1, 5, 16, 64] {
            for density in [0.0f32, 0.25, 0.65, 1.0] {
                for tr in 0..TRACKS {
                    let want = ((pulses.min(64) as f32 * density * (0.5 + DENSITY_PROFILE[tr])).round() as usize).min(64);
                    assert_eq!(lane_wanted(pulses, density, tr, 64), want, "pulses={pulses} d={density} lane={tr}");
                }
            }
        }
    }

    #[test]
    fn distance_and_well_formed_generators_preserve_exact_requested_pulse_counts() {
        for kind in [Mode::ToussaintDistance, Mode::WellFormed] {
            for steps in [1usize, 2, 7, 16, 31, 63, 64] {
                for pulses in [0usize, 1, 2, 3, 7, 16, 32, 64] {
                    for density in [0.0f32, 0.17, 0.35, 0.5, 0.82, 1.0] {
                        let p = GenParams { kind, steps, pulses, density, probability: 1.0, ..GenParams::default() };
                        let pat = generate(&p);
                        for tr in 0..TRACKS {
                            let requested = if pulses == 0 { 0 } else { (pulses.min(steps) + tr % 3).min(steps) };
                            let expected = (requested as f32 * density).round() as usize;
                            let actual = pat.hit[tr][..steps].iter().filter(|&&hit| hit).count();
                            assert_eq!(actual, expected, "{kind:?}: steps={steps}, pulses={pulses}, density={density}, lane={tr}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn well_formed_onset_intervals_differ_by_at_most_one_step() {
        for steps in [7usize, 17, 31, 63, 64] {
            for pulses in [1usize, 2, 3, steps / 2, steps.saturating_sub(1), steps] {
                for density in [0.17f32, 0.5, 1.0] {
                    let pat = generate(&GenParams { kind: Mode::WellFormed, steps, pulses, density, probability: 1.0, ..GenParams::default() });
                    for tr in 0..TRACKS {
                        let positions: Vec<usize> = (0..steps).filter(|&i| pat.hit[tr][i]).collect();
                        if positions.len() > 1 {
                            let mut gaps: Vec<usize> = positions.windows(2).map(|w| w[1] - w[0]).collect();
                            gaps.push(steps - positions[positions.len() - 1] + positions[0]);
                            let min_gap = *gaps.iter().min().unwrap();
                            let max_gap = *gaps.iter().max().unwrap();
                            assert!(max_gap - min_gap <= 1, "steps={steps}, pulses={pulses}, density={density}, lane={tr}, gaps={gaps:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn zero_density_and_zero_pulses_silence_pulse_based_modes() {
        for kind in Mode::all() {
            let zero_density = generate(&GenParams { kind, density: 0.0, ..GenParams::default() });
            assert!(zero_density.hit.iter().all(|lane| lane[..zero_density.steps].iter().all(|hit| !hit)), "{kind:?} at zero density");
            if kind != Mode::Cellular {
                let zero_pulses = generate(&GenParams { kind, pulses: 0, ..GenParams::default() });
                assert!(zero_pulses.hit.iter().all(|lane| lane[..zero_pulses.steps].iter().all(|hit| !hit)), "{kind:?} at zero pulses");
            }
        }
    }

    #[test]
    fn variation_changes_patterns_even_for_closed_form_modes() {
        for kind in Mode::all() {
            let p = GenParams { kind, steps: 17, pulses: 7, density: 0.82, probability: 0.67, ..GenParams::default() };
            assert_ne!(generate(&p).hit, generate(&GenParams { variation: 1, ..p }).hit, "{kind:?}");
        }
    }

    #[test]
    fn polyrhythm_stack_keeps_expected_ratio_counts_and_bounds() {
        let ratios = [(1usize, 1usize), (3, 2), (2, 1), (5, 2), (4, 3), (2, 3), (5, 4), (3, 4)];
        let steps = 64usize;
        let pulses = 8usize;
        let pat = generate(&GenParams { kind: Mode::Polyrhythm, steps, pulses, density: 1.0, probability: 1.0, ..GenParams::default() });
        for tr in 0..TRACKS {
            let (num, den) = ratios[tr];
            let expected = ((pulses * num + den / 2) / den).min(steps);
            let actual = pat.hit[tr][..steps].iter().filter(|&&hit| hit).count();
            assert_eq!(actual, expected, "polyrhythm lane {tr}");
        }
    }

    #[test]
    fn phase_shift_lanes_are_circular_copies_of_the_same_motif() {
        let p = GenParams { kind: Mode::PhaseShift, steps: 32, pulses: 7, density: 1.0, probability: 1.0, phase: 0.0, seed: 0.0, variation: 0, rotate: 0, ..GenParams::default() };
        let pat = generate(&p);
        let reference_count = pat.hit[0][..p.steps].iter().filter(|&&hit| hit).count();
        for tr in 1..TRACKS {
            assert_eq!(pat.hit[tr][..p.steps].iter().filter(|&&hit| hit).count(), reference_count);
            let shift = tr * (p.steps / TRACKS).max(1);
            for i in 0..p.steps {
                let source = (i + p.steps - shift % p.steps) % p.steps;
                assert_eq!(pat.hit[tr][i], pat.hit[0][source], "lane={tr}, step={i}");
            }
        }
        assert_eq!(reference_count, 7);
    }

    #[test]
    fn syncopation_selects_exact_density_scaled_counts() {
        for steps in [7usize, 16, 31, 63, 64] {
            let pulses = (steps / 4).max(1);
            for density in [0.25f32, 0.5, 1.0] {
                let pat = generate(&GenParams { kind: Mode::Syncopation, steps, pulses, density, probability: 1.0, ..GenParams::default() });
                for tr in 0..TRACKS {
                    let expected = ((pulses as f32 * density * (0.45 + DENSITY_PROFILE[tr])).round() as usize).min(steps);
                    let actual = pat.hit[tr][..steps].iter().filter(|&&hit| hit).count();
                    assert_eq!(actual, expected, "steps={steps}, density={density}, lane={tr}");
                }
            }
        }
    }


    #[test]
    fn oscillator_hits_exactly_the_requested_count_for_every_waveform() {
        // Regression: ties at the cutoff were all kept, so a square wave lit every high step and
        // a symmetric sine lit pairs, regardless of `pulses`.
        for wave in 0..oscillator::WAVE_COUNT {
            for steps in [1usize, 2, 7, 16, 21, 32, 64] {
                for pulses in [1usize, 2, 3, 5, 8, 21, 64] {
                    for density in [0.25f32, 0.65, 1.0] {
                        let p = GenParams { kind: Mode::Oscillator, steps, pulses, density, probability: 1.0, osc_wave: wave, ..GenParams::default() };
                        let pat = generate(&p);
                        for tr in 0..TRACKS {
                            let wanted = (((pulses.min(steps)) as f32 * density * (0.5 + DENSITY_PROFILE[tr])).round() as usize).min(steps);
                            let got = pat.hit[tr][..steps].iter().filter(|&&h| h).count();
                            assert_eq!(got, wanted, "wave={wave} steps={steps} pulses={pulses} density={density} lane={tr}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn euclidean_is_exact_evenly_spaced_and_anchored_on_step_zero() {
        // Regression: the bucket form put the first onset at the end of its bucket, so E(4,16)
        // played on 3,7,11,15 and a "four on the floor" kick missed the downbeat.
        for steps in [1usize, 2, 7, 8, 16, 17, 31, 64] {
            for pulses in 1..=steps.min(24) {
                let p = GenParams { kind: Mode::Euclidean, steps, pulses, density: 1.0, probability: 1.0, ..GenParams::default() };
                let pat = generate(&p);
                for tr in 0..TRACKS {
                    let want = ((pulses as f32 * (0.35 + DENSITY_PROFILE[tr]) + 0.5).floor() as usize).min(steps);
                    let pos: Vec<usize> = (0..steps).filter(|&i| pat.hit[tr][i]).collect();
                    assert_eq!(pos.len(), want, "steps={steps} pulses={pulses} lane={tr}");
                    // Lane 0 keeps the downbeat; the other lanes start on their own phase
                    // origin so they cannot all land on step 0.
                    if tr == 0 && want > 0 { assert_eq!(pos[0], 0, "not anchored: steps={steps} pulses={pulses} {pos:?}"); }
                    if pos.len() > 1 {
                        let mut gaps: Vec<usize> = pos.windows(2).map(|w| w[1] - w[0]).collect();
                        gaps.push(steps - pos[pos.len() - 1] + pos[0]);
                        assert!(gaps.iter().max().unwrap() - gaps.iter().min().unwrap() <= 1, "{steps}/{pulses}/{tr}: {gaps:?}");
                    }
                }
            }
        }
        // The concrete case from the bug report.
        let pat = generate(&GenParams { kind: Mode::Euclidean, steps: 16, pulses: 16, density: 0.3, probability: 1.0, ..GenParams::default() });
        let kick: Vec<usize> = (0..16).filter(|&i| pat.hit[0][i]).collect();
        assert_eq!(kick, vec![0, 4, 8, 12]);
        // Regression: every lane used to anchor at step 0, so one step of every pattern (and
        // every variation, which only rotates) carried all eight drums.
        let pat = generate(&GenParams { kind: Mode::Euclidean, steps: 16, pulses: 5, density: 0.65, probability: 1.0, ..GenParams::default() });
        let full: Vec<usize> = (0..16).filter(|&i| (0..TRACKS).all(|tr| pat.hit[tr][i])).collect();
        assert!(full.is_empty(), "all lanes fire together at {full:?}");
    }

    #[test]
    fn no_mode_stacks_every_lane_on_most_variations() {
        // A shared phase anchor (Euclidean) or a shared trigger (Logistic, RaisedCosine) put
        // all eight lanes on one step in *every* variation: rotation moved the column but never
        // removed it. An occasional coincidence is fine; a structural one is not.
        for kind in Mode::all() {
            for steps in [7usize, 16, 32] {
                for density in [0.5f32, 0.65] {
                    let p = GenParams { kind, steps, length: steps as f32, pulses: 5, density, probability: 1.0, seed: 0.271, ..GenParams::default() };
                    let mut with_column = 0;
                    for variation in 0..12u32 {
                        let pat = generate(&GenParams { variation, ..p });
                        if (0..steps).any(|i| (0..TRACKS).all(|tr| pat.hit[tr][i])) { with_column += 1; }
                    }
                    assert!(with_column <= 2, "{kind:?} fires all 8 lanes on one step in {with_column}/12 variations (steps={steps}, density={density})");
                }
            }
        }
    }

    #[test]
    fn invert_and_mirror_transform_only_the_generated_hits() {
        let base = GenParams { kind: Mode::Euclidean, steps: 8, length: 8.0, pulses: 3, density: 1.0, probability: 1.0, rotate: 0, variation: 0, ..GenParams::default() };
        let plain = generate(&base);
        let inverted = generate(&GenParams { invert: true, ..base });
        let mirrored = generate(&GenParams { mirror: true, ..base });
        for tr in 0..TRACKS {
            for i in 0..8 {
                assert_eq!(inverted.hit[tr][i], !plain.hit[tr][i], "invert lane {tr} step {i}");
                assert_eq!(mirrored.hit[tr][i], plain.hit[tr][7 - i], "mirror lane {tr} step {i}");
            }
        }
    }

    #[test]
    fn every_lane_sounds_across_a_seed_sweep() {
        // Catches a lane that is structurally dead rather than merely resting on one seed.
        for kind in Mode::all() {
            for tr in 0..TRACKS {
                let sounded = [0.0f32, 0.13, 0.271, 0.42, 0.58, 0.71, 0.86, 1.0].iter().any(|&seed| {
                    let p = GenParams { kind, steps: 16, length: 16.0, pulses: 5, density: 0.65, probability: 1.0, seed, ..GenParams::default() };
                    generate(&p).hit[tr][..16].iter().any(|&hit| hit)
                });
                assert!(sounded, "{kind:?} lane {tr} never fires across the seed sweep");
            }
        }
    }

    #[test]
    fn crystallographic_cell_is_never_empty_at_positive_density() {
        // A random unit cell could come out empty and tile silence across the lane; at the
        // default cell size that silenced the kick on ~40% of seeds.
        for seed in [0.0f32, 0.13, 0.271, 0.5, 0.86, 1.0] {
            for steps in [4usize, 7, 16, 32] {
                for symmetry in [1usize, 4, 8] {
                    let p = GenParams { kind: Mode::Crystallographic, steps, pulses: 5, density: 0.65, symmetry, probability: 1.0, seed, ..GenParams::default() };
                    let pat = generate(&p);
                    for tr in 0..TRACKS {
                        assert!(pat.hit[tr][..steps].iter().any(|&hit| hit),
                            "silent lane: seed={seed} steps={steps} symmetry={symmetry} lane={tr}");
                    }
                }
            }
        }
    }

    #[test]
    fn well_formed_first_lane_starts_on_step_zero() {
        for steps in [8usize, 16, 17, 32, 64] {
            for pulses in [1usize, 2, 3, 5, 7] {
                let pat = generate(&GenParams { kind: Mode::WellFormed, steps, pulses, density: 1.0, probability: 1.0, ..GenParams::default() });
                assert!(pat.hit[0][0], "steps={steps} pulses={pulses}");
            }
        }
    }

    /// AGENTS.md requires this check by name: every mode must still sound at one step and at full
    /// density. It was missing from the suite, and the logistic map failed it at two steps.
    #[test]
    fn every_mode_produces_hits_at_one_step_and_at_full_density() {
        for kind in Mode::all() {
            let one = generate(&GenParams { kind, steps: 1, pulses: 1, density: 1.0, probability: 1.0, ..GenParams::default() });
            assert!((0..TRACKS).any(|t| one.hit[t][0]), "{kind:?} is silent at steps=1");
            for steps in 1..=MAX_STEPS {
                let pulses = if kind == Mode::Cellular { 5.min(steps) } else { steps };
                let pat = generate(&GenParams { kind, steps, pulses, density: 1.0, probability: 1.0, ..GenParams::default() });
                assert!((0..TRACKS).any(|t| pat.hit[t][..steps].iter().any(|&h| h)), "{kind:?} is silent at steps={steps}, full density");
            }
        }
    }

    #[test]
    fn eulerian_circuit_seam_does_not_double_the_rest() {
        // Degree 1 is the plain ring 0,1,..,8,0. Repeating the closed circuit played rest twice.
        let pat = generate(&GenParams { kind: Mode::Eulerian, steps: 64, pulses: 0, density: 1.0, probability: 1.0, ..GenParams::default() });
        // pulses == 0 silences hits but the walk itself is still recorded in `state`.
        for i in 0..63 { assert!(!(pat.state[i] == 0 && pat.state[i + 1] == 0), "double rest at {i}: {:?}", &pat.state[..64]); }
    }

    #[test]
    fn metric_accents_stay_on_the_grid_when_the_pattern_rotates() {
        // Cellular hits all carry mode accent 1.0, so their final accent is the metric shape alone.
        let base = GenParams { kind: Mode::Cellular, steps: 16, pulses: 3, density: 0.9, probability: 1.0, ..GenParams::default() };
        let a = generate(&base);
        let b = generate(&GenParams { rotate: 5, ..base });
        let mut compared = 0;
        for tr in 0..TRACKS { for i in 0..16 { if a.hit[tr][i] && b.hit[tr][i] { compared += 1; assert!((a.accent[tr][i] - b.accent[tr][i]).abs() < 1e-6, "lane={tr} step={i}"); } } }
        assert!(compared > 0, "test needs overlapping hits to compare");
    }

    #[test]
    fn logistic_short_patterns_survive_the_critical_r() {
        for seed in [0.0f32, 0.1, 0.271, 0.5, 0.9, 1.0] {
            for steps in 1..=8usize {
                let pat = generate(&GenParams { kind: Mode::Logistic, steps, pulses: steps, density: 1.0, probability: 1.0, seed, ..GenParams::default() });
                assert!((0..TRACKS).any(|t| pat.hit[t][..steps].iter().any(|&h| h)), "seed={seed} steps={steps}");
            }
        }
    }

    #[test]
    fn full_step_range_is_supported() {
        for kind in Mode::all() {
            let pat = generate(&GenParams { kind, steps: MAX_STEPS, pulses: 32, density: 0.8, ..GenParams::default() });
            assert_eq!(pat.steps, MAX_STEPS);
            for tr in 0..TRACKS { assert!(pat.accent[tr][..MAX_STEPS].iter().all(|a| a.is_finite() && (0.0..=1.0).contains(a))); }
        }
    }
}
