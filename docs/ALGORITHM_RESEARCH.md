# Algorithm research and implementation notes

This sequencer intentionally offers different *families* of rhythmic behavior rather than treating every mode as a variation on even pulse spacing. MIT OpenCourseWare's course on algorithmic and generative music treats distributions and stochastic processes, serialism/phasing, permutations/chaos, Markov chains, cellular automata, genetic algorithms, and L-systems as distinct compositional approaches ([lecture index](https://ocw.mit.edu/courses/21m-380-music-and-technology-algorithmic-and-generative-music-spring-2010/pages/lecture-notes/)). The following four additions were selected to bring grammar-based, phase-based, ratio-based, and metric-weighted ideas into the current fixed-size MIDI pattern engine.

## Newly implemented

### L-System Grammar

L-systems are parallel rewriting grammars. A finite Fibonacci-style grammar expands A/B symbols into a longer word, which is then interpreted as a repeating hit/accent structure. Production family depends on the seed and generation variation, so the mode remains controllable but regenerations can reveal related forms. The rewrite buffer is fixed-size and bounded to keep work predictable in the generator. This is based on the established use of finite L-systems for rhythmic-sequence generation (Kaliakatsos-Papakostas, Floros & Vrahatis, *Intelligent Generation of Rhythmic Sequences Using Finite L-systems*, 2012, [DOI: 10.1109/IIH-MSP.2012.109](https://doi.org/10.1109/IIH-MSP.2012.109)).

### Phase Shift / Reich

Every lane shares a maximally-even pulse word but starts at a different circular offset. The resulting layers are related rather than independent, making their interactions easy to hear as the offsets accumulate. This draws on phasing approaches associated with Steve Reich's *Clapping Music*; a technical implementation discussion shows the practical operation as a circular shift of rhythmic events ([Shifting Euclidean Rhythm Generator](https://github.com/hfoley03/SERG-Shifting-Euclidean-Rhythm-Generator)).

### Polyrhythm Stack

Each lane uses a different rational pulse ratio within the same full pattern duration. For a base pulse count k, lane targets use ratios such as 3/2, 5/2, 4/3 and 2/3, then use integer boundary crossings to distribute those hits across n steps. This is distinct from polymeter, where individual lanes have different loop lengths. The Buchla 252e documentation likewise describes simultaneous clocks and ring assignments as a way to generate two- and three-part polyrhythms ([Buchla 252e documentation](https://buchlausa.github.io/buchla_doc/docs/200e/252e)).

### Weighted Syncopation

Each step receives a metrical weight, a lane-dependent offset, and seeded jitter. The generator selects the highest-scoring steps up to a density-scaled target count. Offbeats are favored, but seeded jitter makes ties and repeated patterns less mechanical. The result is deterministic for the same seed/variation and does not allocate dynamic memory.

### Three-Body Chaos

The logistic map is a one-dimensional chaotic system whose orbit the panel draws directly. The
three-body mode is a genuinely different one: a real Newtonian N-body integration of three
masses, integrated with velocity-Verlet leapfrog under softened gravity
`a_i = Σ G m_j (r_j - r_i) / (|r_i - r_j|² + ε²)^(3/2)`.

The configuration is a tight binary (masses 1.0 and 0.6) plus a lighter third body (0.4) on a
wider orbit, which is the standard setup for Kozai-Lidov exchange: the third body's eccentricity
breathes on a long cycle, so close approaches arrive irregularly rather than on a schedule.
That irregularity is the point — it is a source of genuinely aperiodic rhythm structure rather
than a periodic sequence read at an irrational offset. The chaotic nature of the three-body
problem is established in the literature (Bruns 1887 on non-integrability; Poincaré 1890 on
sensitivity to initial conditions; Saari and Xia 1991 on non-collision singularities).

Onsets come from a per-step score rather than from thresholding a raw signal:

- **Sweep** — how far the third body swept about the barycentre during the step, in revolutions.
- **Encounter** — proximity to the binary, as `1 / (1 + d_min)`.

Each lane combines the two with weights that favour encounters for quiet lanes and sweep for loud
ones, so the eight lanes are not nested subsets of one ranking. Hits are then chosen by **rank
selection** — a step hits when fewer than `wanted` steps outrank it — the same bounded rule the
oscillator and syncopation modes use. This guarantees an exact, density-scaled onset count at
every step count, keeps the mode non-silent at `steps=1` and at full density, and needs no sort
or allocation.

Accents come from a **rotating line**: a line rotating from the barycentre completes `pulses`
turns across the pattern, and steps where the third body crosses it are the loud ones. That is
the rotating-line model of Euclidean rhythms (Rosin & Worby, "Introducing Circular Euclidean
Rhythms", 2014) driven by a chaotic angle instead of a linear one.

Control mapping: `seed` perturbs the initial conditions, `phase` widens the third body's orbit
(tight orbits produce frequent encounters, wide ones produce long quiet sweeps), `osc_rate` sets
the integration resolution, `pulses` is the onset target, `density` is the usual lane weight. No
new host parameters were added.

## Further candidates for sequencing (not implemented)

Shortlist, filtered by what a fixed 64-step, allocation-free, deterministic snapshot can
actually express. "Cost" is per generated pattern, worst case at 64 steps.

| Candidate | Math | Musical use | Cost / risk |
|---|---|---|---|
| Xenakis sieve | Union/intersection/complement of residue classes modulo primes (Xenakis, *Formalized Music*, 1971) | Sparse, non-periodic pitch/rhythm sets; exactly reproducible | 64-bit mask, O(steps), trivial |
| Kuramoto phase bank | `dθᵢ/dt = ωᵢ + (K/N)Σ sin(θⱼ−θᵢ)` | Eight coupled lanes that drift into and out of unison; emergent polyrhythm | O(8²·steps), stack-only; needs a stable fixed-step integration |
| Arnold circle map | `θₙ₊₁ = θₙ + Ω + (K/2π)sin(2πθₙ)` | Mode-locked rational ratios from an irrational drive (devil's staircase) | O(steps), one state; exact ratio emerges but only for some (Ω,K) |
| de Bruijn sequence | `B(k,n)`: every n-symbol word exactly once | Maximal non-repeating canons over a small alphabet | Ring buffer of n states, O(1) amortised |
| Thue–Morse / paperfolding | Automatic (morphic) sequences with self-similarity | Self-similar canons and accents; complements between lanes | O(steps) bit tricks |
| Vuza canons | Factor `Zₙ = A ⊕ B` into supplementary sets | Aperiodic rhythmic canons that never repeat a translation | Needs a precomputed factorisation table; larger implementation |

Three that fit this engine best, in implementation order:

1. **Xenakis sieves.** A sieve is a 64-bit mask built from `(modulus, residue)` pairs:
   `M = (m₁,r₁) ∪ (m₂,r₂) \ (m₃,r₃)`. The generator is a pure function of three small
   integers, so the existing knobs map naturally (`ca_rule` → modulus pair, `symmetry` →
   residue, `pulses`/`density` → number of sieve terms). Starts at `steps=1`, supports full
   density, and the panel can draw the sieve lattice plus the residue classes directly. Best
   first choice: it adds a genuinely different rhythmic family for almost no code.
2. **Kuramoto coupled lanes.** Eight phase oscillators, one per lane, coupled by a strength
   derived from `density` and detuned by `phase`; a lane fires when its phase crosses 1. The
   integration is 64 steps × 64 pairs of `sin`, which is bounded and stack-only. The panel is
   the eight phase pointers on one circle, so drift and lock are visible. It reuses `osc_rate`
   as coupling, not as frequency, and the tests are the same hit-count and determinism checks.
3. **Arnold circle map.** This restores the reference project's dropped circle-map mode with a
   real mechanism: `Ω` from `phase`, coupling `K` from `density`, and a per-lane detune from
   `seed`. Near a rational `Ω` the map locks (`p/q`), so the panel can show the locked ratio
   while the rhythm changes in steps of that ratio. It is the cheapest chaos-family addition
   after Three-Body and shares its fixed-step discipline.

The canons (de Bruijn, Thue–Morse, Vuza) are attractive but lower priority: they produce
striking non-repetition, yet their parameter surfaces are small and they need either a table or
a longer word buffer before they sound different from the existing L-System and Phase Shift
modes.

## Validation strategy

Unit tests cover mode dispatch, determinism, bounds for 1–64 steps, global probability, rotation,
zero density/pulses, the polyrhythm / syncopation count rules, the three-body orbit's finiteness
across the seed/phase/rate space, its exact onset counts, and that neighbouring seeds diverge
while one seed repeats. Every mode is also swept for view geometry that escapes the panel.
