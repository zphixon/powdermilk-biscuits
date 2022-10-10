#![cfg_attr(all(windows, feature = "pmb-release"), windows_subsystem = "windows")]

use backend_wgpu::{Graphics, WgpuCoords, WgpuStrokeBackend};
use powdermilk_biscuits::{
    config::Config,
    egui::Context as EguiContext,
    loop_::{loop_, LoopContext, PerEvent},
    ui::widget::SketchWidget,
    winit::{dpi::PhysicalSize, event::Event as WinitEvent, event_loop::EventLoop, window::Window},
    Sketch,
};

fn main() {
    loop_::<WgpuStrokeBackend, WgpuCoords, GlLoop>();
}

struct GlLoop {
    egui_winit: egui_winit::State,
    egui_ctx: EguiContext,
    graphics: Graphics,
    egui_painter: egui_wgpu::Renderer,
}

impl LoopContext<WgpuStrokeBackend, WgpuCoords> for GlLoop {
    fn setup(
        ev: &EventLoop<()>,
        window: &Window,
        sketch: &mut Sketch<WgpuStrokeBackend>,
    ) -> GlLoop {
        let mut graphics = futures::executor::block_on(Graphics::new(window));
        graphics.buffer_all_strokes(sketch);

        GlLoop {
            egui_winit: egui_winit::State::new(ev),
            egui_ctx: EguiContext::default(),
            egui_painter: egui_wgpu::Renderer::new(&graphics.device, graphics.surface_format, 1, 0),
            graphics,
        }
    }

    fn per_event(
        &mut self,
        event: &WinitEvent<()>,
        _: &Window,
        _: &mut Sketch<WgpuStrokeBackend>,
        _: &mut SketchWidget<WgpuCoords>,
        _: &mut Config,
    ) -> PerEvent {
        if let WinitEvent::WindowEvent { event, .. } = &event {
            let response = self.egui_winit.on_event(&self.egui_ctx, event);

            if response.consumed {
                return PerEvent::ConsumedByEgui(response.repaint);
            }
        }

        PerEvent::Nothing
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.graphics.resize(new_size);
    }

    fn render(
        &mut self,
        window: &Window,
        sketch: &mut Sketch<WgpuStrokeBackend>,
        widget: &mut SketchWidget<WgpuCoords>,
        config: &mut Config,
        size: PhysicalSize<u32>,
        cursor_visible: bool,
    ) {
        let egui_data = self
            .egui_ctx
            .run(self.egui_winit.take_egui_input(window), |ctx| {
                powdermilk_biscuits::ui::egui(ctx, sketch, widget, config);
            });

        let egui_tris = self.egui_ctx.tessellate(egui_data.shapes);

        match self.graphics.render(
            sketch,
            widget,
            cursor_visible,
            &egui_tris,
            &egui_data.textures_delta,
            &mut self.egui_painter,
        ) {
            Err(wgpu::SurfaceError::Lost) => self.graphics.resize(size),
            Err(wgpu::SurfaceError::OutOfMemory) => {
                powdermilk_biscuits::ui::error(powdermilk_biscuits::s!(&OutOfMemory));
                panic!();
            }
            _ => {}
        }
    }

    fn egui_ctx(&self) -> &EguiContext {
        &self.egui_ctx
    }
}
