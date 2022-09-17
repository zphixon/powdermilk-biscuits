#![cfg_attr(all(windows, feature = "pmb-release"), windows_subsystem = "windows")]

use glow::{Context, HasContext};
use glutin::{
    event::{
        ElementState as GlutinElementState, Event as GlutinEvent, KeyboardInput, MouseScrollDelta,
        Touch, TouchPhase, VirtualKeyCode, WindowEvent,
    },
    event_loop::EventLoop,
    window::WindowBuilder,
    ContextBuilder,
};
use pmb_gl::GlStrokeBackend;
use powdermilk_biscuits::{
    bytemuck,
    event::{ElementState, Event},
    ui::Ui,
    Config, Device, Sketch, Tool,
};
use std::sync::Arc;

derive_pmb_loop::pmb_loop!(
    loop_name: pmb_loop,
    windowing_crate_name: glutin,
    event_enum_name: GlutinEvent,
    element_state_name: GlutinElementState,
    backend_crate_name: pmb_gl,
    coords_name: GlCoords,
    stroke_backend_name: GlStrokeBackend,
    keycode_translation: glutin_to_pmb_keycode,
    mouse_button_translation: glutin_to_pmb_mouse_button,
    key_state_translation: glutin_to_pmb_key_state,
    touch_translation: glutin_to_pmb_touch,
    window: {context.window()},

    bindings:
        context = { unsafe {
            ContextBuilder::new()
                .with_vsync(true)
                .with_gl(glutin::GlRequest::Latest)
                .with_multisampling(4)
                .build_windowed(builder, &ev)
                .unwrap()
                .make_current()
                .unwrap()
        }}

        gl = { Arc::new(unsafe {
            Context::from_loader_function(|name| context.get_proc_address(name) as *const _)
        })}

        egui_glow = mut {
            egui_glow::EguiGlow::new(&ev, Arc::clone(&gl))
        }

        clear_color = mut { [0., 0., 0.] }

        strokes_program = no_init
        pen_cursor_program = no_init

        strokes_view = no_init
        strokes_color = no_init
        strokes_brush_size = no_init

        pen_cursor_view = no_init
        pen_cursor_erasing = no_init
        pen_cursor_pen_down = no_init

        cursor_vao = no_init
        cursor_buffer = no_init;

    graphics_setup:
        _nada = {unsafe {
            gl.enable(glow::SRGB8_ALPHA8);
            gl.enable(glow::FRAMEBUFFER_SRGB);
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

            cursor_vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(cursor_vao));
            cursor_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(cursor_buffer));

            let float_size = std::mem::size_of::<f32>();
            let circle = powdermilk_biscuits::graphics::cursor_geometry(1., 50);
            let bytes =
                std::slice::from_raw_parts(circle.as_ptr() as *const u8, circle.len() * float_size);

            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 2 * float_size as i32, 0);
        }};

    per_event: {
        match &event {
            GlutinEvent::WindowEvent { event, .. } => {
                let response = egui_glow.on_event(&event);
                if response.repaint {
                    context.window().request_redraw();
                    flow.set_poll();
                }
                if response.consumed {
                    return;
                }
            }

            _ => {}
        }

        let redraw_after = egui_glow.run(context.window(), |ctx| {
            egui::SidePanel::left("side panel").show(ctx, |ui| {
                ui.heading("Real Hot Item");
                ui.color_edit_button_rgb(&mut clear_color);
            });
        });

        if redraw_after.is_zero() {
            flow.set_poll();
            context.window().request_redraw();
        } else if let Some(after) = std::time::Instant::now().checked_add(redraw_after) {
            flow.set_wait_until(after);
        } else {
            flow.set_wait();
        }
    },

    resize: {
        size = new_size;
        ui.resize(new_size.width, new_size.height, &mut sketch);
        context.resize(new_size);
        unsafe {
            gl.viewport(0, 0, new_size.width as i32, new_size.height as i32);
        }
        context.window().request_redraw();
        config.resize_window(new_size.width, new_size.height);
    },

    render: {
        use std::mem::size_of;

        unsafe {
            gl.use_program(Some(strokes_program));
            let view = pmb_gl::view_matrix(sketch.zoom, sketch.zoom, size, sketch.origin);
            gl.uniform_matrix_4_f32_slice(
                Some(&strokes_view),
                false,
                &view.to_cols_array(),
            );
            gl.clear_color(clear_color[0], clear_color[1], clear_color[2], 1.);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }

        sketch
            .strokes
            .values_mut()
            .filter(|stroke| stroke.is_dirty())
            .for_each(|stroke| {
                log::debug!("replace stroke with {} points", stroke.points.len());
                stroke.backend.replace(unsafe {
                    let f32_size = size_of::<f32>() as i32;

                    let line_vao = gl.create_vertex_array().unwrap();
                    gl.bind_vertex_array(Some(line_vao));

                    let points = gl.create_buffer().unwrap();
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(points));
                    gl.buffer_data_u8_slice(
                        glow::ARRAY_BUFFER,
                        bytemuck::cast_slice(&stroke.points),
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
                        bytemuck::cast_slice(&stroke.mesh.vertices),
                        glow::STATIC_DRAW,
                    );
                    gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, f32_size * 2, 0);
                    gl.enable_vertex_attrib_array(0);

                    let mesh_ebo = gl.create_buffer().unwrap();
                    gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(mesh_ebo));
                    gl.buffer_data_u8_slice(
                        glow::ELEMENT_ARRAY_BUFFER,
                        bytemuck::cast_slice(&stroke.mesh.indices),
                        glow::STATIC_DRAW,
                    );

                    GlStrokeBackend {
                        line_vao,
                        line_len: stroke.points.len() as i32,
                        mesh_vao,
                        mesh_len: stroke.mesh.indices.len() as i32,
                        dirty: false,
                    }
                });
            });

        sketch.visible_strokes().for_each(|stroke| unsafe {
            gl.uniform_3_f32(
                Some(&strokes_color),
                stroke.color()[0] as f32 / 255.0,
                stroke.color()[1] as f32 / 255.0,
                stroke.color()[2] as f32 / 255.0,
            );
            gl.uniform_1_f32(Some(&strokes_brush_size), stroke.brush_size());

            if stroke.draw_tesselated {
                let GlStrokeBackend {
                    mesh_vao, mesh_len, ..
                } = stroke.backend().unwrap();
                gl.bind_vertex_array(Some(*mesh_vao));
                gl.draw_elements(glow::TRIANGLES, *mesh_len, glow::UNSIGNED_SHORT, 0);
            } else {
                let GlStrokeBackend {
                    line_vao, line_len, ..
                } = stroke.backend().unwrap();
                gl.bind_vertex_array(Some(*line_vao));
                gl.draw_arrays(glow::LINE_STRIP, 0, *line_len);
            }
        });

        if !cursor_visible {
            unsafe {
                gl.use_program(Some(pen_cursor_program));
                gl.bind_vertex_array(Some(cursor_vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(cursor_buffer));

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

                let view = pmb_gl::view_matrix(
                    sketch.zoom,
                    ui.brush_size as f32,
                    size,
                    ui.stylus.point,
                );

                gl.uniform_matrix_4_f32_slice(
                    Some(&pen_cursor_view),
                    false,
                    &view.to_cols_array(),
                );

                gl.draw_arrays(glow::LINES, 0, 50 * 2);
            }
        }

        egui_glow.paint(context.window());
        context.swap_buffers().unwrap();
    },
);

fn main() {
    env_logger::init();
    pmb_loop();
}
