use crate::{
    graphics::{Color, ColorExt, PixelPos, StrokePoint, StrokePos},
    StrokeBackend,
};

#[derive(Default, Debug, Clone, Copy, derive_disk::Disk)]
#[repr(C)]
pub struct StrokeElement {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

impl pmb_tess::Point for StrokeElement {
    fn new(x: f32, y: f32) -> Self {
        StrokeElement {
            x,
            y,
            pressure: -1.,
        }
    }

    fn x(&self) -> f32 {
        self.x
    }

    fn y(&self) -> f32 {
        self.y
    }
}

#[rustfmt::skip]
#[derive(derive_disk::Disk)]
pub struct Stroke<S>
where
    S: StrokeBackend,
{
    pub points: Vec<StrokeElement>,
    pub color: Color,
    pub brush_size: f32,
    pub erased: bool,

    #[disk_skip] pub visible: bool,
    #[disk_skip] pub bottom_right: StrokePos,
    #[disk_skip] pub top_left: StrokePos,
    #[disk_skip] pub draw_tesselated: bool,
    #[disk_skip] pub mesh: Vec<StrokeElement>,
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
            color: Color::WHITE,
            brush_size: 0.01,
            erased: false,
            visible: true,
            bottom_right: StrokePos::default(),
            top_left: StrokePos::default(),
            draw_tesselated: true,
            mesh: Vec::new(),
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
    pub fn with_points(points: Vec<StrokeElement>, color: Color) -> Self {
        let mut this = Self {
            points,
            color,
            backend: None,
            ..Default::default()
        };
        this.update_bounding_box();
        this.generate_full_mesh();
        this
    }

    pub fn points_as_bytes(&self) -> &[u8] {
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

    pub fn mesh_as_bytes(&self) -> &[u8] {
        unsafe {
            let mesh_flat =
                std::slice::from_raw_parts(self.mesh.as_ptr() as *const f32, self.mesh.len() * 3);

            std::slice::from_raw_parts(
                mesh_flat.as_ptr() as *const u8,
                mesh_flat.len() * std::mem::size_of::<f32>(),
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

    fn points_mut(&mut self) -> &mut Vec<StrokeElement> {
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
        F: FnMut(&[u8], &[u8], usize) -> S,
    {
        let backend = with(
            self.points_as_bytes(),
            self.mesh_as_bytes(),
            self.mesh.len(),
        );
        self.backend = Some(backend);
    }

    pub fn is_dirty(&self) -> bool {
        self.backend().is_none() || self.backend().unwrap().is_dirty()
    }

    pub fn update_bounding_box(&mut self) {
        let mut top = f32::NEG_INFINITY;
        let mut bottom = f32::INFINITY;
        let mut right = f32::NEG_INFINITY;
        let mut left = f32::INFINITY;

        for point in self.mesh.iter() {
            if point.x < left {
                left = point.x;
            }

            if point.x > right {
                right = point.x;
            }

            if point.y > top {
                top = point.y;
            }

            if point.y < bottom {
                bottom = point.y;
            }
        }

        self.top_left = StrokePos { x: left, y: top };

        self.bottom_right = StrokePos {
            x: right,
            y: bottom,
        };
    }

    pub fn update_visible<B: crate::Backend>(
        &mut self,
        backend: B,
        origin: StrokePoint,
        zoom: f32,
        width: u32,
        height: u32,
    ) {
        use crate::graphics::xform_point_to_pos as xform;

        let screen_top_left = xform(
            origin,
            backend.pixel_to_stroke(width, height, zoom, PixelPos::default()),
        );

        let screen_bottom_right = xform(
            origin,
            backend.pixel_to_stroke(
                width,
                height,
                zoom,
                PixelPos {
                    x: width as f32,
                    y: height as f32,
                },
            ),
        );

        let left = self.top_left.x;
        let right = self.bottom_right.x;
        let top = self.top_left.y;
        let bottom = self.bottom_right.y;
        let screen_left = screen_top_left.x;
        let screen_right = screen_bottom_right.x;
        let screen_top = screen_top_left.y;
        let screen_bottom = screen_bottom_right.y;

        self.visible = left <= screen_right
            && right >= screen_left
            && bottom <= screen_top
            && top >= screen_bottom;
    }

    pub fn add_point(&mut self, stylus: &crate::Stylus) {
        let x = stylus.pos.x;
        let y = stylus.pos.y;

        self.points_mut().push(StrokeElement {
            x,
            y,
            pressure: stylus.pressure,
        });

        if self.points.len() >= 4 {
            self.generate_partial_mesh();
        }

        if self.points.len() == 1 {
            self.top_left = stylus.pos;
            self.bottom_right = stylus.pos;
        }

        if let Some(backend) = self.backend_mut() {
            backend.make_dirty();
        }
    }

    fn generate_partial_mesh(&mut self) {
        use pmb_tess::Hermite;
        let subset = &self.points[self.points.len() - 4..];

        let ribs = subset
            .flat_ribs(4, self.brush_size())
            .into_iter()
            .zip(subset.iter())
            .map(|(mut rib, stroke)| {
                rib.pressure = stroke.pressure;
                rib
            })
            .collect::<Vec<_>>();

        for point in ribs.iter() {
            let x = point.x;
            let y = point.y;
            if x > self.bottom_right.x {
                self.bottom_right.x = x;
            }

            if x < self.top_left.x {
                self.top_left.x = x;
            }

            if y > self.top_left.y {
                self.top_left.y = y;
            }

            if y < self.bottom_right.y {
                self.bottom_right.y = y;
            }
        }

        self.mesh.pop();
        self.mesh.pop();
        self.mesh.extend(ribs);
    }

    pub fn generate_full_mesh(&mut self) {
        use pmb_tess::Hermite;

        if self.points.is_empty() {
            return;
        }

        while self.points.len() < 4 {
            self.points.push(*self.points.last().unwrap());
        }

        let mut mesh = self
            .points
            .flat_ribs((self.points.len() + 3) * 2, self.brush_size())
            .into_iter()
            .collect::<Vec<_>>();

        mesh.chunks_mut(2)
            .zip(self.points().iter())
            .for_each(|(rib, point)| {
                rib.iter_mut()
                    .for_each(|rib_point| rib_point.pressure = point.pressure)
            });

        self.mesh = mesh;
        self.update_bounding_box();
    }

    pub fn finish(&mut self) {
        self.done = true;
    }
}
