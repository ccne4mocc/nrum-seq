# Changelog

## 1.0.0 — Initial release

First release under the v1.0.0 version line. It includes all work previously tagged
v1.2.0–v1.3.1; those tags and releases were withdrawn. The entries below are kept for
provenance.

## 1.3.1 — Documentation cleanup

### Changed

- Consolidated the docs tree under a single index: architecture, reference mapping, edit overlay,
  LFO system, mode examples, verification, release checklist and changelog are the canonical
  current documents.
- Rewrote the nice-plug migration note against the current pin and moved verification results to
  `HOST_VERIFICATION.md`.
- Trimmed the fractal/geometric/cellular research to a future backlog; Life-like CA is marked
  implemented (mode 39).
- Pruned completed wishlist entries and refreshed host-verification counts and status.
- Updated the nrum architecture document for mode 39, the `GATE` edit mode and the current
  contact metadata.
- Split CI into tests and a tag-driven `Bundles` workflow that builds Linux/Windows artifacts and
  attaches them to the matching GitHub release.
- Updated the workflow actions to Node 24-compatible major versions and pinned Linux runners to
  Ubuntu 24.04.
- Replaced the README support text with a Buy Me A Coffee banner and removed the single-project
  derivation claim.

## 1.3.0 — Editor, randomize and release cleanup

### Added

- Unipolar-negative LFO polarity (downward only) beside bipolar and unipolar-up.
- LFO depth now runs 0–200%, where 100% is one full parameter range.
- Per-step edit overlay: hit overrides, probability, accent, ratchets (1–4), step-wide
  microtiming, **per-cell gate length**, `CLR ALL`, and editor-local undo/redo. Edits are project
  state and are applied in display space, so they survive Rotate, Mirror and Generate.
- Matrix navigation: horizontal/vertical zoom, `FIT`, native scrollbars, wheel-to-scroll
  vertically, Alt+wheel for horizontal panning, follow-mode centering and a lane-label column
  synchronized with the vertical scroll.
- Grid ruler with bar numbers, beat/bar separators and alternating bar shading.
- Rich step visuals: width = gate length, height = velocity, opacity = probability, one segment
  per ratchet, microtiming offset, and a playhead column highlight.
- Thin bottom status strip for hover descriptions, LFO status and export feedback.
- Built-in starting-point presets (House, Break, Poly, Deep, Glitch) that write ordinary host
  parameters.
- Lane labels show MIDI note numbers.
- Eight-lane phyllotaxis rosettes with a per-theme lane palette.
- Life-like cellular automaton mode (index 39): a 16×8 torus with Life, HighLife, Seeds, Day & Night and a dense survival rule, plus selectable initial shapes.
- Cellular initial-state policies, L-system grammar-family selection and four Markov transition bias fields.
- Mode-aware Randomize palettes: lengths, pulses, density, probability and velocity snap to
  musically legible sets, and the pulse vocabulary is keyed on the mode that survives the scope.
- CI workflow for Linux, macOS and Windows, plus a release checklist and host verification matrix.

### Changed

- The fresh default palette is now **Lava**; stored global themes and project parameters still
  override it.
- The matrix edit mode formerly labelled `LEN` is now `GATE`, matching the value it edits, with
  matching callback and drag-state names. Its snap selector now offers 1/4, 1/2, 3/4 or one step.
- Matrix cell gaps, step lines and beat/bar strokes adapt to zoom and use theme text colours, so
  the grid stays legible at FIT and in every palette.
- The plug-in credits and CLAP metadata now expose only the author and GitHub profile. The
  contact email and Buy Me a Coffee link moved out of the plug-in; the support link lives in the
  README.
- README rewritten as a quick-start overview; architecture, verification and research details
  stay in `docs/`.

### Fixed

- Reloading the editor no longer leaves the Sketch palette on screen: the first update tick now
  always publishes the persisted/global palette instead of treating the default UI index as
  already applied.
- Editor spawn diagnostics are only written when technical logging is enabled.
- MIDI export now expands per-step ratchets and keeps the substep gate inside its own spacing,
  matching the scheduler.
- Default ceiling is 256 steps instead of 1024. Selecting a mode raises the loop (and the
  ceiling, if it was lowered) to that mode's functionality floor: attractors, walks, word modes
  and the long-form figures ask for 24–81 steps while closed-form rhythms are untouched. Saved
  patterns and host automation are never rewritten.
- LFO depth 100% now modulates the full parameter range; it used to cover only half, so
  e.g. Swing at full depth never left 0–25% of its 0–50% range. The depth bands, hover
  percentages and docs follow the same scale.
- Slowcat lane 0 no longer tiles silence when the pattern is no longer than its first motif.
- Documentation now matches the reduced 256-step default ceiling.

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
