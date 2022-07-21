use glutin::dpi::PhysicalPosition;
use std::fmt::{Display, Formatter};

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

#[derive(Debug, Clone, Copy)]
pub struct NewGlPos {
    pub x: f32,
    pub y: f32,
}

pub fn physical_position_to_gl(width: u32, height: u32, pos: PhysicalPosition<f64>) -> NewGlPos {
    NewGlPos {
        x: (2.0 * pos.x as f32) / width as f32 - 1.0,
        y: -((2.0 * pos.y as f32) / height as f32 - 1.0),
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
}

pub fn gl_to_stroke(width: u32, height: u32, gl: NewGlPos) -> StrokePoint {
    StrokePoint {
        x: gl.x * width as f32,
        y: gl.y * height as f32,
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct StrokePos {
    pub x: f32,
    pub y: f32,
}

pub fn xform_stroke(gis: StrokePos, zoom: f32, stroke: StrokePoint) -> StrokePos {
    let dx = stroke.x - gis.x;
    let dy = stroke.y - gis.y;
    StrokePos {
        x: dx / zoom,
        y: dy / zoom,
    }
}

impl Display for NewGlPos {
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
