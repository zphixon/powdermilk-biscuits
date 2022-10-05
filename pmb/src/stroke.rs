use crate::{
    graphics::{Color, ColorExt, StrokePos},
    StrokeBackend,
};
use lyon::{
    lyon_tessellation::{
        GeometryBuilderError, StrokeOptions, StrokeTessellator, TessellationError, VertexBuffers,
    },
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

pub type MeshBuffer = VertexBuffers<Point, u16>;

pub struct Mesh {
    pub buffer: MeshBuffer,
    from: usize,
    to: usize,
}

impl Mesh {
    pub fn vertices(&self) -> &[Point] {
        &self.buffer.vertices
    }

    pub fn indices(&self) -> &[u16] {
        &self.buffer.indices
    }

    pub fn len(&self) -> usize {
        self.to - self.from
    }
}

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
            meshes: Vec::new(),
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
        max_points: Option<usize>,
    ) {
        let x = stylus.pos.x;
        let y = stylus.pos.y;

        self.points_mut().push(StrokeElement {
            x,
            y,
            pressure: stylus.pressure,
        });

        if self.points.len() >= 2 {
            self.rebuild_partial_mesh(tesselator, options, max_points);
        }

        if self.points.len() == 1 {
            self.top_left = stylus.pos;
            self.bottom_right = stylus.pos;
        }

        if let Some(backend) = self.backend_mut() {
            backend.make_dirty();
        }
    }

    pub fn vertices(&self) -> impl Iterator<Item = &Point> {
        self.meshes.iter().flat_map(|mesh| mesh.vertices().iter())
    }

    pub fn rebuild_entire_mesh(
        &mut self,
        tessellator: &mut StrokeTessellator,
        stroke_options: &StrokeOptions,
    ) {
        match crate::tess::tessellate(tessellator, stroke_options, self.brush_size, self.points()) {
            Ok(buffer) => self.meshes.push(Mesh {
                buffer,
                from: 0,
                to: self.points.len(),
            }),

            Err(err) if is_tmv(&err) => {
                log::warn!("have to split stroke (entire mesh)");
                self.meshes.clear();

                // start with two segments
                let mut num_segments = 2;
                'with_more_segments: loop {
                    // split the points into num_breaks segments
                    let per_segment = self.points.len() / num_segments;
                    log::info!(
                        "trying {} segments, {} points per segment",
                        num_segments,
                        per_segment,
                    );

                    let mut meshes = Vec::new();
                    for (i, (from, to, subset)) in (0..num_segments)
                        .map(|offset| {
                            let from = per_segment * offset;
                            let to = per_segment * (offset + 1);
                            (from, to, &self.points[from..to])
                        })
                        .enumerate()
                    {
                        // try tessellating the segment
                        match crate::tess::tessellate(
                            tessellator,
                            stroke_options,
                            self.brush_size,
                            subset,
                        ) {
                            // if it works, hooray
                            Ok(buffer) => {
                                log::debug!("got segment {}/{}", i, num_segments);
                                meshes.push(Mesh { buffer, from, to });
                            }

                            // if it's too many, try again with more segments
                            Err(err) if is_tmv(&err) => {
                                log::debug!("it didn't work ({}/{})", i, num_segments);
                                meshes.clear();
                                num_segments += 1;
                                continue 'with_more_segments;
                            }

                            Err(err) => {
                                log::error!("{}", err);
                                return;
                            }
                        }
                    }

                    // all the segments were tessellable (sp.?)
                    log::info!("tessellated with {} segments", num_segments);
                    self.meshes = meshes;
                    break;
                }
            }

            Err(err) => {
                log::error!("couldn't build mesh: {}", err,);
            }
        }

        self.update_bounding_box();
    }

    pub fn rebuild_partial_mesh(
        &mut self,
        tessellator: &mut StrokeTessellator,
        options: &StrokeOptions,
        max_points: Option<usize>,
    ) {
        let mut to_add = None;

        let split =
            |tessellator: &mut StrokeTessellator, to_add: &mut Option<Mesh>, subset: &Mesh| {
                match crate::tess::tessellate(
                    tessellator,
                    options,
                    self.brush_size,
                    &self.points[subset.to..],
                ) {
                    Ok(buffer) => {
                        *to_add = Some(Mesh {
                            buffer,
                            from: subset.to,
                            to: self.points.len(),
                        });
                    }

                    Err(err) => {
                        log::error!(
                            "couldn't tessellate last part {}..{}: {}",
                            subset.to,
                            self.points.len(),
                            err,
                        );
                    }
                }
            };

        match self.meshes.last_mut() {
            Some(subset) => {
                if max_points.is_some() && subset.len() > max_points.unwrap() {
                    log::warn!(
                        "have to split after {}..{} (max points reached)",
                        subset.from,
                        subset.to
                    );
                    split(tessellator, &mut to_add, subset);
                } else {
                    match crate::tess::tessellate(
                        tessellator,
                        options,
                        self.brush_size,
                        &self.points[subset.from..],
                    ) {
                        Ok(buffer) => {
                            subset.buffer = buffer;
                            subset.to = self.points.len();
                        }

                        Err(err) if is_tmv(&err) => {
                            log::warn!("have to split after {}..{}", subset.from, subset.to);
                            split(tessellator, &mut to_add, subset);
                        }

                        Err(err) => {
                            log::error!(
                                "couldn't tessellate {}..{}: {}",
                                subset.from,
                                subset.to,
                                err,
                            );
                        }
                    }
                }
            }

            None => {
                self.rebuild_entire_mesh(tessellator, options);
            }
        }

        if let Some(to_add) = to_add {
            self.meshes.push(to_add);
        }

        self.update_bounding_box();
    }

    pub fn finish(&mut self) {
        self.done = true;
    }
}

fn is_tmv(err: &TessellationError) -> bool {
    matches!(
        err,
        TessellationError::GeometryBuilder(GeometryBuilderError::TooManyVertices)
    )
}
