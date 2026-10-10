# AlgoDrumNemShine

**Algorithmic percussion MIDI sequencer** in Rust, using `nih-plug` for CLAP/VST3 and Slint + `slint-baseview` for the embedded GUI. It is a MIDI generator effect: put it before a drum sampler or drum rack on a MIDI/instrument track. It intentionally does not synthesize audio or load samples in this starter version.

## What is implemented

- Eight MIDI lanes: Kick, Snare, Closed Hat, Open Hat, Clap, Rim, Low Tom, and Percussion.
- Sixteen deterministic generator modes with mode-specific visualization and mode-sensitive parameters.
- Additional generators: Raised-Cos Threshold, Slowcat Rhythm, Toussaint Distance, Well-Formed Rhythm, L-System Grammar, Phase Shift / Reich, Polyrhythm Stack, and Weighted Syncopation.
- Global hit probability and velocity controls, automatic regeneration after N complete pattern lengths (0 disables it), manual parameter randomization, and optional auto-randomization every N host-transport beats.
- Host-automatable lane enables, note numbers, velocities, Gate, Swing, rate, mode, Generate, and Free Run.
- Swing delays alternate steps in the beat domain, from straight up to a half-step shuffle; a third of a step gives triplet feel.
- The mouse wheel edits whatever it is hovering: mode, rate, waveform, theme, every parameter slider, and the spin boxes in Settings. The wheel steps the mode list rather than scrolling it, and the list follows the selection.
- Settings are shown only while in use; the editor window is sized for the worst case so nothing jumps when they appear.
- Theme and editor scale are remembered per user in `$XDG_CONFIG_HOME/AlgoDrumNemShine/settings.json` (`%APPDATA%` on Windows) and follow you across projects. Everything else travels with the DAW project.
- Default lane notes follow a GM-style mapping: Kick 36, Snare 38, Closed Hat 42, Open Hat 46, Clap 39, Rim 37, Low Tom 45, Percussion 50. MIDI is sent on zero-based channel 9 (MIDI channel 10).
- Fixed-size patterns support 1–64 steps. The generation path uses bounded work, and the audio callback uses fixed-size patterns without allocating or locking.
- Settings can snap the pattern length to whole steps or leave it fractional. With snapping off, a length like 16.5 wraps mid-cell, so the pattern drifts against the step grid instead of repeating in phase. Exact values can be typed into the spin boxes rather than dragged, which matters because a 64-value range does not fit in the slider's pixel width.
- Settings include Aurora (reference), Neon, Lava, and Forest themes. Generator parameters, rate, waveform, and theme selectors support mouse-wheel stepping while hovered.

## Generation modes and views

| Mode | Core method | Main view |
|---|---|---|
| Euclidean | Evenly distribute `pulses` over `steps`, per-lane density profile | Radial clock |
| Eulerian Walk | Hierholzer traversal of a balanced directed graph; rest is vertex 0, lanes are vertices 1–8 | Directed drum-state network |
| Golden Ratio / Phyllotaxis | Golden-angle / irrational fractional phase tests, with density and pulse ratio | Golden-angle spiral |
| Cellular Automaton | Ring-based elementary cellular automaton (Wolfram `ca_rule`); `pulses` selects a generation | Space-time cell grid |
| Oscillator / Phase | Sine, triangle, saw, or square oscillator sampled at each step; explicit phase and cycles | Waveform with trigger threshold |
| Crystallographic | Periodic motif transformed by translation, mirror, glide, inversion and centering-like symmetry operations | 8-lane lattice and symmetry axes |
| Logistic Map | Logistic orbit `x[n+1] = r*x[n]*(1-x[n])`; thresholded orbit peaks | Orbit diagram |
| Markov Walk | Seeded first-order transition walk through rest and drum states | State-transition graph |
| Raised-Cos Threshold | Smooth raised-cosine envelope threshold | Step lattice |
| Slowcat Rhythm | Slow phrase concatenation with evolving lane motifs | Step lattice |
| Toussaint Distance | Seeded circular offset, integer boundary-crossing onset word, and modular-distance accents | Step lattice |
| Well-Formed Rhythm | Christoffel/Bresenham word with two interval lengths | Step lattice |
| L-System Grammar | Bounded Fibonacci-style parallel symbol rewriting interpreted as lane and accent roles | Step lattice |
| Phase Shift / Reich | Common maximally-even motif circularly offset per lane | Step lattice |
| Polyrhythm Stack | Rational pulse ratios across lanes over a shared duration | Step lattice |
| Weighted Syncopation | Metric/offbeat scoring with deterministic top-k hit selection | Step lattice |

The crystallographic mode is an **algorithmic symmetry palette inspired by periodic/crystallographic operations**, not a claim to enumerate all formal 2D/3D crystallographic space groups.

## Build

The Slint UI uses a hierarchical `TrackRow` model: each lane carries its enabled state and its own step-cell model. This avoids dynamic array element access inside repeaters and keeps lane labels, enable toggles, and grid cells aligned.

Requirements: Rust toolchain compatible with the reference project (Rust 1.88 or newer is a good baseline), a C/C++ build toolchain, and the system graphics/window development libraries required by Slint/Baseview on your platform. The exact Linux package names depend on the distribution.

```sh
cargo check --lib
cargo test --lib
cargo xtask bundle algodrumnemshine --release
```

The bundle task places distributable artifacts under `target/bundled/` when the selected platform/backend is supported. For a quick compile-only check use `cargo check --lib` first. `cargo check --all-targets` and `cargo test` pass, but **the plug-in has never been loaded into a DAW** — treat host behaviour as unverified until it has been.

## Reference project

The visual and host integration patterns were adapted from the supplied `_AlgoNemLody` project:

- `src/pattern.rs` → pure deterministic pattern generation (`src/sequence/`).
- `src/view.rs` → pure mode-specific geometry (`src/view.rs`).
- `src/editor.rs` → Slint `Editor` adapter and host parameter setters (`src/editor.rs`).
- `ui/app.slint` → mode-specific panel plus step matrix.
- `Plugin::process()` in `src/lib.rs` → transport-aware, sample-timed MIDI event scheduling.

The existing melody/tuning-specific material (pitch contours, scales, tuning/EDO, Tonnetz triads, piano-roll history) was intentionally not carried into the percussion model. See [`docs/REFERENCE_MAPPING.md`](docs/REFERENCE_MAPPING.md) for the exact reference-to-new-view correspondence and [`docs/WISHLIST.md`](docs/WISHLIST.md) for an unordered list of ideas someone liked.

## Project layout

```text
src/
  lib.rs                  nih-plug parameters, transport scheduler, CLAP/VST3 exports
  editor.rs               Slint adapter; host parameter callbacks; UI model updates
  view.rs                 pure geometry for the sixteen generation-mode panels
  sequence/
    mod.rs                immutable generator params, fixed-size Pattern, dispatch, PRNG, invariants
    euclidean.rs          Euclidean rhythms
    eulerian.rs           Eulerian circuit on a balanced directed graph
    phyllotaxis.rs        golden ratio / phyllotaxis
    cellular.rs           elementary cellular automaton
    oscillator.rs         waveform and phase thresholding
    crystallographic.rs   periodic symmetry transforms
    logistic.rs           logistic-map orbit
    markov.rs             Markov chain state walk
    lsystem.rs            finite L-system rhythm grammar
    phase_shift.rs        interlocking circular phase offsets
    polyrhythm.rs         rational pulse-ratio stack
    syncopation.rs        weighted metric/offbeat selection
ui/app.slint             Slint interface and per-lane step matrix
docs/                     reference map, architecture, wishlist, agent ownership
```

## Current limitations / next milestones

This is a functional starter architecture, not a release-certified plugin. The next steps are to build and smoke-test it in at least one CLAP host and one VST3 host; validate tempo/loop/transport edge cases; verify Slint layout on Linux/macOS/Windows; add swing, ratchets, probability per hit, ghost notes and presets; and then add user-facing MIDI export or optional internal sound generation only if that becomes a requirement.


## Additional rhythm algorithms

The four researched additions are alongside Euclidean, Eulerian, Phyllotaxis, Cellular Automaton, Oscillator, Crystallographic, Logistic Map, Markov Walk, Raised-Cos Threshold, Slowcat Rhythm, Toussaint Distance, and Well-Formed Rhythm:

- **L-System Grammar** — bounded Fibonacci-style symbol rewriting mapped to drum accents and lane roles.
- **Phase Shift / Reich** — the same maximally-even pulse train is circularly offset across lanes to form interlocking phases.
- **Polyrhythm Stack** — simultaneous integer-ratio pulse trains (1:1, 3:2, 2:1, 5:2, 4:3, 2:3, 5:4, 3:4) over a shared pattern duration.
- **Weighted Syncopation** — deterministic weighted selection emphasizes offbeats and weak subdivisions while preserving a density-scaled number of hits.

Research rationale and source links are in [`docs/ALGORITHM_RESEARCH.md`](docs/ALGORITHM_RESEARCH.md).

See [`docs/ALGORITHM_RESEARCH.md`](docs/ALGORITHM_RESEARCH.md) for the research rationale and citations.
