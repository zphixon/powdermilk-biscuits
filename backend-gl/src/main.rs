#![cfg_attr(all(windows, feature = "pmb-release"), windows_subsystem = "windows")]

use backend_gl::{GlCoords, GlStrokeBackend, Renderer};
use egui_glow::EguiGlow;
use ezgl::Ezgl;
use powdermilk_biscuits::{
    config::Config,
    egui::Context as EguiContext,
    loop_::{loop_, LoopContext, LoopEvent, PerEvent, RenderResult},
    ui::widget::SketchWidget,
    winit::{dpi::PhysicalSize, event::Event as WinitEvent, event_loop::EventLoop, window::Window},
    Sketch,
};

fn no_winit_ezgl(window: &Window, size: PhysicalSize<u32>) -> Ezgl {
    #[cfg(all(unix, not(target_os = "macos")))]
    let reg = Some(
        Box::new(powdermilk_biscuits::winit::platform::x11::register_xlib_error_hook)
            as ezgl::glutin::api::glx::XlibErrorHookRegistrar,
    );

    #[cfg(not(all(unix, not(target_os = "macos"))))]
    let reg = None;

    Ezgl::new(
        &window,
        size.width,
        size.height,
        reg,
        Some(backend_gl::SAMPLE_COUNT as u8),
    )
    .unwrap()
}

fn main() {
    tracing_subscriber::fmt::init();
    loop_::<GlStrokeBackend, GlCoords, GlLoop>();
}

struct GlLoop {
    gl: Ezgl,
    renderer: Renderer,
    egui_glow: EguiGlow,
}

impl LoopContext<GlStrokeBackend, GlCoords> for GlLoop {
    fn setup(ev: &EventLoop<LoopEvent>, window: &Window, _: &mut Sketch<GlStrokeBackend>) -> Self {
        let gl = no_winit_ezgl(window, window.inner_size());
        let size = window.inner_size();
        GlLoop {
            renderer: Renderer::new(&gl, size.width, size.height),
            egui_glow: EguiGlow::new(ev, gl.glow_context(), None),
            gl,
        }
    }

    fn per_event(
        &mut self,
        event: &WinitEvent<LoopEvent>,
        window: &Window,
        sketch: &mut Sketch<GlStrokeBackend>,
        widget: &mut SketchWidget<GlCoords>,
        config: &mut Config,
    ) -> PerEvent {
        if let WinitEvent::WindowEvent { event, .. } = &event {
            let response = self.egui_glow.on_event(event);

            if response.consumed {
                return PerEvent::ConsumedByEgui(response.repaint);
            }
        }

        let redraw_after = self.egui_glow.run(window, |ctx| {
            powdermilk_biscuits::ui::egui(ctx, sketch, widget, config);
        });

        if redraw_after.is_zero() {
            PerEvent::Redraw
        } else {
            PerEvent::Nothing
        }
    }

    fn egui_ctx(&self) -> &EguiContext {
        &self.egui_glow.egui_ctx
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.gl.resize(new_size.width, new_size.height);
        self.renderer.resize(new_size, &self.gl);
    }

    fn render(
        &mut self,
        window: &Window,
        sketch: &mut Sketch<GlStrokeBackend>,
        widget: &mut SketchWidget<GlCoords>,
        _: &mut Config,
        size: PhysicalSize<u32>,
        cursor_visible: bool,
    ) -> RenderResult {
        self.renderer
            .render(&self.gl, sketch, widget, size, cursor_visible);
        self.egui_glow.paint(window);
        self.gl.swap_buffers().unwrap();
        RenderResult::Nothing
    }
}
