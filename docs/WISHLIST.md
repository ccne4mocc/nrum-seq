# Wishlist

Not a roadmap. Not a contract. Nothing here is scheduled, prioritized, or promised. These are ideas that seemed worth writing down.

The binding rules live in `AGENTS.md` and `docs/ARCHITECTURE.md`.

## Build and host verification

- Bundle CLAP + VST3 with `cargo xtask bundle ndrum --release` and open the result in one CLAP host and one VST3 host. Nothing here has been run in a host yet.
- Check no-audio MIDI-effect routing into a sampler, host parameter automation, plug-in scan, editor close/reopen, tempo changes, loop points, stop/start and free-run.
- A repeatable smoke-test checklist with host/version/OS, distinguishing untested from passing.
- Run CI for Linux, macOS and Windows; package CLAP + VST3 in release artifacts.
- **Windows** (updated 2026-10-28): cross-compile for `x86_64-pc-windows-gnu` builds clean and the bundled VST3 + CLAP load, run and show the full editor in REAPER 7.83 x64 under Wine. Still unverified: any real Windows system, MSVC targets, REAPER bridge mode (editor does not appear when the VST3 is bridged).
- Check plug-in metadata, class ID stability, GPL-3.0-or-later notices, crash/reload state, parameter versioning and host preset compatibility.
- Test several hosts, because VST3 MIDI-only routing and CLAP note-effect routing vary by host.
- Document known host limitations and keep release notes honest.

## Scheduler and timing tests

- A harness around beat-boundary → sample-offset conversion: block-boundary hits, negative/looped transport, tempo changes, transport jumps, `steps=1`, one-frame blocks, and NoteOff before retrigger.
- Per-generator tests across all lengths, zero/full density, 0/full pulse settings, rotation wrap, seed repeatability and variation changes.
- Verify no algorithm reads outside `0..steps`, velocities are finite 0–1, notes clamp 0–127, and disabling a lane suppresses only that lane.
- `generate()` is now proven allocation-free by a thread-local counting allocator in `sequence/mod.rs`. The same check does not yet cover `Plugin::process()`, which the audio thread actually runs.
- Long-duration test and CPU profiling with 8 lanes, 64 steps and host automation across all parameters.

## Sequencing features

- Per-step probability and accent overrides, deterministic per step so the editor preview stays stable. Swing covers uniform grid feel; per-hit velocity and probability do not exist yet.
- Ratchets with a strict maximum and sample-accurate substep event distribution; define the interaction with Gate and retriggered notes.
- **Per-lane generator selection** — built and then removed again, so treat this as a fresh design question rather than a resumption. It worked: `GenKind::PerLane` dispatched each *distinct* generator named in `GenParams.lane_mix` once and kept only its assigned lanes, so cost tracked distinct modes rather than eight. With `MAX_STEPS` at 64 the worst case measured 23 µs, about 3% of a 32-sample block. What to settle next time:
  - Where the UI lives. A 16-option `Selector` side-by-side is unreadable at any lane-row width, and a 2x4 grid of steppers made the window jumpy. Consider a dropdown, or putting lane assignment in the settings area rather than the main layout.
  - `Pattern.state`, `ca` and `orbit` are single-mode view data. The previous attempt had the lowest-numbered lane donate them, which is arbitrary; a per-lane view state would be cleaner but costs a `Pattern` field.
  - Whether per-lane *density*, phase and pulse counts should come along, or generator only. Generator-only is a smaller change but leaves lanes fighting over one global density profile.
  - Per-lane *lengths* are a separate and harder feature: eight lengths is a polyrhythm the single `loop_length` cannot express, and it needs scheduler work rather than generator work.
  - Whether it earns the parameter surface at all: it adds nine host parameters for a mode most sessions will leave off.
- Step editing by click/drag, with defined precedence between manual override and the algorithm, and defined behavior when Generate is pressed.
- Per-lane pattern lengths and polymeter, while preserving deterministic host synchronization.
- Per-step microtiming as a data-model field, which needs the same treatment `GenParams.length` got: a whole-number field the generators read, plus an exact value the scheduler reads.

## UX

- Lane labels and note-number tooltips; named sampler-map layouts (GM, user-defined) rather than assuming GM notes everywhere.
- Responsive layout and keyboard accessibility; high-DPI scaling and editor open/close lifecycle.
- Preset snapshots for genre/use cases with versioned serialization, keeping plug-in parameter IDs stable.
- Copy/Generate variations, lock-lane/lock-step, undo/redo and variation history, once it is clear how those edits serialize.

## Algorithm depth

- Markov transition editing with explicit weights instead of hard-coded transition preferences.
- L-system / grammar expansion with depth and step caps, using an explicit bounded algorithm.
- 1D/2D cellular rules, configurable initial-state policies, CA generation selection.
- Formalize the crystallographic-inspired mode: name each implemented operation, document its invariants, and distinguish real group action from artistic periodic symmetry.
- Chaotic maps (circle, tent) as separate modes only if each has a genuinely different control and view. Both are one-dimensional and would need controls that are not `pulses`/`density` again.
- Every algorithm should end up with a concise mathematical description, an example, deterministic seed behavior, a mode view and invariant tests.
- Fixed and covered by tests: modes going silent at `steps=1`, and the Oscillator density floor. Both were user-visible defects rather than missing features.
- Still worth checking: Crystallographic's motif cell is `steps / symmetry` steps wide, so its smallest possible change in hit count is that wide too. Its density slider is inherently steppy rather than smooth. The threshold was rebalanced but the quantisation is structural.

## Visual try-outs

- **Phyllotaxis: full eight-lane rosette variant.** Draw all eight rosettes with per-lane colours and matching trigger wedges, rather than overlaid dots in one colour with staggered arcs. This needs a `ViewNode.lane`/palette contract shared by `src/view.rs` and `ui/app.slint`; the current panel intentionally keeps the eight-rosette drawing and fixes only the trigger art.

## Notes on working here

- View agents can add drawings in `view.rs` after the corresponding algorithm emits stable view-state data.
- QA can add invariant tests independently of UI polish.
- A scheduler test harness can be developed in parallel with new algorithms.
- `GenParams`, `Pattern`, mode enum order and the `StepCell`/`ViewNode` schemas are cross-layer contracts.
- `vendor/slint-baseview` is a crates.io copy carrying a one-line fix for mouse-wheel delivery; see `vendor/slint-baseview/PATCH-NOTES.md`. Do not edit it casually, and check whether upstream has shipped a fix before carrying the fork further.
- The parameter struct and IDs in `src/lib.rs`, the `opts` mapping and callbacks in `ui/app.slint` + `src/editor.rs` must be changed as one task.
- Changing `Rate`, `gate`, swing or ratchet semantics affects scheduling, UI description and timing tests together.
- A per-lane generator selection was implemented and removed; see the Sequencing features section. If it returns, do not call eight full generators unconditionally — dispatch each *distinct* mode once instead.