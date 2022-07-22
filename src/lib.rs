pub mod graphics;
pub mod input;

use bspline::BSpline;
use glutin::event::{Force, Touch, TouchPhase};
use graphics::{Color, ColorExt, StrokePoint};

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
    pub pos: StrokePoint,
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
            use std::iter::repeat;
            let mut strokes = vec![Stroke {
                points: graphics::circle_points(1.0, 50)
                    .chunks_exact(2)
                    .map(|arr| StrokeElement {
                        x: arr[0],
                        y: arr[1],
                        pressure: 1.0,
                    })
                    .collect(),
                color: Color::WHITE,
                ..Default::default()
            }];

            strokes.extend(repeat(-25.0).take(50).enumerate().map(|(i, x)| {
                Stroke {
                    points: repeat(-25.0)
                        .take(50)
                        .enumerate()
                        .map(|(j, y)| StrokeElement {
                            x: i as f32 + x,
                            y: j as f32 + y,
                            pressure: 1.0,
                        })
                        .collect(),
                    color: Color::grey(0.1),
                    ..Default::default()
                }
            }));

            strokes.extend(repeat(-25.0).take(50).enumerate().map(|(i, y)| {
                Stroke {
                    points: repeat(-25.0)
                        .take(50)
                        .enumerate()
                        .map(|(j, x)| StrokeElement {
                            x: j as f32 + x,
                            y: i as f32 + y,
                            pressure: 1.0,
                        })
                        .collect(),
                    color: Color::grey(0.1),
                    ..Default::default()
                }
            }));

            strokes.push(Stroke {
                points: repeat(-25.0)
                    .take(50)
                    .enumerate()
                    .map(|(i, x)| StrokeElement {
                        x: i as f32 + x,
                        y: 0.0,
                        pressure: 1.0,
                    })
                    .collect(),
                color: Color::grey(0.3),
                ..Default::default()
            });

            strokes.push(Stroke {
                points: repeat(-25.0)
                    .take(50)
                    .enumerate()
                    .map(|(i, y)| StrokeElement {
                        x: 0.0,
                        y: i as f32 + y,
                        pressure: 1.0,
                    })
                    .collect(),
                color: Color::grey(0.3),
                ..Default::default()
            });

            State {
                stylus: Default::default(),
                brush_size: BRUSH_DEFAULT,
                fill_brush_head: false,
                strokes,
                stroke_style: Default::default(),
                use_individual_style: false,
            }
        }
    }
}

pub const BRUSH_DEFAULT: f32 = 0.6;
pub const MAX_BRUSH: f32 = 4.0;
pub const MIN_BRUSH: f32 = 0.1;
pub const BRUSH_DELTA: f32 = 0.5;

impl State {
    pub fn increase_brush(&mut self) {
        self.brush_size += BRUSH_DELTA;
        self.brush_size = self.brush_size.clamp(MIN_BRUSH, MAX_BRUSH);
    }

    pub fn decrease_brush(&mut self) {
        self.brush_size -= BRUSH_DELTA;
        self.brush_size = self.brush_size.clamp(MIN_BRUSH, MAX_BRUSH);
    }

    pub fn clear_strokes(&mut self) {
        std::mem::take(&mut self.strokes);
    }

    pub fn undo_stroke(&mut self) {
        self.strokes.pop();
    }

    pub fn update(&mut self, gis: StrokePoint, zoom: f32, width: u32, height: u32, touch: Touch) {
        let Touch {
            force,
            phase,
            location,
            inverted,
            ..
        } = touch;

        let gl_pos = graphics::physical_position_to_gl(width, height, location);
        let point = graphics::gl_to_stroke(width, height, zoom, gl_pos);

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

        self.stylus.pos = point;
        self.stylus.pressure = pressure as f32;
        self.stylus.state = state;

        self.handle_update(gis, phase);
    }

    fn handle_update(&mut self, gis: StrokePoint, phase: TouchPhase) {
        let pos = graphics::xform_stroke(gis, self.stylus.pos);

        if self.stylus.inverted() {
            if phase == TouchPhase::Moved && self.stylus.down() {
                for stroke in self.strokes.iter_mut() {
                    if stroke.erased {
                        continue;
                    }

                    'inner: for point in stroke.points.iter() {
                        let dist = ((pos.x - point.x).powi(2) + (pos.y - point.y).powi(2)).sqrt();
                        if dist < self.brush_size {
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
                                x: pos.x,
                                y: pos.y,
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
