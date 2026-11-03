# nrum seq

**Algorithmic percussion MIDI sequencer (CLAP + VST3), written in Rust.**

![nrum seq demo](assets/nrum-seq.gif)

nrum seq is a MIDI generator effect, not a synthesizer. Place it before a drum sampler or drum
rack on an instrument track and it streams generated note events into the host. It does not
produce audio, load samples or replace an instrument.

The design favours deterministic, inspectable generators over random note soup. The same
parameter snapshot and seed produce the same pattern, and every mode has its own visualisation
and mode-specific controls.

## Highlights

- **8 drum lanes** on MIDI channel 10, with GM-style defaults for kick, snare, hats, clap, rim,
  low tom and percussion. Each lane has its own note, velocity, density, phase and enable.
- **40 generators** across rhythmic, stochastic, physical, chaotic and formal families.
- **Fixed 1–1024 step grid**, with a configurable ceiling (64–1024, default 256).
- **Real-time safe audio path**: no allocation, locking, file I/O or unbounded loops inside
  `Plugin::process()`.
- **Per-step editor** with hit overrides, probability, accent, ratchets, microtiming and
  per-cell gate length, plus undo/redo and a matrix ruler.
- **Four runtime LFOs** that can be dragged onto numeric parameters and persisted with the
  project.
- **Mode-aware Randomize** and per-length randomization with a scope mask.
- **One-loop MIDI export** to a timestamped Standard MIDI File.
- **Four themes** and built-in starting-point presets.

## Quick start

1. Build the plug-in (see [Build](#build)).
2. Place the resulting CLAP or VST3 before a drum sampler on an instrument track.
3. Pick a mode, set the length and pulse count, then hit Generate.
4. Route the plug-in's MIDI output to the sampler.

nrum seq is a MIDI effect. If it is placed on a bus or on a track without an instrument
downstream, it will generate events but nothing will make a sound.

## Pattern editing

The step matrix is also an editor. Each cell can override the generator's hit, probability,
accent, ratchet count and gate length. Step-wide microtiming shifts the feel without moving the
grid, and edits are applied in display space so they survive Rotate, Mirror and Generate.

Navigation supports horizontal and vertical zoom, `FIT`, native scrollbars, wheel-to-scroll and
Alt+wheel horizontal panning. A lane-label column follows the vertical scroll, and follow mode
keeps the playhead centred while the transport runs.

Hover descriptions and edit/export feedback appear in a thin status strip at the bottom, away
from the pointer.

## Modulation and randomization

Four LFOs sit in a 2×2 rack below the matrix. Each has a waveform, Hz or beat-synced speed,
depth (0–200%), phase, polarity and an enable toggle. Drag an LFO point onto any numeric
parameter to create a link; linked controls show a depth band and a second animated value bar.
The modulation matrix lists every link and removes them with a per-row ×.

Randomize draws from mode-aware palettes instead of uniform noise, so lengths, pulse counts,
density, probability and velocity land on musically useful values. The scope mask can restrict
randomization to Mode, Length, Rhythm, Tone or Feel.

## Modes

Modes are grouped rather than listed exhaustively here. The full mapping, derivation and view
geometry live in [`docs/REFERENCE_MAPPING.md`](docs/REFERENCE_MAPPING.md) and
[`docs/MODE_EXAMPLES.md`](docs/MODE_EXAMPLES.md).

- **Rhythmic structures:** Euclidean, Euclidean-Swing, Phase Shift, Polyrhythm Stack,
  Well-Formed, Phase Canon, Slowcat Rhythm, Weighted Syncopation.
- **Stochastic and probabilistic:** Markov Walk, Bernoulli, Genetic Mutation, Toussaint
  Distance.
- **Physics, chaos and reaction systems:** Logistic Map, Lorenz Attractor, Reaction-Diffusion,
  Cantor Set, Pebble Box, Kuramoto Lanes, Brownian Walk, Levy Flight, Tent Map, Henon Map.
- **Mathematical and formal patterns:** Phyllotaxis, Cellular Automaton, Life-like CA,
  Oscillator Phase, Crystallographic, Xenakis Sieve, Fibonacci, Sturmian Word, Eulerian Walk,
  African Bell, Raised-Cos Threshold, L-System Grammar, Arnold Circle Map, Hilbert Curve,
  Ulam Spiral Primes, Vuza Canon, Thue-Morse, Rudin-Shapiro.

## Build

Requirements: Rust 1.88 or newer, a C/C++ build toolchain, and the usual Slint windowing
dependencies on Linux.

```sh
cargo check --lib
cargo test --lib
cargo xtask bundle nrum-seq --release
```

The bundle command writes CLAP and VST3 builds under `target/bundled/`.

The `nice-plug` dependency is pinned to a commit in the
[`ccne4mocc/nice-plug`](https://github.com/ccne4mocc/nice-plug) fork. It carries state-loading
and Carla activation fixes that the crates.io `0.4.x` line does not. See
[`docs/MIGRATION_nice-plug.md`](docs/MIGRATION_nice-plug.md) for the details.

### Windows

Cross-compiling from Linux with MinGW is the verified path:

```sh
rustup target add x86_64-pc-windows-gnu
cargo check --target x86_64-pc-windows-gnu
cargo xtask bundle nrum-seq --release --target x86_64-pc-windows-gnu
```

MSVC targets are also configured:

```sh
rustup target add x86_64-pc-windows-msvc
cargo xtask bundle nrum-seq --release --target x86_64-pc-windows-msvc
```

MSVC builds are not verified on a real Windows system. The `Bundles` workflow builds Linux and
Windows artifacts on `v*` tags and can also be dispatched manually from GitHub Actions.

## Compatibility

Version 1.3.1 uses the host identity `com.nrumseq.nrumseq` (CLAP) and `nrum-seq` (VST3 class ID).
Sessions created with the old `com.ndrum.ndrum` / `nDrum` IDs must replace the plug-in instance.

## Status and limitations

- Not release-certified. Host smoke-testing is still required for each DAW.
- `clap-validator` passes 33 tests with 11 skipped and 0 failures on the pinned `nice-plug`
  fork; host-side state round-trips still need manual verification.
- Windows/MinGW builds scan, load and process under Wine/REAPER, but have not been run on real
  Windows. REAPER's bridged VST3 editor path is a known failure mode; run unbridged.
- macOS is not configured.
- Fixed UI size (860×768), with no separate scaling preference.
- MIDI export writes one loop and uses the plug-in process working directory, not the DAW
  project path.
- Microtiming is step-wide and only positive delays are scheduled. Ratchets are capped at four.

## Project layout

```text
src/
  lib.rs              parameters, transport scheduler, CLAP/VST3 exports
  modes.rs            generator list, display order and host metadata
  editor.rs           Slint adapter and host parameter callbacks
  globals.rs          shared state and user-global settings
  help.rs             mode descriptions and formulas
  modulation.rs       LFO clocks, signals and link masks
  randomize.rs        mode-aware randomization vocabularies
  midi_export.rs      Standard MIDI File writer
  view.rs             pure geometry for every panel
  sequence/           one generator per file, each with unit tests
ui/app.slint          Slint widgets, layout and mode picker
docs/                 architecture, reference mapping, verification and research notes
xtask/                bundle and assembly runner
vendor/               vendored Slint/baseview adapter
```

## Documentation

| Document | Contents |
|---|---|
| [`docs/README.md`](docs/README.md) | Documentation index |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Layers, invariants and data flow |
| [`docs/REFERENCE_MAPPING.md`](docs/REFERENCE_MAPPING.md) | Where every mode and view came from |
| [`docs/MODE_EXAMPLES.md`](docs/MODE_EXAMPLES.md) | A starting point for every mode |
| [`docs/LFO_SYSTEM.md`](docs/LFO_SYSTEM.md) | LFO clocks, targets, links and persistence |
| [`docs/EDIT_OVERLAY.md`](docs/EDIT_OVERLAY.md) | Per-step edit overlay semantics |
| [`docs/HOST_VERIFICATION.md`](docs/HOST_VERIFICATION.md) | What has actually been tested |
| [`docs/RELEASE_CHECKLIST.md`](docs/RELEASE_CHECKLIST.md) | Release and host smoke-test checklist |
| [`docs/ALGORITHM_RESEARCH.md`](docs/ALGORITHM_RESEARCH.md) | Algorithm sources and citations |
| [`docs/CHANGELOG.md`](docs/CHANGELOG.md) | Release notes |

## Credits and license

- Author: `ccne4mocc / feN`
- Copyright (C) 2026 ccne4mocc / feN
- License: **GPL-3.0-or-later** (see [LICENSE](LICENSE)).

## Support

<a href="https://buymeacoffee.com/awpe6kbwk9">
  <img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" width="180">
</a>
