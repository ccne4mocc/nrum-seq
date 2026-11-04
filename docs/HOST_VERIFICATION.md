# Host verification

This file records what has actually been exercised. A host is **verified** only when the
checklist in `RELEASE_CHECKLIST.md` has been run against the current build on that host/API/OS
combination. Everything else is **untested**, even when the plug-in bundles and scans.

## Build under test

- Version: 1.0.1 (documentation-only release; code and results below are unchanged from 1.0.0)
- Bundles: `target/bundled/nrum-seq.clap`, `target/bundled/nrum-seq.vst3`
- Build command: `cargo xtask bundle nrum-seq --release`
- Date: 2026-11-03

## Automated checks

| Check | Result | Notes |
|---|---|---|
| `cargo test --lib` | **pass** | 233 passed, 2 ignored (2026-11-03). |
| Carla CLAP discovery | **pass** | `carla-discovery-native CLAP target/bundled/nrum-seq.clap` reports name `nrum seq`, label `com.nrumseq.nrumseq`, category `utility`, and the expected CLAP extensions (`clap.gui`, `clap.params`, `clap.latency`, …). |
| Carla VST3 discovery | **pass** | `carla-discovery-native VST3 target/bundled/nrum-seq.vst3` reports name `nrum seq`, label, maker and 84 input parameters. Requires the pinned nice-plug fork that defers activation until `setupProcessing()`. |
| CLAP validator | **pass** | `clap-validator 0.4.1`: **33 passed, 0 failed, 11 skipped**. The pinned nice-plug fork bounds malformed state lengths and rescans parameter values after a state load. |
| Shared-library resolution | **pass** | `ldd -r` on the `.clap` and VST3 `.so`: no unresolved symbols or missing libraries. Hard dependencies are libc/libm/libgcc/libfontconfig plus their transitive libraries. |
| VST3 validator | **not run** | `pluginval` is not installed on the test machine. |

## Manual host matrix

| Host | Version | OS | API | Status | Notes |
|---|---|---|---|---|---|
| REAPER | 7.83 | Linux x86_64 | CLAP | untested | Bundle builds; not yet scanned/loaded interactively. |
| REAPER | 7.83 | Linux x86_64 | VST3 | untested | Bundle builds; not yet scanned/loaded interactively. |
| Bitwig Studio | installed | Linux x86_64 | CLAP | untested | Not exercised. |
| Carla | installed | Linux x86_64 | CLAP | untested | Discovery passes; load/editor/MIDI not exercised. |
| Carla | installed | Linux x86_64 | VST3 | untested | Discovery passes; host load/editor/MIDI not exercised. |
| REAPER | 7.83 | Windows x64 under Wine 11.5 | CLAP | **verified on 2026-10-28** | Cross-compiled MinGW build scans, loads, processes and shows the full editor. |
| REAPER | 7.83 | Windows x64 under Wine 11.5 | VST3 | **verified on 2026-10-28** | Same build; editor unbridged. |
| REAPER | 7.83 | real Windows x64 | CLAP/VST3 | untested | No real Windows machine available. |
| REAPER | 7.83 | Windows x64 | VST3 | **known failure in bridge mode** | The editor does not appear when REAPER bridges the VST3. Run unbridged until the editor-parenting path is investigated. |
| macOS | — | macOS | CLAP/VST3 | untested | No macOS build has been produced or loaded. |

## Manual checklist per host/API

Run every row in `RELEASE_CHECKLIST.md` and record `pass`, `fail` or `n/a` with a note. Do not
mark a host verified from a successful scan alone.

The `Bundles` workflow builds Linux and Windows artifacts on `v*` tags and can also be dispatched
manually. MSVC artifacts are still unverified on real Windows hardware. The `CI` workflow runs
tests and checks on pushes to `main` and on pull requests; bundles are built separately.

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

## State compatibility

The pinned nice-plug fork bounds malformed state lengths, rescans parameters after state load and
tolerates VST3 activation before `setupProcessing()`. `clap-validator` now passes its state tests.
Project and preset save/load still need a manual round-trip in at least one CLAP and one VST3 host;
a passing validator is not a substitute for that check.
