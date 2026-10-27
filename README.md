# nDrum

**Algorithmic percussion MIDI sequencer plugin** in Rust, using `nice-plug` for CLAP/VST3 and Slint + `slint-baseview` for the embedded GUI. It is a MIDI generator effect: place it before a drum sampler or drum rack on an instrument track to receive generated events. It intentionally does not synthesize audio or load samples.

nDrum grew out of many other interests and projects. `_AlgoNemLody`, its direct ancestor, prototyped the host-integration layer for a melodic sequencer. Rather than expanding that into another pitch-focused tool, nDrum kept the subset worth keeping — pure generators, a separate view layer, and a transport-aware MIDI scheduler — and retooled it for eight drum lanes under one shared step grid. The mode set is deliberately broad: mathematical genre excess is the point. There are Euclidean and well-formed rhythms, a logistic map, phyllotaxis, cellular automata, a pebble-box physics toy, a sieve, a Kuramoto oscillator bank, an L-system, phase canons, reaction–diffusion, Lorenz attractors, Cantor sets, Fibonacci/Sturmian words, and genetic mutation.

## Features

- Eight MIDI lanes on channel 10, with GM-style default notes (Kick 36, Snare 38, Closed Hat 42, Open Hat 46, Clap 39, Rim 37, Low Tom 45, Percussion 50).
- 28 deterministic generator modes, each with a mode-specific visualization and mode-specific parameters.
- Fixed-size 1–1024 step pattern; Settings sets a ceiling (64–1024, default 64) and applies immediately.
- Fixed-size audio scheduler with no allocation or locking; generation runs on the editor thread and snapshots parameters.
- Real-time safe audio path: no file I/O, allocations, locks, or unbounded loops inside `Plugin::process()`.
- Lane-independent hit generation; each lane has its own density profile, phase origin, and enable/note/velocity host parameter.
- Automatic regeneration and per-length randomization with a scope mask (Mode / Length / Rhythm / Tone / Feel).
- One-loop MIDI export to a timestamped SMF in a user-chosen folder.
- Four colour themes: Sketchbook (light), Neon, Lava, Forest.
- Technical logging toggle; when enabled, mode and parameter snapshots are appended to `~/nDrum.log` (`%USERPROFILE%\nDrum.log` on Windows).

## Modes

Modes are grouped rather than listed exhaustively. Full derivation and view geometry live in `src/sequence/` and `src/view.rs` respectively.

**Rhythmic structures**
- Euclidean / Euclidean-Swing hybrid / Phase Shift (Reich) / Polyrhythm Stack / Well-Formed (Bresenham) / Phase Canon / Slowcat Rhythm / Weighted Syncopation.

**Stochastic and probabilistic**
- Markov Walk / Bernoulli / Genetic Mutation / Toussaint Distance.

**Physics, chaos, reaction systems, fractals**
- Logistic Map / Lorenz Attractor / Reaction-Diffusion / Cantor Set / Pebble Box / Kuramoto Lanes.

**Mathematical and formal patterns**
- Phyllotaxis (golden ratio) / Cellular Automaton / Oscillator Phase / Crystallographic / Xenakis Sieve / Fibonacci / Sturmian Word / Eulerian Walk / African Bell / Raised-Cos Threshold / L-System Grammar.

Many modes draw a panel tied specifically to their algorithm: pulse rings, the CA spacetime, a phyllotaxis rosette, a Lorenz attractor body, phase pointers, or a walking pebble box.

## Build

Requirements: Rust ≥ 1.88, C/C++ build toolchain, Slint dependencies on Linux. Windows uses only MSVC.

```sh
cargo check --lib
cargo test --lib
cargo xtask bundle ndrum --release
```

`nice-plug` is pinned to a git commit (see `Cargo.toml`), not a crates.io version. The crates.io `nice-plug 0.4.x` line uses a different, `baseview 0.4` based editor API that the vendored `slint-baseview` does not target; moving to it means writing a new Slint adapter. [`docs/MIGRATION_nice-plug.md`](docs/MIGRATION_nice-plug.md) records what changed from nice-plug and why. Rust 1.88+ is required (nice-plug itself uses edition 2024).

Optional debug aid: `cargo xtask bundle ndrum --features assert_process_allocs` builds a plug-in that aborts if `process()` allocates (it bundles fine; `cargo test` with this feature is unsupported because the tests install their own counting allocator). Caveat found by running `clap-validator` against such a build: all well-formed processing tests pass, but malformed host events (wrong parameter namespace, inconsistent notes) make nice-plug's *own* wrapper log from the audio thread, and that logging allocates, so the guard aborts. That is framework behaviour, not this plug-in's code. Use the guarded build for development, never ship it.

Windows 32/64-bit:

```sh
rustup target add x86_64-pc-windows-msvc i686-pc-windows-msvc
cargo xtask bundle ndrum --release --target x86_64-pc-windows-msvc
cargo xtask bundle ndrum --release --target i686-pc-windows-msvc
```

CI runs the Windows bundle in `.github/workflows/windows.yml` on every push and PR. Linux bundles are built as a manual step. Artifacts land under `target/bundled/` (or `target/<target>/bundled/` for cross-compiled targets).

## Limitations

- Not release-certified; needs thorough host smoke-testing (CLAP and VST3 hosts).
- macOS build is not configured.
- Fixed UI size 860×760; no scaling preference beyond what Slint offers by default.
- The plug-in generates MIDI rather than audio; it cannot be assigned to a bus and will not do anything without an instrument track.
- Export MIDI outputs one loop and uses the plug-in process working directory, not the DAW project path.

## Project layout

```text
src/
  lib.rs              nice-plug parameters, transport scheduler, CLAP/VST3 exports
  modes.rs            the 28 generation modes and their host id/name metadata
  editor.rs           Slint adapter and host parameter callbacks
  globals.rs          cross-component shared state (settings, runtime locks)
  help.rs             mode math text shown in the editor
  midi_export.rs      Standard MIDI File writer
  view.rs             pure geometry for each global/panel view
  sequence/           one file per generator, each with its own unit tests
ui/app.slint          Slint widgets, layout and mode picker panel
docs/                 reference map, architecture, wishlist, agent ownership
xtask/                bundle and assembly runner
```

## Credits & License

- Author: `ccne4mocc / feN`
- Derived from the `_AlgoNemLody` reference project.
- Inspired by countless online articles, forum threads and prior prototypes; exact citations are listed in [`docs/ALGORITHM_RESEARCH.md`](docs/ALGORITHM_RESEARCH.md).
- License: **GPL-3.0-or-later** (see [LICENSE](LICENSE)).
