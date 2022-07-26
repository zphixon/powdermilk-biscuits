use glow::{Context, HasContext};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use powdermilk_biscuits::{State, StrokeStyle, TITLE_MODIFIED, TITLE_UNMODIFIED};
use std::mem::size_of;

fn main() {
    // build window and GL context
    let ev = EventLoop::new();
    let builder = WindowBuilder::new()
        .with_position(glutin::dpi::LogicalPosition {
            x: 1920. / 2. - 800. / 2.,
            y: 1080. + 1080. / 2. - 600. / 2.,
        })
        .with_title(TITLE_UNMODIFIED);
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

        pen_cursor_program =
            pmb_gl::compile_program(&gl, "src/shaders/cursor.vert", "src/shaders/cursor.frag");
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
            "src/shaders/stroke_line.vert",
            "src/shaders/stroke_line.frag",
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

    let mut cursor_visible = true;
    let mut input_handler = pmb_gl::InputHandler::default();
    let mut aa = true;
    let mut stroke_style = glow::LINE_STRIP;

    let mut state: State<pmb_gl::GlBackend, pmb_gl::StrokeBackend> = if let Some(filename) =
        std::env::args()
            .nth(1)
            .map(|file| std::path::PathBuf::from(file))
    {
        State::with_filename(filename)
    } else {
        State::default()
    };

    println!("stroke style {:?}", state.settings.stroke_style);

    // gl origin in stroke space
    ev.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(key),
                                state: key_state,
                                ..
                            },
                        ..
                    },
                ..
            } if key != VirtualKeyCode::Escape => {
                use VirtualKeyCode::*;
                input_handler.handle_key(key, key_state);

                if input_handler.just_pressed(C) {
                    state.clear_strokes();
                    context.window().request_redraw();
                }

                if input_handler.just_pressed(D) {
                    for stroke in state.strokes.iter() {
                        println!("stroke");
                        for point in stroke.disk.points.iter() {
                            let x = point.x;
                            let y = point.y;
                            let pressure = point.pressure;
                            println!("{x}, {y}, {pressure}");
                        }
                    }
                    println!("brush={}", state.settings.brush_size);
                    println!("zoom={:.02}", state.settings.zoom);
                    println!("origin={}", state.settings.origin);
                }

                if input_handler.just_pressed(A) {
                    aa = !aa;

                    if aa {
                        unsafe { gl.enable(glow::MULTISAMPLE) };
                    } else {
                        unsafe { gl.disable(glow::MULTISAMPLE) };
                    }

                    context.window().request_redraw();
                }

                if input_handler.just_pressed(P) {
                    stroke_style = match stroke_style {
                        glow::LINE_STRIP => glow::POINTS,
                        glow::POINTS => glow::LINE_STRIP,
                        _ => glow::LINE_STRIP,
                    };

                    context.window().request_redraw();
                }

                match (input_handler.control(), input_handler.just_pressed(Z)) {
                    (true, true) => {
                        state.undo_stroke();
                        context.window().request_redraw();
                    }
                    (false, true) => {
                        state.settings.origin = Default::default();
                        state.settings.zoom = powdermilk_biscuits::DEFAULT_ZOOM;
                        context.window().request_redraw();
                    }
                    _ => {}
                }

                if input_handler.just_pressed(Key1)
                    || input_handler.just_pressed(Key2)
                    || input_handler.just_pressed(Key3)
                    || input_handler.just_pressed(Key4)
                    || input_handler.just_pressed(Key5)
                    || input_handler.just_pressed(Key6)
                    || input_handler.just_pressed(Key7)
                    || input_handler.just_pressed(Key8)
                    || input_handler.just_pressed(Key9)
                    || input_handler.just_pressed(Key0)
                {
                    state.settings.stroke_style = unsafe {
                        std::mem::transmute(
                            match key {
                                Key1 => 0,
                                Key2 => 1,
                                Key3 => 2,
                                Key4 => 3,
                                Key5 => 4,
                                Key6 => 5,
                                Key7 => 6,
                                Key8 => 7,
                                Key9 => 8,
                                Key0 => 9,
                                _ => unreachable!(),
                            } % StrokeStyle::NUM_VARIANTS,
                        )
                    };
                    context.window().request_redraw();

                    println!("stroke style {:?}", state.settings.stroke_style);
                }

                if input_handler.just_pressed(R) {
                    state.settings.use_individual_style = !state.settings.use_individual_style;
                    context.window().request_redraw();
                }

                if input_handler.just_pressed(E) {
                    state.stylus.state.inverted = !state.stylus.state.inverted;
                    context.window().request_redraw();
                }

                // TODO probably move all the filename handling to State
                if input_handler.control() && input_handler.just_pressed(O) {
                    let _ = state.read_file(Option::<&str>::None);
                    context.window().request_redraw();
                }

                if !input_handler.shift()
                    && input_handler.control()
                    && input_handler.just_pressed(S)
                {
                    let _ = state.save_file();
                }

                if input_handler.shift() && input_handler.just_pressed(S) {
                    let num_string = std::fs::read_to_string("img/num.txt").expect("read num.txt");
                    let num = num_string.trim().parse::<usize>().expect("parse num.txt");
                    let filename = format!("img/strokes{num}.png");

                    let image = unsafe {
                        let PhysicalSize { width, height } = context.window().inner_size();
                        let mut data = std::iter::repeat(0)
                            .take(width as usize * height as usize * 4)
                            .collect::<Vec<_>>();
                        gl.read_pixels(
                            0,
                            0,
                            width as i32,
                            height as i32,
                            glow::RGBA,
                            glow::UNSIGNED_BYTE,
                            glow::PixelPackData::Slice(data.as_mut_slice()),
                        );
                        image::DynamicImage::ImageRgba8(
                            image::RgbaImage::from_raw(width, height, data).unwrap(),
                        )
                    };

                    image.flipv().save(&filename).unwrap();
                    let next_num = num + 1;
                    std::fs::write("img/num.txt", format!("{next_num}")).unwrap();
                    println!("wrote image as {filename}");
                }
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            }
            | Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                state: glutin::event::ElementState::Pressed,
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if state.modified {
                    if state
                        .ask_to_save_then_save("Would you like to save before exiting?")
                        .unwrap_or(false)
                    {
                        *control_flow = ControlFlow::Exit;
                    }
                } else {
                    *control_flow = ControlFlow::Exit;
                }
            }

            Event::WindowEvent {
                event: WindowEvent::Touch(touch),
                ..
            } => {
                cursor_visible = false;

                // TODO handle fingers
                let prev_cursor_y = input_handler.cursor_pos().y as f32;
                input_handler.handle_mouse_move(touch.location);
                let next_cursor_y = input_handler.cursor_pos().y as f32;
                let cursor_dy = next_cursor_y - prev_cursor_y;

                context.window().set_cursor_visible(cursor_visible);

                let PhysicalSize { width, height } = context.window().inner_size();
                let prev_stylus_gl =
                    pmb_gl::stroke_to_ndc(width, height, state.settings.zoom, state.stylus.point);
                let prev_stylus = pmb_gl::ndc_to_pixel(width, height, prev_stylus_gl);

                state.update(width, height, pmb_gl::glutin_to_pmb_touch(touch));

                let next_stylus_gl =
                    pmb_gl::stroke_to_ndc(width, height, state.settings.zoom, state.stylus.point);
                let next_stylus = pmb_gl::ndc_to_pixel(width, height, next_stylus_gl);

                match (
                    input_handler.button_down(MouseButton::Middle),
                    input_handler.control(),
                ) {
                    (true, false) => {
                        state.move_origin(width, height, prev_stylus, next_stylus);
                    }
                    (true, true) => state.change_zoom(cursor_dy),
                    _ => {}
                }

                context.window().request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                let zoom_in = match delta {
                    MouseScrollDelta::LineDelta(_, y) if y.is_sign_positive() => true,
                    MouseScrollDelta::PixelDelta(pos) if pos.y.is_sign_positive() => true,
                    MouseScrollDelta::LineDelta(_, y) if y.is_sign_negative() => false,
                    MouseScrollDelta::PixelDelta(pos) if pos.y.is_sign_negative() => false,
                    _ => unreachable!(),
                };
                const ZOOM_SPEED: f32 = 4.25;

                let dzoom = if zoom_in { ZOOM_SPEED } else { -ZOOM_SPEED };
                state.change_zoom(dzoom);

                context.window().request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter(c @ ('[' | ']')),
                ..
            } => {
                match c {
                    '[' => state.decrease_brush(),
                    ']' => state.increase_brush(),
                    _ => unreachable!(),
                };
                context.window().request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                input_handler.handle_mouse_button(button, state);
            }

            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                let prev = input_handler.cursor_pos();
                input_handler.handle_mouse_move(position);

                if input_handler.button_down(MouseButton::Left) {
                    let next = input_handler.cursor_pos();
                    let PhysicalSize { width, height } = context.window().inner_size();
                    state.move_origin(
                        width,
                        height,
                        pmb_gl::physical_pos_to_pixel_pos(prev),
                        pmb_gl::physical_pos_to_pixel_pos(next),
                    );
                    context.window().request_redraw();
                }

                if !cursor_visible {
                    cursor_visible = true;
                    context.window().set_cursor_visible(cursor_visible);
                    context.window().request_redraw();
                }
            }

            Event::MainEventsCleared => match (state.path.as_ref(), state.modified) {
                (Some(path), true) => {
                    let title = format!("{} (modified)", path.display());
                    context.window().set_title(title.as_str());
                }
                (Some(path), false) => context.window().set_title(&path.display().to_string()),
                (None, true) => context.window().set_title(TITLE_MODIFIED),
                (None, false) => context.window().set_title(TITLE_UNMODIFIED),
            },

            Event::RedrawRequested(_) => {
                unsafe {
                    gl.use_program(Some(strokes_program));
                    let view = pmb_gl::view_matrix(
                        state.settings.zoom,
                        state.settings.zoom,
                        context.window().inner_size(),
                        state.settings.origin,
                    );
                    gl.uniform_matrix_4_f32_slice(
                        Some(&strokes_view),
                        false,
                        &view.to_cols_array(),
                    );
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }

                for stroke in state.strokes.iter_mut() {
                    if stroke.disk.points.is_empty() || stroke.disk.erased {
                        continue;
                    }

                    unsafe {
                        let buffers = stroke.backend.get_or_insert_with(|| pmb_gl::StrokeBackend {
                            vbo: gl.create_buffer().unwrap(),
                            vao: gl.create_vertex_array().unwrap(),
                        });

                        let points_flat = std::slice::from_raw_parts(
                            stroke.disk.points.as_ptr() as *const f32,
                            stroke.disk.points.len() * 3,
                        );

                        let bytes = std::slice::from_raw_parts(
                            points_flat.as_ptr() as *const u8,
                            points_flat.len() * size_of::<f32>(),
                        );

                        gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffers.vbo));
                        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);

                        gl.bind_vertex_array(Some(buffers.vao));

                        gl.vertex_attrib_pointer_f32(
                            0,
                            2,
                            glow::FLOAT,
                            false,
                            size_of::<f32>() as i32 * 3,
                            0,
                        );
                        gl.enable_vertex_attrib_array(0);

                        gl.vertex_attrib_pointer_f32(
                            1,
                            1,
                            glow::FLOAT,
                            false,
                            size_of::<f32>() as i32 * 3,
                            size_of::<f32>() as i32 * 2,
                        );
                        gl.enable_vertex_attrib_array(1);

                        gl.uniform_3_f32(
                            Some(&strokes_color),
                            stroke.disk.color[0] as f32 / 255.0,
                            stroke.disk.color[1] as f32 / 255.0,
                            stroke.disk.color[2] as f32 / 255.0,
                        );

                        gl.uniform_1_f32(Some(&strokes_brush_size), stroke.disk.brush_size);

                        gl.draw_arrays(stroke_style, 0, stroke.disk.points.len() as i32);
                    }
                }

                if !cursor_visible {
                    let circle = powdermilk_biscuits::graphics::circle_points(
                        state.settings.brush_size as f32,
                        32,
                    );
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
                        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
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
                            if state.stylus.inverted() { 1.0 } else { 0.0 },
                        );
                        gl.uniform_1_f32(
                            Some(&pen_cursor_pen_down),
                            if state.stylus.down() { 1.0 } else { 0.0 },
                        );

                        let view = pmb_gl::view_matrix(
                            state.settings.zoom,
                            1.0,
                            context.window().inner_size(),
                            state.stylus.point,
                        );

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

            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                context.resize(size);
                unsafe {
                    gl.viewport(0, 0, size.width as i32, size.height as i32);
                };
            }

            _ => {}
        }
    });
}