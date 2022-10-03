use crate::stroke::{Mesh, StrokeElement};
use lyon::{
    lyon_algorithms::path::Path,
    lyon_tessellation::{
        GeometryBuilderError, LineCap, LineJoin, StrokeOptions, StrokeTessellator,
        TessellationError, VertexBuffers,
    },
};
use std::{
    collections::VecDeque,
    sync::{
        mpsc::{self, Sender},
        Arc, RwLock, RwLockReadGuard,
    },
    thread::JoinHandle,
};

pub enum TessResult {
    Mesh(Mesh),
    Error,
}

pub enum TessMess {
    StartStroke(f32),
    AddPoint(StrokeElement),
    FinishStroke,
}

pub struct Tessellator {
    tx: Sender<TessMess>,
    rw: Arc<RwLock<MeshQueue>>,
    #[allow(dead_code)]
    handle: JoinHandle<()>,
}

#[derive(Default)]
pub struct MeshQueue {
    queue: VecDeque<TessResult>,
}

impl MeshQueue {
    fn add(&mut self, result: TessResult) {
        self.queue.push_back(result);
    }

    pub fn next(&mut self) -> Option<TessResult> {
        self.queue.pop_front()
    }
}

pub fn tessellator() -> Tessellator {
    let (tx, rx) = mpsc::channel();
    let rw = Arc::new(RwLock::<MeshQueue>::new(MeshQueue::default()));
    let result = Arc::clone(&rw);

    let handle = std::thread::spawn(move || {
        let mut brush_size = crate::DEFAULT_BRUSH as f32;
        let mut points = Vec::new();

        let mut tesselator = StrokeTessellator::new();
        let stroke_options = StrokeOptions::default()
            .with_line_cap(LineCap::Round)
            .with_line_join(LineJoin::Round)
            .with_tolerance(0.001)
            .with_variable_line_width(0);

        let mut tessellate =
            |brush_size: f32, points: &[StrokeElement]| -> Result<Mesh, TessellationError> {
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

                tesselator.tessellate_path(&path, &stroke_options, &mut builder)?;
                Ok(new_mesh)
            };

        while let Ok(msg) = rx.recv() {
            match msg {
                TessMess::StartStroke(new_brush_size) => {
                    brush_size = new_brush_size;
                    points.clear();
                }

                TessMess::AddPoint(point) => {
                    points.push(point);

                    match tessellate(brush_size, &points) {
                        Ok(new_mesh) => {
                            result.write().unwrap().add(TessResult::Mesh(new_mesh));
                        }

                        Err(TessellationError::GeometryBuilder(
                            GeometryBuilderError::TooManyVertices,
                        )) => {
                            todo!();
                            //let front = &points[..points.len() / 2];
                            //let back = &points[(points.len() / 2)..];
                        }

                        // ...,
                        Err(err) => {
                            log::error!("{}", err);
                            result.write().unwrap().add(TessResult::Error);
                        }
                    }
                }

                TessMess::FinishStroke => {
                    points.clear();
                }
            }
        }
    });

    Tessellator { tx, rw, handle }
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

    pub fn queue(&self) -> RwLockReadGuard<MeshQueue> {
        self.rw.read().unwrap()
    }
}

#[test]
fn test() {
    fn x(x: f32) -> StrokeElement {
        StrokeElement {
            x,
            y: 1.,
            pressure: 1.,
        }
    }

    // what should happen (a simplified overview):
    //
    // - the ideal case (unlikely)
    //   input:   1           4
    //   render:           3
    //   tess:      [2--]
    //
    //   1. the tt (tessellator thread) gets a new point
    //   2. calculates the mesh very quickly
    //   3. it's finished by the time it needs to be rendered
    //   4. all of this happens before the next point comes in
    //
    // - the common case
    //   input:   1   4   6
    //   render:    2   5   7
    //   tess:      [3----]
    //   1. a point comes in
    //   2. immediately after the renderer renders an empty screen
    //   3. the tt receives the point
    //   4. another point comes in, gets put in the mpsc queue
    //   5. another empty screen
    //   6. another point into the mpsc queue
    //   7. the tt is finally finished and the stroke can be rendered
    //
    // - the common case, extended
    //   input:  1   4   5   8   10
    //   render:   2   3   6   9   11
    //   tess:     [3-----][7-------]
    //   1. a point
    //   2. an empty frame
    //   3. at the same time mesh calculation starts
    //   4-5. empty frames, un-tessellated points
    //   6. finally the stroke is rendered
    //   7. work begins on the stroke with points 1, 4 AND 5
    //   8. etc etc
    //
    // - the worst case
    //   input:  1   4   5   8   10     14    17
    //   render:   2   3   6   9    12     15    18  20
    //   tess:   ---------][7------11![13---][16---][19---]
    //   1-6. stale frames are rendered and un-tessellated points are queued as the tessellator
    //        thread works on the previous stroke
    //   7. work starts on the previously queued points 1, 4 and 5
    //   8-10. more points get queued* as the tessellator splits the previously queued points and
    //         re-tessellates the new mesh, and stale frames get rendered
    //   11. the resulting mesh has too many vertices
    //   13-16. the tt has to start again on half of the previously queued points
    //   19. the tt can start on 8,10,14,17
    //   20. the two split strokes can be rendered
    //
    // * theoretically it could queue up too many for the tessellator to handle and it would run
    //   away splitting strokes forever and never really finishing as in 11. but in practice you'd
    //   have to draw faster than a human can to have that happen. (TODO determine how many points
    //   it usually would take to cause a split to occur)
    todo!();
}
