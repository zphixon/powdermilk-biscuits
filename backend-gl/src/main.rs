#![cfg_attr(all(windows, feature = "pmb-release"), windows_subsystem = "windows")]

use ezgl::Ezgl;
use powdermilk_biscuits::winit::{dpi::PhysicalSize, window::Window};

fn no_winit_ezgl(window: &Window, size: PhysicalSize<u32>) -> Ezgl {
    #[cfg(all(unix, not(target_os = "macos")))]
    let reg = Some(
        Box::new(powdermilk_biscuits::winit::platform::x11::register_xlib_error_hook)
            as ezgl::glutin::api::glx::XlibErrorHookRegistrar,
    );

    #[cfg(not(all(unix, not(target_os = "macos"))))]
    let reg = None;

    Ezgl::new(&window, size.width, size.height, reg).unwrap()
}

fn main() {
    env_logger::init();

    derive_loop::pmb_loop!(
        backend_crate_name: backend_gl,
        coords_name: GlCoords,
        stroke_backend_name: GlStrokeBackend,

        window: { &window },
        egui_ctx: { &egui_glow.egui_ctx },

        before_setup:
            window = { builder.build(&ev).unwrap() }
            gl = { no_winit_ezgl(&window, window.inner_size()) }
            renderer = { backend_gl::Renderer::new(&gl) }
            egui_glow = mut {
                egui_glow::EguiGlow::new(&ev, gl.glow_context(), None)
            };

        after_setup:;

        per_event: {
            if let WinitEvent::WindowEvent { event, .. } = &event {
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
            gl.resize(new_size.width, new_size.height);
            renderer.resize(new_size, &gl);
        },

        render: {
            renderer.render(&gl, &mut sketch, &widget, size, cursor_visible);
            egui_glow.paint(&window);
            gl.swap_buffers().unwrap();
        },
    );
}
