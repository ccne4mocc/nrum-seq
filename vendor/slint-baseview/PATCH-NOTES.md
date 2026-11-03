# slint-baseview (vendored patch)

This is a copy of `slint-baseview` 0.1.0 from crates.io with **two** changes, each marked
in `src/lib.rs` with a `LOCAL PATCH` comment.

## Why it is vendored

### 1. Mouse-wheel position

Mouse-wheel editing did not work on any control in the plug-in. `scroll-event` handlers
were present and correct on the mode list, the parameter sliders and the settings spin boxes,
but none of them ever fired.

The cause is upstream. `baseview::MouseEvent::WheelScrolled` carries only a delta — it has no
position field — so `slint-baseview` could not tell Slint where the wheel happened. It
dispatched:

```rust
WindowEvent::PointerScrolled { position: LogicalPosition::new(0.0, 0.0), delta_x, delta_y }
```

Slint hit-tests scroll events by position, so every wheel notch was delivered to whatever
sits in the top-left corner of the window. Hover and clicks were unaffected, because those
events do carry a position — which is why the UI looked correct while the wheel did nothing.

The crate already tracks the cursor: `last_cursor_pos` is updated on every `CursorMoved`, and
the button arms two lines above use it. The wheel arm simply hardcoded `(0.0, 0.0)` instead.
The fix is to use the field that was already there:

```rust
WindowEvent::PointerScrolled {
    position: *self.last_cursor_pos.borrow(),
    delta_x,
    delta_y,
}
```

### 2. Alt+wheel horizontal scrolling

Slint's `Flickable` swaps wheel axes for Shift only, and `ScrollView` does not expose wheel
modifiers to Slint code. To let Alt+wheel pan the pattern matrix horizontally — while plain
wheel keeps scrolling vertically — the wheel arm swaps the deltas when baseview reports Alt:

```rust
if modifiers.contains(keyboard_types::Modifiers::ALT) {
    std::mem::swap(&mut delta_x, &mut delta_y);
}
```

## Removing this

Drop the `path` in the root `Cargo.toml` and delete `vendor/`. Worth checking first whether a
released `slint-baseview` has fixed this upstream; if so, bump the version instead of
re-patching.

## Local trim

The vendored copy is trimmed to the library only: the `counter` example, its `build.rs`
(which compiled the example's `.slint` file on every build of this crate) and the crates.io
packaging metadata (`README.md`, `Cargo.toml.orig`, `.cargo_vcs_info.json`) are removed.
`src/lib.rs` and `src/translate.rs` stay byte-identical to the published crate apart from the
two input patches above, so the diff stays reviewable.