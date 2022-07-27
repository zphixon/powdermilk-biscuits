use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub type Color = [u8; 3];

pub trait ColorExt {
    const WHITE: [u8; 3] = [0xff, 0xff, 0xff];
    const BLACK: [u8; 3] = [0x00, 0x00, 0x00];

    fn grey(level: f32) -> Color {
        [
            (level * 0xff as f32) as u8,
            (level * 0xff as f32) as u8,
            (level * 0xff as f32) as u8,
        ]
    }

    fn to_float(&self) -> [f32; 3];
}

impl ColorExt for Color {
    fn to_float(&self) -> [f32; 3] {
        [
            self[0] as f32 / 255.,
            self[1] as f32 / 255.,
            self[2] as f32 / 255.,
        ]
    }
}

pub fn circle_points(radius: f32, num_segments: usize) -> Vec<f32> {
    let mut segments = Vec::with_capacity(num_segments);

    let mut angle = 0.0;
    let segments_f32 = num_segments as f32;
    for _ in 0..num_segments {
        let d_theta = std::f32::consts::TAU / segments_f32;
        angle += d_theta;
        let (x, y) = angle.sin_cos();
        segments.push(x * radius);
        segments.push(y * radius);
    }

    segments
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PixelPos {
    pub x: f32,
    pub y: f32,
}

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct StrokePos {
    pub x: f32,
    pub y: f32,
}

pub fn xform_point_to_pos(origin: StrokePoint, stroke: StrokePoint) -> StrokePos {
    let x = stroke.x - origin.x;
    let y = stroke.y - origin.y;
    StrokePos { x, y }
}

impl Display for PixelPos {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.02},{:.02}", self.x, self.y)
    }
}

impl Display for StrokePoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.02},{:.02}", self.x, self.y)
    }
}

impl Display for StrokePos {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.02},{:.02}", self.x, self.y)
    }
}
