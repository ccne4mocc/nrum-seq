# Wishlist

Not a roadmap. Not a contract. Nothing here is scheduled, prioritized or promised. Completed work
is recorded in [`CHANGELOG.md`](CHANGELOG.md), not here.

## Hosts and release

- Run the full smoke test in at least one native Linux CLAP host and one native Linux VST3 host.
- Verify on real Windows hardware. The MinGW build works under Wine/REAPER; MSVC, real Windows and
  REAPER's bridged VST3 editor path remain untested or known-bad.
- Verify macOS if a build is ever configured.
- Test several hosts: VST3 MIDI-only routing and CLAP note-effect routing vary by host.
- Manually verify state save/load and host preset round-trips in REAPER and Bitwig.

## Scheduling and editing

- Per-lane pattern lengths and polymeter, while preserving deterministic host synchronisation.
- Per-lane microtiming and negative timing offsets; both need scheduler lookahead.
- Drag painting across steps, plus lane/step locks that survive Generate and Randomize.
- Copy/paste and variation history for generated patterns.

## Sequencing and UX

- A full editable Markov matrix instead of the four 2-bit bias fields.
- User-defined, versioned preset files. The built-in starting points already write ordinary host
  parameters.
- Named sampler maps (GM or user-defined) instead of only showing MIDI note numbers.
- Responsive layout, keyboard accessibility, high-DPI scaling and editor open/close lifecycle.
- Chaotic maps such as circle and tent as separate modes only if each has a genuinely different
  control and view.

## Research backlog

Fractal, geometric and cellular candidates live in
[`RESEARCH_FRACTAL_GEOMETRIC_CELLULAR.md`](RESEARCH_FRACTAL_GEOMETRIC_CELLULAR.md). That document is
research only; nothing there is scheduled.

## Closed lessons

- **Per-lane generator selection** was implemented twice and removed twice. Treat it as closed
  unless a concrete request returns. The useful lessons are preserved here:
  - dispatch each *distinct* generator once rather than running eight full generators;
  - a 16-option side-by-side selector is unreadable at lane-row width; a dropdown or settings
    panel is a better fit;
  - `Pattern.state`, `ca` and `orbit` are single-mode view data, so per-lane views need a
    `Pattern` change;
  - per-lane lengths are a separate scheduler problem, not a generator-selection problem.
- Per-step probability, accent, ratchets, microtiming and gate length are implemented in the edit
  overlay; see [`EDIT_OVERLAY.md`](EDIT_OVERLAY.md).

## Working notes

- `GenParams`, `Pattern`, mode enum order and the `StepCell` / `ViewNode` schemas are cross-layer
  contracts.
- `vendor/slint-baseview` is a crates.io copy with local mouse-wheel and Alt+wheel fixes. Check
  whether upstream has shipped a fix before carrying the fork further.
- The parameter struct and IDs in `src/lib.rs`, the `opts` mapping in `ui/app.slint` and the
  callbacks in `src/editor.rs` must change as one task.
