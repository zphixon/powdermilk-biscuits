pub mod graphics;
pub mod input;

use crate::graphics::coords::{PixelPos, StrokePos};
use bspline::BSpline;
use glutin::event::{Force, Touch, TouchPhase};
use graphics::coords::GlPos;
use std::io::Write;

pub type Color = [u8; 3];

#[derive(Default, Debug, Clone, Copy)]
pub struct StrokePoint {
    pub pos: StrokePos,
    pub pressure: f32,
}

#[derive(Default, Debug)]
pub struct Stroke {
    pub points: Vec<StrokePoint>,
    pub color: Color,
    pub brush_size: f32,
    pub style: StrokeStyle,
    pub spline: Option<BSpline<StrokePos, f32>>,
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
                .map(|point| point.into())
                .collect::<Vec<_>>();

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

impl State {
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

    pub fn update(&mut self, gis: StrokePos, zoom: f32, width: u32, height: u32, touch: Touch) {
        let Touch {
            force,
            phase,
            location,
            inverted,
            ..
        } = touch;

        let pixel_pos = PixelPos::from_physical_position(location);
        let gl_pos = GlPos::from_pixel(width, height, pixel_pos);
        let pos = StrokePos::from_gl(gis, zoom, gl_pos);
        let pos = StrokePos {
            x: pos.x * width as f32,
            y: pos.y * height as f32,
        };

        let pressure_str = if let Some(force) = force {
            if phase == TouchPhase::Moved && self.stylus.down() {
                format!("{:.02}", force.normalized())
            } else {
                String::from("    ")
            }
        } else {
            String::from("    ")
        };
        let inverted_str = if inverted { " (inverted) " } else { " " };
        let pixel_str = format!("{},{}", pixel_pos.x, pixel_pos.y);
        let gl_str = format!("{:.02},{:.02}", gl_pos.x, gl_pos.y);
        let stroke_str = format!("{:.02},{:.02}", pos.x, pos.y);
        let gis_str = format!("{:.02},{:.02}", gis.x, gis.y);
        let stroke_str = format!(
            "{gis_str} {pixel_str} -> {gl_str} -> {stroke_str}{inverted_str}{:?}            ",
            self.stroke_style
        );

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
                    print!("\r{pressure_str}         {stroke_str}");
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

        self.stylus.pos = pos;
        self.stylus.pressure = pressure as f32;
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
                            stroke.points.push(StrokePoint {
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

    pub fn draw_strokes(
        &mut self,
        frame: &mut [u8],
        width: usize,
        height: usize,
        zoom: f32,
        gis: StrokePos,
    ) {
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
                })(stroke, frame, width, height, zoom, gis);
            }
        }
    }
}
