// Compiles the Slint UI used by the `counter` example. Only the example needs
// this; the library itself has no .slint files.
fn main() {
    slint_build::compile("examples/counter/app.slint").unwrap();
}
