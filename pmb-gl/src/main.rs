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
use pmb_gl::{GlCoords, GlStrokeBackend};
use powdermilk_biscuits::{
    event::{ElementState, Event},
    ui::Ui,
    Config, Device, Sketch, Tool,
};

fn main() {
    let ev = EventLoop::new();
    let builder = WindowBuilder::new().with_position(glutin::dpi::LogicalPosition {
        x: 1920. / 2. - 800. / 2.,
        y: 1080. + 1080. / 2. - 600. / 2.,
    });

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
    let pen_cursor_program;

    let strokes_view;
    let strokes_color;
    let strokes_brush_size;

    let pen_cursor_view;
    let pen_cursor_erasing;
    let pen_cursor_pen_down;

    // set up shaders
    unsafe {
        gl.enable(glow::MULTISAMPLE);
        gl.enable(glow::VERTEX_PROGRAM_POINT_SIZE);
        gl.enable(glow::DEBUG_OUTPUT);
        gl.disable(glow::CULL_FACE);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);

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
        strokes_brush_size = gl
            .get_uniform_location(strokes_program, "brushSize")
            .unwrap();
        gl.uniform_matrix_4_f32_slice(
            Some(&strokes_view),
            false,
            &glam::Mat4::IDENTITY.to_cols_array(),
        );
    };

    let mut ui = {
        let PhysicalSize { width, height } = context.window().inner_size();
        Ui::<GlCoords>::new(width, height)
    };
    let mut sketch: Sketch<pmb_gl::GlStrokeBackend> =
        if let Some(filename) = std::env::args().nth(1) {
            Sketch::with_filename(&mut ui, std::path::PathBuf::from(filename))
        } else {
            Sketch::default()
        };

    let mut config = Config::default();
    let mut cursor_visible = true;
    let mut size = context.window().inner_size();

    for stroke in sketch.strokes.iter_mut() {
        stroke.replace_backend_with(|points_bytes, mesh_bytes, mesh_len| unsafe {
            let f32_size = std::mem::size_of::<f32>() as i32;

            let line_vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(line_vao));

            let points = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(points));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, points_bytes, glow::STATIC_DRAW);

            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, f32_size * 3, 0);
            gl.vertex_attrib_pointer_f32(1, 1, glow::FLOAT, false, f32_size * 3, f32_size * 2);
            gl.enable_vertex_attrib_array(0);
            gl.enable_vertex_attrib_array(1);

            let mesh_vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(mesh_vao));
            let mesh = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(mesh));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, mesh_bytes, glow::STATIC_DRAW);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, f32_size * 3, 0);
            gl.vertex_attrib_pointer_f32(1, 1, glow::FLOAT, false, f32_size * 3, f32_size * 2);
            gl.enable_vertex_attrib_array(0);
            gl.enable_vertex_attrib_array(1);

            GlStrokeBackend {
                line_vao,
                points,
                mesh_vao,
                mesh,
                mesh_len: mesh_len as i32,
                dirty: false,
            }
        });
    }

    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;

        //println!("{:?} {:?}", config.active_tool, event);
        //println!("{}", ui.stylus.pos);
        //println!("{:?}", ui.state);
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
                ui.handle_key(&config, &mut sketch, key, state, size.width, size.height);

                if ui
                    .input
                    .combo_just_pressed(&config.debug_toggle_use_mouse_for_pen)
                {
                    config.use_mouse_for_pen = !config.use_mouse_for_pen;
                    println!("using mouse for pen? {}", config.use_mouse_for_pen);
                }

                if ui
                    .input
                    .combo_just_pressed(&config.debug_toggle_use_finger_for_pen)
                {
                    config.use_finger_for_pen = !config.use_finger_for_pen;
                    println!("using finger for pen? {}", config.use_finger_for_pen);
                }

                if ui
                    .input
                    .combo_just_pressed(&config.debug_toggle_stylus_invertability)
                {
                    config.stylus_may_be_inverted = !config.stylus_may_be_inverted;
                    println!("stylus invertable? {}", config.stylus_may_be_inverted);
                }

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
                size = new_size;
                ui.resize(new_size.width, new_size.height, &mut sketch);
                context.resize(new_size);
                unsafe {
                    gl.viewport(0, 0, new_size.width as i32, new_size.height as i32);
                }
                window.request_redraw();
            }

            GlutinEvent::RedrawRequested(_) => {
                use std::mem::size_of;

                unsafe {
                    gl.use_program(Some(strokes_program));
                    let view = pmb_gl::view_matrix(sketch.zoom, sketch.zoom, size, sketch.origin);
                    gl.uniform_matrix_4_f32_slice(
                        Some(&strokes_view),
                        false,
                        &view.to_cols_array(),
                    );
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }

                if let Some(last) = sketch.strokes.last_mut() {
                    if last.is_dirty() {
                        last.replace_backend_with(|points_bytes, mesh_bytes, mesh_len| unsafe {
                            let f32_size = size_of::<f32>() as i32;

                            let line_vao = gl.create_vertex_array().unwrap();
                            gl.bind_vertex_array(Some(line_vao));

                            let points = gl.create_buffer().unwrap();
                            gl.bind_buffer(glow::ARRAY_BUFFER, Some(points));
                            gl.buffer_data_u8_slice(
                                glow::ARRAY_BUFFER,
                                points_bytes,
                                glow::STATIC_DRAW,
                            );

                            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, f32_size * 3, 0);
                            gl.vertex_attrib_pointer_f32(
                                1,
                                1,
                                glow::FLOAT,
                                false,
                                f32_size * 3,
                                f32_size * 2,
                            );
                            gl.enable_vertex_attrib_array(0);
                            gl.enable_vertex_attrib_array(1);

                            let mesh_vao = gl.create_vertex_array().unwrap();
                            gl.bind_vertex_array(Some(mesh_vao));
                            let mesh = gl.create_buffer().unwrap();
                            gl.bind_buffer(glow::ARRAY_BUFFER, Some(mesh));
                            gl.buffer_data_u8_slice(
                                glow::ARRAY_BUFFER,
                                mesh_bytes,
                                glow::STATIC_DRAW,
                            );
                            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, f32_size * 3, 0);
                            gl.vertex_attrib_pointer_f32(
                                1,
                                1,
                                glow::FLOAT,
                                false,
                                f32_size * 3,
                                f32_size * 2,
                            );
                            gl.enable_vertex_attrib_array(0);
                            gl.enable_vertex_attrib_array(1);

                            GlStrokeBackend {
                                line_vao,
                                points,
                                mesh_vao,
                                mesh,
                                mesh_len: mesh_len as i32,
                                dirty: false,
                            }
                        });
                    }
                }

                for stroke in sketch.strokes.iter() {
                    if !stroke.visible || stroke.points().is_empty() || stroke.erased() {
                        continue;
                    }

                    if stroke.draw_tesselated {
                        let GlStrokeBackend {
                            mesh_vao,
                            mesh,
                            mesh_len,
                            ..
                        } = stroke.backend().unwrap();
                        unsafe {
                            gl.bind_vertex_array(Some(*mesh_vao));
                            gl.bind_buffer(glow::ARRAY_BUFFER, Some(*mesh));
                            gl.uniform_3_f32(
                                Some(&strokes_color),
                                stroke.color()[0] as f32 / 255.0,
                                stroke.color()[1] as f32 / 255.0,
                                stroke.color()[2] as f32 / 255.0,
                            );
                            gl.uniform_1_f32(Some(&strokes_brush_size), stroke.brush_size());
                            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, *mesh_len);
                        }
                    } else {
                        let GlStrokeBackend {
                            line_vao, points, ..
                        } = stroke.backend().unwrap();
                        unsafe {
                            gl.bind_vertex_array(Some(*line_vao));
                            gl.bind_buffer(glow::ARRAY_BUFFER, Some(*points));
                            gl.uniform_3_f32(
                                Some(&strokes_color),
                                stroke.color()[0] as f32 / 255.0,
                                stroke.color()[1] as f32 / 255.0,
                                stroke.color()[2] as f32 / 255.0,
                            );
                            gl.uniform_1_f32(Some(&strokes_brush_size), stroke.brush_size());
                            gl.draw_arrays(glow::LINE_STRIP, 0, stroke.points().len() as i32);
                        }
                    }
                }

                if !cursor_visible {
                    let circle =
                        powdermilk_biscuits::graphics::circle_points(ui.brush_size as f32, 32);

                    unsafe {
                        gl.use_program(Some(pen_cursor_program));
                        let vbo = gl.create_buffer().unwrap();
                        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                        let vao = gl.create_vertex_array().unwrap();
                        gl.bind_vertex_array(Some(vao));
                        let bytes = std::slice::from_raw_parts(
                            circle.as_ptr() as *const u8,
                            circle.len() * size_of::<f32>(),
                        );
                        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
                        gl.enable_vertex_attrib_array(0);
                        gl.vertex_attrib_pointer_f32(
                            0,
                            2,
                            glow::FLOAT,
                            false,
                            size_of::<f32>() as i32 * 2,
                            0,
                        );

                        gl.uniform_1_f32(
                            Some(&pen_cursor_erasing),
                            if ui.active_tool == Tool::Eraser {
                                1.0
                            } else {
                                0.0
                            },
                        );
                        gl.uniform_1_f32(
                            Some(&pen_cursor_pen_down),
                            if ui.stylus.down() { 1.0 } else { 0.0 },
                        );

                        let view = pmb_gl::view_matrix(sketch.zoom, 1.0, size, ui.stylus.point);

                        gl.uniform_matrix_4_f32_slice(
                            Some(&pen_cursor_view),
                            false,
                            &view.to_cols_array(),
                        );

                        gl.draw_arrays(glow::LINE_LOOP, 0, circle.len() as i32 / 2);
                    }
                }

                context.swap_buffers().unwrap();
            }

            _ => {}
        }
    });
}
