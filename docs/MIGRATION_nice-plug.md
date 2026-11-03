# Why nrum seq is pinned to the nice-plug fork

This is a short maintenance note, not a release log. Current verification results live in
[`HOST_VERIFICATION.md`](HOST_VERIFICATION.md).

## Current pin

`Cargo.toml` and `xtask/Cargo.toml` use:

```toml
nice-plug = { git = "https://github.com/ccne4mocc/nice-plug.git", rev = "65c87588ce2d915f3095b9c077447c23ce4406b0", default-features = false, features = ["vst3"] }
```

The fork is based on `AlexCharlton/nice-plug` `50fed0e` and adds:

- bounded state loading;
- parameter rescans after state load;
- tolerant VST3 activation before `setupProcessing()` for Carla discovery.

The project migrated from nih-plug `de42101`; that migration established the current crate names,
macros and xtask layout.

## Why a git pin instead of crates.io

- The linked fork still uses the nih-plug style editor model
  (`Editor::spawn(parent, ctx)`, `raw-window-handle` 0.5), which is what `vendor/slint-baseview`
  targets.
- crates.io `nice-plug 0.4.x` replaced that editor with the `baseview 0.4`
  `SpawnedEditor`/`HostCallbacks` API. Supporting it would require a new Slint adapter.
- `version = "0.2"` resolves to an older API line.
- The xtask crate on crates.io is `nice-plug-xtask`, not `nice_plug_xtask`.

## What the migration changed here

- `nih_plug` → `nice_plug`; `nih_export_clap!` / `nih_export_vst3!` →
  `nice_export_clap!` / `nice_export_vst3!`; xtask uses `nice-plug-xtask`.
- Logging crate names changed; this plug-in does not call a framework log macro.
- `assert_no_alloc` became `nice-assert-no-alloc`; the `assert_process_allocs` feature name is
  unchanged.
- The Windows workflow writes its artifacts to `target/bundled` regardless of `--target`.
- The host identity and state format are unchanged by the framework swap; project compatibility
  should still be checked manually after parameter changes.

## Current status

Do not duplicate test counts or validator results here. Run the checklist in
[`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md) and record the outcome in
[`HOST_VERIFICATION.md`](HOST_VERIFICATION.md). The current release line passes `cargo test --lib`,
`clap-validator` and Carla discovery; real-host smoke tests remain the gating item.
