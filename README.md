# AlgoDrumNemShine

**Algorithmic percussion MIDI sequencer** in Rust, using `nih-plug` for CLAP/VST3 and Slint + `slint-baseview` for the embedded GUI. It is a MIDI generator effect: put it before a drum sampler or drum rack on a MIDI/instrument track. It intentionally does not synthesize audio or load samples in this starter version.

## What is implemented

- Eight MIDI lanes: Kick, Snare, Closed Hat, Open Hat, Clap, Rim, Low Tom, and Percussion.
- Eighteen deterministic generator modes with mode-specific visualization and mode-sensitive parameters.
- Additional generators: Raised-Cos Threshold, Slowcat Rhythm, Toussaint Distance, Well-Formed Rhythm, L-System Grammar, Phase Shift / Reich, Polyrhythm Stack, Weighted Syncopation, Three-Body Chaos, and Pebble Box.
- Every mode draws a panel specific to its own algorithm: per-lane pulse rings, motif phrase timelines, modular distance circles, Christoffel slack plots, grammar nesting bars, phasing spokes, ratio rings, metrical score bars, a live three-body orbit, and an animated pebble box.
- Global hit probability and velocity controls, automatic regeneration after N complete pattern lengths (0 disables it), manual parameter randomization, and optional auto-randomization every N host-transport beats.
- Host-automatable lane enables, note numbers, velocities, Gate, Swing, rate, mode, Generate, and Free Run.
- Swing delays alternate steps in the beat domain, from straight up to a half-step shuffle; a third of a step gives triplet feel.
- The mouse wheel edits whatever it is hovering: mode, rate, waveform, theme, every parameter slider, and the spin boxes in Settings. The wheel steps the mode list rather than scrolling it, and the list follows the selection.
- Each mode shows a multi-sentence description plus a live math line with the current parameters substituted (onsets, thresholds, cell sizes, cutoffs). The numbers come from the generators' own helpers, so the text tracks what is audible.
- Settings are shown only while in use; the editor window is sized for the worst case so nothing jumps when they appear.
- The window is a fixed 860×760; the pattern matrix uses the full width and never reflows or scrolls.
- Theme and editor scale are remembered per user in `~/.config/AlgoDrumNemShine/settings.json` (`$XDG_CONFIG_HOME` when set, `%APPDATA%` on Windows) and follow you across projects. Everything else travels with the DAW project.
- Default lane notes follow a GM-style mapping: Kick 36, Snare 38, Closed Hat 42, Open Hat 46, Clap 39, Rim 37, Low Tom 45, Percussion 50. MIDI is sent on zero-based channel 9 (MIDI channel 10).
- Fixed-size patterns support 1–64 steps. The generation path uses bounded work, and the audio callback uses fixed-size patterns without allocating or locking.
- Settings can snap the pattern length to whole steps or leave it fractional. With snapping off, a length like 16.5 wraps mid-cell, so the pattern drifts against the step grid instead of repeating in phase. Exact values can be typed into the spin boxes rather than dragged, which matters because a 64-value range does not fit in the slider's pixel width.
- Settings include Aurora (reference), Neon, Lava, and Forest themes. Generator parameters, rate, waveform, and theme selectors support mouse-wheel stepping while hovered.

## Generation modes and views

The **Math** column shows the formula with the default snapshot substituted: `steps = 16`, `pulses = 5`, `density = 0.65`, `seed = 0.271`, `phase = 0`, `osc_rate = 2`, `ca_rule = 30`, `symmetry = 4`, `variation = 0`. The editor shows the same line with the *current* values, read from the generators' own helpers so the text cannot disagree with the pattern that is playing.

| Mode | Core method | Math (default values) | Main view |
|---|---|---|---|
| Euclidean | Evenly distribute pulses around the ring; each lane scales the count by its density profile and anchors on its own phase origin | `k = round(pulses·density·(0.35+w))` → lane 0 `round(5·0.65·0.85) = 3`; onset `j` at `floor(j·n/k)` | Per-lane pulse rings |
| Eulerian Walk | Hierholzer traversal of a balanced directed graph; rest is vertex 0, lanes are vertices 1–8 | `d = 1 + (pulses mod 4)` → `1+1 = 2`; circuit length `9·d = 18` | Directed drum-state network |
| Golden Ratio / Phyllotaxis | Golden-angle spiral; the beat test steps by `1/φ`, with a slower radial term | threshold `T = (pulses/n)·(0.35+w)·(0.5+density) = 0.305`; `beat(i) = frac(i·0.618034 + seed + phase + lane·0.137)` | Golden-angle spiral |
| Cellular Automaton | Ring-based elementary cellular automaton (Wolfram rule); pulses selects a generation | `row = (pulses + round(phase·16) + 2·lane) mod 16` → lane 0 row 5; gate `0.45 + 0.55·density·w = 0.63` | Space-time cell grid |
| Oscillator / Phase | Sine, triangle, saw or square sampled at each step with explicit phase and cycles | `wanted = round(pulses·density·(0.5+w)) = 3`; keep the 3 highest wave samples | Waveform with trigger threshold |
| Crystallographic | Seeded motif tiled across the pattern, transformed per unit cell by a symmetry operation | `order = 4`, `unit = n/order = 4`; motif threshold `T = 0.184`; operation `(order−1) mod 8` | 8-lane lattice and symmetry axes |
| Logistic Map | Logistic orbit; onsets on local peaks above a per-lane cutoff | `x[n+1] = r·x[n]·(1−x[n])`, `r = 3.5+0.5·density = 3.825`, `x0 = 0.328`; cutoff `1−0.236 = 0.764` | Orbit diagram |
| Markov Walk | Seeded first-order transition walk through rest and the eight drum voices | `p(rest) = 5`; event chance `density·(0.25 + 1.5·pulses/n) = 0.47` | State-transition graph |
| Raised-Cos Threshold | Smooth raised-cosine envelope whose cycle count is the requested hit count | `env(i) = 0.5−0.5·cos(2π(hits·i/n+φ))`, `hits = 3`, `φ = 0.108`, cutoff `0.916` | Envelope curve and cutoff |
| Slowcat Rhythm | Short per-lane motifs concatenated over slow subdivisions | `motif = 2+(lane+pulses) mod 7 = 7`; `shift = 4`; `slot = (i+shift) mod motif` | Phrase timeline |
| Toussaint Distance | Integer boundary crossings of a maximally even pulse train, seeded circular origin, modular-distance accents | `k = round(pulses·density) = 3`, origin `4`; onset when `floor((pos+1)·k/n) ≠ floor(pos·k/n)` | Distance circle |
| Well-Formed Rhythm | Christoffel/Bresenham word; every lane gets its own phase origin | `k = 3`; `d = (pos·k) mod n`; hit when `(n−d) mod n < k` | Bresenham slack plot |
| L-System Grammar | Bounded parallel rewriting of a two-symbol alphabet into lane and accent roles | `family = (floor(seed·4)+variation) mod 4 = 1`; lane 0 chance `0.23·grammar·metrical` | Nesting-depth bars |
| Phase Shift / Reich | One maximally even motif circularly offset per lane (Reich-style phasing) | `k = round(pulses·density) = 3`; `shift = lane·2 + round(phase·n) + round(seed·n)` → lane 0 shift 4 | Per-lane motif rings, each a rotated copy |
| Polyrhythm Stack | Fixed integer ratios (1:1, 3:2, 2:1, 5:2, 4:3, 2:3, 5:4, 3:4) over one duration | lane 0 `k = round(5·0.65) = 3`; lane 1 (3:2) `k = round(8·0.65) = 5` | Ratio rings |
| Weighted Syncopation | Metric, lane-shifted and jittered scores; the top-k steps become onsets | `target = round(pulses·density·(0.45+w)) = 3`; `score(i) = w(i mod 4)·w((i+3·lane) mod 4)·jitter` | Metrical score bars |
| Three-Body Chaos | Soft-gravity three-body leapfrog orbit; sweep and close encounters are ranked into onsets | `substeps = clamp(n·(8+32·osc_rate), 64, 2048) = 1152`; `wanted = 3`; line crossings accent | Live orbit trace |
| Pebble Box | Eight pebbles (one per lane) bounce in a box; a shaker impulse kicks a rotating third and wall impacts above the density threshold fire their lanes | `substeps = osc_rate` (`2`), gravity `0.05+ca_rule/255·1.15 = 0.19` tilted `0°`; impulse every `n/pulses = 3` steps; threshold `(1−density)·4.2 = 1.47` | Animated pebble box with wall-contact flashes |

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
  modes.rs                the eighteen generation modes, as the host parameter and the dispatch key
  editor.rs               Slint adapter; host parameter callbacks; UI model updates
  view.rs                 pure geometry for the eighteen generation-mode panels
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
    syncopation.rs         weighted metric/offbeat selection
    nbody.rs              three-body gravity, chaotic onset score
ui/app.slint             Slint interface and per-lane step matrix
docs/                     reference map, architecture, wishlist, agent ownership
```

## Current limitations / next milestones

This is a functional starter architecture, not a release-certified plugin. The next steps are to build and smoke-test it in at least one CLAP host and one VST3 host; validate tempo/loop/transport edge cases; verify Slint layout on Linux/macOS/Windows; add swing, ratchets, probability per hit, ghost notes and presets; and then add user-facing MIDI export or optional internal sound generation only if that becomes a requirement.

Research rationale and citations for the algorithms are in [`docs/ALGORITHM_RESEARCH.md`](docs/ALGORITHM_RESEARCH.md).
