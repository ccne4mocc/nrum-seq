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
                                   fixed-size Pattern [8 x 1024], 104464 bytes (64-bit)
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

DrumParams + host tempo ---> editor-side MIDI export
                                      |
                                      v
                            midi_export::smf() -> .mid
```

### Why this split

The reference `_AlgoNemLody` project has a strong seam between pure generation (`src/pattern.rs`), pure visualization geometry (`src/view.rs`), and Slint host glue (`src/editor.rs`). nrum seq preserves that seam and replaces pitch-oriented output with a multi-lane drum event matrix.

`sequence::Pattern` is a fixed 104464-byte data structure on 64-bit targets: eight hit arrays at up to 1024 steps, eight accent arrays, and the mode visualization state (CA history, logistic orbit, symbolic word, normalized pebble-box frames, and the Kuramoto phase bank). Algorithms are deterministic: parameter snapshots contain seed and a persisted variation counter; no entropy source is consulted during playback. Rotation and mirror are shared post-transforms and apply to every per-step array, including `orbit`, `theta`, `u`, the symbolic word, the CA history, the phase bank and the current step, so the playhead and every panel stay aligned with what is drawn. Rotation happens before mirror; a view that redraws generator geometry maps a display slot back through `view::src_index` (undo mirror, then rotation) instead of restating the formula.

## Audio/MIDI thread

`NrumSeq::process()` consumes upstream note events (this is a generator effect, not a MIDI thru). It reads host transport, tempo and position; if stopped, it can optionally use the free-run beat cursor. For each step boundary inside the current buffer it emits MIDI events with a sample offset derived from the host beat timeline. NoteOff is emitted before retriggering a lane, on expiry, during stop, and after a detected transport jump. Per-lane note numbers and velocities come from host parameters. The generator has no audio outputs.

The audio callback remains a bounded MIDI-event scheduler. It publishes current-step and auto-randomization requests through lock-free atomics; the editor applies requested randomized values through nice-plug's `ParamSetter`. Avoid moving view strings, `Vec` construction, Slint callbacks, or blocking work into the callback. The generator uses fixed-size patterns; the current GUI allocates model vectors on the editor/update side, which is acceptable.

Four runtime LFOs are evaluated from a host-beat clock. Hz LFOs (0.01–10 Hz) use `beat × 60 / BPM`; beat-synced LFOs use `beat / rate`. Swing is sampled at the nominal step beat; gate and velocity at the resolved onset. Pattern-shaping targets (Steps, Pulses, Rotate, Density, Seed, Osc phase/rate, CA rule, Probability, Symmetry, Max steps) are sampled once per loop through `gen_params_at()`, so the line changes at a loop boundary instead of every step. The unmodulated `gen_params()` snapshot drives change detection, keeping manual edits immediate. Modulation never writes a host parameter.

nrum seq emits MIDI from `process()` and declares note output. Settings can also export one loop of the current pattern as a Standard MIDI File; that path runs only on the editor thread. Live recording into a host clip remains host-dependent: hosts that support capturing MIDI from instrument/effect tracks can do it, while hosts that only record MIDI arriving on the track input cannot. Route the track to a drum instrument and use the host's normal MIDI-recording/printing workflow when you need more than one exported loop.

## MIDI export

`src/midi_export.rs` writes a format-0 Standard MIDI File at 480 PPQ on zero-based channel 9 (MIDI channel 10). The editor snapshots `DrumParams` through the same `gen_params()`/`generate()` path as the preview, then flattens one complete loop: enabled lanes only, lane notes and velocities, accent, gate and swing. Tempo comes from the last host tempo stored in `Shared::tempo_bits`, falling back to 120 BPM when the host has not reported one. Note-offs sort before note-ons at the same tick, so a retrigger cannot cut the incoming note.

The writer allocates a `Vec` for the track and event list; it is editor-only and never called from `process()`. The export folder is typed in Settings, stored in user-global settings, created if missing, and opened with `xdg-open`/`open`/`explorer` after a successful write. Because CLAP/VST3 expose no project-path API, the first-run default is the host process working directory.

## Parameters

| Parameter group | Parameters | Intended use |
|---|---|---|
| Pattern | `mode`, `steps` (1–1024), `max_steps` (64–1024), `round_steps`, `pulses`, `rotate`, `density`, `probability`, `velocity`, `regenerate_every`, `seed`, `generate`, `theme` | Pattern family, variation, and palette controls. `steps` is bounded by `max_steps` at generation time; the default ceiling is 1024, and changing `max_steps` takes effect immediately because the `Pattern` arrays always hold 1024 steps. Regeneration counts complete pattern lengths and is evaluated at step boundaries; zero disables it |
| Randomization | `randomize_after_lengths` (0–256), `randomize_scope` (0–255) | Manual Randomize changes generator parameters immediately; automatic randomization queues a request every N complete pattern lengths while the custom editor is open, and zero disables it. `randomize_scope` is an eight-bit mask (Mode, Length, Rhythm, Tone, Clock, Phase, Acid, Feel); disabled groups keep their current values and randomize never touches swing |
| Algorithm shape | `phase`, `osc_rate`, `osc_wave` (8 waveforms), `ca_rule`, `symmetry` | Mode-specific controls, shown in the GUI only when relevant to the selected mode; waveform ids and names come from the `OscWave` enum so new shapes append without renumbering host state |
| Per-lane mode | `lane1_mode` … `lane8_mode` | 0 follows `mode`, otherwise 1 + the `Mode` index. Distinct modes are dispatched once per regeneration and only their assigned lanes are copied; the global mode still supplies the panel view. |
| Clock | `rate`, `gate`, `swing`, `freerun` | Beat grid, NoteOff duration, stopped-transport behavior |
| LFO | `lfoN_enabled`, `lfoN_wave`, `lfoN_rate_mode`, `lfoN_freq`, `lfoN_rate`, `lfoN_depth`, `lfoN_phase`, `lfoN_polarity` for N = 1–4; persisted `lfoN_links` masks | Four runtime LFOs. Hz (0.01–10) is the default speed, with beat sync as the alternative. Bipolar spans ±depth; unipolar spans 0..depth. Links are drag/drop targets stored as one 16-bit mask per LFO. Feel targets are sampled per note; generator targets at loop boundaries. Runtime modulation is not written back to host parameters and is not part of MIDI export |
| Lane state | `<lane>_on` | Host-automatable lane enable switches |
| Lane MIDI | `<lane>_note` | Configurable output note 0–127 |
| Lane velocity | `<lane>_velocity` | Lane base velocity multiplied by pattern accent |

## Where state lives

Two independent mechanisms, deliberately:

- **Project state** is owned by the host. `GuiContext::get_state`/`set_state` serialise every parameter plus any `#[persist]` field (`variation`, `lfo1_links`…`lfo4_links`, and `edit_overlay`), so mode, pattern settings, lane routing, LFO links and manual step edits all travel with the DAW project. `edit_overlay` is stored as a sparse list of edited cells; see `EDIT_OVERLAY.md`.
- **User-global settings** live in `src/globals.rs` and are written to `$XDG_CONFIG_HOME/nrum-seq/settings.json`, falling back to `$HOME/.config/nrum-seq/settings.json` when `XDG_CONFIG_HOME` is unset or empty (`%APPDATA%` on Windows). If neither a config home nor `HOME` is resolvable — a host launched from a bare systemd unit — settings do not persist rather than the editor failing. Only preferences that should follow the user across projects belong here: the colour theme, the editor scale and the MIDI export folder. Loaded at editor open and applied over the host's state, so a project cannot pin a palette; written on a 750 ms debounce and forced once when the editor closes.

The consequence of theme being global is that host automation of `theme` will not survive an editor reopen. That is an accepted trade-off for a cosmetic setting.

The default palette is **Neon**; fresh projects and fresh global settings open there. **Sketch** is still available and keeps the host id `aurora` and index 0, so projects and automation that stored the old Aurora choice open in Sketch. Palette values come from `Theme::colors()` and are pushed into named Slint tokens (`surface`, `control`, `text`, `border`, `cell-off`, `on-accent`, and the accents); new UI colours should use those tokens instead of hardcoded hex values so every palette stays legible. A persisted global theme or the project's `theme` parameter overrides the fresh default.

`MAX_STEPS` is 1024 and `max_steps` now defaults to 1024 as well, so a fresh project pays the full 1024-step ceiling cost. The user can lower the ceiling to 64 for the historical cost. Generation cost stays linear in the ceiling, and the modes that select a top-k subset rank through a fixed in-place array instead of an O(n²) scan. The ignored `bench_generation_at_max_steps` test measures the eight-lane worst case at 1024; run it before changing the cap.

`swing` (0–0.5) delays alternate steps by that fraction of a step in the beat domain, so 1/3 gives triplet feel. Because a swung step can fall past the block end, the scheduler keeps a persistent `next_step` cursor instead of re-deriving the step index from the host position each block; re-deriving silently dropped every deferred odd step. The cursor resets on stop, on `reset()`, and on a detected transport jump. NoteOff is derived from the swung onset, so gate length follows the shift.

`steps` is a float so the loop length can fall between grid cells. `round_steps` (on by default) snaps it to whole numbers; turning it off lets the loop wrap mid-cell, so each cycle starts a fraction of a step later and the pattern drifts against the step grid. `GenParams.steps` is always the whole-number cell count the generators fill, while `GenParams.length` carries the exact loop length; `cell_count()` in `src/lib.rs` is the single place that converts one to the other, and both the scheduler and the UI clamp through it.

`pulses` is mode-dependent: target count scaled by lane density for Euclidean, graph degree selector for Eulerian, generation index (0–15) for cellular automata, and target count/density control in distance and oscillator modes. Toussaint-distance and well-formed rhythms use integer circular boundary crossings, giving exactly `round(effective_pulses × density)` pre-probability hits per lane for any supported length; the well-formed mode distributes circular gaps that differ by at most one step. The L-system mode maps bounded parallel symbol rewriting into seeded hit decisions; Phase Shift repeats one maximally-even motif at circular lane offsets; Polyrhythm scales pulse counts by lane-specific integer ratios and distributes them with boundary crossings; Weighted Syncopation selects a density-scaled hit count from metrical/offbeat scores; Pebble Box uses `pulses` as the shaker-impulse count, `phase` as the gravity tilt, `osc_rate` as the physics substep count, `ca_rule` as gravity strength (`pebble::gravity()`) and `density` as the impact threshold. Xenakis Sieve uses `ca_rule` for the moduli, `symmetry`/`phase` for the residues, `pulses` for the number of union terms and `density` for how many members survive; Kuramoto Lanes uses `osc_rate` as coupling, `phase` as frequency spread, `pulses` as the target onset count and `density` as the per-lane weight. Document future mode-specific semantics when changing this mapping.

## UI contracts

- `ViewNode` is a panel coordinate: coordinates and radius are in a 210×210 viewbox. The Slint panel renders that space at 300px: the paths scale through the viewbox, and the node rectangles multiply position, radius and font size by the same `px-per-unit` factor. Keep new geometry in the 210-unit space; do not pre-scale coordinates.
- The editor layout is fixed at 860×740 (`base-width`/`base-height`) and the window never resizes. The pattern matrix gives each step a fixed 14px cell and scrolls horizontally as the pattern grows; lane labels stay in a fixed left column. Do not shrink cells to fit or add `transform-scale`, a responsive breakpoint, or a size setting to the tree.
- Lane independence: when every lane derives from one shared curve, score or orbit, give each lane its own phase origin (as Euclidean, well-formed, raised-cosine and logistic do). Otherwise the highest-ranked or highest-amplitude steps coincide and all eight drums fire on one step in every variation.
- `StepCell` is one lane/step entry with hit state, playhead state, effective velocity, effective probability, hit override, edited flag, ratchet count and microtiming offset. The editor only publishes cells for current `steps`. Hit overrides are display-space and survive rotation, mirror and Generate; a forced-on hit bypasses probability and a forced-off hit always wins. Probability and accent overrides replace the corresponding value for that cell.
- Matrix edit modes are `HIT`, `PROB`, `ACC`, `CLR`, `RAT` and `TIME`; editing is disabled at mode 0 so normal scrolling is unaffected. `CLR ALL` clears the overlay, and undo/redo is editor-local and capped at 64 snapshots.
- Ratchets use at most 12 subdivisions per step so counts 1–4 divide evenly. A ratchet gate is capped to its own substep spacing, so one lane never needs polyphony. Microtiming is currently step-wide and positive-only in the scheduler; the editor writes the same offset to all eight lane cells.
- The LFO rack is a fixed 2×2 grid below the step matrix. Each `LfoCell` has a drag point, clickable enable name, waveform picker with a phase dot, Hz/BT chip, speed/depth/phase strips and a bipolar/unipolar chip.
- Slint 1.16 keeps `DragArea`/`DropArea` out of the public element set. Modulation links use a `TouchArea` on the LFO point, the `ModDrag` global and `ModRegistry` target rectangles; the editor hit-tests the newest visible rectangle on release.
- `ModView` publishes each target's base/effective value, depth band and hover summary. `OptSlider`/`NumBox` draw a low-opacity depth band plus an animated base→value bar. `ModHover` drives one tooltip below the hovered control; it has no TouchArea, so the parameter stays editable.
- The modulation matrix slides up from below, updates its `ModRow` model in place and removes a link with the row × button.
- `Mode` order is stable: 0 Euclidean, 1 Eulerian, 2 Phyllotaxis, 3 Cellular, 4 Oscillator, 5 Crystallographic, 6 Logistic, 7 Markov, 8 Raised-Cos Threshold, 9 Slowcat Rhythm, 10 Toussaint Distance, 11 Well-Formed Rhythm, 12 L-System Grammar, 13 Phase Shift / Reich, 14 Polyrhythm Stack, 15 Weighted Syncopation, 16 Pebble Box, 17 Xenakis Sieve, 18 Kuramoto Lanes, 19 Euclidean-Swing, 20 Bernoulli, 21 African Bell, 22 Phase Canon, 23 Reaction-Diffusion, 24 Lorenz, 25 Cantor, 26 Fibonacci, 27 Genetic, 28 Arnold Circle Map, 29 Sturmian Word, 30 Hilbert Curve, 31 Ulam Spiral Primes, 32 Vuza Canon, 33 Thue-Morse, 34 Rudin-Shapiro, 35 Brownian Walk, 36 Levy Flight, 37 Tent Map, 38 Henon Map. It lives in `src/modes.rs` and is the single list: the host parameter, the generators and the panels all use the same enum, and a test pins each index the UI hardcodes to the mode it means. New modes append; existing indices never move. `Mode::GROUPS` is the one presentation order shared by the dropdown and the header wheel. Its labels and order are shared with nelody seq: Rhythm, Stochastic, Chaos / Physics, Fractal, Cellular, Geometric, Number Theory, Music Theory and Acid (nrum seq omits the empty Acid group).
- The header shows the DAW track name when the host reports one. It is captured on the host main thread (`initialize`, `track_context_changed`) into `Shared::track_name`; the editor never calls `GuiContext::track_name()` itself because the CLAP wrapper forwards that to the host on the calling thread, and the editor tick runs on the window thread.
- Host-stolen keys (VST3 only; the CLAP wrapper has no hook): `Editor::on_virtual_key_from_host` claims Backspace, Delete, arrows, Home/End, Return and Space only while a Slint text field has focus (`text-focused` in `app.slint`, published each frame as `Shared::text_focus`), queues them, and the editor tick replays them into Slint on the window thread. Any new text field must be added to `text-focused` or hosts will keep stealing its keys. Unverified in a real host.
- All UI controls call a parameter setter; the GUI must not become an independent source of sequencing state.
- Each mode draws a panel specific to its own algorithm. Where a panel needs the generator's arithmetic — the raised-cosine envelope and cutoff, the oscillator rank cutoff (read from the published `orbit` and `hit`), the modular distance, the Slowcat motif length, the syncopation score, the cellular generation selection, the pebble frames, the sieve classes, the Kuramoto phases — it reads that value from `Pattern` or from a helper exported by the generator module. A panel must never re-derive a generator's formula: two copies of one formula is how the previous oscillator cutoff line drifted away from the threshold the generator actually applied. Phyllotaxis and L-System export their phase/threshold/word helpers for exactly this reason. Sieve exports its residue classes and per-step membership; Kuramoto publishes its per-step phase bank and draws pointers, trails and the order-parameter vector from it. The word modes draw the published `lsym`/`orbit`; the walk, tent and Hénon panels draw `orbit`/`theta`/`u`, and read the map constants through `fibonacci_alpha`, `brownian_sigma`, `levy_alpha`/`levy_step_min`, `tent_mu`, `henon_a` and `arnold_coupling`. Phyllotaxis samples its rosette into at most 128 angular groups and aggregates the lanes there, so a 1024-step pattern stays a bounded drawing rather than 16k rectangles.
- Waveform names are not duplicated in Slint: `editor.rs` pushes `OscWave::variants()` into `wave-names`, the 2×4 picker grid renders that model, and `help.rs` reads `OscWave::ids()`. Adding a shape means appending an enum variant plus one shape function in `oscillator.rs`; the picker, help line and `WAVE_COUNT` test follow.
- The eight RANDOMIZE scope chips mirror the bits of `randomize_scope` (`lib.rs::scope`): Mode, Length, Rhythm, Tone, Clock, Phase, Acid, Feel. The chips are views of the host parameter, so automating the mask updates them and turning a group off preserves its current values.
- Credits and CLAP/VST3 vendor contact read the same `contact::*` constants, so the Settings panel cannot disagree with the plug-in metadata.
- `Pattern.pebbles` stores normalized 0–255 `[x, y]` pairs per lane per step, not panel units; `view::build` maps them into the box. `Pattern.impact` stores which wall (left/right/top/bottom) the hit struck, because a frame records the end of the step after the bounce; the panel snaps hit nodes and flashes that wall. The Slint panel tweens node `x`/`y` for 120 ms, so Pebble Box pushes its eight nodes first and in lane order; other panels' nodes never move, so the tween is invisible there.
- The panel has three path slots (`edges`, `lines`, `hand`) and single-colour nodes, so a drawing that needs per-lane colour would require changing `ViewNode` and the Slint panel. Views stay within the existing slots until that is worth doing.

## Extension points

The per-step edit overlay covers probability, accent, ratchets and microtiming without growing `Pattern`. Future per-lane microtiming needs a scheduler with lookahead for early events; per-lane lengths still need a pattern-contract change rather than a scheduler tweak.
