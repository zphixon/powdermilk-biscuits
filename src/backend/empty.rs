use crate::graphics::{PixelPos, StrokePoint, StrokePos};

pub fn pixel_to_ndc(_width: u32, _height: u32, _pos: PixelPos) {}

pub fn ndc_to_pixel(_width: u32, _height: u32, _pos: ()) -> PixelPos {
    Default::default()
}

pub fn ndc_to_stroke(_width: u32, _height: u32, _zoom: f32, _ndc: ()) -> StrokePoint {
    Default::default()
}

pub fn stroke_to_ndc(_width: u32, _height: u32, _zoom: f32, _point: StrokePoint) {}

pub fn xform_point_to_pos(_origin: StrokePoint, _stroke: StrokePoint) -> StrokePos {
    Default::default()
}

#[derive(Debug)]
pub struct StrokeBackend;
