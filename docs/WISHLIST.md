# Wishlist

Not a roadmap. Not a contract. Nothing here is scheduled, prioritized, or promised. These are ideas that seemed worth writing down.

The binding rules live in `AGENTS.md` and `docs/ARCHITECTURE.md`.

## Build and host verification

- Bundle CLAP + VST3 with `cargo xtask bundle nrum-seq --release` and open the result in one CLAP host and one VST3 host. Linux host verification is still pending; the Windows/Wine run below is the only verified path. The CI workflow now builds and tests on Linux, macOS and Windows, and a release checklist exists in `RELEASE_CHECKLIST.md`.
- Check no-audio MIDI-effect routing into a sampler, host parameter automation, plug-in scan, editor close/reopen, tempo changes, loop points, stop/start and free-run.
- A repeatable smoke-test checklist with host/version/OS, distinguishing untested from passing.
- ~~Run CI for Linux, macOS and Windows; package CLAP + VST3 in release artifacts.~~ Added in `.github/workflows/ci.yml`; the Linux bundle artifact is produced, macOS/Windows bundle artifacts are still manual.
- **Windows** (updated 2026-10-28): cross-compile for `x86_64-pc-windows-gnu` builds clean and the bundled VST3 + CLAP load, run and show the full editor in REAPER 7.83 x64 under Wine. Still unverified: any real Windows system, MSVC targets, REAPER bridge mode (editor does not appear when the VST3 is bridged).
- Check plug-in metadata, class ID stability, GPL-3.0-or-later notices, crash/reload state, parameter versioning and host preset compatibility.
- Test several hosts, because VST3 MIDI-only routing and CLAP note-effect routing vary by host.
- Document known host limitations and keep release notes honest.

## Scheduler and timing tests

- A harness around beat-boundary → sample-offset conversion: block-boundary hits, negative/looped transport, tempo changes, transport jumps, `steps=1`, one-frame blocks, and NoteOff before retrigger.
- Per-generator tests across all lengths, zero/full density, 0/full pulse settings, rotation wrap, seed repeatability and variation changes.
- Verify no algorithm reads outside `0..steps`, velocities are finite 0–1, notes clamp 0–127, and disabling a lane suppresses only that lane.
- ~~`generate()` is now proven allocation-free by a thread-local counting allocator in `sequence/mod.rs`. The same check does not yet cover `Plugin::process()`.~~ Both paths are now covered by `generation_never_allocates` and `process_never_allocates`.
- Long-duration test and CPU benchmark exist (`a_long_scheduler_run_stays_bounded`, ignored `bench_process_at_max_steps`); broad host automation profiling is still open.

## Sequencing features

- ~~Per-step probability and accent overrides, deterministic per step.~~ Implemented in the edit overlay; the editor matrix exposes `HIT`, `PROB` and `ACC` modes.
- ~~Ratchets with a strict maximum and sample-accurate substep event distribution.~~ Implemented with a maximum of four; gate is capped to the lane's own substep spacing.
- **Per-lane generator selection** — implemented twice and removed both times. Treat it as closed unless a concrete request returns. The notes below record the design lessons. It worked: `GenKind::PerLane` dispatched each *distinct* generator named in `GenParams.lane_mix` once and kept only its assigned lanes, so cost tracked distinct modes rather than eight. With the ceiling at 64 (the historical default) the worst case measured 23 µs, about 3% of a 32-sample block. Re-measure at the current 1024 default before shipping. What to settle next time:
  - Where the UI lives. A 16-option `Selector` side-by-side is unreadable at any lane-row width, and a 2x4 grid of steppers made the window jumpy. Consider a dropdown, or putting lane assignment in the settings area rather than the main layout.
  - `Pattern.state`, `ca` and `orbit` are single-mode view data. The previous attempt had the lowest-numbered lane donate them, which is arbitrary; a per-lane view state would be cleaner but costs a `Pattern` field.
  - Whether per-lane *density*, phase and pulse counts should come along, or generator only. Generator-only is a smaller change but leaves lanes fighting over one global density profile.
  - Per-lane *lengths* are a separate and harder feature: eight lengths is a polyrhythm the single `loop_length` cannot express, and it needs scheduler work rather than generator work.
  - Whether it earns the parameter surface at all: it adds nine host parameters for a mode most sessions will leave off.
- ~~Step editing by click/drag, with defined precedence between manual override and the algorithm, and defined behavior when Generate is pressed.~~ Implemented as click editing with `HIT`/`PROB`/`ACC` modes; edits are display-space and survive Generate. Drag painting and lane/step locks remain open.
- Per-lane pattern lengths and polymeter, while preserving deterministic host synchronization.
- ~~Per-step microtiming as a data-model field.~~ Implemented as step-wide positive delay; per-lane and negative offsets still need scheduler lookahead.

## UX

- Lane labels now show MIDI note numbers. Named sampler-map layouts (GM, user-defined) are still open.
- Responsive layout and keyboard accessibility; high-DPI scaling and editor open/close lifecycle.
- Built-in starting-point presets write ordinary host parameters; user-defined, versioned preset files are still open.
- Undo/redo for manual step edits is implemented (editor-local, 64 snapshots). Copy/Generate variations, lane/step locks and variation history remain open.

## Algorithm depth

- ~~Markov transition editing with explicit weights.~~ Implemented as four 2-bit bias fields (`markov_bias`): rest, repeat, role-compatible and novelty. A full 9×9 matrix remains open.
- ~~L-system / grammar expansion with depth and step caps.~~ Grammar families are selectable through `symmetry`; the rewrite is bounded to 1024 symbols and 16 passes.
- 1D CA has selectable initial-state policies and a separate Life-like 2-D mode now exists (index 39).
- ~~Formalize the crystallographic-inspired mode.~~ Documented in `REFERENCE_MAPPING.md`, including which operation is a real translation action and why density is structurally steppy.
- Chaotic maps (circle, tent) as separate modes only if each has a genuinely different control and view. Both are one-dimensional and would need controls that are not `pulses`/`density` again.
- Every algorithm now has a help line and a mode view, and the major gaps in targeted invariant tests are filled. Worked examples and explicit seed-behaviour notes are still open.
- Fixed and covered by tests: modes going silent at `steps=1`, and the Oscillator density floor. Both were user-visible defects rather than missing features.
- Still worth checking: Crystallographic's motif cell is `steps / symmetry` steps wide, so its smallest possible change in hit count is that wide too. Its density slider is inherently steppy rather than smooth. The threshold was rebalanced but the quantisation is structural.

## Visual try-outs

- ~~**Phyllotaxis: full eight-lane rosette variant.**~~ Implemented with `ViewNode.lane`, eight rosettes and a per-theme lane palette.

## Notes on working here

- View agents can add drawings in `view.rs` after the corresponding algorithm emits stable view-state data.
- QA can add invariant tests independently of UI polish.
- A scheduler test harness can be developed in parallel with new algorithms.
- `GenParams`, `Pattern`, mode enum order and the `StepCell`/`ViewNode` schemas are cross-layer contracts.
- `vendor/slint-baseview` is a crates.io copy carrying a one-line fix for mouse-wheel delivery; see `vendor/slint-baseview/PATCH-NOTES.md`. Do not edit it casually, and check whether upstream has shipped a fix before carrying the fork further.
- The parameter struct and IDs in `src/lib.rs`, the `opts` mapping and callbacks in `ui/app.slint` + `src/editor.rs` must be changed as one task.
- Changing `Rate`, `gate`, swing or ratchet semantics affects scheduling, UI description and timing tests together.
- A per-lane generator selection was implemented and removed twice; see the Sequencing features section. If it returns, do not call eight full generators unconditionally — dispatch each *distinct* mode once instead.