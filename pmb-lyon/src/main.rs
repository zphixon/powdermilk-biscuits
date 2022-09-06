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
use pmb_gl::GlCoords;
use powdermilk_biscuits::{
    event::{ElementState, Event},
    ui::Ui,
    Config,
    Device,
    Sketch, //Tool,
};

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

    let mut tesselator = StrokeTessellator::new();
    let options = StrokeOptions::default()
        .with_line_cap(LineCap::Round)
        .with_line_join(LineJoin::Round)
        .with_tolerance(0.001)
        .with_variable_line_width(0);

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

                if cursor_visible {
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
                            use lyon::geom::point as point2d;
                            let mut path = Path::builder_with_attributes(1);
                            if let Some(first) = stroke.points.first() {
                                path.begin(
                                    point2d(first.x, first.y),
                                    &[first.pressure * stroke.brush_size * 2.],
                                );
                            }
                            stroke.points.iter().skip(1).for_each(|point| {
                                path.line_to(
                                    point2d(point.x, point.y),
                                    &[point.pressure * stroke.brush_size * 2.],
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

                            backend::LyonStrokeBackend {
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

                sketch.visible_strokes().for_each(|stroke| unsafe {
                    gl.uniform_3_f32(
                        Some(&strokes_color),
                        stroke.color()[0] as f32 / 255.0,
                        stroke.color()[1] as f32 / 255.0,
                        stroke.color()[2] as f32 / 255.0,
                    );

                    if stroke.draw_tesselated {
                        let backend::LyonStrokeBackend {
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
                        let backend::LyonStrokeBackend {
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
