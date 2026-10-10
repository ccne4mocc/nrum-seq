# slint-baseview

A [baseview](https://github.com/RustAudio/baseview) backend for
[Slint](https://slint.dev) - run a Slint UI inside a baseview window.

This is most useful for audio plugin editors, where the GUI has to live inside a
window the host provides. It's the Slint counterpart to
[`egui-baseview`](https://codeberg.org/RustAudio/egui-baseview) and
[`iced_baseview`](https://github.com/BillyDM/iced_baseview): a generic
windowing/rendering layer with **no plugin-framework dependency**. Framework
integrations (e.g. `nice-plug-slint`) build on top of it.

## What it does

- Owns a baseview window and its render loop.
- Sets up an OpenGL + FemtoVG renderer and a custom Slint
  [`Platform`](https://docs.slint.dev/latest/docs/rust/slint/platform/)/`WindowAdapter`.
- Translates baseview mouse, keyboard, and resize events into Slint events.
- Drives your component each frame via a `setup` (once) and `update` (per-frame)
  closure.

## Usage

```rust
use slint_baseview::{SlintWindow, WindowOpenOptions, WindowScalePolicy, Size, default_gl_config};

let options = WindowOpenOptions {
    title: "My Plugin".into(),
    size: Size::new(400.0, 300.0),
    scale: WindowScalePolicy::SystemScaleFactor,
    gl_config: Some(default_gl_config()), // FemtoVG needs an OpenGL context
};

let _handle = SlintWindow::open_parented(
    &parent,                       // anything implementing raw-window-handle 0.5's HasRawWindowHandle
    options,
    || MyComponent::new(),         // component factory
    |_handler, _window| {          // setup: runs once after the component is shown
        // register Slint callbacks here
    },
    |handler, _window| {           // update: runs every frame
        handler.component().set_value(42.0);
    },
    |_w, _h| {},                   // on_resize: notified when the window size changes
);
```

For a standalone window (no plugin host) — handy for examples and testing — use
`SlintWindow::open_blocking(options, factory, setup, update, on_resize)`, which
opens a top-level window and blocks until it closes. See the runnable example:

```sh
cargo run --example counter
```

Inside the closures, the `SlintWindowHandler` gives you `component()`,
`window()` (the Slint window), `scale_factor()`, and resize control
(`resize`, `queue_resize` for use from Slint callbacks).

## A couple of things worth knowing

- **Two `raw-window-handle` versions.** Slint uses rwh 0.6 internally, but
  baseview 0.1.x's `open_parented` takes rwh **0.5**'s `HasRawWindowHandle`.
  `open_parented`'s `parent` is bound by the 0.5 trait (re-exported as
  `slint_baseview::HasRawWindowHandle`). If your parent handle comes from a
  framework using a different rwh version, wrap it in a small adapter type that
  implements the 0.5 trait (this is what `nice-plug-slint` does).
- **`keyboard-types` is pinned to 0.6** to match what baseview's `KeyboardEvent`
  uses; bumping it independently produces type mismatches.
- **Slint is pinned to `=1.16.1`** with the `renderer-femtovg`,
  `raw-window-handle-06`, and `compat-1-2` features.

## Status

Extracted from the `nice-plug-slint` adapter and generalized. Works on Windows
(developed there); macOS/Linux use the same baseview paths but want testing.

## License

ISC.