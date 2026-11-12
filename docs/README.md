# Documentation index

Start with the architecture and the mode map; the rest is reference material, release process or
research.

## Core contracts

| Document | Contents |
|---|---|
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Runtime layers, scheduler, parameters, state and UI contracts |
| [`REFERENCE_MAPPING.md`](REFERENCE_MAPPING.md) | Where every mode, view and control came from |
| [`MODE_EXAMPLES.md`](MODE_EXAMPLES.md) | One starting point per mode |
| [`EDIT_OVERLAY.md`](EDIT_OVERLAY.md) | Per-step edit data model, precedence and scheduler behaviour |
| [`LFO_SYSTEM.md`](LFO_SYSTEM.md) | LFO signal model, targets, links, drag interaction and persistence |

## Release and verification

| Document | Contents |
|---|---|
| [`HOST_VERIFICATION.md`](HOST_VERIFICATION.md) | What has actually been tested, and what is still untested |
| [`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md) | Build, validator and host smoke-test checklist |
| [`CHANGELOG.md`](CHANGELOG.md) | Release notes |
| [`MIGRATION_nice-plug.md`](MIGRATION_nice-plug.md) | Why the nice-plug fork is pinned and what the migration changed |

## Research and backlog

| Document | Contents |
|---|---|
| [`ALGORITHM_RESEARCH.md`](ALGORITHM_RESEARCH.md) | Sources and design notes for implemented algorithm families |
| [`RESEARCH_FRACTAL_GEOMETRIC_CELLULAR.md`](RESEARCH_FRACTAL_GEOMETRIC_CELLULAR.md) | Future fractal, geometric and cellular candidates |
| [`WISHLIST.md`](WISHLIST.md) | Unordered ideas; not a roadmap |

Historical detail lives in Git history. This directory intentionally keeps one canonical document
per topic rather than separate plans and status reports.

## Demo capture

`assets/` holds the README GIF. It is rendered headlessly from the real Slint UI, not screen-captured:

```sh
cargo test --lib write_ui_frames -- --ignored --nocapture   # writes PPM frames to the temp dir
magick -delay 12 -loop 0 <tmp>/nrum-seq-frames/frame-*.ppm assets/nrum-seq.gif
```

The frame test feeds the UI a fixture pattern and a synthetic melody, then advances the playhead one
step per frame. Keep the frame count and delay so the loop closes seamlessly.

## Shared UI

`ui/shared.slint` holds the control and data definitions both products must agree on: `TipTouch`,
`HoverTip`, `FocusRing`, the modulation globals and their view structs, `Opt` and `ModeRow`.
`ui/app.slint` imports it and re-exports it, so Rust bindings stay generated from one file.

The file is copied verbatim into both repositories. `the_shared_ui_matches_its_pinned_hash`
fails when it changes, so the order is: edit, run the test, copy to the sibling repository, then
update the pin in both tests.
