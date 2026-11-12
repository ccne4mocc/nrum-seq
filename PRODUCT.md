# Product

<!-- impeccable:product-schema 1 -->

## Platform

desktop

<!-- The schema's values are web/ios/android/adaptive. This product is a native desktop
     CLAP/VST3 plug-in with a fixed 860x768 Slint editor; no browser surface exists. -->

## Users

- **Primary — sound designers and experimental musicians.** They load nrum seq on an instrument
  track before a drum sampler and treat it as a texture and rhythm source, not a step sequencer.
  They reach for the odd generators and push parameters until something usable or surprising
  comes out.
- **Secondary — learners and curious beginners.** They are drawn in by the live drawing of each
  algorithm and want to see what the math does.
- **Tertiary — DAW producers** who want a usable generative percussion pattern quickly and then
  automate or recall it.

The user ranked these in this order; it is a deliberate priority, not an accident of wording.

## Product Purpose

nrum seq generates deterministic MIDI percussion for an eight-lane GM drum map on MIDI channel 10.
It exists because the author was unsatisfied with existing generative sequencers (unstable, or
limited in what their algorithms could express) and wanted the pattern logic to be visible while
it runs. Success is a pattern the user keeps: found quickly, understood well enough to steer,
recalled exactly with the project, and playable downstream by any drum instrument.

## Positioning

One chain that a neighboring generator cannot truthfully copy in full: deterministic algorithmic
generation, polymetric and polyrhythmic structure, per-step humanisation, and host recall and
automation, with the editor drawing the pattern from the same math the scheduler plays. Individual
algorithms are borrowed and cited (see `docs/REFERENCE_MAPPING.md`); the combination and the
live visualization are the product.

## Operating Context

- Runs as a CLAP and VST3 plug-in inside a DAW, before a drum sampler or drum rack. It emits MIDI
  events, never audio; inserted without an instrument downstream it is silent by design.
- Linux is the primary target. Windows builds are experimental. No macOS bundle is produced.
- Host coverage: bundle build and Carla CLAP/VST3 discovery pass; REAPER and Carla load, editor
  and MIDI paths are still untested (see `docs/HOST_VERIFICATION.md`). The shipped 1.0.x identity
  is `com.nrumseq.nrumseq` (CLAP) and `nrum-seq` (VST3).
- The editor is a fixed 860x768 logical-pixel window, pointer-first but fully keyboard-operable.
- Patterns are recalled with the project through host state; parameters are automatable; a loop
  can be exported as a Standard MIDI File.
- Real-time constraints: the audio/MIDI path must not allocate, lock, do file I/O or run
  unbounded loops inside `Plugin::process()`.

## Capabilities and Constraints

- 40 generators in nine families (rhythm, stochastic, chaos/physics, fractal, cellular, geometric,
  number theory, music theory, with the Acid family inherited from nelody seq).
- Eight lanes, each with note, velocity, density, phase and enable; GM-style defaults.
- Grid from 1 to a configurable 1024-step ceiling (64-1024, default 256).
- Per-step overlay for hit, probability, accent, ratchet (max 4), microtiming (positive delays
  only) and gate length, applied in display space so it survives Rotate, Mirror and Generate.
- Undo/redo for manual step edits; `FIT` and follow mode handle matrix navigation.
- Mode-aware Randomize with a scope mask over Mode, Length, Rhythm, Tone, Phase, Acid and Feel
  (a retired Clock bit is carried but ignored). The Acid group does not touch this plug-in's
  parameters.
- Four themes and built-in starting-point presets.
- Four LFOs still compute and keep existing links modulating, but their rack and matrix UI was
  removed: a project can carry links, yet none can be created or edited in the plug-in. Do not
  present LFO editing as available.
- No screen-reader tree: roles and labels are declared but the vendored Slint backend emits
  nothing. Keyboard operation and visible focus are complete. Treated as a decided gap for this
  release, not an open bug.
- Fixed 860x768 window that follows the host scale factor; no size preference of its own. MIDI
  export writes one loop into the plug-in process working directory rather than the DAW project
  path.

## Brand Commitments

- Name: **nrum seq**. It is the percussive sibling of **nelody seq**: same architecture and editor
  layout, with a drum matrix instead of a piano roll. That sibling relationship is binding.
- Author `ccne4mocc / feN`; license GPL-3.0-or-later; Linux-first.
- Voice: first-person, informal, self-taught and candid ("Enjoy :3"). Technical claims are stated
  plainly with their limits attached. Do not replace it with neutral product-marketing prose.
- The demo GIF (`assets/nrum-seq.gif`) and the existing README are the public face.

## Evidence on Hand

- `README.md` — download, motivation, highlights, keyboard map, editing, modes, build,
  compatibility, and a candid limitations list. The authoritative public description.
- `docs/ARCHITECTURE.md` — layers, invariants, state ownership, UI contracts and measured costs.
- `docs/REFERENCE_MAPPING.md`, `docs/MODE_EXAMPLES.md` — where each generator came from and a
  starting point for every mode.
- `docs/ACCESSIBILITY.md` — what keyboard support covers and why screen readers do not.
- `docs/HOST_VERIFICATION.md`, `docs/RELEASE_CHECKLIST.md` — what has actually been tested.
- `assets/nrum-seq.gif`, `assets/og.png` — real captures of the running editor.
- A stored critique at `.impeccable/critique/2026-11-12T11-45-21Z__ui-app-slint.md` scoring the
  surface 22/40. Its target path is `_nDrums/ui/app.slint`, the project's former directory name
  (now `nrun/`), so it is a measurement of this repository.
- Absent: no user interviews, no analytics, no customer quotes, no pricing, no macOS evidence.
  Do not fabricate any of them.

## Product Principles

1. **Deterministic first.** For a given parameter snapshot and seed/variation, the pattern is the
   same; recall, automation and export depend on it.
2. **One math, two surfaces.** The editor preview and the scheduler call the same generator. A
   UI-only variant of a mode is a defect.
3. **Real-time safety is not negotiable.** The generator path stays allocation-free and bounded.
4. **Show the algorithm.** The drawing of the pattern is the product's signature, not decoration;
   the strongest ideas are the ones you can watch.
5. **Keep host truth.** Stable parameter IDs, honest bounds, and state that survives recall and
   automation outrank convenience refactors.

## Accessibility & Inclusion

Keyboard operation is complete and tested: Tab order, visible focus ring, window shortcuts, and a
matrix cursor with a status-strip readout. Reduced motion is a user-global preference that zeroes
panel tweens and slides. Screen-reader output is a decided gap for this release. The fixed
860x768 editor is the current constraint; only the host scale factor is applied.
