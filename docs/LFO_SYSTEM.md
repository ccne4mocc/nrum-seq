# LFO system

nrum sequencer has four runtime LFOs in a 2×2 rack below the step matrix. The LFOs never write host
parameters. They compute an effective value from the base parameter plus the linked LFO signals,
and the scheduler, preview and matrix all read that effective value.

## Signal model

```text
Hz mode:    phase = (seconds × frequency + phase_offset) mod 1
Beat mode:  phase = (beat / rate_beats + phase_offset) mod 1

wave = oscillator_value(waveform, phase)      // 0..1
Bipolar:   signal = depth × (2 × wave − 1)    // −depth..+depth
Unipolar:  signal = depth × wave              // 0..depth
```

Hz LFOs (0.01–10 Hz) use the host beat converted through the current tempo
(`beat × 60 / BPM`). Beat-synced LFOs divide the host beat by the beat rate. Transport jumps move
the LFO phase with the transport, and free-run uses the same clock. When transport is stopped and
free-run is off, the phase holds and the picker dot freezes. Clicking an LFO name disables it;
disabled LFOs keep their links but contribute zero signal.

## Applying a link

For a target range `[min, max]`:

```text
span  = (max − min) / 2
value = clamp(base + Σ(signal_i × span for linked LFOs), min, max)
```

Integer targets round after summing. The half-range span matches the old feel scales.

## Update cadence

| Targets | When sampled |
|---|---|
| Steps, Pulses, Rotate, Density, Seed, Osc phase/rate, CA rule, Probability, Symmetry, Max steps | Once per loop, then `sequence::generate()` runs from `gen_params_at()` |
| Swing | Per note, at the nominal step beat |
| Gate, Velocity | Per note, at the resolved onset |
| Regenerate after, Randomize after | Per block |

## Targets

Included numeric controls:

- Pattern: Steps, Pulses, Rotate, Density, Seed, Osc phase, Osc rate, CA rule, Probability,
  Symmetry, Max steps.
- Feel/scheduling: Swing, Gate, Velocity, Regenerate after, Randomize after.

Excluded: LFO parameters themselves, mode/rate/wave/theme pickers, boolean toggles, Randomize
scope and Variation. Lane note/velocity have no user-editable controls in the current UI, so they
are not targets yet.

## Link storage

Each LFO has a persisted `AtomicU64` mask (`lfo1_links`…`lfo4_links`). Bit
`1 << ModTarget::index` means the LFO is linked to that target. Reads in `process()` are relaxed
and allocation-free. The matrix is the only link editor besides drag/drop.

## Drag interaction

- Each LFO cell's colored point is a `TouchArea`. Pointer down/move updates `ModDrag` with the
  window-space position; pointer up calls `ModDrag.dropped(lfo, x, y)`.
- Every visible `OptSlider`/`NumBox` with a `mod-target` reports its window rectangle through the
  `ModRegistry` global. The editor stores the newest rectangle per target.
- On drop, the editor picks the newest visible rectangle containing the pointer, toggles the
  link, and updates the status line. Dragging onto an already-linked target removes the link.
- Registration is timer-driven (30 ms initial, 250 ms refresh); reading `absolute-position` from
  its own `changed` handler triggers a Slint 1.16 property-recursion panic.

## Feedback

`ModView` publishes each target's base/effective value, depth band and a linked-LFO summary.
Linked strips draw a low-opacity depth band plus a second animated base→value bar. Hovering a
linked strip shows a tooltip just below it with the LFO name, speed and signed contribution; the
tooltip has no TouchArea, so the parameter stays editable.

`Shared` publishes `last_beat`, `pattern_beat`, `lfo_phase[4]` and `lfo_running`. The editor
preview calls `gen_params_at(pattern_clock())`, matching the audio thread's loop snapshot. MIDI
export continues to use the base snapshot.
