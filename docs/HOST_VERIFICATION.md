# Host verification

This file records what has actually been exercised. A host is **verified** only when the
checklist in `RELEASE_CHECKLIST.md` has been run against the current build on that host/API/OS
combination. Everything else is **untested**, even when the plug-in bundles and scans.

## Build under test

- Version: 1.2.0
- Bundles: `target/bundled/nrum-seq.clap`, `target/bundled/nrum-seq.vst3`
- Build command: `cargo xtask bundle nrum-seq --release`
- Date: 2026-11-03

## Automated checks

| Check | Result | Notes |
|---|---|---|
| Carla CLAP discovery | **pass** | `carla-discovery-native CLAP target/bundled/nrum-seq.clap` reports name `nrum seq`, label `com.nrumseq.nrumseq`, category `utility`, and the expected CLAP extensions (`clap.gui`, `clap.params`, `clap.latency`, …). |
| Carla VST3 discovery | **fail** | `carla-discovery-native VST3 target/bundled/nrum-seq.vst3` hits `Carla assertion failure: "res == V3_OK"` in `carla-discovery.cpp`. This may be Carla's scanner rather than the plug-in; it is not evidence that the VST3 is broken. |
| CLAP validator | **partial** | `clap-validator 0.4.1`: 29 passed, 11 skipped, 3 `state-reproducibility-*` failed, and `state-invalid-random` crashed with an absurd allocation from random JSON. The same failures reproduce on commit `8d7257c` without the edit-overlay changes, so they are framework/state-handling issues, not regressions from this work. |
| VST3 validator | **not run** | `pluginval` is not installed on the test machine. |

## Manual host matrix

| Host | Version | OS | API | Status | Notes |
|---|---|---|---|---|---|
| REAPER | 7.83 | Linux x86_64 | CLAP | untested | Bundle builds; not yet scanned/loaded interactively. |
| REAPER | 7.83 | Linux x86_64 | VST3 | untested | Bundle builds; not yet scanned/loaded interactively. |
| Bitwig Studio | installed | Linux x86_64 | CLAP | untested | Not exercised. |
| Carla | installed | Linux x86_64 | CLAP | untested | Discovery passes; load/editor/MIDI not exercised. |
| Carla | installed | Linux x86_64 | VST3 | untested | Discovery fails in Carla; host load not exercised. |
| REAPER | 7.83 | Windows x64 under Wine 11.5 | CLAP | **verified on 2026-10-28** | Cross-compiled MinGW build scans, loads, processes and shows the full editor. |
| REAPER | 7.83 | Windows x64 under Wine 11.5 | VST3 | **verified on 2026-10-28** | Same build; editor unbridged. |
| REAPER | 7.83 | real Windows x64 | CLAP/VST3 | untested | No real Windows machine available. |
| REAPER | 7.83 | Windows x64 | VST3 | **known failure in bridge mode** | The editor does not appear when REAPER bridges the VST3. Run unbridged until the editor-parenting path is investigated. |
| macOS | — | macOS | CLAP/VST3 | untested | No macOS build has been produced or loaded. |

## Manual checklist per host/API

Run every row in `RELEASE_CHECKLIST.md` and record `pass`, `fail` or `n/a` with a note. Do not
mark a host verified from a successful scan alone.

## Reproduce the automated checks

```sh
cargo xtask bundle nrum-seq --release
/usr/lib/carla/carla-discovery-native CLAP target/bundled/nrum-seq.clap
/usr/lib/carla/carla-discovery-native VST3 target/bundled/nrum-seq.vst3
```

If `clap-validator` is available:

```sh
clap-validator validate target/bundled/nrum-seq.clap
```

Use the unguarded release build for validation. The `assert_process_allocs` feature aborts when
nice-plug's own wrapper logs malformed host events from the audio thread; that is framework
behaviour, not this plug-in's code.

## State compatibility warning

The CLAP state tests fail on the pre-edit baseline as well. Until the framework state path is
investigated, do not claim CLAP project/preset state compatibility. Verify save/load manually in
REAPER and Bitwig, and treat `state-invalid-random` as an upstream hardening issue. VST3 state
loading has not been isolated from this.
