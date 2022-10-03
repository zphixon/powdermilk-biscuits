use crate::{
    error::PmbError,
    graphics::{Color, ColorExt, StrokePos},
    StrokeBackend,
};
use lyon::{
    lyon_tessellation::{StrokeOptions, StrokeTessellator, VertexBuffers},
    math::Point,
};

#[derive(Default, Debug, Clone, Copy, pmb_macros::Disk, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct StrokeElement {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

impl From<StrokeElement> for StrokePos {
    fn from(elt: StrokeElement) -> StrokePos {
        StrokePos { x: elt.x, y: elt.y }
    }
}

impl From<&StrokeElement> for StrokePos {
    fn from(elt: &StrokeElement) -> StrokePos {
        StrokePos { x: elt.x, y: elt.y }
    }
}

impl std::fmt::Display for StrokeElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.02},{:.02},{:.02}", self.x, self.y, self.pressure)
    }
}

pub type Mesh = VertexBuffers<Point, u16>;

#[rustfmt::skip]
#[derive(pmb_macros::Disk)]
pub struct Stroke<S>
where
    S: StrokeBackend,
{
    pub points: Vec<StrokeElement>,
    pub color: Color,
    pub brush_size: f32,

    #[skip] pub erased: bool,
    #[skip] pub visible: bool,
    #[skip] pub bottom_right: StrokePos,
    #[skip] pub top_left: StrokePos,
    #[skip] pub draw_tesselated: bool,
    #[skip] pub meshes: Vec<Mesh>,
    #[skip] pub backend: Option<S>,
    #[skip] pub done: bool,
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
            meshes: vec![Mesh::new()],
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
        Self {
            points,
            color,
            backend: None,
            ..Default::default()
        }
    }

    pub fn new(color: Color, brush_size: f32, draw_tesselated: bool) -> Self {
        Self {
            color,
            brush_size,
            backend: None,
            draw_tesselated,
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
        self.visible = false;
    }

    pub fn backend(&self) -> Option<&S> {
        self.backend.as_ref()
    }

    pub fn backend_mut(&mut self) -> Option<&mut S> {
        self.backend.as_mut()
    }

    pub fn is_dirty(&self) -> bool {
        self.backend().is_none() || self.backend().unwrap().is_dirty()
    }

    pub fn update_bounding_box(&mut self) {
        let mut top = f32::NEG_INFINITY;
        let mut bottom = f32::INFINITY;
        let mut right = f32::NEG_INFINITY;
        let mut left = f32::INFINITY;

        for point in self.vertices() {
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

    pub fn aabb(&self, top_left: StrokePos, bottom_right: StrokePos) -> bool {
        let screen_top_left = top_left;
        let screen_bottom_right = bottom_right;

        let this_left = self.top_left.x;
        let this_right = self.bottom_right.x;
        let this_top = self.top_left.y;
        let this_bottom = self.bottom_right.y;
        let other_left = screen_top_left.x;
        let other_right = screen_bottom_right.x;
        let other_top = screen_top_left.y;
        let other_bottom = screen_bottom_right.y;

        this_left <= other_right
            && this_right >= other_left
            && this_bottom <= other_top
            && this_top >= other_bottom
    }

    pub fn update_visible(&mut self, top_left: StrokePos, bottom_right: StrokePos) {
        self.visible = self.aabb(top_left, bottom_right);
    }

    pub fn add_point(
        &mut self,
        stylus: &crate::Stylus,
        tesselator: &mut StrokeTessellator,
        options: &StrokeOptions,
    ) -> bool {
        let mut should_be_split = false;
        let x = stylus.pos.x;
        let y = stylus.pos.y;

        self.points_mut().push(StrokeElement {
            x,
            y,
            pressure: stylus.pressure,
        });

        if self.points.len() >= 2 {
            use crate::error::ErrorKind;
            use lyon::lyon_tessellation::{GeometryBuilderError, TessellationError};
            should_be_split = matches!(
                self.rebuild_mesh(tesselator, options),
                Err(PmbError {
                    kind: ErrorKind::Tessellator(TessellationError::GeometryBuilder(
                        GeometryBuilderError::TooManyVertices,
                    )),
                    ..
                })
            );
        }

        if self.points.len() == 1 {
            self.top_left = stylus.pos;
            self.bottom_right = stylus.pos;
        }

        if let Some(backend) = self.backend_mut() {
            backend.make_dirty();
        }

        should_be_split
    }

    pub fn vertices(&self) -> impl Iterator<Item = &Point> {
        self.meshes.iter().flat_map(|mesh| mesh.vertices.iter())
    }

    pub fn rebuild_mesh(
        &mut self,
        tesselator: &mut StrokeTessellator,
        options: &StrokeOptions,
    ) -> Result<(), PmbError> {
        // TODO handle splitting here instead
        let mesh = crate::tess::tessellate(tesselator, options, self.brush_size, self.points())?;
        self.meshes[0] = mesh;
        self.update_bounding_box();

        Ok(())
    }

    pub fn finish(&mut self) {
        self.done = true;
    }
}
