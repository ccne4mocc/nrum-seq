# Migration from nih-plug to nice-plug

Target: `AlexCharlton/nice-plug`, pinned to commit `50fed0e` (2026-05-29), `default-features = false, features = ["vst3"]`.
Migrated from nih-plug `de42101`. Rust 1.88+ (nice-plug itself needs 1.87, edition 2024; nih-plug needed 1.80).

## Why a git pin and not crates.io
The linked repository still has nih-plug's editor model (`Editor::spawn(parent, ctx)`, `raw-window-handle 0.5`),
which is what `vendor/slint-baseview` is built on. crates.io `nice-plug 0.4.x` renamed `initialize` to `activate`,
replaced the editor with a `baseview 0.4` `SpawnedEditor`/`HostCallbacks` model, and would need a new Slint adapter.
`version = "0.2"` resolves to an older, third API. `nice_plug_xtask` does not exist on crates.io (the crate is
`nice-plug-xtask`, 0.1.x).

## What changed in this project
- `nih_plug` -> `nice_plug`; `nih_export_clap!/nih_export_vst3!` -> `nice_export_clap!/nice_export_vst3!`; xtask uses `nice-plug-xtask`.
- No other change to the `Plugin`, `Params`, `Editor`, `ProcessContext` or `NoteEvent` usage was needed.
- Existing projects keep working (verified for CLAP, see below): same plug-in id, same 51 parameter ids/ranges/defaults, same state format.
- `.github/workflows/windows.yml`: artifact path fixed to `target/bundled` (the xtask writes there regardless of `--target`; this was already true under nih-plug).

## Things that change when moving from nih-plug to nice-plug
- Crate and macro names: `nih_plug*` -> `nice-plug*`, `nih_export_*!` -> `nice_export_*!`, `cargo_nih_plug` -> `cargo-nice-plug`.
- Logging: the `nih_log` crate became `nice-log`; the environment variable is `NICE_LOG` (was `NIH_LOG`). This plug-in calls no framework log macro.
- Crate layout: core traits/types moved into `nice-plug-core`; `nice-plug` holds the wrappers and macros.
- Minimum Rust 1.80 -> 1.87; the framework uses edition 2024 (this project stays on 2021).
- VST3 wrapper rewritten on the `vst3` crate (MIT OR Apache-2.0) instead of `vst3-sys` (GPLv3). nih-plug's README says VST3 builds had to comply with GPLv3 because of those bindings; nice-plug's README does not carry that statement. This project is GPL-3.0-or-later regardless. Not legal advice.
- Because the VST3 wrapper is new code, host behaviour can differ even though the plug-in API is the same. Only CLAP has been exercised here.
- `assert_no_alloc` -> `nice-assert-no-alloc` (feature name `assert_process_allocs` is unchanged).
- `nih_plug_vizia` no longer exists (this project uses Slint). `nice-plug-egui` and `nice-plug-iced` remain.
- baseview comes from crates.io; a `nice-plug` 0.4.x editor API is a separate, incompatible line (see above).

## Things nice-plug makes available
- Track name: `InitContext::track_name()`, `GuiContext::track_name()`, and the `Plugin::track_context_changed()` hook. Used for the header label.
- Host key forwarding: `Editor::on_virtual_key_from_host(VirtualKeyCode, is_down, Modifiers)` (VST3 only; the CLAP wrapper never calls it). Used so Space/Backspace/arrows reach text fields in hosts that steal them.
- Public transport fields and constructor: `Transport::new` and its `pos_*`/`loop_range_*` fields are `pub`, so `process()` logic can be unit-tested with a hand-built transport. `AsyncExecutor::new` is public too. (Not yet used here.)
- `unsafe_flush_denormals` feature; `simd` now forwards to the core crate.
- `assert_process_allocs`: debug builds abort if `process()` allocates (exposed here as a feature of the same name).
- Note: the framework's own CLAP wrapper logs (and allocates) when a host sends malformed events, which the guard reports. Use the guarded build for development only.

## Fixes taken from the deepseek diffs
- Oscillator panel draws the generator's published, rotated `orbit` and its real rank cutoff; the dead `oscillator::threshold` and its re-export are removed.
  The cutoff is one function (`view::oscillator_cutoff`) and is Invert-aware (with Invert on the hits are the samples *below* the cutoff). Regression test: `oscillator_panel_follows_the_rotated_ranked_curve` covers rotate x wave x invert x mirror.
  Known limit: with Hit Probability below 100% a dropped onset can move the drawn cutoff slightly.
- Not taken: `nice-plug = { version = "0.2" }`, `nice_plug_xtask = { version = "0.2" }` (wrong source / nonexistent crate), and the `xtask/src/main.rs` hunk (its context did not match the file).

## Verification (Linux x86_64)
- `cargo test --lib`: 102 passed, 0 failed, 1 ignored benchmark.
- `cargo xtask bundle nrum-sequencer` builds CLAP and VST3; exports present.
- `clap-validator` 0.3.2: 15 passed / 3 failed / 3 skipped, identical to the nih-plug build of the same source. The 3 failures are the validator's output-note-port queries; querying the extension directly succeeds on both builds.
- nih-plug build vs nice-plug build of the same source: all 51 CLAP parameter ids, names, ranges and defaults identical; default saved state byte-identical; a modified state saved by the nih build loads into the nice build (and back) with identical values and identical re-saved JSON. `hash_param_id` (used by VST3 too) is byte-identical in both frameworks.
- Not verified: loading in any DAW, the VST3 build beyond its exported symbols, the UI rendering. (There is no Windows CI in the repository.)
