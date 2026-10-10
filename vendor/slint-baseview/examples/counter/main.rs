//! Minimal standalone example for `slint-baseview`.
//!
//! Opens a Slint UI in a top-level baseview window (no plugin host) using
//! [`SlintWindow::open_blocking`], wires a button to a shared counter, and pushes
//! the count into the UI each frame.
//!
//! Run with: `cargo run --example counter`

use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc,
};

use slint_baseview::{default_gl_config, Size, SlintWindow, WindowOpenOptions, WindowScalePolicy};

// Pull in the Slint component generated from `app.slint` (see build.rs).
slint::include_modules!();

fn main() {
    // Shared state between the UI callback and the per-frame update.
    let count = Arc::new(AtomicI32::new(0));

    let options = WindowOpenOptions {
        title: "slint-baseview counter".to_string(),
        size: Size::new(360.0, 240.0),
        scale: WindowScalePolicy::SystemScaleFactor,
        gl_config: Some(default_gl_config()),
    };

    let setup_count = count.clone();
    let update_count = count.clone();

    SlintWindow::open_blocking(
        options,
        // Component factory.
        || AppWindow::new(),
        // setup: runs once after the component is shown. Register UI callbacks.
        move |handler, _window| {
            let count = setup_count.clone();
            handler.component().on_increment(move || {
                count.fetch_add(1, Ordering::Relaxed);
            });
        },
        // update: runs every frame. Push the latest count into the UI.
        move |handler, _window| {
            handler
                .component()
                .set_counter(update_count.load(Ordering::Relaxed));
        },
        // on_resize: nothing to do in a standalone window.
        |_w, _h| {},
    );
}
