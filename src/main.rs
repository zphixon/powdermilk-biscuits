#[cfg(feature = "gl")]
mod run_gl;
#[cfg(feature = "gl")]
fn main() {
    run_gl::main();
}

#[cfg(feature = "wgpu")]
mod run_wgpu;
#[cfg(feature = "wgpu")]
fn main() {
    run_wgpu::main();
}

#[cfg(not(any(feature = "gl", feature = "wgpu",)))]
fn main() {
    panic!("can only be built with gl for now");
}
