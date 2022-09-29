use std::fmt::{Display, Formatter};
use winit::dpi::PhysicalPosition;

pub type Color = [f32; 3];

pub trait ColorExt {
    const WHITE: Color = [1., 1., 1.];
    const BLACK: Color = [0., 0., 0.];
    const NICE_RED: Color = [1., 0.65, 0.65];
    const NICE_GREEN: Color = [0.65, 1., 0.75];
    const PMB: Color = [0.314, 0.301, 0.259];

    fn grey(level: f32) -> Color {
        [level, level, level]
    }

    fn to_u8(&self) -> [u8; 3];
    fn from_u8(color: [u8; 3]) -> Self;
}

impl ColorExt for Color {
    fn from_u8(color: [u8; 3]) -> Color {
        [
            color[0] as f32 / 255.,
            color[1] as f32 / 255.,
            color[2] as f32 / 255.,
        ]
    }

    fn to_u8(&self) -> [u8; 3] {
        [
            (self[0] * 255.) as u8,
            (self[1] * 255.) as u8,
            (self[2] * 255.) as u8,
        ]
    }
}

/// disjoint set of lines. for use with gl_LINES or PrimitiveTopology::LineList
pub fn cursor_geometry(radius: f32, num_points: usize) -> Vec<f32> {
    let mut points = Vec::with_capacity(num_points + 2);

    let dtheta = std::f32::consts::TAU / (num_points as f32);
    let mut theta: f32 = 0.;

    for _ in 0..=(num_points * 2) {
        let (sin, cos) = theta.sin_cos();
        points.push(cos * radius);
        points.push(sin * radius);

        let (sin, cos) = (theta + dtheta).sin_cos();
        points.push(cos * radius);
        points.push(sin * radius);

        theta += dtheta;
    }

    points
}

/// continuous set of points on a circle
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

macro_rules! coordinate_types {
    ($($Coord:ident),*) => {$(
        #[derive(Default, Debug, Clone, Copy, derive_disk::Disk)]
        pub struct $Coord {
            pub x: f32,
            pub y: f32,
        }

        impl Display for $Coord {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:.02},{:.02}", self.x, self.y)
            }
        }
    )*};
}

coordinate_types!(PixelPos, StrokePoint, StrokePos);

impl From<PhysicalPosition<f64>> for PixelPos {
    fn from(pos: PhysicalPosition<f64>) -> Self {
        Self {
            x: pos.x as f32,
            y: pos.y as f32,
        }
    }
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
