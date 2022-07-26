#[cfg(feature = "gl")]
pub mod gl;
#[cfg(feature = "gl")]
pub use gl as run_impl;

#[cfg(feature = "wgpu")]
pub mod wgpu;
#[cfg(feature = "wgpu")]
pub use self::wgpu as run_impl;

#[cfg(not(any(feature = "gl", feature = "wgpu")))]
pub mod run_impl {
    pub fn main() {
        panic!("a backend must be selected, enable one of the features \"gl\" or \"wgpu\"");
    }
}
