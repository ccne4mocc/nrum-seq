# Changelog

## Unreleased

### Added

- Per-step edit overlay: hit overrides, probability, accent, ratchets (1–4), step-wide
  microtiming, `CLR ALL`, and editor-local undo/redo. Edits are project state and are applied in
  display space, so they survive Rotate, Mirror and Generate.
- Per-lane generator selection through `lane1_mode` … `lane8_mode`; each lane badge opens the
  shared mode picker, and the global mode still supplies the panel view.
- Built-in starting-point presets (House, Break, Poly, Deep, Glitch) that write ordinary host
  parameters.
- Lane labels show MIDI note numbers.
- Eight-lane phyllotaxis rosettes with a per-theme lane palette.
- Cellular initial-state policies, L-system grammar-family selection and four Markov transition bias fields.
- CI workflow for Linux, macOS and Windows, plus a release checklist and host verification matrix.

### Fixed

- Slowcat lane 0 no longer tiles silence when the pattern is no longer than its first motif.
- Documentation now matches the 1024-step default ceiling.

### Tests

- Transport-aware scheduler harness covering block boundaries, one-sample blocks, swing,
  transport jumps, stop/reset NoteOffs, lane disable and one-step patterns.
- Allocation guards for both `generate()` and `Plugin::process()`.
- Targeted generator invariants for the modes that had only cross-mode coverage.
- Ratchet substep spacing, microtiming timing, edit-overlay precedence and preset bounds.


## 1.2.0 — LFO subsystem

### Added

- Four runtime LFOs with Hz (0.01–10) or beat-synced speed, waveform, depth, phase,
  bipolar/unipolar polarity and per-LFO enable.
- Drag/drop modulation links, a read-only matrix with per-row delete, depth bands and animated
  modulation bars on linked parameters, and a below-control hover summary.
- New `Gate` control in the feel row so it is user-editable and modulation-targetable.
- Floating LFO drag badge, waveform-shape picker for both oscillator and LFO slots, active
  styling for the bipolar/unipolar chip, and a timed LFO status line.
- Shared mode-group taxonomy with nelody seq: Rhythm, Stochastic, Chaos / Physics, Fractal,
  Cellular, Geometric, Number Theory, Music Theory and Acid (Acid is omitted here).

### Changed

- Renamed the plug-in to `nrum seq`. The CLAP ID is now `com.nrumseq.nrumseq` and the VST3
  class ID is `nrum-seq`; sessions created with the old IDs must replace the plug-in instance.
- Credit / Contact rows are flush, with no inter-row spacing.
- Mode entries in the mode picker are 11px (up 2px).
- LFO drag points are 28×26.
- Version is 1.2.0.

## 1.1.0 — nelody seq migration

### Added

- Six drum-adapted modes appended to the enum: Thue–Morse, Rudin–Shapiro, Brownian Walk,
  Levy Flight, Tent Map and Henon Map (indices 33–38).
- `select_spread` structural thinning, exact `fold01` reflection and lane onset-count helper.
- `view::src_index`, the inverse of the shared rotation/mirror transform.

### Fixed

- Rotation and mirror now reorder `theta` and `u` as well as every other per-step array.
  Arnold, Hilbert, Ulam, Bernoulli and Reaction-Diffusion panels no longer drift after Generate,
  Rotate or Mirror.
- Mode header wheel steps the grouped display order; the dropdown and wheel share one list.
- L-system expands far enough for 1024-step loops instead of repeating a 377-symbol word.
- Fibonacci uses the golden slope; density 1 lane 0 is the exact Fibonacci word.
- Logistic includes `variation` in its initial condition and the panel plots `(x, r·x·(1−x))`.
- Arnold fires on phase wraps instead of an unrelated random draw.
- Ulam walks a real square spiral from `phase`/`variation`, primes the cells and publishes the
  coordinates its panel draws.
- Hilbert now follows nelody seq's sampling: `pulses` selects the order, `phase` rotates by quarter
  turns, `seed` shifts the lane mapping, and the panel draws the same rotated curve.
- Arnold now matches nelody seq's parameter mapping: `pulses` is the rotation number Ω, `density`
  is the coupling K `(1−density)·1.4`, and `phase`/`seed` set the start.
- The seed control is hidden in modes whose generator never reads it (Oscillator, Well-Formed,
  Ulam, Thue–Morse, Rudin–Shapiro); it still affects the shared accent humanizer.
- Sieve thins its members evenly instead of ranking them by jitter.
- Cellular and Hilbert panels read the generator's own row/curve helpers.
- Neon is the fresh default theme; version 1.1.0 is shown in Settings.
- The header clock picker is labelled `CLOCK RATE` and matches nelody seq's field width.

### Behaviour changes

- Arnold, Ulam, Fibonacci, Hilbert and L-system patterns will sound different. Existing presets
  keep their host parameter values and mode indices; no migration is required.
- Mode names are pushed from Rust, so the UI no longer carries a second name list.
