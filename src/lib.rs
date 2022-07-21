pub mod graphics;
pub mod input;

use bspline::BSpline;
use glutin::event::{Force, Touch, TouchPhase};
use graphics::StrokePos;

pub type Color = [u8; 3];

#[derive(Default, Debug, Clone, Copy)]
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

#[derive(Default, Debug)]
pub struct Stroke {
    pub points: Vec<StrokeElement>,
    pub color: Color,
    pub brush_size: f32,
    pub style: StrokeStyle,
    pub spline: Option<BSpline<StrokeElement, f32>>,
    pub erased: bool,
    pub vbo: Option<glow::Buffer>,
    pub vao: Option<glow::VertexArray>,
}

impl Stroke {
    pub const DEGREE: usize = 3;

    pub fn calculate_spline(&mut self) {
        if self.points.len() > Stroke::DEGREE {
            let points = [self.points.first().cloned().unwrap(); Stroke::DEGREE]
                .into_iter()
                .chain(self.points.iter().cloned())
                .chain([self.points.last().cloned().unwrap(); Stroke::DEGREE])
                .map(|point| point.into())
                .collect::<Vec<StrokeElement>>();

            let knots = std::iter::repeat(())
                .take(points.len() + Stroke::DEGREE + 1)
                .enumerate()
                .map(|(i, ())| i as f32)
                .collect::<Vec<_>>();

            self.spline = Some(BSpline::new(Stroke::DEGREE, points, knots));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, evc_derive::EnumVariantCount)]
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

#[derive(Debug, Clone, Copy)]
pub enum StylusPosition {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy)]
pub struct StylusState {
    pub pos: StylusPosition,
    pub inverted: bool,
}

impl Default for StylusState {
    fn default() -> Self {
        StylusState {
            pos: StylusPosition::Up,
            inverted: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Stylus {
    pub state: StylusState,
    pub pressure: f32,
    pub pos: StrokePos,
}

impl Stylus {
    pub fn down(&self) -> bool {
        matches!(self.state.pos, StylusPosition::Down)
    }

    pub fn inverted(&self) -> bool {
        self.state.inverted
    }
}

pub struct State {
    pub stylus: Stylus,
    pub brush_size: f32,
    pub fill_brush_head: bool,
    pub strokes: Vec<Stroke>,
    pub stroke_style: StrokeStyle,
    pub use_individual_style: bool,
}

mod hide {
    use super::*;
    impl Default for State {
        fn default() -> Self {
            State {
                stylus: Default::default(),
                brush_size: 1.0,
                fill_brush_head: false,
                strokes: Default::default(),
                stroke_style: Default::default(),
                use_individual_style: false,
            }
        }
    }
}

pub const MAX_BRUSH: f32 = 32.0;
pub const MIN_BRUSH: f32 = 1.0;

impl State {
    pub fn increase_brush(&mut self) {
        if self.brush_size + 1. > MAX_BRUSH {
            self.brush_size = MAX_BRUSH;
        } else {
            self.brush_size += 1.;
        }
    }

    pub fn decrease_brush(&mut self) {
        if self.brush_size - 1. < MIN_BRUSH {
            self.brush_size = MIN_BRUSH;
        } else {
            self.brush_size -= 1.;
        }
    }

    pub fn clear_strokes(&mut self) {
        std::mem::take(&mut self.strokes);
    }

    pub fn undo_stroke(&mut self) {
        self.strokes.pop();
    }

    pub fn update(&mut self, gis: StrokePos, zoom: f32, width: u32, height: u32, touch: Touch) {
        let Touch {
            force,
            phase,
            location,
            inverted,
            ..
        } = touch;

        let ngl = graphics::physical_position_to_gl(width, height, location);
        let nsp = graphics::gl_to_stroke(width, height, ngl);
        let pos = graphics::xform_stroke(gis, zoom, nsp);

        let pressure = match force {
            Some(Force::Normalized(force)) => force,

            Some(Force::Calibrated {
                force,
                max_possible_force,
                altitude_angle: _,
            }) => force / max_possible_force,

            _ => 0.0,
        };

        let state = match phase {
            TouchPhase::Started => StylusState {
                pos: StylusPosition::Down,
                inverted,
            },

            TouchPhase::Moved => {
                self.stylus.state.inverted = inverted;
                self.stylus.state
            }

            TouchPhase::Ended | TouchPhase::Cancelled => StylusState {
                pos: StylusPosition::Up,
                inverted,
            },
        };

        self.stylus.pos = pos;
        self.stylus.pressure = pressure as f32;
        self.stylus.state = state;

        self.handle_update(phase);
    }

    fn handle_update(&mut self, phase: TouchPhase) {
        if self.stylus.inverted() {
            if phase == TouchPhase::Moved && self.stylus.down() {
                for stroke in self.strokes.iter_mut() {
                    if stroke.erased {
                        continue;
                    }

                    'inner: for point in stroke.points.iter() {
                        let dist = ((self.stylus.pos.x - point.x).powi(2)
                            + (self.stylus.pos.y - point.y).powi(2))
                        .sqrt();
                        if dist < self.brush_size {
                            println!("poof d={dist:.02} bs={}", self.brush_size as usize);
                            stroke.erased = true;
                            break 'inner;
                        }
                    }
                }
            }
        } else {
            match phase {
                TouchPhase::Started => {
                    self.strokes.push(Stroke {
                        points: Vec::new(),
                        color: rand::random(),
                        brush_size: self.brush_size,
                        style: self.stroke_style,
                        erased: false,
                        spline: None,
                        vbo: None,
                        vao: None,
                    });
                }

                TouchPhase::Moved => {
                    if let Some(stroke) = self.strokes.last_mut() {
                        if self.stylus.down() {
                            stroke.points.push(StrokeElement {
                                x: self.stylus.pos.x,
                                y: self.stylus.pos.y,
                                pressure: self.stylus.pressure,
                            });

                            stroke.calculate_spline();
                        }
                    }
                }

                TouchPhase::Ended | TouchPhase::Cancelled => {
                    if let Some(stroke) = self.strokes.last_mut() {
                        stroke.calculate_spline();
                    }
                }
            };
        }
    }
}
