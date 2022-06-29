use glutin::dpi::PhysicalPosition;
use std::ops::{Add, Mul, Sub};

#[derive(Default, Debug, Clone, Copy)]
pub struct ScreenPos {
    pub x: isize,
    pub y: isize,
}

impl ScreenPos {
    #[inline]
    pub fn from_stroke(pos: StrokePos, zoom: f32, screen_in_paper: StrokePos) -> Self {
        let diff = pos - screen_in_paper;
        let screen_x = zoom * diff.x;
        let screen_y = zoom * -diff.y;
        ScreenPos {
            x: screen_x as isize,
            y: screen_y as isize,
        }
    }

    #[inline]
    pub fn from_physical_position(pos: PhysicalPosition<f64>) -> Self {
        ScreenPos {
            x: pos.x as isize,
            y: pos.y as isize,
        }
    }
}

impl std::ops::Sub for ScreenPos {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        ScreenPos {
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
    pub fn from_physical_position(
        p: PhysicalPosition<f64>,
        zoom: f32,
        screen_in_paper: StrokePos,
    ) -> Self {
        let x = p.x as f32 / zoom;
        let y = p.y as f32 / zoom;
        StrokePos {
            x: screen_in_paper.x + x,
            y: screen_in_paper.y - y,
        }
    }

    pub fn from_screen_pos(p: ScreenPos, zoom: f32, screen_in_paper: StrokePos) -> Self {
        let x = p.x as f32 / zoom;
        let y = p.y as f32 / zoom;
        StrokePos {
            x: screen_in_paper.x + x,
            y: screen_in_paper.y - y,
        }
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
