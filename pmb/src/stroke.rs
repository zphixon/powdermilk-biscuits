use crate::{
    graphics::{Color, ColorExt, StrokePoint},
    StrokeBackend,
};

#[rustfmt::skip]
#[derive(derive_disk::Disk)]
pub struct Stroke<S>
where
    S: StrokeBackend,
{
    pub points: Vec<StrokePoint>,
    pub pressure: Vec<f32>,
    pub color: Color,
    pub brush_size: f32,
    pub erased: bool,

    #[disk_skip] pub backend: Option<S>,
    #[disk_skip] pub done: bool,
}

impl<S> Default for Stroke<S>
where
    S: StrokeBackend,
{
    fn default() -> Self {
        Self {
            points: Default::default(),
            pressure: Default::default(),
            color: Color::WHITE,
            brush_size: crate::DEFAULT_BRUSH as f32,
            erased: false,
            backend: None,
            done: false,
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
    pub fn with_points(points: Vec<StrokePoint>, color: Color) -> Self {
        Self {
            pressure: std::iter::repeat(1.).take(points.len()).collect(),
            points,
            color,
            backend: None,
            ..Default::default()
        }
    }

    pub fn points_as_bytes(&self) -> &[u8] {
        unsafe {
            let points_flat = std::slice::from_raw_parts(
                self.points().as_ptr() as *const f32,
                self.points().len() * 2,
            );

            std::slice::from_raw_parts(
                points_flat.as_ptr() as *const u8,
                points_flat.len() * std::mem::size_of::<f32>(),
            )
        }
    }

    pub fn pressure_as_bytes(&self) -> &[u8] {
        unsafe {
            let points_flat = std::slice::from_raw_parts(
                self.pressure.as_ptr() as *const f32,
                self.points().len(),
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

    pub fn points(&self) -> &[StrokePoint] {
        &self.points
    }

    pub fn points_mut(&mut self) -> &mut Vec<StrokePoint> {
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
        F: FnMut(&[u8], &[u8]) -> S,
    {
        let backend = with(self.points_as_bytes(), self.pressure_as_bytes());
        self.backend = Some(backend);
    }

    pub fn is_dirty(&self) -> bool {
        self.backend().is_none() || self.backend().unwrap().is_dirty()
    }

    pub fn add_point(&mut self, stylus: &crate::Stylus) {
        self.pressure.push(stylus.pressure);
        self.points_mut().push(StrokePoint {
            x: stylus.pos.x,
            y: stylus.pos.y,
        });

        if let Some(backend) = self.backend_mut() {
            backend.make_dirty();
        }
    }

    pub fn finish(&mut self) {
        self.done = true;
    }
}
