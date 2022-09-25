#![cfg_attr(all(windows, feature = "pmb-release"), windows_subsystem = "windows")]

use backend_gl::GlStrokeBackend;
use glow::{Context, HasContext};
use glutin::{
    config::{ConfigSurfaceTypes, ConfigTemplateBuilder},
    context::{ContextAttributesBuilder, PossiblyCurrentGlContext},
    display::{Display, DisplayApiPreference},
    prelude::{GlDisplay, NotCurrentGlContextSurfaceAccessor},
    surface::{GlSurface, SurfaceAttributesBuilder, WindowSurface},
};
use powdermilk_biscuits::bytemuck;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::{num::NonZeroU32, sync::Arc};

fn main() {
    env_logger::init();
    pmb_loop();
}

derive_loop::pmb_loop!(
    windowing_crate_name: winit,
    backend_crate_name: backend_gl,
    coords_name: GlCoords,
    stroke_backend_name: GlStrokeBackend,
    keycode_translation: glutin_to_pmb_keycode,
    mouse_button_translation: glutin_to_pmb_mouse_button,
    key_state_translation: glutin_to_pmb_key_state,
    touch_translation: glutin_to_pmb_touch,

    window: { &window },
    egui_ctx: { &egui_glow.egui_ctx },

    bindings:
        window = { builder.build(&ev).unwrap() }

        display = { unsafe {
            Display::from_raw(
                window.raw_display_handle(),
                DisplayApiPreference::EglThenWgl(Some(window.raw_window_handle())),
            )
            .unwrap()
        }}

        gl_config = { unsafe {
            display
                .find_configs(
                    ConfigTemplateBuilder::new()
                        .compatible_with_native_window(window.raw_window_handle())
                        .with_surface_type(ConfigSurfaceTypes::WINDOW)
                        .with_sample_buffers(4)
                        .build(),
                )
                .unwrap()
                .nth(1)
                .unwrap()
        }}

        gl_attrs = {
            let PhysicalSize { width, height } = window.inner_size();
            SurfaceAttributesBuilder::<WindowSurface>::new().build(
                window.raw_window_handle(),
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            )
        }

        surface = { unsafe {
            display
                .create_window_surface(&gl_config, &gl_attrs)
                .unwrap()
        }}

        context = { unsafe {
            display
                .create_context(
                    &gl_config,
                    &ContextAttributesBuilder::new().build(Some(window.raw_window_handle())),
                )
                .unwrap()
                .make_current(&surface)
                .unwrap()
        }}

        gl = { Arc::new(unsafe {
            Context::from_loader_function(|name| {
                context.get_proc_address(&std::ffi::CString::new(name).unwrap()) as *const _
            })
        })}

        egui_glow = mut {
            egui_glow::EguiGlow::new(&ev, Arc::clone(&gl), None)
        }

        line_strokes_program = no_init
        mesh_strokes_program = no_init
        pen_cursor_program = no_init

        strokes_view = no_init
        strokes_color = no_init

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

            pen_cursor_program = backend_gl::compile_program(
                &gl,
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/cursor.vert")),
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/cursor.frag")),
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

            line_strokes_program = backend_gl::compile_program(
                &gl,
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/stroke_line.vert")),
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/stroke_line.frag")),
            );

            mesh_strokes_program = backend_gl::compile_program(
                &gl,
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/stroke_line.vert")),
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/stroke_mesh.frag")),
            );

            strokes_view = gl.get_uniform_location(line_strokes_program, "view").unwrap();
            strokes_color = gl
                .get_uniform_location(line_strokes_program, "strokeColor")
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
        if let winit::event::Event::WindowEvent { event, .. } = &event {
            let response = egui_glow.on_event(event);
            if response.repaint {
                window.request_redraw();
                flow.set_poll();
            }

            if response.consumed {
                return;
            }
        }

        let redraw_after = egui_glow.run(&window, |ctx| {
            powdermilk_biscuits::ui::egui(ctx, &mut sketch, &mut widget, &mut config);
        });

        if redraw_after.is_zero() {
            flow.set_poll();
            window.request_redraw();
        } else if let Some(after) = std::time::Instant::now().checked_add(redraw_after) {
            flow.set_wait_until(after);
        }
    },

    resize: {
        size = new_size;
        widget.resize(new_size.width, new_size.height, &mut sketch);
        surface.resize(
            &context,
            NonZeroU32::new(new_size.width).unwrap(),
            NonZeroU32::new(new_size.height).unwrap(),
        );
        unsafe {
            gl.viewport(0, 0, new_size.width as i32, new_size.height as i32);
        }
        window.request_redraw();
        config.resize_window(new_size.width, new_size.height);
    },

    render: {
        use std::mem::size_of;

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

        unsafe {
            gl.clear_color(sketch.bg_color[0], sketch.bg_color[1], sketch.bg_color[2], 1.);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }

        sketch.visible_strokes().for_each(|stroke| unsafe {
            gl.use_program(Some(line_strokes_program));
            let view = backend_gl::view_matrix(sketch.zoom, sketch.zoom, size, sketch.origin);
            gl.uniform_matrix_4_f32_slice(Some(&strokes_view), false, &view.to_cols_array());
            gl.uniform_3_f32(
                Some(&strokes_color),
                stroke.color[0],
                stroke.color[1],
                stroke.color[2],
            );

            let GlStrokeBackend {
                line_vao, line_len, ..
            } = stroke.backend().unwrap();
            gl.bind_vertex_array(Some(*line_vao));
            gl.draw_arrays(glow::LINE_STRIP, 0, *line_len);

            if stroke.draw_tesselated {
                gl.use_program(Some(mesh_strokes_program));
                gl.uniform_matrix_4_f32_slice(Some(&strokes_view), false, &view.to_cols_array());
                gl.uniform_3_f32(
                    Some(&strokes_color),
                    stroke.color[0],
                    stroke.color[1],
                    stroke.color[2],
                );

                let GlStrokeBackend {
                    mesh_vao, mesh_len, ..
                } = stroke.backend().unwrap();
                gl.bind_vertex_array(Some(*mesh_vao));
                gl.draw_elements(glow::TRIANGLES, *mesh_len, glow::UNSIGNED_SHORT, 0);
            }
        });

        if !cursor_visible {
            unsafe {
                gl.use_program(Some(pen_cursor_program));
                gl.bind_vertex_array(Some(cursor_vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(cursor_buffer));

                gl.uniform_1_f32(
                    Some(&pen_cursor_erasing),
                    if widget.active_tool == powdermilk_biscuits::Tool::Eraser {
                        1.0
                    } else {
                        0.0
                    },
                );
                gl.uniform_1_f32(
                    Some(&pen_cursor_pen_down),
                    if widget.stylus.down() { 1.0 } else { 0.0 },
                );

                let view = backend_gl::view_matrix(
                    sketch.zoom,
                    widget.brush_size as f32,
                    size,
                    widget.stylus.point,
                );

                gl.uniform_matrix_4_f32_slice(
                    Some(&pen_cursor_view),
                    false,
                    &view.to_cols_array(),
                );

                gl.draw_arrays(glow::LINES, 0, 50 * 2);
            }
        }

        egui_glow.paint(&window);
        surface.swap_buffers(&context).unwrap();
    },
);
