#[cfg(feature = "gl")]
mod run_gl;

#[cfg(feature = "gl")]
fn main() {
    run_gl::main();
}

#[cfg(not(feature = "gl"))]
fn main() {
    panic!("can only be built with gl for now");
}
