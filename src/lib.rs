pub mod graphics;

//#[cfg(windows)]
//pub mod myrts;

use {
    bspline::BSpline,
    std::{
        collections::HashMap,
        io::Write,
        ops::{Add, Mul, Sub},
    },
    winit::{
        dpi::PhysicalPosition,
        event::{ElementState, Force, Touch, TouchPhase, VirtualKeyCode},
    },
};

pub type Color = [u8; 3];

#[derive(Default, Debug, Clone, Copy)]
pub struct Point {
    pub pos: PhysicalPosition<f64>,
    pub pressure: f64,
}

impl Mul<Point> for f64 {
    type Output = Point;
    fn mul(self, rhs: Point) -> Self::Output {
        Point {
            pos: PhysicalPosition {
                x: rhs.pos.x * self,
                y: rhs.pos.y * self,
            },
            // ?
            pressure: rhs.pressure * self,
        }
    }
}

impl Mul<f64> for Point {
    type Output = Point;
    fn mul(self, rhs: f64) -> Self::Output {
        Point {
            pos: PhysicalPosition {
                x: self.pos.x * rhs,
                y: self.pos.y * rhs,
            },
            // ??
            pressure: self.pressure * rhs,
        }
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, rhs: Self) -> Self::Output {
        Point {
            pos: PhysicalPosition {
                x: self.pos.x + rhs.pos.x,
                y: self.pos.y + rhs.pos.y,
            },
            // ????
            pressure: self.pressure + rhs.pressure,
        }
    }
}

impl Sub for Point {
    type Output = Point;
    fn sub(self, rhs: Self) -> Self::Output {
        Point {
            pos: PhysicalPosition {
                x: self.pos.x - rhs.pos.x,
                y: self.pos.y - rhs.pos.y,
            },
            // ??????
            pressure: self.pressure - rhs.pressure,
        }
    }
}

#[derive(Default, Debug)]
pub struct Stroke {
    pub points: Vec<Point>,
    pub color: Color,
    pub brush_size: f64,
    pub style: StrokeStyle,
    pub spline: Option<BSpline<Point, f64>>,
    pub erased: bool,
}

impl Stroke {
    pub const DEGREE: usize = 3;

    pub fn calculate_spline(&mut self) {
        if self.points.len() > Stroke::DEGREE {
            let points = [self.points.first().cloned().unwrap(); Stroke::DEGREE]
                .into_iter()
                .chain(self.points.iter().cloned())
                .chain([self.points.last().cloned().unwrap(); Stroke::DEGREE])
                .collect::<Vec<_>>();

            let knots = std::iter::repeat(())
                .take(points.len() + Stroke::DEGREE + 1)
                .enumerate()
                .map(|(i, ())| i as f64)
                .collect::<Vec<_>>();

            self.spline = Some(BSpline::new(Stroke::DEGREE, points, knots));
        }
    }
}

#[derive(Debug, Clone, Copy, evc_derive::EnumVariantCount)]
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
pub enum KeyState {
    Downstroke,
    Held,
    Released,
}

impl KeyState {
    pub fn is_down(&self) -> bool {
        use KeyState::*;
        matches!(self, Downstroke | Held)
    }

    pub fn just_pressed(&self) -> bool {
        use KeyState::*;
        matches!(self, Downstroke)
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
    pub pressure: f64,
    pub pos: PhysicalPosition<f64>,
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
    pub brush_size: f64,
    pub fill_brush_head: bool,
    pub strokes: Vec<Stroke>,
    pub keys: HashMap<VirtualKeyCode, KeyState>,
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
                keys: Default::default(),
                stroke_style: Default::default(),
                use_individual_style: false,
            }
        }
    }
}

impl State {
    pub fn key(&mut self, key: VirtualKeyCode, element_state: ElementState) {
        let key_state = self.keys.entry(key).or_insert(KeyState::Released);

        let next_key_state = match (*key_state, element_state) {
            (KeyState::Released, ElementState::Pressed) => KeyState::Downstroke,
            (_, ElementState::Released) => KeyState::Released,
            (_, ElementState::Pressed) => KeyState::Held,
        };

        *key_state = next_key_state;
    }

    pub fn is_down(&self, key: VirtualKeyCode) -> bool {
        self.keys.contains_key(&key) && self.keys[&key].is_down()
    }

    pub fn just_pressed(&self, key: VirtualKeyCode) -> bool {
        self.keys.contains_key(&key) && self.keys[&key].just_pressed()
    }

    pub fn shift(&self) -> bool {
        use VirtualKeyCode::{LShift, RShift};
        self.is_down(LShift) || self.is_down(RShift)
    }

    pub fn control(&self) -> bool {
        use VirtualKeyCode::{LControl, RControl};
        self.is_down(LControl) || self.is_down(RControl)
    }

    pub fn increase_brush(&mut self) {
        let max_brush = 32.0;
        if self.brush_size + 1. > max_brush {
            self.brush_size = max_brush;
        } else {
            self.brush_size += 1.;
        }
    }

    pub fn decrease_brush(&mut self) {
        let min_brush = 1.0;
        if self.brush_size - 1. < min_brush {
            self.brush_size = min_brush;
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

    pub fn update(&mut self, touch: Touch) {
        let Touch {
            force,
            phase,
            location,
            inverted,
            ..
        } = touch;

        let inverted_str = if inverted { " (inverted) " } else { " " };
        let location_str = format!("{:.02},{:.02}", location.x, location.y);
        let stroke_str = format!("{location_str}{inverted_str}{:?}", self.stroke_style);

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
            TouchPhase::Started => {
                println!("start stroke {stroke_str}");

                StylusState {
                    pos: StylusPosition::Down,
                    inverted,
                }
            }

            TouchPhase::Moved => {
                if self.stylus.down() {
                    print!("\r             {stroke_str}");
                    std::io::stdout().flush().unwrap();
                }

                self.stylus.state.inverted = inverted;
                self.stylus.state
            }

            TouchPhase::Ended | TouchPhase::Cancelled => {
                println!("\rend stroke   {stroke_str}\n");

                StylusState {
                    pos: StylusPosition::Up,
                    inverted,
                }
            }
        };

        self.stylus.pos = location;
        self.stylus.pressure = pressure;
        self.stylus.state = state;

        self.handle_update(phase);
    }

    fn handle_update(&mut self, phase: TouchPhase) {
        if self.stylus.inverted() {
            if phase == TouchPhase::Moved && self.stylus.down() {
                for stroke in self.strokes.iter_mut() {
                    'inner: for point in stroke.points.iter() {
                        let dist = ((self.stylus.pos.x - point.pos.x).powi(2)
                            + (self.stylus.pos.y - point.pos.y).powi(2))
                        .sqrt();
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
                    });
                }

                TouchPhase::Moved => {
                    if let Some(stroke) = self.strokes.last_mut() {
                        if self.stylus.down() {
                            stroke.points.push(Point {
                                pos: self.stylus.pos,
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

    pub fn draw_strokes(&mut self, frame: &mut [u8], width: usize, height: usize) {
        for stroke in self.strokes.iter_mut() {
            if !stroke.erased {
                (match if self.use_individual_style {
                    stroke.style
                } else {
                    self.stroke_style
                } {
                    StrokeStyle::Lines => graphics::lines,
                    StrokeStyle::Circles => graphics::circles,
                    StrokeStyle::CirclesPressure => graphics::circles_pressure,
                    StrokeStyle::Points => graphics::points,
                    StrokeStyle::Spline => graphics::spline,
                })(stroke, frame, width, height);
            }
        }
    }
}
