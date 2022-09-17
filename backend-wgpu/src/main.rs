#![cfg_attr(all(windows, feature = "pmb-release"), windows_subsystem = "windows")]

use powdermilk_biscuits::{
    event::{ElementState, Event},
    ui::{self, Ui},
    Config, Device, Sketch,
};
use wgpu::SurfaceError;
use winit::{
    event::{
        ElementState as WinitElementState, Event as WinitEvent, KeyboardInput, MouseScrollDelta,
        Touch, TouchPhase, VirtualKeyCode, WindowEvent,
    },
    event_loop::EventLoop,
    window::WindowBuilder,
};

derive_loop::pmb_loop!(
    loop_name: pmb_loop,
    windowing_crate_name: winit,
    event_enum_name: WinitEvent,
    element_state_name: WinitElementState,
    backend_crate_name: backend_wgpu,
    coords_name: WgpuCoords,
    stroke_backend_name: WgpuStrokeBackend,
    keycode_translation: winit_to_pmb_keycode,
    mouse_button_translation: winit_to_pmb_mouse_button,
    key_state_translation: winit_to_pmb_key_state,
    touch_translation: winit_to_pmb_touch,
    window: {&window},

    bindings:
        window = {
            builder.build(&ev).unwrap()
        };

    graphics_setup:
        graphics = mut {
            let mut graphics = futures::executor::block_on(backend_wgpu::Graphics::new(&window));
            graphics.buffer_all_strokes(&mut sketch);
            graphics
        };

    per_event: {},

    resize: {
        size = new_size;
        ui.resize(new_size.width, new_size.height, &mut sketch);
        graphics.resize(new_size);
        window.request_redraw();
        config.resize_window(new_size.width, new_size.height);
    },

    render: {
        match graphics.render(&mut sketch, &ui, size, cursor_visible) {
            Err(SurfaceError::Lost) => graphics.resize(graphics.size),
            Err(SurfaceError::OutOfMemory) => {
                ui::error("Out of memory!");
                flow.set_exit();
            }
            _ => {}
        }
    },
);

fn main() {
    env_logger::init();
    pmb_loop();
}
