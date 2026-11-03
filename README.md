# nrum seq

**Algorithmic percussion MIDI sequencer plugin** in Rust, using `nice-plug` for CLAP/VST3 and Slint + `slint-baseview` for the embedded GUI. It is a MIDI generator effect: place it before a drum sampler or drum rack on an instrument track to receive generated events. It intentionally does not synthesize audio or load samples.

nrum seq grew out of many other interests and projects. `_AlgoNemLody`, its direct ancestor, prototyped the host-integration layer for a melodic sequencer. Rather than expanding that into another pitch-focused tool, nrum seq kept the subset worth keeping — pure generators, a separate view layer, and a transport-aware MIDI scheduler — and retooled it for eight drum lanes under one shared step grid. The mode set is deliberately broad: mathematical genre excess is the point. There are Euclidean and well-formed rhythms, a logistic map, phyllotaxis, cellular automata, a pebble-box physics toy, a sieve, a Kuramoto oscillator bank, an L-system, phase canons, reaction–diffusion, Lorenz attractors, Cantor sets, Fibonacci/Sturmian words, and genetic mutation.

## Features

- Eight MIDI lanes on channel 10, with GM-style default notes (Kick 36, Snare 38, Closed Hat 42, Open Hat 46, Clap 39, Rim 37, Low Tom 45, Percussion 50).
- 40 deterministic generator modes, each with a mode-specific visualization and mode-specific parameters.
- Fixed-size 1–1024 step pattern; Settings sets a ceiling (64–1024, default 256) and applies immediately; long-horizon modes raise the loop and ceiling to their own floor when selected.
- Fixed-size audio scheduler with no allocation or locking; generation runs on the editor thread and snapshots parameters.
- Real-time safe audio path: no file I/O, allocations, locks, or unbounded loops inside `Plugin::process()`.
- Lane-independent hit generation; each lane has its own density profile, phase origin, enable/note/velocity host parameter.
- Per-step editing: hit overrides, probability, accent, ratchets (1–4), step-wide microtiming and
  per-cell gate length, with undo/redo and a `CLR ALL` reset. Edits are display-space and survive
  rotation, mirror and Generate. The matrix draws width = gate, height = velocity,
  opacity = probability and one segment per ratchet, over a bar/beat ruler.
- Matrix navigation: horizontal and vertical zoom (`+`, `−`, `FIT`), native scrollbars,
  wheel-to-scroll vertically, Alt+wheel for horizontal panning, follow-mode centering and a
  lane-label column that scrolls with the rows.
- Hover descriptions and edit/export feedback appear in a thin bottom status strip, keeping the
  header clear and the text away from the pointer.
- Automatic regeneration and per-length randomization with a scope mask (Mode / Length / Rhythm / Tone / Feel). Randomize draws from mode-aware palettes, so lengths, pulse counts, density and probability land on musically legible values instead of uniform noise.
- One-loop MIDI export to a timestamped SMF in a user-chosen folder.
- **Four runtime LFOs** in a 2×2 rack below the step matrix: waveform, Hz (0.01–10) or beat-synced speed, depth (0–200%), phase, bipolar / unipolar / unipolar-negative polarity and per-LFO enable. Drag an LFO point onto any numeric parameter to link it; linked controls show a depth band plus a second animated value bar, and the matrix shows/removes links.
- Built-in starting-point presets for house, broken beat, polyrhythm, deep and glitch material. Presets write ordinary host parameters.
- Four colour themes: Lava (default), Sketch (light), Neon, Forest.
- Technical logging toggle; when enabled, mode and parameter snapshots are appended to `~/nrum-seq.log` (`%USERPROFILE%\nrum-seq.log` on Windows).

## Modes

Modes are grouped rather than listed exhaustively. Full derivation and view geometry live in `src/sequence/` and `src/view.rs` respectively; one starting point per mode is in [`docs/MODE_EXAMPLES.md`](docs/MODE_EXAMPLES.md).

**Rhythmic structures**
- Euclidean / Euclidean-Swing hybrid / Phase Shift (Reich) / Polyrhythm Stack / Well-Formed (Bresenham) / Phase Canon / Slowcat Rhythm / Weighted Syncopation.

**Stochastic and probabilistic**
- Markov Walk / Bernoulli / Genetic Mutation / Toussaint Distance.

**Physics, chaos, reaction systems, fractals**
- Logistic Map / Lorenz Attractor / Reaction-Diffusion / Cantor Set / Pebble Box / Kuramoto Lanes / Brownian Walk / Levy Flight / Tent Map / Henon Map.

**Mathematical and formal patterns**
- Phyllotaxis (golden ratio) / Cellular Automaton / Life-like CA / Oscillator Phase / Crystallographic / Xenakis Sieve / Fibonacci / Sturmian Word / Eulerian Walk / African Bell / Raised-Cos Threshold / L-System Grammar / Arnold Circle Map / Hilbert Curve / Ulam Spiral Primes / Vuza Canon / Thue-Morse / Rudin-Shapiro.

Many modes draw a panel tied specifically to their algorithm: pulse rings, the CA spacetime, a phyllotaxis rosette, a Lorenz attractor body, phase pointers, or a walking pebble box.

## Compatibility

Version 1.3.0 uses the new host identity `com.nrumseq.nrumseq` (CLAP) and `nrum-seq` (VST3 class
ID). Sessions created with the old `com.ndrum.ndrum` / `nDrum` IDs must replace the plug-in instance.

## Build

Requirements: Rust ≥ 1.88, C/C++ build toolchain, Slint dependencies on Linux. Windows can be built cross-compiled from Linux with MinGW, or natively/on CI with MSVC (not yet exercised).

```sh
cargo check --lib
cargo test --lib
cargo xtask bundle nrum-seq --release
```

`nice-plug` is pinned to a commit in the `ccne4mocc/nice-plug` fork (see `Cargo.toml`), not a crates.io version; the fork carries the state-loading and Carla-compatible activation fixes. The crates.io `nice-plug 0.4.x` line uses a different, `baseview 0.4` based editor API that the vendored `slint-baseview` does not target; moving to it means writing a new Slint adapter. [`docs/MIGRATION_nice-plug.md`](docs/MIGRATION_nice-plug.md) records what changed from nice-plug and why. Rust 1.88+ is required (nice-plug itself uses edition 2024).

Optional debug aid: `cargo xtask bundle nrum-seq --features assert_process_allocs` builds a plug-in that aborts if `process()` allocates (it bundles fine; `cargo test` with this feature is unsupported because the tests install their own counting allocator). Caveat found by running `clap-validator` against such a build: all well-formed processing tests pass, but malformed host events (wrong parameter namespace, inconsistent notes) make nice-plug's *own* wrapper log from the audio thread, and that logging allocates, so the guard aborts. That is framework behaviour, not this plug-in's code. Use the guarded build for development, never ship it.

Windows (cross-compiled from Linux, verified path):

```sh
rustup target add x86_64-pc-windows-gnu
cargo check --target x86_64-pc-windows-gnu
cargo xtask bundle nrum-seq --release --target x86_64-pc-windows-gnu
```

MSVC targets (`x86_64-pc-windows-msvc`, `i686-pc-windows-msvc`) should also build but have never been compiled or loaded on a real Windows system; treat them as untested until one is.

```sh
rustup target add x86_64-pc-windows-msvc
cargo xtask bundle nrum-seq --release --target x86_64-pc-windows-msvc
```

The Windows bundle workflow is manual-only (`workflow_dispatch`) and GitHub-specific. MSVC builds remain unverified on real Windows. Artifacts always land under `target/bundled/` — the xtask does not move them for cross-compiled targets.

### Host verification status (2026-10-28)

- Cross-compiled MinGW x64 build verified in **REAPER 7.83 for Windows x64 running under Wine 11.5** (fresh 64-bit Wine prefix, no dxvk): VST3 and CLAP both scan, load, process and show the full Slint/femtovg editor with no GL/wgl errors in the Wine log. The femtovg-over-wgl editor path therefore holds on a 64-bit Windows ABI.
- Not yet verified on real Windows. One known Windows failure mode from testing in a VM: with REAPER's per-plugins bridging enabled (which for a 64-bit plugin is a manual REAPER setting, not an architecture mismatch), the editor window fails to appear — bridged VST3 GUI goes through REAPER's IPC and this plugin's custom GL editor expects a directly parented window. Run the plugin unbridged, or investigate the editor path before trusting bridge mode.
- macOS build is not configured.

## Limitations

- Not release-certified; needs thorough host smoke-testing (CLAP and VST3 hosts).
- `clap-validator 0.4.1` passes 33 tests with 11 skipped and 0 failures on the pinned nice-plug
  fork. Host-side state round-trips still need manual verification in each DAW.
- Windows builds are cross-compilable and load correctly under Wine/REAPER (see Build below), but have never been run on a real Windows system; REAPER bridge mode for the VST3 editor is known to fail there.
- macOS build is not configured.
- Fixed UI size 860×768; no scaling preference beyond what Slint offers by default.
- The plug-in generates MIDI rather than audio; it cannot be assigned to a bus and will not do anything without an instrument track.
- Export MIDI outputs one loop and uses the plug-in process working directory, not the DAW project path.
- Microtiming is step-wide and only positive delays are scheduled; per-lane timing and negative offsets are future work. Ratchets are capped at four.

## Project layout

```text
src/
  lib.rs              nice-plug parameters, transport scheduler, CLAP/VST3 exports
  modes.rs            the 40 generation modes, grouped display order and host id/name metadata
  editor.rs           Slint adapter and host parameter callbacks
  globals.rs          cross-component shared state (settings, runtime locks)
  help.rs             mode math text shown in the editor
  modulation.rs       LFO clocks, signals, target metadata and link masks
  randomize.rs        mode-aware value vocabularies for the Randomize button
  midi_export.rs      Standard MIDI File writer
  view.rs             pure geometry for each global/panel view
  sequence/           one file per generator, each with its own unit tests
ui/app.slint          Slint widgets, layout and mode picker panel
docs/                 reference map, architecture, wishlist, agent ownership
xtask/                bundle and assembly runner
```

## Credits & License

- Author: `ccne4mocc / feN`
- Copyright (C) 2026 `ccne4mocc / feN`.
- Derived from the `_AlgoNemLody` reference project.
- Inspired by countless online articles, forum threads and prior prototypes; exact citations are listed in [`docs/ALGORITHM_RESEARCH.md`](docs/ALGORITHM_RESEARCH.md).
- License: **GPL-3.0-or-later** (see [LICENSE](LICENSE)).

## Support

If this project is useful to you, you can buy me a coffee: <https://buymeacoffee.com/awpe6kbwk9>
