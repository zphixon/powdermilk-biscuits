use std::fmt::{Display, Formatter};

pub type Color = [u8; 3];

pub trait ColorExt {
    const WHITE: Color = [0xff, 0xff, 0xff];
    const BLACK: Color = [0x00, 0x00, 0x00];
    const NICE_RED: Color = [255, 166, 166];
    const NICE_GREEN: Color = [166, 255, 190];
    const PMB: Color = [0x50, 0x4d, 0x42];

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

pub fn circle_points(radius: f32, num_points: usize) -> Vec<f32> {
    let mut points = Vec::with_capacity(num_points + 2);

    let dtheta = std::f32::consts::TAU / (num_points) as f32;
    let mut theta: f32 = 0.;

    for _ in 0..num_points + 1 {
        let (sin, cos) = theta.sin_cos();
        points.push(cos * radius);
        points.push(sin * radius);

        theta += dtheta;
    }

    points.push(radius);
    points.push(0.);

    points
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PixelPos {
    pub x: f32,
    pub y: f32,
}

#[derive(Default, Debug, Clone, Copy, derive_disk::Disk)]
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

pub fn xform_pos_to_point(origin: StrokePoint, stroke: StrokePos) -> StrokePoint {
    let x = stroke.x + origin.x;
    let y = stroke.y + origin.y;
    StrokePoint { x, y }
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
