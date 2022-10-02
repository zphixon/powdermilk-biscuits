use crate::stroke::{Mesh, StrokeElement};
use lyon::{
    lyon_algorithms::path::Path,
    lyon_tessellation::{
        GeometryBuilderError, LineCap, LineJoin, StrokeOptions, StrokeTessellator,
        TessellationError, VertexBuffers,
    },
};
use std::sync::{
    mpsc::{self, Sender},
    Arc, RwLock, RwLockReadGuard,
};

pub enum TessResult {
    Mesh(Mesh),
    Split,
    None,
    Error,
}

pub enum TessMess {
    StartStroke(f32),
    AddPoint(StrokeElement),
    FinishStroke,
}

pub struct Tessellator {
    tx: Sender<TessMess>,
    rw: Arc<RwLock<TessResult>>,
}

// when tessellator thread receives TessMess::AddPoint
// - add the point to the buffer
// - re-tessellate
// - update the mesh so it can be buffered to the GPU

pub fn asdf() -> Tessellator {
    let (tx, rx) = mpsc::channel();
    let rw = Arc::new(RwLock::<TessResult>::new(TessResult::None));
    let mesh = Arc::clone(&rw);

    // TODO join handle?
    std::thread::spawn(move || {
        let mut brush_size = crate::DEFAULT_BRUSH as f32;
        let mut points = Vec::new();

        let mut tesselator = StrokeTessellator::new();
        let stroke_options = StrokeOptions::default()
            .with_line_cap(LineCap::Round)
            .with_line_join(LineJoin::Round)
            .with_tolerance(0.001)
            .with_variable_line_width(0);

        while let Ok(msg) = rx.recv() {
            match msg {
                TessMess::StartStroke(new_brush_size) => {
                    brush_size = new_brush_size;
                    points.clear();
                }

                TessMess::AddPoint(point) => {
                    points.push(point);

                    use lyon::geom::point as point2d;
                    let mut path = Path::builder_with_attributes(1);
                    if let Some(first) = points.first() {
                        path.begin(
                            point2d(first.x, first.y),
                            &[first.pressure * brush_size * 2.],
                        );
                    }
                    points.iter().skip(1).for_each(|point| {
                        path.line_to(
                            point2d(point.x, point.y),
                            &[point.pressure * brush_size * 2.],
                        );
                    });
                    path.end(false);
                    let path = path.build();
                    let mut new_mesh = VertexBuffers::new();
                    let mut builder =
                        lyon::lyon_tessellation::geometry_builder::simple_builder(&mut new_mesh);

                    match tesselator.tessellate_path(&path, &stroke_options, &mut builder) {
                        Ok(()) => {
                            *mesh.write().unwrap() = TessResult::Mesh(new_mesh);
                        }

                        Err(TessellationError::GeometryBuilder(
                            GeometryBuilderError::TooManyVertices,
                        )) => {
                            *mesh.write().unwrap() = TessResult::Split;
                        }

                        // ...,
                        Err(err) => {
                            log::error!("{}", err);
                            *mesh.write().unwrap() = TessResult::Error;
                        }
                    }
                }

                TessMess::FinishStroke => {
                    points.clear();
                }
            }
        }
    });

    Tessellator { tx, rw }
}

impl Tessellator {
    pub fn add_point(&self, point: StrokeElement) {
        self.tx.send(TessMess::AddPoint(point)).unwrap();
    }

    pub fn start_stroke(&self, brush_size: f32) {
        self.tx.send(TessMess::StartStroke(brush_size)).unwrap();
    }

    pub fn finish_stroke(&self) {
        self.tx.send(TessMess::FinishStroke).unwrap();
    }

    pub fn mesh(&self) -> RwLockReadGuard<TessResult> {
        self.rw.read().unwrap()
    }
}
