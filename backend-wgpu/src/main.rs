#![cfg_attr(all(windows, feature = "pmb-release"), windows_subsystem = "windows")]
#![allow(clippy::unnecessary_operation)] // -Z macro-backtrace doesn't help so. shh.

fn main() {
    env_logger::init();

    pmb_macros::pmb_loop!(
        coords: backend_wgpu::WgpuCoords,
        stroke_backend: backend_wgpu::WgpuStrokeBackend,

        egui_ctx: { &egui_ctx },

        setup:
            egui_winit = mut { egui_winit::State::new(&ev) }
            egui_ctx = mut { powdermilk_biscuits::egui::Context::default() }
            graphics = mut {
                let mut graphics = futures::executor::block_on(backend_wgpu::Graphics::new(&window));
                graphics.buffer_all_strokes(&mut sketch);
                graphics
            }
            egui_painter = mut {
                egui_wgpu::Renderer::new(&graphics.device, graphics.surface_format, 1, 0)
            };

        per_event: {
            if let winit::event::Event::WindowEvent { event, .. } = &event {
                let response = egui_winit.on_event(&egui_ctx, event);

                if response.repaint {
                    window.request_redraw();
                }

                if response.consumed {
                    return;
                }
            }
        },

        resize: {
            graphics.resize(new_size);
        },

        render: {
            let egui_data = egui_ctx.run(egui_winit.take_egui_input(&window), |ctx| {
                powdermilk_biscuits::ui::egui(ctx, &mut sketch, &mut widget, &mut config)
            });

            let egui_tris = egui_ctx.tessellate(egui_data.shapes);

            match graphics.render(
                &mut sketch,
                &widget,
                size,
                cursor_visible,
                &egui_tris,
                &egui_data.textures_delta,
                &mut egui_painter,
            ) {
                Err(wgpu::SurfaceError::Lost) => graphics.resize(graphics.size),
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    powdermilk_biscuits::ui::error(powdermilk_biscuits::s!(&OutOfMemory));
                    flow.set_exit();
                }
                _ => {}
            }
        },
    );
}
