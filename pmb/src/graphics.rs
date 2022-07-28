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

#[derive(Default, Debug, Clone, Copy)]
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
