#![allow(clippy::new_without_default, clippy::derive_partial_eq_without_eq)]

pub mod config;
pub mod error;
pub mod event;
pub mod graphics;
pub mod i18n;
pub mod loop_;
pub mod migrate;
pub mod stroke;
pub mod tess;
pub mod ui;

pub extern crate bytemuck;
pub extern crate dirs;
pub extern crate egui;
pub extern crate gumdrop;
pub extern crate lyon;
pub extern crate winit;

use crate::{
    error::PmbErrorExt,
    graphics::{Color, ColorExt, PixelPos, StrokePoint, StrokePos},
    stroke::{Stroke, StrokeElement},
};
use lyon::lyon_tessellation::{StrokeOptions, StrokeTessellator};
use slotmap::{DefaultKey, SlotMap};
use std::path::PathBuf;

pub const TITLE_UNMODIFIED: &str = "hi! <3";
pub const TITLE_MODIFIED: &str = "hi! <3 (modified)";
pub const PMB_MAGIC: [u8; 3] = [b'P', b'M', b'B'];

pub const DEFAULT_ZOOM: f32 = 50.;
pub const MAX_ZOOM: f32 = 500.;
pub const MIN_ZOOM: f32 = 1.;

pub const DEFAULT_BRUSH: usize = 5;
pub const MAX_BRUSH: usize = 20;
pub const MIN_BRUSH: usize = 1;
pub const BRUSH_DELTA: usize = 1;

pub trait CoordinateSystem: std::fmt::Debug + Default + Clone + Copy {
    type Ndc: std::fmt::Display + Clone + Copy;

    fn pixel_to_ndc(width: u32, height: u32, pos: PixelPos) -> Self::Ndc;
    fn ndc_to_pixel(width: u32, height: u32, pos: Self::Ndc) -> PixelPos;

    fn ndc_to_stroke(width: u32, height: u32, zoom: f32, ndc: Self::Ndc) -> StrokePoint;
    fn stroke_to_ndc(width: u32, height: u32, zoom: f32, point: StrokePoint) -> Self::Ndc;

    fn pixel_to_stroke(width: u32, height: u32, zoom: f32, pos: PixelPos) -> StrokePoint {
        let ndc = Self::pixel_to_ndc(width, height, pos);
        Self::ndc_to_stroke(width, height, zoom, ndc)
    }

    fn stroke_to_pixel(width: u32, height: u32, zoom: f32, pos: StrokePoint) -> PixelPos {
        let ndc = Self::stroke_to_ndc(width, height, zoom, pos);
        Self::ndc_to_pixel(width, height, ndc)
    }

    fn pixel_to_pos(
        width: u32,
        height: u32,
        zoom: f32,
        origin: StrokePoint,
        pos: PixelPos,
    ) -> StrokePos {
        graphics::xform_point_to_pos(origin, Self::pixel_to_stroke(width, height, zoom, pos))
    }

    fn pos_to_pixel(
        width: u32,
        height: u32,
        zoom: f32,
        origin: StrokePoint,
        pos: StrokePos,
    ) -> PixelPos {
        Self::stroke_to_pixel(
            width,
            height,
            zoom,
            graphics::xform_pos_to_point(origin, pos),
        )
    }
}

pub trait StrokeBackend: std::fmt::Debug {
    fn make_dirty(&mut self);
    fn is_dirty(&self) -> bool;
}

#[derive(gumdrop::Options, Debug)]
pub struct Args {
    #[options(help = "Show this message")]
    help: bool,

    #[options(help = "Print the version", short = "V")]
    pub version: bool,

    #[options(help = "Config file location")]
    pub config: Option<PathBuf>,

    #[options(free, help = "File to open")]
    pub file: Option<PathBuf>,
}

#[derive(Default, PartialEq, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Tool {
    #[default]
    Pen,
    Eraser,
    Pan,
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Device {
    #[default]
    Mouse,
    Touch,
    Pen,
}

#[derive(pmb_macros::Disk)]
pub struct Sketch<S: StrokeBackend> {
    #[custom_codec(to_vec, map_from_vec)]
    pub strokes: SlotMap<DefaultKey, Stroke<S>>,
    pub zoom: f32,
    pub origin: StrokePoint,
    pub bg_color: Color,
    pub fg_color: Color,
}

pub fn map_from_vec<S: StrokeBackend>(strokes: Vec<Stroke<S>>) -> SlotMap<DefaultKey, Stroke<S>> {
    strokes
        .into_iter()
        .fold(SlotMap::default(), |mut map, stroke| {
            map.insert(stroke);
            map
        })
}

impl<S: StrokeBackend> Default for Sketch<S> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<S: StrokeBackend> Sketch<S> {
    pub fn new(strokes: Vec<Stroke<S>>) -> Self {
        Self {
            strokes: map_from_vec(strokes),
            zoom: crate::DEFAULT_ZOOM,
            origin: StrokePoint::default(),
            bg_color: Color::NICE_WHITE,
            fg_color: Color::NICE_GREY,
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn with_filename<C: CoordinateSystem>(
        widget: &mut ui::widget::SketchWidget<C>,
        path: impl AsRef<std::path::Path>,
    ) -> Self {
        log::info!("create State from {}", path.as_ref().display());

        let mut this = Sketch::empty();
        ui::read_file(widget, Some(path), &mut this)
            .problem(s!(CouldNotOpenFile))
            .display();

        this
    }

    fn to_vec(&self) -> Vec<Stroke<S>> {
        self.strokes
            .values()
            .filter(|stroke| !stroke.erased)
            .map(|stroke| Stroke {
                points: stroke.points.clone(),
                color: stroke.color,
                brush_size: stroke.brush_size,
                ..Default::default()
            })
            .collect()
    }

    fn screen_rect<C: CoordinateSystem>(&self, width: u32, height: u32) -> (StrokePos, StrokePos) {
        let top_left = C::pixel_to_pos(width, height, self.zoom, self.origin, PixelPos::default());

        let bottom_right = C::pixel_to_pos(
            width,
            height,
            self.zoom,
            self.origin,
            PixelPos {
                x: width as f32,
                y: height as f32,
            },
        );

        (top_left, bottom_right)
    }

    pub fn update_visible_strokes<C: CoordinateSystem>(&mut self, width: u32, height: u32) {
        let (top_left, bottom_right) = self.screen_rect::<C>(width, height);
        for stroke in self.strokes.values_mut() {
            stroke.update_visible(top_left, bottom_right);
        }
    }

    fn update_stroke_primitive(&mut self) {
        for stroke in self.strokes.values_mut() {
            stroke.draw_tesselated = stroke.brush_size * self.zoom > 1.0;
        }
    }

    pub fn clear_strokes(&mut self) {
        self.strokes.clear();
    }

    pub fn visible_strokes(&self) -> impl Iterator<Item = &Stroke<S>> {
        self.strokes
            .values()
            .filter(|stroke| stroke.visible && !stroke.erased)
    }

    pub fn update_zoom<C: CoordinateSystem>(&mut self, width: u32, height: u32, next_zoom: f32) {
        self.zoom = if next_zoom < crate::MIN_ZOOM {
            crate::MIN_ZOOM
        } else if next_zoom > crate::MAX_ZOOM {
            crate::MAX_ZOOM
        } else {
            next_zoom
        };

        self.update_visible_strokes::<C>(width, height);
        self.update_stroke_primitive();
    }

    pub fn move_origin<C: CoordinateSystem>(
        &mut self,
        width: u32,
        height: u32,
        prev: StrokePos,
        next: StrokePos,
    ) {
        let dx = next.x - prev.x;
        let dy = next.y - prev.y;
        self.origin.x += dx;
        self.origin.y += dy;
        self.update_visible_strokes::<C>(width, height);
    }

    pub fn force_update<C: CoordinateSystem>(
        &mut self,
        width: u32,
        height: u32,
        tessellator: &mut StrokeTessellator,
        options: &StrokeOptions,
    ) {
        log::info!("forcing update");
        self.strokes
            .values_mut()
            .flat_map(|stroke| {
                stroke.rebuild_entire_mesh(tessellator, options);
                stroke.backend_mut()
            })
            .for_each(|backend| backend.make_dirty());
        self.update_visible_strokes::<C>(width, height);
        self.update_stroke_primitive();
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
    pub eraser: bool,
}

impl Default for StylusState {
    fn default() -> Self {
        StylusState {
            pos: StylusPosition::Up,
            eraser: false,
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

    pub fn eraser(&self) -> bool {
        self.state.eraser
    }
}

#[allow(dead_code)]
fn grid<S>() -> Vec<Stroke<S>>
where
    S: StrokeBackend,
{
    use std::iter::repeat;
    let mut strokes = vec![Stroke::with_points(
        graphics::circle_points(1.0, 50)
            .chunks_exact(2)
            .map(|arr| StrokeElement {
                x: arr[0],
                y: arr[1],
                pressure: 1.,
            })
            .collect(),
        Color::WHITE,
    )];

    strokes.extend(repeat(-25.0).take(50).enumerate().map(|(i, x)| {
        Stroke::with_points(
            repeat(-25.0)
                .take(50)
                .enumerate()
                .map(|(j, y)| StrokeElement {
                    x: i as f32 + x,
                    y: j as f32 + y,
                    pressure: 1.,
                })
                .collect(),
            Color::grey(0.1),
        )
    }));

    strokes.extend(repeat(-25.0).take(50).enumerate().map(|(i, y)| {
        Stroke::with_points(
            repeat(-25.0)
                .take(50)
                .enumerate()
                .map(|(j, x)| StrokeElement {
                    x: j as f32 + x,
                    y: i as f32 + y,
                    pressure: 1.,
                })
                .collect(),
            Color::grey(0.1),
        )
    }));

    strokes.push(Stroke::with_points(
        repeat(-25.0)
            .take(50)
            .enumerate()
            .map(|(i, x)| StrokeElement {
                x: i as f32 + x,
                y: 0.0,
                pressure: 1.,
            })
            .collect(),
        Color::grey(0.3),
    ));

    strokes.push(Stroke::with_points(
        repeat(-25.0)
            .take(50)
            .enumerate()
            .map(|(i, y)| StrokeElement {
                x: 0.0,
                y: i as f32 + y,
                pressure: 1.,
            })
            .collect(),
        Color::grey(0.3),
    ));

    strokes
}
