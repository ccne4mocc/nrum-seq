# Reference project mapping: `_AlgoNemLody` → nDrum

This map records the supplied melody sequencer's existing view conventions so follow-up agents can preserve its design language instead of guessing.

## Reference modes and current counterparts

The reference is explicit in `src/lib.rs` (`Mode::kind()`), `src/pattern.rs` (`GenKind`), `src/view.rs` (`build()`), and `ui/app.slint` (`pulses-labels`, `shape-label`, `ModePanel`). nDrum keeps one mode enum instead of the reference's two: `Mode` in `src/modes.rs` is both the host parameter and the generator's dispatch key.

| Reference mode | Reference generator | Reference view | nDrum adaptation |
|---|---|---|---|
| Euclidean | `E(pulses, steps)` | Radial clock; active hit nodes and a playhead hand | **Euclidean** — one pulse ring per drum lane, so per-lane densities stay visible |
| Golomb Ruler | Optimal Golomb ruler marks | Pitch/step strip only (`view_kind = 6`) | Replaced by **Crystallographic** — periodic motif with symmetry-lattice view |
| Logistic Map | Top-N / thresholded logistic orbit | Pitch/step strip only (`view_kind = 6`) | **Logistic Map** — logistic orbit diagram, plus lane trigger matrix |
| Eulerian Walk | Hierholzer circuit on balanced directed graph, vertex 0 = rest | Directed graph nodes/edges and moving path | **Eulerian Walk** — balanced drum-state graph, rest and eight lane vertices |
| Arnold Circle Map | Circle-map phase iteration | Phasor plot | Replaced by **Oscillator / Phase** — waveform, phase and threshold |
| Mozart Dice | Mozart dice-table variation | Step velocity bars, with dice roll tags | Replaced by **Golden Ratio / Phyllotaxis** — golden-angle spiral |
| Tonnetz Walk | P/L/R path through triad lattice | Hexagonal triad mesh | Replaced by **Markov Walk** — transition graph over rest/drum states |
| Rule 30 | Rule 30 on a ring; pulses selects generation | Concentric CA rings (`view_kind = 5`) | **Cellular Automaton** — 1D CA space-time grid, selected row drives lane hits |

These replacements prioritize relevant drum sequencing methods. The new names and enum order are intentionally stable because the UI uses the same index order as the host enum.

## New mode → view index

The `Mode` enum in `src/modes.rs`, the mode order in `ui/app.slint` and the hardcoded mode indices in the UI's `shows-param`/`pulse-label` functions must match:

| Index | Mode | Visualization |
|---:|---|---|
| 0 | Euclidean | Per-lane pulse rings, one ring per drum lane |
| 1 | Eulerian | Directed state-transition graph |
| 2 | Golden Ratio / Phyllotaxis | Eight-lane √i rosette of the 1/φ rotation, per-lane primary and secondary trigger arcs, lit dots inside each lane's wedge, and the 1/φ³ rule as its own inner band |
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
| 16 | Three-Body Chaos | Live absolute orbit with axes, clamp circle, trails and the three bodies at the playhead |
| 17 | Pebble Box | Animated pebble box; eight lane pebbles at their recorded per-step positions |
| 18 | Xenakis Sieve | Residue-class lattice rows plus the generator's step mask and its onsets |
| 19 | Kuramoto Lanes | Eight phase pointers, their trails and the mean phase vector on one circle |

Indices 8–19 all draw their own panel. There is no shared step-lattice fallback: a mode whose
structure is not a grid shows that structure instead. Each panel reads the generator's own
formula or published state through an exported helper; if a panel's constants cannot be traced
to a `Pattern` field or a generator helper, the panel is drawing a decoration, not the algorithm.

Three-Body Chaos is not a reference-project mode; it follows the same chaos family as the
existing Logistic Map but with genuinely different controls (`phase` widens the third orbit,
`osc_rate` sets integration resolution) and a different panel. Because it is a live simulation
rather than a closed form, it re-rolls the orbit at every cycle boundary: `regenerate_every`
widens the interval but its `0 = off` value means "every cycle" in this mode, so the panel and
the audio keep changing instead of freezing on one orbit.

When adding or reordering a mode, update together: the `Mode` enum in `src/modes.rs`, dispatch in `sequence::generate()`, `view::build()`, the `Mode` labels in Slint, the mode indices the UI hardcodes, the editor's `from_index()` bound, and the expected parameter/view tests.

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
| Three-Body | Target onsets, Onset density, Seed, Orbit phase, Solver detail |
| Pebble Box | Shaker impulses, Impact sensitivity, Seed, Gravity tilt, Substeps, Gravity |
| Sieve | Sieve terms (0–2), Sieve density, Seed, Residue shift, Sieve moduli, Symmetry |
| Kuramoto | Target onsets, Onset density, Seed, Detune spread, Coupling |

Mode-scoped reuse is deliberate: `ca_rule` is the cellular rule, Pebble Box's gravity and the
Sieve moduli; `symmetry` selects Crystallographic cells and Sieve residues; `osc_rate` is
oscillator cycles, Three-Body solver detail, Pebble Box substeps or Kuramoto coupling. Seed stays
visible in every mode because the shared accent humanizer reads it even where the hit pattern
does not. A control that the selected mode never reads is hidden, not left inert. `ca_rule` maps
to gravity as `pebble::gravity()` (`0.01 + ca_rule/255 × 2.15` box-heights per step²).


## Shared interaction patterns retained from the reference

- One shared pattern snapshot drives both audio output and preview.
- A Generate switch is host-mappable and changes the persistent variation counter when it flips, including on→off and off→on.
- A stable Rotate knob post-transforms all generated lanes.
- UI mode views are generated as pure geometry and rendered via Slint paths/nodes.
- All user-facing generator parameters are host-automatable. Lane enable, note and velocity parameters are also exposed even when the first UI pass only shows lane enables.
