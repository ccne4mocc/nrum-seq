# Reference project mapping: `_AlgoNemLody` → nrum seq

This map records the supplied melody sequencer's existing view conventions so follow-up agents can preserve its design language instead of guessing.

## Reference modes and current counterparts

The reference is explicit in `src/lib.rs` (`Mode::kind()`), `src/pattern.rs` (`GenKind`), `src/view.rs` (`build()`), and `ui/app.slint` (`pulses-labels`, `shape-label`, `ModePanel`). nrum seq keeps one mode enum instead of the reference's two: `Mode` in `src/modes.rs` is both the host parameter and the generator's dispatch key.

| Reference mode | Reference generator | Reference view | nrum seq adaptation |
|---|---|---|---|
| Euclidean | `E(pulses, steps)` | Radial clock; active hit nodes and a playhead hand | **Euclidean** — one pulse ring per drum lane, so per-lane densities stay visible |
| Golomb Ruler | Optimal Golomb ruler marks | Pitch/step strip only (`view_kind = 6`) | Replaced by **Crystallographic** — periodic motif with symmetry-lattice view |
| Logistic Map | Top-N / thresholded logistic orbit | Pitch/step strip only (`view_kind = 6`) | **Logistic Map** — logistic orbit diagram, plus lane trigger matrix |
| Eulerian Walk | Hierholzer circuit on balanced directed graph, vertex 0 = rest | Directed graph nodes/edges and moving path | **Eulerian Walk** — balanced drum-state graph, rest and eight lane vertices |
| Arnold Circle Map | Circle-map phase iteration | Phasor plot | **Arnold Circle Map** — a real wrap-triggered map on eight lanes, with a phasor plot |
| Mozart Dice | Mozart dice-table variation | Step velocity bars, with dice roll tags | Replaced by **Golden Ratio / Phyllotaxis** — golden-angle spiral |
| Tonnetz Walk | P/L/R path through triad lattice | Hexagonal triad mesh | Replaced by **Markov Walk** — transition graph over rest/drum states |
| Rule 30 | Rule 30 on a ring; pulses selects generation | Concentric CA rings (`view_kind = 5`) | **Cellular Automaton** — 1D CA space-time grid, selected row drives lane hits |

These replacements prioritize relevant drum sequencing methods. The new names and enum order are intentionally stable because the UI uses the same index order as the host enum.

## New mode → view index

The `Mode` enum in `src/modes.rs`, `Mode::GROUPS` (the one display order used by the dropdown and the header wheel) and the hardcoded mode indices in the UI's `shows-param`/`pulse-label` functions must match. Mode names are pushed from Rust, so there is no second list in Slint. Group labels and order are shared with nelody seq and must stay identical: Rhythm, Stochastic, Chaos / Physics, Fractal, Cellular, Geometric, Number Theory, Music Theory, Acid (nrum seq omits the empty Acid group):

| Index | Mode | Visualization |
|---:|---|---|
| 0 | Euclidean | Per-lane pulse rings, one ring per drum lane |
| 1 | Eulerian | Directed state-transition graph |
| 2 | Golden Ratio / Phyllotaxis | Sampled √i rosette of the 1/φ rotation (at most 128 groups), per-lane primary and secondary trigger arcs, aggregated lane density per group, and the 1/φ³ rule as its own inner band |
| 3 | Cellular Automaton | Ring-based CA history / space-time cells, each lane's own selected generation marked |
| 4 | Oscillator / Phase | One of eight waveform curves, the generator's rank cutoff, and sampled step markers |
| 5 | Crystallographic | Eight-lane symmetry lattice with unit-cell axes |
| 6 | Logistic Map | Orbit diagram and orbit trace |
| 7 | Markov Walk | Directed percussion-state transition graph |
| 8 | Raised-Cos Threshold | The generator's envelope curve and its cutoff |
| 9 | Slowcat Rhythm | Phrase timeline with per-lane motif boundaries |
| 10 | Toussaint Distance | Distance circle: onsets on lag rings, rim ticks for the step grid |
| 11 | Well-Formed Rhythm | Christoffel/Bresenham slack plot — the two inter-onset lengths |
| 12 | L-System Grammar | The generator's actual A/B word per step, with per-lane gate bars |
| 13 | Phase Shift / Reich | One motif ring per lane, each a rotated copy of the shared outline |
| 14 | Polyrhythm Stack | Concentric ratio rings, dot size by accent |
| 15 | Weighted Syncopation | The generator's metrical score as bars, selection marked |
| 16 | Pebble Box | Animated pebble box; eight lane pebbles at their recorded per-step positions |
| 17 | Xenakis Sieve | Residue-class lattice rows plus the generator's per-step membership and its onsets |
| 18 | Kuramoto Lanes | Eight phase pointers, their trails and the mean phase vector on one circle |
| 19 | Euclidean-Swing | Euclidean necklaces with off-beat accents |
| 20 | Bernoulli | Per-step coin flips, one lane per row |
| 21 | African Bell | Rotated 12-step bell figure per lane |
| 22 | Phase Canon | Shifted pulse-train copies |
| 23 | Reaction-Diffusion | Activator curve and threshold |
| 24 | Lorenz Attractor | Butterfly path and current orbit |
| 25 | Cantor Set | Surviving Cantor intervals and onsets |
| 26 | Fibonacci / Sturmian | Golden-slope staircase with onsets; lane 0 is the exact word at density 1 |
| 27 | Genetic Mutation | Mutated base pattern per lane |
| 28 | Arnold Circle Map | Phasor trajectory; Ω from Pulses, K from Density, onsets are phase wraps |
| 29 | Sturmian Word | Phase-controlled Sturmian slope |
| 30 | Hilbert Curve | nelody seq sampling `i·total/n`; Pulses = order, Phase = quarter turns, Seed shifts the lane mapping; density thins curve cells |
| 31 | Ulam Spiral Primes | Real square spiral from phase/variation; prime cells mapped by diagonal; Pulses sets target onsets |
| 32 | Vuza Canon | Modulus rows; `ca_rule` picks N |
| 33 | Thue-Morse | Lane-0 A010060 word strip plus eight lane onset bands |
| 34 | Rudin-Shapiro | Lane-0 A020985 word strip plus eight lane onset bands |
| 35 | Brownian Walk | Lane-0 reflected walk trace and step-size stems |
| 36 | Levy Flight | Lane-0 folded walk trace and jump-size stems |
| 37 | Tent Map | Return map `y = mu·min(x,1−x)` plus orbit dots |
| 38 | Henon Map | Attractor path from the published `orbit`/`theta` |

Indices 8–38 all draw their own panel. There is no shared step-lattice fallback: a mode whose
structure is not a grid shows that structure instead. Each panel reads the generator's own
formula or published state through an exported helper; if a panel's constants cannot be traced
to a `Pattern` field or a generator helper, the panel is drawing a decoration, not the algorithm.

When adding or reordering a mode, update together: the `Mode` enum in `src/modes.rs`, dispatch in `sequence::generate()`, `view::build()`, the `Mode` labels in Slint, the mode indices the UI hardcodes, the editor's `from_index()` bound, and the expected parameter/view tests.

## Crystallographic mode: what is actually a group action

The generator is deliberately named after crystallographic language, but the implementation is a
one-dimensional periodic motif with artistic symmetry operations, not a faithful space-group
action. Its operations (`src/sequence/crystallographic.rs`) are:

| `symmetry` | Code path | Honest description |
|---:|---|---|
| 1 | `offset` | p1 translation: every cell is the same motif. This is a real translation action on the ring. |
| 2 | reverse every other cell | Alternating mirror. It is not a closed group action on the ring. |
| 3 | half-cell shift on odd cells | Glide-like phase offset, not a glide plane. |
| 4 | reverse by the cell index | Motif inversion around a moving centre. |
| 5 | add the cell index | Screw-like phase drift. |
| 6 | alternate half-cell shift | Centering-like alternation. |
| 7 | reverse with a cell-index shift | Alternating reflection with drift. |
| 8 | staggered lattice | A second lattice phase; the odd cells also invert at high density. |

The motif cell is `steps / symmetry` wide (or the whole pattern when that leaves fewer than two
slots). The smallest possible change in hit count is therefore one cell, so the density control is
structurally steppy rather than smooth. That is a property of the quantisation, not a tuning bug.

The panel draws the same unit-cell axes and lane lattice. Future work could split `symmetry` into
an order control and a separate operation selector, but that adds a host parameter and changes
existing patterns, so it is not part of this pass.

## Per-mode controls

Every mode also shows Steps, Rotate, Probability, Velocity, Regenerate after, Randomize after
(lengths) and Swing. Those are shared post-transforms or clock settings. The table lists the remaining controls
in the order the panel exposes them; `ui/app.slint` (`shows-param`, `pulse-label`,
`density-label`, `phase-label`, `osc-label`, `ca-label`) is the presentation source of truth, and
the generator's own use of `GenParams` is the behavioural source of truth.

| Mode | Mode-specific controls |
|---|---|
| Euclidean | Target pulses, Density, Seed |
| Eulerian | Degree (0–3), Density, Seed |
| Phyllotaxis | Pulses, Density, Seed, Spiral phase |
| Cellular | Generation (0–15), Density, Seed, Generation offset, CA rule |
| Oscillator | Requested hits, Density, Seed, Osc phase, Osc cycles, Wave (8 shapes) |
| Crystallographic | Motif density, Density, Seed, Symmetry |
| Logistic | Target peaks, Growth (r), Seed, Initial x |
| Markov | Walk activity, Density, Seed |
| Raised-Cos | Envelope cycles, Density, Seed, Envelope phase |
| Slowcat | Motif pulses, Density, Seed |
| Toussaint | Pulses, Density, Seed |
| Well-Formed | Pulses, Density, Seed |
| L-System | Pulses, Density, Seed |
| Phase Shift | Pulses, Density, Seed, Phase offset |
| Polyrhythm | Pulses, Density, Seed |
| Syncopation | Target onsets, Density, Seed |
| Pebble Box | Shaker impulses, Impact sensitivity, Seed, Gravity tilt, Substeps, Gravity |
| Sieve | Sieve terms (0–2), Sieve density, Seed, Residue shift, Sieve moduli, Symmetry |
| Kuramoto | Target onsets, Onset density, Seed, Detune spread, Coupling |
| Fibonacci | Density (golden slope), Seed, phase; `pulses` hidden |
| Arnold | Rotation Ω (Pulses), Coupling K (Density), Seed, Start phase |
| Hilbert | Order (Pulses 1–5), Density, Seed, Rotation (Phase) |
| Ulam | Target onsets, Density, Spiral start; Seed hidden |
| Thue-Morse | Target onsets, Onset density, Window offset; Seed hidden |
| Rudin-Shapiro | Target onsets, Onset density, Window offset; Seed hidden |
| Brownian | Target onsets, Step sigma, Seed, Start position |
| Levy Flight | Target onsets, Tail alpha, Seed, Start position |
| Tent Map | Target onsets, Slope (mu), Seed, Initial x |
| Henon Map | Target onsets, Map (a), Seed, Start offset |

Mode-scoped reuse is deliberate: `ca_rule` is the cellular rule, Pebble Box's gravity and the
Sieve moduli; `symmetry` selects Crystallographic cells and Sieve residues; `osc_rate` is
oscillator cycles, Pebble Box substeps or Kuramoto coupling. Seed is hidden where the generator
never reads it (Oscillator, Well-Formed, Ulam, Thue–Morse, Rudin–Shapiro); the shared accent
humanizer still reads it, so it keeps affecting velocities but not the generated line. A control
that the selected mode never reads is hidden, not left inert. `ca_rule` maps
to gravity as `pebble::gravity()` (`0.01 + ca_rule/255 × 2.15` box-heights per step²).


## Shared interaction patterns retained from the reference

- One shared pattern snapshot drives both audio output and preview.
- A Generate switch is host-mappable and changes the persistent variation counter when it flips, including on→off and off→on.
- A stable Rotate knob post-transforms all generated lanes.
- UI mode views are generated as pure geometry and rendered via Slint paths/nodes.
- All user-facing generator parameters are host-automatable. Lane enable, note and velocity parameters are also exposed even when the first UI pass only shows lane enables.
