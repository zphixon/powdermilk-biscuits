#[cfg(feature = "gl")]
pub mod gl;
#[cfg(feature = "gl")]
pub use gl as backend_impl;

#[cfg(feature = "wgpu")]
pub mod wgpu;
#[cfg(feature = "wgpu")]
pub use self::wgpu as backend_impl;

#[cfg(not(any(feature = "gl", feature = "wgpu")))]
pub mod backend_impl {
    use crate::graphics::{PixelPos, StrokePoint, StrokePos};

    #[derive(Debug)]
    pub struct StrokeBackend;

    #[derive(Clone, Copy)]
    pub struct EmptyNdc;

    impl std::fmt::Display for EmptyNdc {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "EmptyNdc")
        }
    }

    pub fn pixel_to_ndc(_width: u32, _height: u32, _pos: PixelPos) -> EmptyNdc {
        EmptyNdc
    }

    pub fn ndc_to_pixel(_width: u32, _height: u32, _pos: EmptyNdc) -> PixelPos {
        Default::default()
    }

    pub fn ndc_to_stroke(_width: u32, _height: u32, _zoom: f32, _ndc: EmptyNdc) -> StrokePoint {
        Default::default()
    }

    pub fn stroke_to_ndc(_width: u32, _height: u32, _zoom: f32, _point: StrokePoint) -> EmptyNdc {
        EmptyNdc
    }

    pub fn xform_point_to_pos(_origin: StrokePoint, _stroke: StrokePoint) -> StrokePos {
        Default::default()
    }
}
