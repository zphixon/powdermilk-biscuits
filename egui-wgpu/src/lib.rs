use egui::{Context, Event, RawInput};
use egui_winit::winit::{
    dpi::PhysicalPosition,
    event::{ElementState, ModifiersState, WindowEvent},
};

mod w2e {
    use egui::{Modifiers, PointerButton, Pos2, Vec2};
    use egui_winit::winit::{
        dpi::PhysicalPosition,
        event::{ModifiersState, MouseButton, MouseScrollDelta},
    };

    pub trait PpExt {
        fn to_pos2(self) -> Pos2;
        fn to_vec2(self) -> Vec2;
    }

    impl PpExt for PhysicalPosition<f64> {
        fn to_pos2(self) -> Pos2 {
            Pos2 {
                x: self.x as f32,
                y: self.y as f32,
            }
        }

        fn to_vec2(self) -> Vec2 {
            Vec2 {
                x: self.x as f32,
                y: self.y as f32,
            }
        }
    }

    pub fn mouse_button(button: MouseButton) -> PointerButton {
        match button {
            MouseButton::Left => PointerButton::Primary,
            MouseButton::Middle => PointerButton::Middle,
            MouseButton::Right => PointerButton::Secondary,
            MouseButton::Other(n) if n == 1 => PointerButton::Extra1,
            MouseButton::Other(n) if n == 2 => PointerButton::Extra2,
            _ => PointerButton::Primary, // bruh
        }
    }

    pub fn modifiers(modifiers: ModifiersState) -> Modifiers {
        Modifiers {
            alt: modifiers.alt(),
            ctrl: modifiers.ctrl(),
            shift: modifiers.shift(),
            mac_cmd: false, // bruh
            command: false,
        }
    }

    pub fn scroll(delta: MouseScrollDelta) -> Vec2 {
        match delta {
            MouseScrollDelta::LineDelta(x, y) => Vec2 { x, y },
            MouseScrollDelta::PixelDelta(pos) => pos.to_vec2(),
        }
    }
}

#[derive(Default)]
pub struct EguiWgpu {
    cursor_pos: PhysicalPosition<f64>,
    modifiers: ModifiersState,
    raw_input: Option<RawInput>,
    egui_ctx: Context,
}

impl EguiWgpu {
    // returns should redraw??
    pub fn on_event(&mut self, event: &WindowEvent) {
        use w2e::PpExt;

        let mut has_focus = true;
        let egui_event = match event {
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = *modifiers;
                None
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = *position;
                Some(Event::PointerMoved(position.to_pos2()))
            }

            WindowEvent::MouseInput { state, button, .. } => Some(Event::PointerButton {
                pos: self.cursor_pos.to_pos2(),
                button: w2e::mouse_button(*button),
                pressed: state == &ElementState::Pressed,
                modifiers: w2e::modifiers(self.modifiers),
            }),

            WindowEvent::CursorLeft { .. } => {
                has_focus = false;
                Some(Event::PointerGone)
            }

            WindowEvent::CursorEntered { .. } => {
                has_focus = true;
                None
            }

            WindowEvent::MouseWheel { delta, .. } => Some(Event::Scroll(w2e::scroll(*delta))),

            // TODO
            WindowEvent::Ime(_) => None,

            // TODO
            WindowEvent::Touch(_) => None,

            _ => None,
        };

        self.raw_input = Some(RawInput {
            events: egui_event.map(|event| vec![event]).unwrap_or_default(),
            has_focus,
            ..Default::default()
        });
    }

    pub fn run(&mut self, ui: impl FnOnce(&Context) -> ()) {
        let output = self.egui_ctx.run(
            self.raw_input
                .take()
                .expect("must call on_event before run"),
            ui,
        );

        // TODO platform_output
    }
}
