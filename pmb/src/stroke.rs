use crate::{graphics::Color, StrokeBackend};
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
    points: Vec<StrokeElement>,
    color: Color,
    brush_size: f32,
    style: StrokeStyle,
    erased: bool,
}

impl DiskPart {
    pub fn new(color: Color, brush_size: f32) -> Self {
        DiskPart {
            color,
            brush_size,
            ..Default::default()
        }
    }
}

impl<S> From<Stroke<S>> for DiskPart
where
    S: StrokeBackend,
{
    fn from(stroke: Stroke<S>) -> Self {
        stroke.disk
    }
}

#[derive(Debug)]
pub struct Stroke<S> {
    disk: DiskPart,
    spline: Option<BSpline<StrokeElement, f32>>,
    backend: Option<S>,
}

impl<S> Default for Stroke<S>
where
    S: StrokeBackend,
{
    fn default() -> Self {
        Self {
            disk: DiskPart::default(),
            spline: None,
            backend: None,
        }
    }
}

impl<S> Clone for Stroke<S>
where
    S: StrokeBackend,
{
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
}

impl<S> Stroke<S>
where
    S: StrokeBackend,
{
    pub fn with_disk(disk: DiskPart) -> Self {
        Self {
            disk,
            ..Default::default()
        }
    }

    pub fn with_points(points: Vec<StrokeElement>, color: Color) -> Self {
        Self {
            disk: DiskPart {
                points,
                color,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub unsafe fn as_bytes(&self) -> &[u8] {
        let points_flat = std::slice::from_raw_parts(
            self.points().as_ptr() as *const f32,
            self.points().len() * 3,
        );

        std::slice::from_raw_parts(
            points_flat.as_ptr() as *const u8,
            points_flat.len() * std::mem::size_of::<f32>(),
        )
    }

    pub fn new(color: Color, brush_size: f32) -> Self {
        Self {
            disk: DiskPart::new(color, brush_size),
            spline: None,
            backend: None,
        }
    }

    pub fn points(&self) -> &[StrokeElement] {
        &self.disk.points
    }

    pub fn points_mut(&mut self) -> &mut Vec<StrokeElement> {
        &mut self.disk.points
    }

    pub fn color(&self) -> Color {
        self.disk.color
    }

    pub fn brush_size(&self) -> f32 {
        self.disk.brush_size
    }

    pub fn erased(&self) -> bool {
        self.disk.erased
    }

    pub fn erase(&mut self) {
        self.disk.erased = true;
    }

    pub fn backend(&self) -> Option<&S> {
        self.backend.as_ref()
    }

    pub fn backend_mut(&mut self) -> &mut Option<S> {
        &mut self.backend
    }

    pub fn replace_backend_with<F>(&mut self, mut with: F)
    where
        F: FnMut(&[u8]) -> S,
    {
        let backend = with(unsafe { self.as_bytes() });
        self.backend = Some(backend);
    }

    pub fn is_dirty(&self) -> bool {
        self.backend().map(|b| b.is_dirty()).unwrap_or(true)
    }

    pub fn calculate_spline(&mut self) {
        if self.points().len() > Self::DEGREE {
            let points = [self.points().first().cloned().unwrap(); Stroke::<()>::DEGREE]
                .into_iter()
                .chain(self.points().iter().cloned())
                .chain([self.points().last().cloned().unwrap(); Stroke::<()>::DEGREE])
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
