# Per-step edit overlay

The edit overlay adds manual, per-step changes on top of a generated pattern. It is project state:
it travels with the host's saved state and is not a host parameter.

## Data model

`src/edits.rs` owns two representations:

- `EditStore` — the persistent, lock-free mirror. Every cell field is an atomic byte, and a
  revision counter is bumped after every change.
- `EditView` — the plain, copyable snapshot the audio thread, preview and MIDI export read. The
  audio callback refreshes it only when the revision changes, so it never locks.

Each lane/step cell has:

| Field | Values |
|---|---|
| hit | `0` follow the generator, `1` force on, `2` force off |
| accent | `0` follow the generated accent, otherwise `1..255` mapped to `0.05..1.0` |
| probability | `255` follow the global probability, otherwise `0..254` mapped to `0..1` |
| ratchet | `1` no ratchet, `2..4` substeps |
| offset | signed fraction of one step; currently only positive values are scheduled |
| length | `0` follows the global gate, otherwise `0.02..1.0` of one step |

`EditState` serializes only non-default cells as a sparse list, so an untouched project stores an
empty list. `EditStore` implements nice-plug's `PersistentField` directly instead of using an
`RwLock`, which is what makes state restore and audio-thread reads lock-free.

## Precedence and apply order

1. The generator runs with the current `GenParams`.
2. Global probability is applied, or the cell's probability override instead.
3. Rotation and mirror place the generated step in display space.
4. Metric accents are applied.
5. Manual hit and accent overrides are applied in display space.

Consequences:

- A manual edit stays on the step the user clicked when Rotate, Mirror or Generate changes.
- A force-on hit survives a zero global probability. A force-off hit always wins.
- A probability override replaces the global value for that cell, so raising it above the global
  value can restore a hit the global value would have dropped.
- Generate and Randomize preserve manual edits. Use `CLR ALL` to remove them. This is the current
  contract; lane/step locks are a later feature.

## Scheduler

- Ratchets use up to 12 subdivisions per step so every count `1..4` divides evenly. A lane with
  count `r` fires on subdivisions `0, 12/r, 2·12/r, …`.
- A ratchet note's gate is capped to `0.9 ×` its own substep spacing, so one lane never needs
  polyphony. NoteOff still precedes every retrigger.
- A per-cell length replaces the global gate for that lane/step. The ratchet cap still applies,
  and export uses the same length, so what you hear and what you export stay identical.
- Microtiming delays the whole step and is shared by every lane. The editor writes the same
  offset to all eight lane cells. Positive values are clamped to `0.9` of a step; negative values
  are stored but ignored until the scheduler has lookahead for early events.
- NoteOff and export use the shifted onset, so gate length follows the microtiming delay.

## Editor

The matrix header has edit modes: `HIT`, `PROB`, `ACC`, `CLR`, `RAT`, `TIME`, `GATE`, plus
`CLR ALL`, `UNDO`, `REDO` and a gate snap selector (`1/4`, `1/2`, `3/4` or one step).

- Editing is disabled until a mode is selected, so normal scrolling is unaffected.
- `HIT` cycles auto → on → off.
- `PROB` cycles 100 → 75 → 50 → 25 → 100 percent; the auto value is the global probability.
- `ACC` cycles auto → 100 → 75 → 50 percent.
- `RAT` cycles 1 → 2 → 3 → 4.
- `TIME` cycles 0 → 1/8 → 1/4 → 1/2 of a step.
- `GATE` drags a step's gate length; the value snaps to the selected fraction of one step and
  one undo entry covers the whole drag.
- `CLR` clears every overlay field on the clicked cell.
- `CLR ALL` clears the whole overlay.
- Undo/redo is editor-local and covers overlay edits only. Host parameter changes remain owned by
  the host's undo and automation.

The UI receives `hit-override`, effective `probability`, `ratchet`, `offset`, effective `length`
and `edited` per cell. Note bodies encode width = gate, height = velocity, opacity = probability
and one segment per ratchet. Force-on and force-off cells are marked, and beat/bar lines plus a
bar ruler make the grid readable at any zoom.

The matrix supports horizontal and vertical zoom (`+`, `−`, `FIT`), native scrollbars,
wheel-to-scroll vertically, Alt+wheel for horizontal panning, follow-mode centering and a
vertical scroll shared with the lane-label column.

## Tests

- `edits.rs`: snapshot round-trip, clear, revision bumps, default view.
- `sequence::tests`: empty overlay parity, display-space hit overrides, probability restoration,
  accent replacement.
- `lib.rs` scheduler tests: edits reach playback, ratchet substep spacing and NoteOff ordering,
  microtiming delay and grid alignment, and per-cell length controls NoteOff.
- `midi_export.rs`: microtiming shifts the exported tick; per-cell length replaces the global
  gate; NoteOff still sorts first.

## Limitations

- Microtiming is step-wide, not per lane, and negative offsets are not scheduled yet.
- Ratchets are limited to four; they appear as note-body segments in the step matrix.
- Per-cell length is capped at one step. Longer legato-style overlaps would need a voice/overlap
  rethink.
- Overlay edits are not host-automatable. They are project state, edited in the GUI.
- The editor's undo history is capped at 64 snapshots and is not saved with the project.
