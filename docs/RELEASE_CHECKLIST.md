# Release checklist

Run this against a clean checkout of the release commit. Record host/API/OS results in
`HOST_VERIFICATION.md`; a successful scan is not a pass.

## 1. Build and metadata

- [ ] `cargo test --lib` passes.
- [ ] `cargo xtask bundle nrum-seq --release` succeeds.
- [ ] `target/bundled/nrum-seq.clap` and `target/bundled/nrum-seq.vst3` exist.
- [ ] CLAP id is `com.nrumseq.nrumseq`; VST3 class id is `nrum-seq`.
- [ ] Version, vendor and the visible author/GitHub credit match `src/lib.rs` (`contact` and
      `Plugin`). No email or support URL should be baked into the plug-in metadata.
- [ ] `LICENSE` is GPL-3.0-or-later and every bundle contains the license/notice text the
      distribution channel requires.
- [ ] No `assert_process_allocs` feature is enabled in the release build.

## 2. Automated validation

- [ ] `clap-validator validate target/bundled/nrum-seq.clap` passes, if available. The expected
      result is 33 passed, 0 failed, 11 skipped.
- [ ] `pluginval` passes on the VST3, if available.
- [ ] `carla-discovery-native CLAP target/bundled/nrum-seq.clap` reports the expected name,
      label, category and extensions.
- [ ] `carla-discovery-native VST3 target/bundled/nrum-seq.vst3` reports the expected metadata.
- [ ] `ldd -r` on both binaries reports no unresolved symbols or missing libraries.

## 3. Host smoke test (each host/API/OS)

- [ ] Plug-in scan completes and the plug-in appears under the expected name/vendor.
- [ ] Load into an instrument track routed to a drum sampler or rack.
- [ ] Editor opens at the fixed 860×768 size and is not clipped.
- [ ] Editor closes and reopens; no stale text focus or stuck notes.
- [ ] Mode changes, Generate, Randomize and Randomize scope work.
- [ ] Host automation moves `mode`, `steps`, `density`, `gate` and a lane note.
- [ ] Tempo change while playing keeps the pattern on the host grid.
- [ ] Loop points and transport jumps do not leave hanging notes.
- [ ] Stop, reset and start produce NoteOffs for every active lane.
- [ ] Free-run mode works while the host transport is stopped.
- [ ] Lane enable suppresses only that lane; note and velocity changes take effect.
- [ ] Project save/load restores every parameter, `variation` and LFO links.
- [ ] Host preset save/load restores the same state.
- [ ] MIDI export writes one loop and the file opens in a MIDI editor.
- [ ] No audio is emitted by the plug-in itself.

## 4. Known limitations to document

- [ ] VST3 editor does not appear in REAPER bridge mode on Windows; run unbridged.
- [ ] No real Windows hardware or macOS build has been verified unless this release says so.
- [ ] MIDI export uses the process working directory by default; there is no host project-path
      API.
- [ ] The plug-in is a MIDI generator effect and needs an instrument after it.
- [ ] Runtime LFO modulation is not written back to host parameters or MIDI export.

## 5. Artifacts and notes

- [ ] Bundles are attached to the release with SHA-256 checksums.
- [ ] Release notes list behavior changes, host limitations and the exact verification status.
- [ ] `HOST_VERIFICATION.md` is updated with the release date and results.
