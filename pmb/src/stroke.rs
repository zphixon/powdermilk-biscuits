use bspline::BSpline;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(packed)]
pub struct StrokeElement {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

impl std::ops::Add for StrokeElement {
    type Output = StrokeElement;
    fn add(self, rhs: Self) -> Self::Output {
        StrokeElement {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            pressure: self.pressure,
        }
    }
}

impl std::ops::Mul<f32> for StrokeElement {
    type Output = StrokeElement;
    fn mul(self, rhs: f32) -> Self::Output {
        StrokeElement {
            x: self.x * rhs,
            y: self.y * rhs,
            pressure: self.pressure,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct DiskPart {
    pub points: Vec<StrokeElement>,
    pub color: crate::graphics::Color,
    pub brush_size: f32,
    pub style: StrokeStyle,
    pub erased: bool,
}

#[derive(Debug)]
pub struct Stroke<S> {
    pub disk: DiskPart,
    pub spline: Option<BSpline<StrokeElement, f32>>,
    pub backend: Option<S>,
}

impl<S> Default for Stroke<S> {
    fn default() -> Self {
        Self {
            disk: DiskPart::default(),
            spline: None,
            backend: None,
        }
    }
}

impl<S> Clone for Stroke<S> {
    fn clone(&self) -> Self {
        Stroke {
            disk: DiskPart {
                points: self.disk.points.clone(),
                color: self.disk.color,
                brush_size: self.disk.brush_size,
                style: self.disk.style,
                erased: self.disk.erased,
            },
            spline: self.spline.clone(),
            backend: None,
        }
    }
}

impl<S> Stroke<S> {
    pub const DEGREE: usize = 3;

    pub fn calculate_spline(&mut self) {
        if self.disk.points.len() > Self::DEGREE {
            let points = [self.disk.points.first().cloned().unwrap(); Stroke::<()>::DEGREE]
                .into_iter()
                .chain(self.disk.points.iter().cloned())
                .chain([self.disk.points.last().cloned().unwrap(); Stroke::<()>::DEGREE])
                .map(|point| point.into())
                .collect::<Vec<StrokeElement>>();

            let knots = std::iter::repeat(())
                .take(points.len() + Self::DEGREE + 1)
                .enumerate()
                .map(|(i, ())| i as f32)
                .collect::<Vec<_>>();

            self.spline = Some(BSpline::new(Self::DEGREE, points, knots));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, evc_derive::EnumVariantCount, Serialize, Deserialize)]
#[repr(usize)]
#[allow(dead_code)]
pub enum StrokeStyle {
    Lines,
    Circles,
    CirclesPressure,
    Points,
    Spline,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        StrokeStyle::Lines
    }
}
