use glutin::dpi::PhysicalPosition;
use std::ops::{Add, Mul, Sub};

#[derive(Default, Debug, Clone, Copy)]
pub struct GlPos {
    pub x: f32,
    pub y: f32,
}

impl GlPos {
    pub fn from_pixel(width: u32, height: u32, pix: PixelPos) -> GlPos {
        let x = pix.x as f32 * 2.0 / width as f32 - 1.0;
        let y = -(pix.y as f32 * 2.0 / height as f32 - 1.0);
        GlPos { x, y }
    }

    pub fn from_stroke(
        sip: StrokePos,
        zoom: f32,
        width: u32,
        height: u32,
        stroke: StrokePos,
    ) -> GlPos {
        let diff_x = stroke.x - sip.x;
        let diff_y = stroke.y - sip.y;
        GlPos {
            x: diff_x * zoom,
            y: diff_y * zoom * (height as f32 / width as f32),
        }
    }
}

impl Sub for GlPos {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        GlPos {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PixelPos {
    pub x: isize,
    pub y: isize,
}

impl PixelPos {
    #[inline]
    pub fn from_stroke(sip: StrokePos, zoom: f32, pos: StrokePos) -> Self {
        let diff = pos - sip;
        let screen_x = zoom * diff.x;
        let screen_y = zoom * -diff.y;
        PixelPos {
            x: screen_x as isize,
            y: screen_y as isize,
        }
    }

    #[inline]
    pub fn from_physical_position(pos: PhysicalPosition<f64>) -> Self {
        PixelPos {
            x: pos.x as isize,
            y: pos.y as isize,
        }
    }
}

impl Sub for PixelPos {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        PixelPos {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct StrokePos {
    pub x: f32,
    pub y: f32,
}

impl StrokePos {
    pub fn from_physical_position(sip: StrokePos, zoom: f32, p: PhysicalPosition<f64>) -> Self {
        let x = p.x as f32 / zoom;
        let y = p.y as f32 / zoom;
        StrokePos {
            x: sip.x + x,
            y: sip.y - y,
        }
    }

    pub fn from_pixel_pos(sip: StrokePos, zoom: f32, p: PixelPos) -> Self {
        let x = p.x as f32 / zoom;
        let y = p.y as f32 / zoom;
        StrokePos {
            x: sip.x + x,
            y: sip.y - y,
        }
    }

    pub fn from_gl(sip: StrokePos, zoom: f32, gl: GlPos) -> StrokePos {
        let diff_x = gl.x / zoom;
        let diff_y = gl.y / zoom;
        let x = diff_x + sip.x;
        let y = diff_y + sip.y;
        StrokePos { x, y }
    }
}

impl From<crate::StrokePoint> for StrokePos {
    fn from(p: crate::StrokePoint) -> Self {
        p.pos
    }
}

impl Mul<StrokePos> for f32 {
    type Output = StrokePos;
    fn mul(self, rhs: StrokePos) -> Self::Output {
        StrokePos {
            x: rhs.x * self,
            y: rhs.y * self,
        }
    }
}

impl Mul<f32> for StrokePos {
    type Output = StrokePos;
    fn mul(self, rhs: f32) -> Self::Output {
        StrokePos {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Add for StrokePos {
    type Output = StrokePos;
    fn add(self, rhs: Self) -> Self::Output {
        StrokePos {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for StrokePos {
    type Output = StrokePos;
    fn sub(self, rhs: Self) -> Self::Output {
        StrokePos {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
