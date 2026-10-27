# Migration from nih-plug to nice-plug

Target: `AlexCharlton/nice-plug`, pinned to commit `50fed0e` (2026-05-29), `default-features = false, features = ["vst3"]`.

## Why a git pin and not crates.io
The linked repository still has nih-plug's editor model (`Editor::spawn(parent, ctx)`, `raw-window-handle 0.5`),
which is what `vendor/slint-baseview` is built on. crates.io `nice-plug 0.4.x` renamed `initialize` to `activate`,
replaced the editor with a `baseview 0.4` `SpawnedEditor`/`HostCallbacks` model, and would need a new Slint adapter.
`version = "0.2"` resolves to an older, third API. `nice_plug_xtask` does not exist on crates.io (the crate is
`nice-plug-xtask`, 0.1.x).

## Mechanical changes
- `nih_plug` -> `nice_plug` (`prelude`), `nih_export_clap!/nih_export_vst3!` -> `nice_export_clap!/nice_export_vst3!`.
- `xtask`: `nice-plug-xtask` git dependency; `fn main() -> nice_plug_xtask::Result<()>`.
- Nothing else in the `Plugin`, `Params`, `Editor`, `ProcessContext` or `NoteEvent` usage needed to change.

## Behaviour added with nice-plug
- Track name in the header (`initialize`, `track_context_changed`).
- Host-stolen editing keys routed into text fields (`Editor::on_virtual_key_from_host`, VST3 only).
- `assert_process_allocs` pass-through feature for debug builds.

## Fixes taken from the deepseek diffs
- Oscillator panel draws the generator's published, rotated `orbit` and its real rank cutoff; the dead
  `oscillator::threshold` and its re-export are removed. Regression test: `oscillator_panel_follows_the_rotated_ranked_curve`.
  Known limit: with Hit Probability below 100% a dropped onset can lift the drawn cutoff slightly.
- Renames in comments and docs.

## Deepseek changes not taken
- `nice-plug = { version = "0.2" }` and `nice_plug_xtask = { version = "0.2" }` (wrong source, wrong API / nonexistent crate).
- The `xtask/src/main.rs` hunk (its context does not match the real file).

## Verification
`cargo test --lib`: 100 passed, 1 ignored benchmark. `cargo xtask bundle ndrum` builds CLAP and VST3.
`clap-validator` 0.3.2: identical 15 passed / 3 failed / 3 skipped before and after migration (the failures are the validator's output-note-port queries; a direct ctypes query of the extension succeeds on both builds). With `assert_process_allocs`, well-formed processing tests pass; malformed-event tests abort because nice-plug's wrapper logs (and allocates) on the audio thread. Never loaded in a DAW.
