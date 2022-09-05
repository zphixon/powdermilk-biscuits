use glow::{Context, HasContext};
use glutin::{
    dpi::PhysicalSize,
    event::{
        ElementState as GlutinElementState, Event as GlutinEvent, KeyboardInput, MouseScrollDelta,
        Touch, TouchPhase, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use lyon::{
    lyon_tessellation::{
        geometry_builder::simple_builder, StrokeOptions, StrokeTessellator, VertexBuffers,
    },
    path::{LineCap, LineJoin, Path},
};
use pmb_gl::{GlCoords, GlStrokeBackend};
use powdermilk_biscuits::stroke::{Stroke, StrokeElement};
use powdermilk_biscuits::{
    event::{ElementState, Event},
    ui::Ui,
    Config, Device, Sketch, Tool,
};

use crate::backend::LyonStrokeBackend;

#[allow(dead_code)]
#[rustfmt::skip]
const BIGPOINTS: &[StrokeElement] = &[
    StrokeElement { x: -3.3346415, y: 4.1671815, pressure: 0.01 },
    StrokeElement { x: -3.4107695, y: 4.1671815, pressure: 0.02 },
    StrokeElement { x: -3.4799757, y: 4.161991, pressure: 0.03 },
    StrokeElement { x: -3.5543728, y: 4.148496, pressure: 0.04 },
    StrokeElement { x: -3.6253088, y: 4.1246195, pressure: 0.05 },
    StrokeElement { x: -3.739501, y: 4.0696, pressure: 0.06 },
    StrokeElement { x: -3.813898, y: 4.018734, pressure: 0.07 },
    StrokeElement { x: -3.8709936, y: 3.9689047, pressure: 0.08 },
    StrokeElement { x: -3.9073267, y: 3.9138858, pressure: 0.09 },
    StrokeElement { x: -3.9678822, y: 3.8007324, pressure: 0.10 },
    StrokeElement { x: -3.9990263, y: 3.7031515, pressure: 0.11 },
    StrokeElement { x: -4.016327, y: 3.6180267, pressure: 0.12 },
    StrokeElement { x: -4.0249777, y: 3.5391312, pressure: 0.13 },
    StrokeElement { x: -4.0249777, y: 3.449855, pressure: 0.14 },
    StrokeElement { x: -4.011136, y: 3.2806442, pressure: 0.15 },
    StrokeElement { x: -3.9903746, y: 3.1197388, pressure: 0.16 },
    StrokeElement { x: -3.9540405, y: 2.9754431, pressure: 0.17 },
    StrokeElement { x: -3.9107876, y: 2.845681, pressure: 0.18 },
    StrokeElement { x: -3.853691, y: 2.7346036, pressure: 0.19 },
    StrokeElement { x: -3.8052464, y: 2.6608994, pressure: 0.20 },
    StrokeElement { x: -3.7533426, y: 2.6079562, pressure: 0.21 },
    StrokeElement { x: -3.7048979, y: 2.5685084, pressure: 0.22 },
    StrokeElement { x: -3.6426115, y: 2.5373654, pressure: 0.23 },
    StrokeElement { x: -3.582056, y: 2.5166037, pressure: 0.24 },
    StrokeElement { x: -3.5197697, y: 2.4999933, pressure: 0.25 },
    StrokeElement { x: -3.4574833, y: 2.4948027, pressure: 0.26 },
    StrokeElement { x: -3.4142294, y: 2.4948027, pressure: 0.27 },
    StrokeElement { x: -3.3796253, y: 2.505184, pressure: 0.28 },
    StrokeElement { x: -3.3623235, y: 2.5238693, pressure: 0.29 },
    StrokeElement { x: -3.353673, y: 2.547746, pressure: 0.30 },
    StrokeElement { x: -3.3571339, y: 2.582003, pressure: 0.31 },
    StrokeElement { x: -3.3830862, y: 2.6484418, pressure: 0.32 },
    StrokeElement { x: -3.4713252, y: 2.8301096, pressure: 0.33 },
    StrokeElement { x: -3.5768652, y: 3.006586, pressure: 0.34 },
    StrokeElement { x: -3.6772146, y: 3.1882539, pressure: 0.35 },
    StrokeElement { x: -3.7879457, y: 3.3730352, pressure: 0.36 },
    StrokeElement { x: -3.888294, y: 3.5526266, pressure: 0.37 },
    StrokeElement { x: -3.976533, y: 3.732218, pressure: 0.38 },
    StrokeElement { x: -4.0301685, y: 3.8630183, pressure: 0.39 },
    StrokeElement { x: -4.0699625, y: 3.9979713, pressure: 0.40 },
    StrokeElement { x: -4.090724, y: 4.095553, pressure: 0.41 },
    StrokeElement { x: -4.095914, y: 4.1775627, pressure: 0.42 },
    StrokeElement { x: -4.087264, y: 4.237772, pressure: 0.43 },
    StrokeElement { x: -4.0595818, y: 4.2990203, pressure: 0.44 },
    StrokeElement { x: -3.9851847, y: 4.4069824, pressure: 0.45 },
    StrokeElement { x: -3.8502314, y: 4.535707, pressure: 0.46 },
    StrokeElement { x: -3.6910563, y: 4.643669, pressure: 0.47 },
    StrokeElement { x: -3.511118, y: 4.7412505, pressure: 0.48 },
    StrokeElement { x: -3.3830862, y: 4.7973084, pressure: 0.49 },
    StrokeElement { x: -3.2515926, y: 4.855442, pressure: 0.50 },
    StrokeElement { x: -3.120101, y: 4.910461, pressure: 0.51 },
];

mod backend {
    use glow::VertexArray;
    use powdermilk_biscuits::StrokeBackend;

    #[derive(Debug)]
    pub struct LyonStrokeBackend {
        pub dirty: bool,
        pub mesh_vao: VertexArray,
        pub indices_len: i32,
        pub line_vao: VertexArray,
        pub points_len: i32,
    }

    impl StrokeBackend for LyonStrokeBackend {
        fn is_dirty(&self) -> bool {
            self.dirty
        }

        fn make_dirty(&mut self) {
            self.dirty = true;
        }
    }
}

fn main() {
    env_logger::init();

    let ev = EventLoop::new();
    let builder = WindowBuilder::new();
    let context = unsafe {
        ContextBuilder::new()
            .with_vsync(true)
            .with_gl(glutin::GlRequest::Latest)
            .with_multisampling(4)
            .build_windowed(builder, &ev)
            .unwrap()
            .make_current()
            .unwrap()
    };

    let gl =
        unsafe { Context::from_loader_function(|name| context.get_proc_address(name) as *const _) };

    let strokes_program;
    let strokes_view;
    let strokes_color;

    let pen_cursor_program;
    let pen_cursor_view;
    let pen_cursor_erasing;
    let pen_cursor_pen_down;
    let cursor_vao;
    let cursor_buffer;

    unsafe {
        gl.enable(glow::MULTISAMPLE);
        gl.enable(glow::VERTEX_PROGRAM_POINT_SIZE);
        gl.enable(glow::DEBUG_OUTPUT);
        gl.disable(glow::CULL_FACE);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);

        strokes_program = pmb_gl::compile_program(
            &gl,
            concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/stroke_line.vert"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/stroke_line.frag"),
        );
        gl.use_program(Some(strokes_program));

        strokes_view = gl.get_uniform_location(strokes_program, "view").unwrap();
        strokes_color = gl
            .get_uniform_location(strokes_program, "strokeColor")
            .unwrap();
        gl.uniform_matrix_4_f32_slice(
            Some(&strokes_view),
            false,
            &glam::Mat4::IDENTITY.to_cols_array(),
        );

        pen_cursor_program = pmb_gl::compile_program(
            &gl,
            concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/cursor.vert"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/cursor.frag"),
        );
        gl.use_program(Some(pen_cursor_program));

        pen_cursor_erasing = gl
            .get_uniform_location(pen_cursor_program, "erasing")
            .unwrap();
        pen_cursor_pen_down = gl
            .get_uniform_location(pen_cursor_program, "penDown")
            .unwrap();
        pen_cursor_view = gl.get_uniform_location(pen_cursor_program, "view").unwrap();
        gl.uniform_1_f32(Some(&pen_cursor_erasing), 0.0);
        gl.uniform_1_f32(Some(&pen_cursor_pen_down), 0.0);
        gl.uniform_matrix_4_f32_slice(
            Some(&pen_cursor_view),
            false,
            &glam::Mat4::IDENTITY.to_cols_array(),
        );

        cursor_vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(cursor_vao));
        cursor_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(cursor_buffer));

        let float_size = std::mem::size_of::<f32>();
        let circle = powdermilk_biscuits::graphics::circle_points(1., 50);
        let bytes =
            std::slice::from_raw_parts(circle.as_ptr() as *const u8, circle.len() * float_size);

        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 2 * float_size as i32, 0);
    }

    //let stroke = Stroke::<()>::with_points(
    //    BIGPOINTS.iter().cloned().collect(),
    //    powdermilk_biscuits::rand::random(),
    //);

    let mut tesselator = StrokeTessellator::new();
    let options = StrokeOptions::default()
        .with_line_cap(LineCap::Round)
        .with_line_join(LineJoin::Round)
        .with_tolerance(0.001)
        .with_variable_line_width(0);

    //use lyon::geom::point as point2d;
    //let mut path = Path::builder_with_attributes(1);
    //path.begin(
    //    point2d(stroke.points[0].x, stroke.points[0].y),
    //    &[stroke.points[0].pressure],
    //);
    //for point in stroke.points.iter() {
    //    path.line_to(point2d(point.x, point.y), &[point.pressure]);
    //}
    //path.end(false);
    //let path = path.build();

    //let mut mesh = VertexBuffers::new();
    //let mut builder = simple_builder(&mut mesh);
    //tesselator
    //    .tessellate_path(&path, &options, &mut builder)
    //    .unwrap();

    //println!("{:?}", mesh);

    //return;

    let mut ui = {
        let PhysicalSize { width, height } = context.window().inner_size();
        Ui::<GlCoords>::new(width, height)
    };
    let mut sketch: Sketch<backend::LyonStrokeBackend> =
        if let Some(filename) = std::env::args().nth(1) {
            Sketch::with_filename(&mut ui, std::path::PathBuf::from(filename))
        } else {
            Sketch::default()
        };

    let mut config = Config::default();
    let mut cursor_visible = true;

    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        let window = context.window();
        match event {
            GlutinEvent::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                state: GlutinElementState::Pressed,
                                ..
                            },
                        ..
                    },
                ..
            }
            | GlutinEvent::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *flow = ControlFlow::Exit;
            }

            GlutinEvent::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(key),
                                state,
                                ..
                            },
                        ..
                    },
                ..
            } => {
                let key = pmb_gl::glutin_to_pmb_keycode(key);
                let state = pmb_gl::glutin_to_pmb_key_state(state);
                ui.handle_key(&mut config, &mut sketch, key, state, ui.width, ui.height);
                window.request_redraw();
            }

            GlutinEvent::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                match delta {
                    MouseScrollDelta::LineDelta(_, delta) => {
                        ui.next(&config, &mut sketch, Event::ScrollZoom(delta));
                    }
                    MouseScrollDelta::PixelDelta(delta) => {
                        ui.next(&config, &mut sketch, Event::ScrollZoom(delta.y as f32));
                    }
                }

                window.request_redraw();
            }

            GlutinEvent::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let button = pmb_gl::glutin_to_pmb_mouse_button(button);
                let state = pmb_gl::glutin_to_pmb_key_state(state);

                match (button, state) {
                    (primary, ElementState::Pressed) if primary == config.primary_button => {
                        ui.next(&config, &mut sketch, Event::MouseDown(button));
                    }
                    (primary, ElementState::Released) if primary == config.primary_button => {
                        ui.next(&config, &mut sketch, Event::MouseUp(button));
                    }
                    (pan, ElementState::Pressed) if pan == config.pan_button => {
                        ui.next(&config, &mut sketch, Event::StartPan);
                    }
                    (pan, ElementState::Released) if pan == config.pan_button => {
                        ui.next(&config, &mut sketch, Event::EndPan);
                    }
                    _ => {}
                }

                ui.prev_device = Device::Mouse;
                window.request_redraw();
            }

            GlutinEvent::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                ui.next(
                    &config,
                    &mut sketch,
                    Event::MouseMove(pmb_gl::physical_pos_to_pixel_pos(position)),
                );
                ui.prev_device = Device::Mouse;

                if config.use_mouse_for_pen {
                    if cursor_visible {
                        cursor_visible = false;
                        window.set_cursor_visible(false);
                    }
                    window.request_redraw();
                } else if !cursor_visible {
                    cursor_visible = true;
                    window.set_cursor_visible(true);
                }

                if ui.state.redraw() {
                    window.request_redraw();
                }
            }

            GlutinEvent::WindowEvent {
                event:
                    WindowEvent::Touch(
                        touch @ Touch {
                            phase,
                            pen_info: Some(_),
                            ..
                        },
                    ),
                ..
            } => {
                let touch = pmb_gl::glutin_to_pmb_touch(touch);

                match phase {
                    TouchPhase::Started => ui.next(&config, &mut sketch, Event::PenDown(touch)),
                    TouchPhase::Moved => ui.next(&config, &mut sketch, Event::PenMove(touch)),
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        ui.next(&config, &mut sketch, Event::PenUp(touch))
                    }
                }

                ui.prev_device = Device::Pen;

                if cursor_visible {
                    cursor_visible = false;
                    window.set_cursor_visible(false);
                }

                window.request_redraw();
            }

            GlutinEvent::WindowEvent {
                event:
                    WindowEvent::Touch(
                        touch @ Touch {
                            phase,
                            pen_info: None,
                            ..
                        },
                    ),
                ..
            } => {
                let touch = pmb_gl::glutin_to_pmb_touch(touch);
                ui.next(
                    &config,
                    &mut sketch,
                    match phase {
                        TouchPhase::Started => Event::Touch(touch),
                        TouchPhase::Moved => Event::TouchMove(touch),
                        TouchPhase::Ended | TouchPhase::Cancelled => Event::Release(touch),
                    },
                );

                ui.prev_device = Device::Touch;

                if cursor_visible && config.use_finger_for_pen {
                    cursor_visible = false;
                    window.set_cursor_visible(false);
                }

                window.request_redraw();
            }

            GlutinEvent::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                ui.resize(new_size.width, new_size.height, &mut sketch);
                context.resize(new_size);
                unsafe {
                    gl.viewport(0, 0, new_size.width as i32, new_size.height as i32);
                }
                window.request_redraw();
            }

            GlutinEvent::RedrawRequested(_) => {
                use std::mem::size_of;

                sketch
                    .strokes
                    .iter_mut()
                    .filter(|stroke| stroke.is_dirty())
                    .for_each(|stroke| {
                        log::debug!("replace stroke with {} points", stroke.points.len());
                        stroke.backend.replace(unsafe {
                            use powdermilk_biscuits::CoordinateSystem;
                            let brush_size_ndc = (GlCoords {})
                                .stroke_to_ndc(
                                    ui.width,
                                    ui.height,
                                    sketch.zoom,
                                    powdermilk_biscuits::graphics::StrokePoint {
                                        x: stroke.brush_size,
                                        y: 0.,
                                    },
                                )
                                .x;

                            use lyon::geom::point as point2d;
                            let mut path = Path::builder_with_attributes(1);
                            if let Some(first) = stroke.points.first() {
                                path.begin(
                                    point2d(first.x, first.y),
                                    &[first.pressure * brush_size_ndc * sketch.zoom],
                                );
                            }
                            stroke.points.iter().skip(1).for_each(|point| {
                                path.line_to(
                                    point2d(point.x, point.y),
                                    &[point.pressure * brush_size_ndc * sketch.zoom],
                                );
                            });
                            path.end(false);
                            let path = path.build();
                            let mut mesh = VertexBuffers::new();
                            let mut builder = simple_builder(&mut mesh);
                            tesselator
                                .tessellate_path(&path, &options, &mut builder)
                                .unwrap();

                            let f32_size = size_of::<f32>() as i32;
                            let mesh_vao = gl.create_vertex_array().unwrap();
                            gl.bind_vertex_array(Some(mesh_vao));

                            let mesh_points = gl.create_buffer().unwrap();
                            gl.bind_buffer(glow::ARRAY_BUFFER, Some(mesh_points));
                            gl.buffer_data_u8_slice(
                                glow::ARRAY_BUFFER,
                                bytemuck::cast_slice(&mesh.vertices),
                                glow::STATIC_DRAW,
                            );
                            gl.enable_vertex_attrib_array(0);
                            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, f32_size * 2, 0);

                            let mesh_indices = gl.create_buffer().unwrap();
                            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(mesh_indices));
                            gl.buffer_data_u8_slice(
                                glow::ELEMENT_ARRAY_BUFFER,
                                bytemuck::cast_slice(&mesh.indices),
                                glow::STATIC_DRAW,
                            );

                            let line_vao = gl.create_vertex_array().unwrap();
                            gl.bind_vertex_array(Some(line_vao));

                            let points = gl.create_buffer().unwrap();
                            gl.bind_buffer(glow::ARRAY_BUFFER, Some(points));
                            gl.buffer_data_u8_slice(
                                glow::ARRAY_BUFFER,
                                stroke.points_as_bytes(),
                                glow::STATIC_DRAW,
                            );
                            gl.enable_vertex_attrib_array(0);
                            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, f32_size * 3, 0);

                            LyonStrokeBackend {
                                dirty: false,
                                mesh_vao,
                                indices_len: mesh.indices.len() as i32,
                                line_vao,
                                points_len: stroke.points.len() as i32,
                            }
                        });
                    });

                unsafe {
                    gl.use_program(Some(strokes_program));
                    let view = pmb_gl::view_matrix(
                        sketch.zoom,
                        sketch.zoom,
                        PhysicalSize {
                            width: ui.width,
                            height: ui.height,
                        },
                        sketch.origin,
                    );
                    gl.uniform_matrix_4_f32_slice(
                        Some(&strokes_view),
                        false,
                        &view.to_cols_array(),
                    );
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }

                sketch
                    .strokes
                    .iter()
                    .filter(|stroke| stroke.visible && !stroke.erased)
                    .for_each(|stroke| unsafe {
                        gl.uniform_3_f32(
                            Some(&strokes_color),
                            stroke.color()[0] as f32 / 255.0,
                            stroke.color()[1] as f32 / 255.0,
                            stroke.color()[2] as f32 / 255.0,
                        );

                        if stroke.draw_tesselated {
                            let LyonStrokeBackend {
                                mesh_vao,
                                indices_len,
                                ..
                            } = stroke.backend().unwrap();
                            gl.bind_vertex_array(Some(*mesh_vao));
                            gl.draw_elements(
                                glow::TRIANGLES,
                                *indices_len,
                                glow::UNSIGNED_SHORT, // simple_builder uses u16 for the index type
                                0,
                            );
                        } else {
                            let LyonStrokeBackend {
                                line_vao,
                                points_len,
                                ..
                            } = stroke.backend().unwrap();
                            gl.bind_vertex_array(Some(*line_vao));
                            gl.draw_arrays(glow::LINE_STRIP, 0, *points_len);
                        }
                    });

                context.swap_buffers().unwrap();
            }

            _ => {}
        }
    });
}
