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

## Validation strategy

Unit tests cover mode dispatch, determinism, bounds for 1–256 steps, global probability, rotation, zero density/pulses, and the new polyrhythm / syncopation count rules. The UI and host build still need to be compiled in an environment with the project's Rust toolchain and Slint 1.16.1 dependency.
