# Architecture

## Runtime layers

```text
Host transport + automation
          |
          v
DrumParams (host-visible, automatable) ---> GenParams snapshot
                                                |
                                                v
                                      sequence::generate()
                                                |
                                   fixed-size Pattern [8 x 64], 7568 bytes (64-bit)
                                                |
                                                v
                                        MIDI scheduler
                                   beat grid -> sample offsets
                                                |
                                                v
                                    MIDI NoteOn / NoteOff
                                   channel 10 -> drum rack

DrumParams + same GenParams snapshot ---> editor-side preview
                                                |
                                                v
                                      view::build() geometry
                                                |
                                                v
                                  Slint AppWindow + step matrix
```

### Why this split

The reference `_AlgoNemLody` project has a strong seam between pure generation (`src/pattern.rs`), pure visualization geometry (`src/view.rs`), and Slint host glue (`src/editor.rs`). AlgoDrumNemShine preserves that seam and replaces pitch-oriented output with a multi-lane drum event matrix.

`sequence::Pattern` is a fixed 7568-byte data structure on 64-bit targets: eight hit arrays at up to 64 steps, eight accent arrays, and the mode visualization state (CA history, logistic orbit, symbolic word, absolute three-body positions, normalized pebble-box frames, and the Kuramoto phase bank). Algorithms are deterministic: parameter snapshots contain seed and a persisted variation counter; no entropy source is consulted during playback. Rotation is a shared post-transform and rotates every per-step view array, including the orbit trace, the symbolic word, the phase bank and the current step, so the playhead always matches what is drawn.

## Audio/MIDI thread

`AlgoDrumNemShine::process()` consumes upstream note events (this is a generator effect, not a MIDI thru). It reads host transport, tempo and position; if stopped, it can optionally use the free-run beat cursor. For each step boundary inside the current buffer it emits MIDI events with a sample offset derived from the host beat timeline. NoteOff is emitted before retriggering a lane, on expiry, during stop, and after a detected transport jump. Per-lane note numbers and velocities come from host parameters. The generator has no audio outputs.

The audio callback remains a bounded MIDI-event scheduler. It publishes current-step and auto-randomization requests through lock-free atomics; the editor applies requested randomized values through NIH-plug's `ParamSetter`. Avoid moving view strings, `Vec` construction, Slint callbacks, or blocking work into the callback. The generator uses fixed-size patterns; the current GUI allocates model vectors on the editor/update side, which is acceptable.

## Parameters

| Parameter group | Parameters | Intended use |
|---|---|---|
| Pattern | `mode`, `steps` (1–64), `round_steps`, `pulses`, `rotate`, `density`, `probability`, `velocity`, `regenerate_every`, `seed`, `generate`, `theme` | Pattern family, variation, and palette controls; regeneration counts complete pattern lengths and is evaluated at step boundaries; zero disables it, except in Three-Body Chaos, where the orbit is a live simulation and re-rolls at least once per cycle |
| Randomization | `auto_randomize`, `randomize_after_beats` (1–256), `randomize_scope` (0–31) | Manual Randomize changes generator parameters immediately; Auto Randomize queues a randomization at every N-beat transport boundary while the custom editor is open. `randomize_scope` is a five-bit mask (Mode, Length, Rhythm, Tone, Feel); disabled groups keep their current values and randomize never touches swing |
| Algorithm shape | `phase`, `osc_rate`, `osc_wave` (8 waveforms), `ca_rule`, `symmetry` | Mode-specific controls, shown in the GUI only when relevant to the selected mode; waveform ids and names come from the `OscWave` enum so new shapes append without renumbering host state |
| Clock | `rate`, `gate`, `swing`, `freerun` | Beat grid, NoteOff duration, stopped-transport behavior |
| Lane state | `<lane>_on` | Host-automatable lane enable switches |
| Lane MIDI | `<lane>_note` | Configurable output note 0–127 |
| Lane velocity | `<lane>_velocity` | Lane base velocity multiplied by pattern accent |

## Where state lives

Two independent mechanisms, deliberately:

- **Project state** is owned by the host. `GuiContext::get_state`/`set_state` serialise every parameter plus any `#[persist]` field (`variation` today), so mode, pattern settings and lane routing all travel with the DAW project. Nothing to implement.
- **User-global settings** live in `src/globals.rs` and are written to `$XDG_CONFIG_HOME/AlgoDrumNemShine/settings.json`, falling back to `$HOME/.config/AlgoDrumNemShine/settings.json` when `XDG_CONFIG_HOME` is unset or empty (`%APPDATA%` on Windows). If neither a config home nor `HOME` is resolvable — a host launched from a bare systemd unit — settings do not persist rather than the editor failing. Only preferences that should follow the user across projects belong here: the colour theme and the editor scale. Loaded at editor open and applied over the host's state, so a project cannot pin a palette; written on a 750 ms debounce and forced once when the editor closes.

The consequence of theme being global is that host automation of `theme` will not survive an editor reopen. That is an accepted trade-off for a cosmetic setting.

`MAX_STEPS` is 64. At 256 steps the eight-distinct-mode worst case measured 587 µs, which is 81–88% of a 32-sample block; at 64 the worst single mode is Three-Body Chaos at about 190 µs (26% of a 32-sample block), the oscillator and Kuramoto banks sit around 25–35 µs, and the rest stay under 25 µs. Generation cost scales linearly with the cap, so raising it is a tuning decision rather than a free change.

`swing` (0–0.5) delays alternate steps by that fraction of a step in the beat domain, so 1/3 gives triplet feel. Because a swung step can fall past the block end, the scheduler keeps a persistent `next_step` cursor instead of re-deriving the step index from the host position each block; re-deriving silently dropped every deferred odd step. The cursor resets on stop, on `reset()`, and on a detected transport jump. NoteOff is derived from the swung onset, so gate length follows the shift.

`steps` is a float so the loop length can fall between grid cells. `round_steps` (on by default) snaps it to whole numbers; turning it off lets the loop wrap mid-cell, so each cycle starts a fraction of a step later and the pattern drifts against the step grid. `GenParams.steps` is always the whole-number cell count the generators fill, while `GenParams.length` carries the exact loop length; `cell_count()` in `src/lib.rs` is the single place that converts one to the other, and both the scheduler and the UI clamp through it.

`pulses` is mode-dependent: target count scaled by lane density for Euclidean, graph degree selector for Eulerian, generation index (0–15) for cellular automata, and target count/density control in distance and oscillator modes. Toussaint-distance and well-formed rhythms use integer circular boundary crossings, giving exactly `round(effective_pulses × density)` pre-probability hits per lane for any supported length; the well-formed mode distributes circular gaps that differ by at most one step. The L-system mode maps bounded parallel symbol rewriting into seeded hit decisions; Phase Shift repeats one maximally-even motif at circular lane offsets; Polyrhythm scales pulse counts by lane-specific integer ratios and distributes them with boundary crossings; Weighted Syncopation selects a density-scaled hit count from metrical/offbeat scores; Pebble Box uses `pulses` as the shaker-impulse count, `phase` as the gravity tilt, `osc_rate` as the physics substep count, `ca_rule` as gravity strength (`pebble::gravity()`) and `density` as the impact threshold. Xenakis Sieve uses `ca_rule` for the moduli, `symmetry`/`phase` for the residues, `pulses` for the number of union terms and `density` for how many members survive; Kuramoto Lanes uses `osc_rate` as coupling, `phase` as frequency spread, `pulses` as the target onset count and `density` as the per-lane weight. Document future mode-specific semantics when changing this mapping.

## UI contracts

- `ViewNode` is a panel coordinate: coordinates and radius are in a 210×210 viewbox. The Slint panel renders that space at 300px: the paths scale through the viewbox, and the node rectangles multiply position, radius and font size by the same `px-per-unit` factor. Keep new geometry in the 210-unit space; do not pre-scale coordinates.
- The editor layout is fixed at 860×760 (`base-width`/`base-height`) and the window never resizes. The pattern matrix fills the width at a fixed cell size; do not add `transform-scale`, a responsive breakpoint, or a size setting to the tree.
- Lane independence: when every lane derives from one shared curve, score or orbit, give each lane its own phase origin (as Euclidean, well-formed, raised-cosine, logistic and three-body do). Otherwise the highest-ranked or highest-amplitude steps coincide and all eight drums fire on one step in every variation.
- `StepCell` is one lane/step entry with hit state, playhead state and effective velocity for brightness and hit probability for opacity. The editor only publishes cells for current `steps`.
- `Mode` order is stable: 0 Euclidean, 1 Eulerian, 2 Phyllotaxis, 3 Cellular, 4 Oscillator, 5 Crystallographic, 6 Logistic, 7 Markov, 8 Raised-Cos Threshold, 9 Slowcat Rhythm, 10 Toussaint Distance, 11 Well-Formed Rhythm, 12 L-System Grammar, 13 Phase Shift / Reich, 14 Polyrhythm Stack, 15 Weighted Syncopation, 16 Three-Body Chaos, 17 Pebble Box, 18 Xenakis Sieve, 19 Kuramoto Lanes. It lives in `src/modes.rs` and is the single list: the host parameter, the generators and the panels all use the same enum, and a test pins each index the UI hardcodes to the mode it means.
- All UI controls call a parameter setter; the GUI must not become an independent source of sequencing state.
- Each mode draws a panel specific to its own algorithm. Where a panel needs the generator's arithmetic — the raised-cosine envelope and cutoff, the oscillator threshold, the modular distance, the Slowcat motif length, the syncopation score, the cellular generation selection, the three-body trajectory, the pebble frames, the sieve classes and mask, the Kuramoto phases — it reads that value from `Pattern` or from a helper exported by the generator module. A panel must never re-derive a generator's formula: two copies of one formula is how the previous oscillator cutoff line drifted away from the threshold the generator actually applied. Phyllotaxis and L-System export their phase/threshold/word helpers for exactly this reason; Three-Body publishes absolute per-step positions and draws them in a fixed scale of `PLOT / R_MAX`, with the clamp circle as the true boundary. Sieve exports its residue classes and mask; Kuramoto publishes its per-step phase bank and draws pointers, trails and the order-parameter vector from it.
- Waveform names are not duplicated in Slint: `editor.rs` pushes `OscWave::variants()` into `wave-names`, the 2×4 picker grid renders that model, and `help.rs` reads `OscWave::ids()`. Adding a shape means appending an enum variant plus one shape function in `oscillator.rs`; the picker, help line and `WAVE_COUNT` test follow.
- The five RANDOMIZE scope chips mirror the bits of `randomize_scope` (`lib.rs::scope`): Mode, Length, Rhythm, Tone, Feel. The chips are views of the host parameter, so automating the mask updates them and turning a group off preserves its current values.
- Credits and CLAP/VST3 vendor contact read the same `contact::*` constants, so the Settings panel cannot disagree with the plug-in metadata.
- `Pattern.pebbles` stores normalized 0–255 `[x, y]` pairs per lane per step, not panel units; `view::build` maps them into the box. `Pattern.impact` stores which wall (left/right/top/bottom) the hit struck, because a frame records the end of the step after the bounce; the panel snaps hit nodes and flashes that wall. The Slint panel tweens node `x`/`y` for 120 ms, so Pebble Box pushes its eight nodes first and in lane order; other panels' nodes never move, so the tween is invisible there.
- The panel has three path slots (`edges`, `lines`, `hand`) and single-colour nodes, so a drawing that needs per-lane colour would require changing `ViewNode` and the Slint panel. Views stay within the existing slots until that is worth doing.

## Extension points

Add per-step probability, ratchets, swing and microtiming to the pattern data model only after defining how they map to sample offsets and NoteOff handling.
