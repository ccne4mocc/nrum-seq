# Mode examples

One starting point per mode. Numbers are `steps`, `pulses`, `density` and any mode-specific
control that matters. Every mode is deterministic for a given seed/variation snapshot; press
Generate for a new variation.

| Mode | Try | Listen for |
|---|---|---|
| Euclidean | 16 / 5 / 65% | Kick on the four-beat grid, other lanes on their own phase origins. |
| Eulerian Walk | 16 / 2 (degree 3) / 70% | A short recurrent walk through rest and drum states. |
| Phyllotaxis | 32 / 8 / 65% / phase 0 | Golden-angle rosette; eight lanes with distinct trigger windows. |
| Cellular Automaton | 16 / 8 / 50% / rule 30 / random | Rule-30 space-time texture; each lane reads a different generation. |
| Life-like CA | 64 / 4 warm-up / 50% / Life / random | Two-dimensional population waves mapped to lane rows. |
| Oscillator | 16 / 8 / 65% / saw / 2 cycles | Rank-cut waveform; the panel shows the same curve and cutoff. |
| Crystallographic | 16 / 6 / 65% / symmetry 4 | Four-cell motif repeated by the selected symmetry operation. |
| Logistic Map | 32 / 8 / 70% / initial x .3 | Chaotic orbit peaks, one lane per density tier. |
| Markov Walk | 16 / 8 / 70% / bias 0 | Role-compatible transitions; raise a bias field for more rests, repeats, roles or novelty. |
| Raised-Cos Threshold | 16 / 8 / 65% / phase .25 | Smooth envelope crossings; even spacing at high density. |
| Slowcat | 32 / 5 / 80% | Lane-length motifs rotating over slow subdivisions. |
| Toussaint Distance | 16 / 5 / 65% | Onsets at circular distance ranks on the distance circle. |
| Well-Formed Rhythm | 16 / 5 / 100% | Christoffel/Bresenham word; gaps differ by at most one step. |
| L-System Grammar | 64 / 10 / 70% / symmetry 1 | Family-A rewrite word; change symmetry to compare families. |
| Phase Shift / Reich | 32 / 7 / 80% / phase .25 | One motif, eight circular phase offsets. |
| Polyrhythm Stack | 16 / 8 / 80% | Lane-specific integer ratios against the same grid. |
| Weighted Syncopation | 16 / 6 / 65% | Metrical-score selection; more offbeat weight as density rises. |
| Pebble Box | 32 / 4 shakers / 65% / rule 128 | Bouncing pebbles; hits mark the wall that was struck. |
| Xenakis Sieve | 32 / 2 terms / 70% / rule 60 / symmetry 4 | Residue-class union minus a subtracted class. |
| Kuramoto Lanes | 64 / 8 / 80% / coupling .5 | Phase-locked pointers and a moving order-parameter vector. |
| Euclidean-Swing | 16 / 5 / 65% / swing 33% | Euclidean line with accented offbeats and host-grid swing. |
| Bernoulli | 32 / — / 35% | Independent coin flips; the top strip shows each probability. |
| African Bell | 24 / — / 100% | A rotated 12-step bell figure per lane. |
| Phase Canon | 32 / 5 / 80% | Eight shifted copies of one pulse train. |
| Reaction-Diffusion | 64 / — / 60% | Activator peaks crossing the threshold. |
| Lorenz Attractor | 64 / — / 80% | z-peaks from the Lorenz orbit. |
| Cantor Set | 81 / — / 100% | Two-level middle-third removal. |
| Fibonacci / Sturmian | 64 / — / 65% | Golden-slope staircase; lane 0 is the exact word at density 1. |
| Genetic Mutation | 16 / — / 50% | A stable base phrase with density-driven mutation. |
| Arnold Circle Map | 16 / 5 (Omega) / 60% (K) / phase .2 | Phase wraps; rational Omega locks the ratio. |
| Sturmian Word | 64 / — / 100% / phase .5 | Mechanical word with an irrational slope. |
| Hilbert Curve | 16 / 3 (order) / 70% / phase .25 | Curve cells sampled across the order-3 Hilbert path. |
| Ulam Spiral Primes | 64 / 8 / 60% | Prime cells on a square spiral. |
| Vuza Canon | 32 / — / 50% / rule 16 | Translation set from the Vuza-style modulus. |
| Thue-Morse | 64 / 12 / 70% | A010060 word; lane windows are shifted. |
| Rudin-Shapiro | 64 / 12 / 70% | A020985 word; compare with Thue-Morse. |
| Brownian Walk | 64 / — / 30% / phase .2 | Reflected Gaussian walk; largest steps fire. |
| Levy Flight | 64 / — / 30% | Heavy-tailed jumps; long rests then clusters. |
| Tent Map | 64 / — / 70% / initial x .2 | Tent return map; slope is density. |
| Henon Map | 64 / — / 70% | Hénon attractor; largest x-moves fire. |

The examples are suggestions, not presets. Built-in starting-point presets in Settings use
ordinary host parameters, so applying one leaves the project fully automatable and saveable.
