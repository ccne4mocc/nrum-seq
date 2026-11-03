# Algorithm research and implementation notes

This sequencer deliberately offers different *families* of rhythmic behaviour instead of treating
every mode as a variation on even pulse spacing. MIT OpenCourseWare's course on algorithmic and
generative music treats distributions and stochastic processes, serialism/phasing,
permutations/chaos, Markov chains, cellular automata, genetic algorithms, and L-systems as distinct
compositional approaches ([lecture index](https://ocw.mit.edu/courses/21m-380-music-and-technology-algorithmic-and-generative-music-spring-2010/pages/lecture-notes/)).

This document records the sources and design intent behind the implemented modes. The remaining
fractal, geometric and cellular candidates live in
[`RESEARCH_FRACTAL_GEOMETRIC_CELLULAR.md`](RESEARCH_FRACTAL_GEOMETRIC_CELLULAR.md) as a future
backlog, not as a roadmap.

## Implemented families

### L-System Grammar

L-systems are parallel rewriting grammars. A finite Fibonacci-style grammar expands A/B symbols
into a longer word, which is then interpreted as a repeating hit/accent structure. The production
family depends on the seed and generation variation, so regenerations can reveal related forms.
The rewrite buffer is fixed-size and bounded to keep work predictable in the generator. This is
based on the established use of finite L-systems for rhythmic-sequence generation
(Kaliakatsos-Papakostas, Floros & Vrahatis, *Intelligent Generation of Rhythmic Sequences Using
Finite L-systems*, 2012, [DOI: 10.1109/IIH-MSP.2012.109](https://doi.org/10.1109/IIH-MSP.2012.109)).

### Phase Shift / Reich

Every lane shares a maximally-even pulse word but starts at a different circular offset. The
resulting layers are related rather than independent, making their interactions easy to hear as
the offsets accumulate. This draws on phasing approaches associated with Steve Reich's
*Clapping Music*; a technical implementation discussion shows the practical operation as a
circular shift of rhythmic events
([Shifting Euclidean Rhythm Generator](https://github.com/hfoley03/SERG-Shifting-Euclidean-Rhythm-Generator)).

### Polyrhythm Stack

Each lane uses a different rational pulse ratio within the same full pattern duration. For a base
pulse count k, lane targets use ratios such as 3/2, 5/2, 4/3 and 2/3, then use integer boundary
crossings to distribute those hits across n steps. This is distinct from polymeter, where
individual lanes have different loop lengths. The Buchla 252e documentation likewise describes
simultaneous clocks and ring assignments as a way to generate two- and three-part polyrhythms
([Buchla 252e documentation](https://buchlausa.github.io/buchla_doc/docs/200e/252e)).

### Weighted Syncopation

Each step receives a metrical weight, a lane-dependent offset, and seeded jitter. The generator
selects the highest-scoring steps up to a density-scaled target count. Offbeats are favoured, but
seeded jitter makes ties and repeated patterns less mechanical. The result is deterministic for
the same seed/variation and does not allocate dynamic memory.

### Xenakis sieve

`src/sequence/sieve.rs` evaluates `M = (m₁,r₁) ∪ (m₂,r₂) \ (m₃,r₃)` per step up to the active
pattern length. `ca_rule` selects the three moduli from `[2,3,5,7,11,13]`, `symmetry` and `phase`
select the residues, `pulses` sets how many union terms are active (0 silences, 1–2 terms) and
`density` keeps a fraction of the members. Each lane reads the same sieve rotated by
`lane · floor(n/8)`, with a small deterministic tie-break so partial density does not pick the
same columns on every lane. The panel draws the three residue rows and the resulting step mask.

### Kuramoto phase bank

`src/sequence/kuramoto.rs` updates eight phases once per step:
`θᵢ += 2π·rate·detuneᵢ + (K/8)(sin(θᵢ₋₁−θᵢ) + sin(θᵢ₊₁−θᵢ))`, nearest-neighbour ring coupling
only. `osc_rate` maps to `K`, `phase` to the frequency spread, `pulses` to the target onset count
and `density` to the lane weight. A phase wrap is the real trigger; rank selection scores wraps
first and near-misses by closeness, so the count stays exact. The phase bank is published per step
in `Pattern.phases`, and the panel draws the pointers, trails and order-parameter vector.

### Melodic-migration modes

The nelody seq review supplied tested implementations for six additional rhythmic families,
adapted here to eight lanes: **Thue–Morse** (A010060 word, Nørgård infinity-series accents),
**Rudin–Shapiro** (A020985 word), **Brownian** (Irwin-Hall Gaussian increments, folded walk),
**Lévy flight** (Pareto steps with a real heavy tail), **Tent map** (attractor start plus
transient) and **Hénon map** (basin-safe start, escape guard). The same migration corrected
Arnold to fire on actual phase wraps, made Fibonacci use the golden slope, made Ulam a real
spiral-prime map, made the Hilbert order follow the loop length, and made the L-system expand far
enough for 1024-step patterns.

Two later modes complete the current set:

- **Life-like 2-D CA** (mode 39): a 16×8 torus with Life, HighLife, Seeds, Day & Night and a
  dense survival rule, plus selectable initial shapes.
- **Vuza Canon** (mode 32): a translation set derived from a Vuza-style modulus.

## Future candidates

The shortlist is kept in
[`RESEARCH_FRACTAL_GEOMETRIC_CELLULAR.md`](RESEARCH_FRACTAL_GEOMETRIC_CELLULAR.md). In brief:

- **Paperfolding words** for another automatic sequence with a different contour.
- **Mandelbrot/Julia, Newton, Feigenbaum, IFS** fractal mappings.
- **Spirograph, Platonic walk, Penrose, Truchet, Flower of Life** geometric mappings.
- **Langton's Ant, cyclic CA, radius-2 1-D CA** cellular mappings.

Candidates should reuse the existing shared knobs where possible. A candidate that needs more than
two independent controls should wait for a typed mode-parameter contract rather than adding another
overloaded host parameter.

## Validation strategy

Unit tests cover mode dispatch, determinism, bounds for 1–1024 steps, global probability, rotation,
zero density/pulses, the polyrhythm and syncopation count rules, the sieve membership rule, the
Kuramoto bank's exact counts and its order parameter under weak versus strong coupling, and that
neighbouring seeds diverge while one seed repeats. Every mode is also swept for view geometry that
escapes the panel. `generation_never_allocates`, `process_never_allocates`,
`every_mode_produces_hits_at_one_step_and_at_full_density` and
`every_mode_supports_all_step_lengths` are the non-negotiable guards.
