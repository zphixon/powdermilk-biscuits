use piet_common::{
    kurbo::{BezPath, Point},
    Color, ImageFormat, RenderContext, StrokeStyle,
};
use powdermilk_biscuits::{graphics::PixelPos, Backend, State};
use softbuffer::GraphicsContext;
use winit::{
    dpi::{LogicalPosition, PhysicalSize},
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod backend {
    use powdermilk_biscuits::{
        graphics::{PixelPos, StrokePoint},
        Backend, StrokeBackend,
    };

    #[derive(Debug, Default, Clone, Copy)]
    pub struct PietBackend;

    impl Backend for PietBackend {
        type Ndc = PixelPos;

        fn pixel_to_ndc(&self, _width: u32, _height: u32, pos: PixelPos) -> Self::Ndc {
            pos
        }

        fn ndc_to_pixel(&self, _width: u32, _height: u32, pos: Self::Ndc) -> PixelPos {
            pos
        }

        fn ndc_to_stroke(&self, width: u32, height: u32, zoom: f32, ndc: Self::Ndc) -> StrokePoint {
            StrokePoint {
                x: ((2. * ndc.x) / width as f32 - 1.),
                y: (-((2. * ndc.y) / height as f32 - 1.)),
            }
        }

        fn stroke_to_ndc(
            &self,
            width: u32,
            height: u32,
            zoom: f32,
            point: StrokePoint,
        ) -> Self::Ndc {
            PixelPos {
                x: (point.x + 1.) * width as f32 / 2.,
                y: (-point.y + 1.) * height as f32 / 2.,
            }
        }
    }

    #[derive(Debug)]
    pub struct PietStrokeBackend;
    impl StrokeBackend for PietStrokeBackend {
        fn is_dirty(&self) -> bool {
            false
        }

        fn make_dirty(&mut self) {}
    }
}

fn main() {
    let ev = EventLoop::new();
    let window = WindowBuilder::new()
        .with_position(LogicalPosition {
            x: 1920. / 2. - 800. / 2.,
            y: 1080. + 1080. / 2. - 600. / 2.,
        })
        .build(&ev)
        .unwrap();

    let mut gc = unsafe { GraphicsContext::new(window) }.unwrap();

    let mut size = gc.window().inner_size();
    let mut buf = vec![0u32; size.width as usize * size.height as usize];
    let mut device = piet_common::Device::new().unwrap();

    let mut style = StrokeStyle::new();
    style.set_line_cap(piet_common::LineCap::Round);
    style.set_line_join(piet_common::LineJoin::Round);

    let backend = backend::PietBackend;
    let mut state: State<backend::PietBackend, backend::PietStrokeBackend> =
        if let Some(filename) = std::env::args().nth(1) {
            State::with_filename(std::path::PathBuf::from(filename))
        } else {
            State::default()
        };
    state.reset_view(size.width, size.height);

    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                    *flow = ControlFlow::Exit;
                }
            }

            Event::WindowEvent {
                event: WindowEvent::Touch(touch),
                ..
            } => {
                let PhysicalSize { width, height } = size;
                let pixel_pos = PixelPos {
                    x: touch.location.x as f32,
                    y: touch.location.y as f32,
                };

                let ndc = backend.pixel_to_ndc(width, height, pixel_pos);
                let stroke_point = backend.pixel_to_stroke(width, height, state.zoom, pixel_pos);
                let stroke_pos =
                    backend.pixel_to_pos(width, height, state.zoom, state.origin, pixel_pos);
                print!(
                    "p={} n={} i={} o={}\r",
                    pixel_pos, ndc, stroke_point, stroke_pos
                );
                use std::io::Write;
                std::io::stdout().flush().unwrap();

                // TODO
                gc.window().request_redraw();
            }

            Event::RedrawRequested(_) => {
                let PhysicalSize { width, height } = size;
                let mut target = device
                    .bitmap_target(width as usize, height as usize, 1.0)
                    .unwrap();

                {
                    let mut ctx = target.render_context();

                    for stroke in state.strokes.iter() {
                        if !stroke.visible {
                            continue;
                        }

                        let mut path = BezPath::new();

                        if let Some(first) = stroke.points.first() {
                            let first = backend.pos_to_pixel(
                                width,
                                height,
                                state.zoom,
                                state.origin,
                                first.into(),
                            );

                            path.move_to(Point {
                                x: first.x as f64,
                                y: first.y as f64,
                            });
                        }

                        for pair in stroke.points.windows(2) {
                            if let [a, b] = pair {
                                let a = backend.pos_to_pixel(
                                    width,
                                    height,
                                    state.zoom,
                                    state.origin,
                                    a.into(),
                                );
                                let b = backend.pos_to_pixel(
                                    width,
                                    height,
                                    state.zoom,
                                    state.origin,
                                    b.into(),
                                );
                                path.quad_to(
                                    Point {
                                        x: a.x as f64,
                                        y: a.y as f64,
                                    },
                                    Point {
                                        x: b.x as f64,
                                        y: b.y as f64,
                                    },
                                );
                            } else if let [a] = pair {
                                let a = backend.pos_to_pixel(
                                    width,
                                    height,
                                    state.zoom,
                                    state.origin,
                                    a.into(),
                                );
                                path.line_to(Point {
                                    x: a.x as f64,
                                    y: a.y as f64,
                                });
                            }
                        }

                        ctx.stroke_styled(
                            path,
                            &Color::rgba8(stroke.color[0], stroke.color[1], stroke.color[2], 0xff),
                            10.0,
                            &style,
                        );
                    }

                    ctx.finish().unwrap();
                }

                target
                    .copy_raw_pixels(ImageFormat::RgbaPremul, bytemuck::cast_slice_mut(&mut buf))
                    .unwrap();
                gc.set_buffer(&buf, width as u16, height as u16);
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                size = new_size;
                buf = vec![0u32; size.width as usize * size.height as usize];
                state.change_zoom(0.0, size.width, size.height);
                gc.window().request_redraw();
            }

            _ => {}
        }
    });
}
