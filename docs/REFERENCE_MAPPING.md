# Reference project mapping: `_AlgoNemLody` → AlgoDrumNemShine

This map records the supplied melody sequencer's existing view conventions so follow-up agents can preserve its design language instead of guessing.

## Reference modes and current counterparts

The reference is explicit in `src/lib.rs` (`Mode::kind()`), `src/pattern.rs` (`GenKind`), `src/view.rs` (`build()`), and `ui/app.slint` (`pulses-labels`, `shape-label`, `ModePanel`).

| Reference mode | Reference generator | Reference view | AlgoDrumNemShine adaptation |
|---|---|---|---|
| Euclidean | `E(pulses, steps)` | Radial clock; active hit nodes and a playhead hand | **Euclidean** — radial clock of any-hit events across drum lanes |
| Golomb Ruler | Optimal Golomb ruler marks | Pitch/step strip only (`view_kind = 6`) | Replaced by **Crystallographic** — periodic motif with symmetry-lattice view |
| Logistic Map | Top-N / thresholded logistic orbit | Pitch/step strip only (`view_kind = 6`) | **Logistic Map** — logistic orbit diagram, plus lane trigger matrix |
| Eulerian Walk | Hierholzer circuit on balanced directed graph, vertex 0 = rest | Directed graph nodes/edges and moving path | **Eulerian Walk** — balanced drum-state graph, rest and eight lane vertices |
| Arnold Circle Map | Circle-map phase iteration | Phasor plot | Replaced by **Oscillator / Phase** — waveform, phase and threshold |
| Mozart Dice | Mozart dice-table variation | Step velocity bars, with dice roll tags | Replaced by **Golden Ratio / Phyllotaxis** — golden-angle spiral |
| Tonnetz Walk | P/L/R path through triad lattice | Hexagonal triad mesh | Replaced by **Markov Walk** — transition graph over rest/drum states |
| Rule 30 | Rule 30 on a ring; pulses selects generation | Concentric CA rings (`view_kind = 5`) | **Cellular Automaton** — 1D CA space-time grid, selected row drives lane hits |

These replacements prioritize relevant drum sequencing methods. The new names and enum order are intentionally stable because the UI uses the same index order as the host enum.

## New mode → view index

The `GenKind::index()` value in `src/sequence/mod.rs`, the `View.kind` in `src/view.rs`, and the mode order in `ui/app.slint` must match:

| Index | Mode | Visualization |
|---:|---|---|
| 0 | Euclidean | Radial step clock |
| 1 | Eulerian | Directed state-transition graph |
| 2 | Golden Ratio / Phyllotaxis | Golden-angle spiral, hit states highlighted |
| 3 | Cellular Automaton | Ring-based CA history / space-time cells |
| 4 | Oscillator / Phase | Waveform, threshold and sampled step markers |
| 5 | Crystallographic | Eight-lane symmetry lattice with unit-cell axes |
| 6 | Logistic Map | Orbit diagram and orbit trace |
| 7 | Markov Walk | Directed percussion-state transition graph |

When adding or reordering a mode, update together: `Mode`, `GenKind`, `GenKind::ALL`, `GenKind::index()`, dispatch in `sequence::generate()`, `view::build()`, `Mode` labels in Slint, the editor's `from_index()` bound, and the expected parameter/view tests.

## Shared interaction patterns retained from the reference

- One shared pattern snapshot drives both audio output and preview.
- A Generate switch is host-mappable and changes the persistent variation counter when it flips, including on→off and off→on.
- A stable Rotate knob post-transforms all generated lanes.
- UI mode views are generated as pure geometry and rendered via Slint paths/nodes.
- All user-facing generator parameters are host-automatable. Lane enable, note and velocity parameters are also exposed even when the first UI pass only shows lane enables.
