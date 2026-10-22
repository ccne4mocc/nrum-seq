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

## Implemented from this list

- **Xenakis sieve** (`src/sequence/sieve.rs`, mode 17). `M = (m₁,r₁) ∪ (m₂,r₂) \ (m₃,r₃)`
  evaluated per step up to the active pattern length. `ca_rule` selects the three moduli from
  `[2,3,5,7,11,13]`, `symmetry`
  and `phase` select the residues, `pulses` sets how many union terms are active (0 silences,
  1–2 terms) and `density` keeps a fraction of the members. Each lane reads the same sieve
  rotated by `lane · floor(n/8)`, with a small deterministic tie-break so partial density does
  not pick the same columns on every lane. The panel draws the three residue rows (the
  subtracted class in amber) and the resulting step mask.
- **Kuramoto phase bank** (`src/sequence/kuramoto.rs`, mode 18). Eight phases update once per
  step: `θᵢ += 2π·rate·detuneᵢ + (K/8)(sin(θᵢ₋₁−θᵢ) + sin(θᵢ₊₁−θᵢ))`, nearest-neighbour ring
  coupling only. `osc_rate` maps to `K`, `phase` to the frequency spread, `pulses` to the
  target onset count and `density` to the lane weight. A phase wrap is the real trigger; rank
  selection scores wraps first and near-misses by closeness, so the count stays exact. The
  phase bank is published per step in `Pattern.phases`, and the panel draws the eight pointers,
  their trails and the order-parameter vector.

## Further candidates for sequencing (not implemented)

Shortlist, filtered by what a fixed 1024-step capacity (64 by default), allocation-free, deterministic snapshot can
actually express. "Cost" is per generated pattern, worst case at the active ceiling.

| Candidate | Math | Musical use | Cost / risk |
|---|---|---|---|
| Arnold circle map | `θₙ₊₁ = θₙ + Ω + (K/2π)sin(2πθₙ)` | Mode-locked rational ratios from an irrational drive (devil's staircase) | O(steps), one state; exact ratio emerges but only for some (Ω,K) |
| de Bruijn sequence | `B(k,n)`: every n-symbol word exactly once | Maximal non-repeating canons over a small alphabet | Ring buffer of n states, O(1) amortised |
| Thue–Morse / paperfolding | Automatic (morphic) sequences with self-similarity | Self-similar canons and accents; complements between lanes | O(steps) bit tricks |
| Vuza canons | Factor `Zₙ = A ⊕ B` into supplementary sets | Aperiodic rhythmic canons that never repeat a translation | Needs a precomputed factorisation table; larger implementation |

The next best fit is the **Arnold circle map**. This restores the reference project's dropped
circle-map mode with a real mechanism: `Ω` from `phase`, coupling `K` from `density`, and a
per-lane detune from `seed`. Near a rational `Ω` the map locks (`p/q`), so the panel can show
the locked ratio while the rhythm changes in steps of that ratio. It is the cheapest
remaining chaos-family addition and shares the existing fixed-step discipline.

The canons (de Bruijn, Thue–Morse, Vuza) are attractive but lower priority: they produce
striking non-repetition, yet their parameter surfaces are small and they need either a table or
a longer word buffer before they sound different from the existing L-System and Phase Shift
modes.

## Validation strategy

Unit tests cover mode dispatch, determinism, bounds for 1–1024 steps, global probability, rotation,
zero density/pulses, the polyrhythm / syncopation count rules, the sieve membership rule,
the Kuramoto bank's exact counts and its order parameter under weak versus strong coupling, and
that neighbouring seeds diverge while one seed repeats. Every mode is also swept for view geometry
that escapes the panel.
