use crate::{
    graphics::{Color, ColorExt},
    StrokeBackend,
};
use bspline::BSpline;

#[derive(Default, Debug, Clone, Copy)]
#[repr(packed)]
pub struct StrokeElement {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

impl bincode::Encode for StrokeElement {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        let x = self.x;
        let y = self.y;
        let pressure = self.pressure;
        x.encode(encoder)?;
        y.encode(encoder)?;
        pressure.encode(encoder)?;
        Ok(())
    }
}

impl bincode::Decode for StrokeElement {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        Ok(Self {
            x: bincode::Decode::decode(decoder)?,
            y: bincode::Decode::decode(decoder)?,
            pressure: bincode::Decode::decode(decoder)?,
        })
    }
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

#[derive(Debug, pmb_derive_disk::Disk)]
pub struct Stroke<S>
where
    S: StrokeBackend,
{
    points: Vec<StrokeElement>,
    color: Color,
    brush_size: f32,
    style: StrokeStyle,
    erased: bool,

    #[disk_skip]
    spline: Option<BSpline<StrokeElement, f32>>,
    #[disk_skip]
    backend: Option<S>,
}

impl<S> Default for Stroke<S>
where
    S: StrokeBackend,
{
    fn default() -> Self {
        Self {
            points: Default::default(),
            color: Color::WHITE,
            brush_size: crate::DEFAULT_BRUSH as f32,
            style: Default::default(),
            erased: false,
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
            points: self.points.clone(),
            color: self.color,
            brush_size: self.brush_size,
            style: self.style,
            erased: self.erased,
            spline: self.spline.clone(),
            backend: None,
        }
    }
}

impl StrokeBackend for () {
    fn is_dirty(&self) -> bool {
        false
    }

    fn make_dirty(&mut self) {}
}

impl Stroke<()> {
    pub const DEGREE: usize = 3;
}

impl<S> Stroke<S>
where
    S: StrokeBackend,
{
    pub fn with_points(points: Vec<StrokeElement>, color: Color) -> Self {
        Self {
            points,
            color,
            backend: None,
            ..Default::default()
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let points_flat = std::slice::from_raw_parts(
                self.points().as_ptr() as *const f32,
                self.points().len() * 3,
            );

            std::slice::from_raw_parts(
                points_flat.as_ptr() as *const u8,
                points_flat.len() * std::mem::size_of::<f32>(),
            )
        }
    }

    pub fn new(color: Color, brush_size: f32) -> Self {
        Self {
            color,
            brush_size,
            backend: None,
            ..Default::default()
        }
    }

    pub fn points(&self) -> &[StrokeElement] {
        &self.points
    }

    pub fn points_mut(&mut self) -> &mut Vec<StrokeElement> {
        &mut self.points
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn brush_size(&self) -> f32 {
        self.brush_size
    }

    pub fn erased(&self) -> bool {
        self.erased
    }

    pub fn erase(&mut self) {
        self.erased = true;
    }

    pub fn backend(&self) -> Option<&S> {
        self.backend.as_ref()
    }

    pub fn backend_mut(&mut self) -> Option<&mut S> {
        self.backend.as_mut()
    }

    pub fn replace_backend_with<F>(&mut self, mut with: F)
    where
        F: FnMut(&[u8]) -> S,
    {
        let backend = with(self.as_bytes());
        self.backend = Some(backend);
    }

    pub fn is_dirty(&self) -> bool {
        self.backend().is_none() || self.backend().unwrap().is_dirty()
    }

    pub fn calculate_spline(&mut self) {
        #[allow(non_upper_case_globals)]
        const degree: usize = Stroke::<()>::DEGREE;
        if self.points().len() > degree {
            let points = [self.points().first().cloned().unwrap(); degree]
                .into_iter()
                .chain(self.points().iter().cloned())
                .chain([self.points().last().cloned().unwrap(); degree])
                .collect::<Vec<StrokeElement>>();

            let knots = std::iter::repeat(())
                .take(points.len() + degree + 1)
                .enumerate()
                .map(|(i, ())| i as f32)
                .collect::<Vec<_>>();

            self.spline = Some(BSpline::new(degree, points, knots));
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, evc_derive::EnumVariantCount, bincode::Encode, bincode::Decode,
)]
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
