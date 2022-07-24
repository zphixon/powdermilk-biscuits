pub mod graphics;
pub mod input;

use bspline::BSpline;
use glutin::{
    dpi::PhysicalPosition,
    event::{Force, Touch, TouchPhase},
};
use graphics::{Color, ColorExt, StrokePoint, StrokePos};
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

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Stroke {
    pub points: Vec<StrokeElement>,
    pub color: Color,
    pub brush_size: f32,
    pub style: StrokeStyle,
    pub erased: bool,
    #[serde(skip)]
    pub spline: Option<BSpline<StrokeElement, f32>>,
    #[serde(skip)]
    pub vbo: Option<glow::Buffer>,
    #[serde(skip)]
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
    pub point: StrokePoint,
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

#[derive(Debug, Clone, Copy)]
pub enum GestureState {
    NoInput,
    Stroke,
    Active(usize),
}

impl GestureState {
    pub fn active(&self) -> bool {
        use GestureState::*;
        !matches!(self, NoInput | Stroke)
    }

    // returns should delete last stroke
    pub fn touch(&mut self) -> bool {
        use GestureState::*;

        let prev = *self;
        *self = match *self {
            NoInput => Stroke,
            Stroke => Active(2),
            Active(num) => Active(num + 1),
        };

        if self.active() {
            println!("do gesture {self:?}");
        }

        matches!(prev, Stroke) && matches!(self, Active(2))
    }

    pub fn release(&mut self) {
        use GestureState::*;
        *self = match *self {
            NoInput | Stroke | Active(1) => NoInput,
            Active(num) => Active(num - 1),
        };

        if self.active() {
            println!("do gesture {self:?}");
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub brush_size: usize,
    pub stroke_style: StrokeStyle,
    pub use_individual_style: bool,
    pub zoom: f32,
    pub origin: StrokePoint,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            brush_size: DEFAULT_BRUSH,
            stroke_style: StrokeStyle::Lines,
            use_individual_style: false,
            zoom: DEFAULT_ZOOM,
            origin: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ToDisk {
    pub strokes: Vec<Stroke>,
    pub settings: Settings,
}

pub struct State {
    pub stylus: Stylus,
    pub strokes: Vec<Stroke>,
    pub gesture_state: GestureState,
    pub settings: Settings,
    pub modified: bool,
}

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
            strokes,
            gesture_state: GestureState::NoInput,
            settings: Default::default(),
            modified: false,
        }
    }
}

pub const DEFAULT_ZOOM: f32 = 50.;
pub const MAX_ZOOM: f32 = 500.;
pub const MIN_ZOOM: f32 = 1.;

pub const DEFAULT_BRUSH: usize = 1;
pub const MAX_BRUSH: usize = 20;
pub const MIN_BRUSH: usize = 1;
pub const BRUSH_DELTA: usize = 1;

impl State {
    pub fn increase_brush(&mut self) {
        self.settings.brush_size += BRUSH_DELTA;
        self.settings.brush_size = self.settings.brush_size.clamp(MIN_BRUSH, MAX_BRUSH);
    }

    pub fn decrease_brush(&mut self) {
        self.settings.brush_size -= BRUSH_DELTA;
        self.settings.brush_size = self.settings.brush_size.clamp(MIN_BRUSH, MAX_BRUSH);
    }

    pub fn move_origin(
        &mut self,
        width: u32,
        height: u32,
        prev: PhysicalPosition<f64>,
        next: PhysicalPosition<f64>,
    ) {
        use graphics::*;

        let prev_gl = physical_position_to_gl(width, height, prev);
        let prev_stroke = gl_to_stroke(width, height, self.settings.zoom, prev_gl);
        let prev_xformed = xform_point_to_pos(self.settings.origin, prev_stroke);

        let next_gl = physical_position_to_gl(width, height, next);
        let next_stroke = gl_to_stroke(width, height, self.settings.zoom, next_gl);
        let next_xformed = xform_point_to_pos(self.settings.origin, next_stroke);

        let dx = next_xformed.x - prev_xformed.x;
        let dy = next_xformed.y - prev_xformed.y;
        self.settings.origin.x += dx;
        self.settings.origin.y += dy;
    }

    pub fn change_zoom(&mut self, dz: f32) {
        if (self.settings.zoom + dz).is_finite() {
            self.settings.zoom += dz;
        }

        self.settings.zoom = self.settings.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    pub fn clear_strokes(&mut self) {
        self.modified = true;
        std::mem::take(&mut self.strokes);
    }

    pub fn undo_stroke(&mut self) {
        self.modified = true;
        self.strokes.pop();
    }

    pub fn update(&mut self, width: u32, height: u32, touch: Touch) {
        let Touch {
            force,
            phase,
            location,
            pen_info,
            ..
        } = touch;

        let gl_pos = graphics::physical_position_to_gl(width, height, location);
        let point = graphics::gl_to_stroke(width, height, self.settings.zoom, gl_pos);

        let pressure = match force {
            Some(Force::Normalized(force)) => force,

            Some(Force::Calibrated {
                force,
                max_possible_force,
                altitude_angle: _,
            }) => force / max_possible_force,

            _ => 0.0,
        };

        let inverted = pen_info
            .map(|info| info.inverted)
            .unwrap_or(self.stylus.state.inverted);

        let state = match phase {
            TouchPhase::Started => {
                if pen_info.is_none() && self.gesture_state.touch() {
                    self.undo_stroke();
                }

                StylusState {
                    pos: StylusPosition::Down,
                    inverted,
                }
            }

            TouchPhase::Moved => {
                self.stylus.state.inverted = inverted;
                self.stylus.state
            }

            TouchPhase::Ended | TouchPhase::Cancelled => {
                if pen_info.is_none() {
                    self.gesture_state.release();
                }

                StylusState {
                    pos: StylusPosition::Up,
                    inverted,
                }
            }
        };

        self.stylus.point = point;
        self.stylus.pos = graphics::xform_point_to_pos(self.settings.origin, self.stylus.point);
        self.stylus.pressure = pressure as f32;
        self.stylus.state = state;

        if pen_info.is_none() && self.gesture_state.active() {
            self.stylus.state.pos = StylusPosition::Up;
            return;
        }

        self.handle_update(width, height, phase);
    }

    fn handle_update(&mut self, width: u32, height: u32, phase: TouchPhase) {
        use graphics::*;

        if self.stylus.inverted() {
            let stylus_gl = stroke_to_gl(
                width,
                height,
                self.settings.zoom,
                StrokePoint {
                    x: self.stylus.pos.x,
                    y: self.stylus.pos.y,
                },
            );
            let stylus_pix = gl_to_physical_position(width, height, stylus_gl);
            let stylus_pix_x = stylus_pix.x as f32;
            let stylus_pix_y = stylus_pix.y as f32;

            if phase == TouchPhase::Moved && self.stylus.down() {
                for stroke in self.strokes.iter_mut() {
                    if stroke.erased {
                        continue;
                    }

                    'inner: for point in stroke.points.iter() {
                        let point_gl = stroke_to_gl(
                            width,
                            height,
                            self.settings.zoom,
                            StrokePoint {
                                x: point.x,
                                y: point.y,
                            },
                        );
                        let point_pix = gl_to_physical_position(width, height, point_gl);
                        let point_pix_x = point_pix.x as f32;
                        let point_pix_y = point_pix.y as f32;

                        let dist = ((stylus_pix_x - point_pix_x).powi(2)
                            + (stylus_pix_y - point_pix_y).powi(2))
                        .sqrt()
                            * 2.0;

                        if dist < self.settings.brush_size as f32 {
                            stroke.erased = true;
                            self.modified = true;
                            break 'inner;
                        }
                    }
                }
            }
        } else {
            match phase {
                TouchPhase::Started => {
                    self.modified = true;
                    self.strokes.push(Stroke {
                        points: Vec::new(),
                        color: rand::random(),
                        brush_size: self.settings.brush_size as f32,
                        style: self.settings.stroke_style,
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
