use crate::graphics::PixelPos;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Touch(Touch),
    TouchMove(Touch),
    Release(Touch),

    PenDown(Touch),
    PenMove(Touch),
    PenUp(Touch),

    MouseDown(MouseButton),
    MouseMove(PixelPos),
    MouseUp(MouseButton),

    StartPan,
    EndPan,
    StartZoom,
    EndZoom,

    IncreaseBrush(usize),
    DecreaseBrush(usize),

    ScrollZoom(f32),
}

#[derive(Clone, Copy, Debug)]
pub struct PenInfo {
    pub barrel: bool,
    pub inverted: bool,
    pub eraser: bool,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

#[derive(Debug, Clone, Copy)]
pub struct Touch {
    pub force: Option<f64>,
    pub phase: TouchPhase,
    pub location: PixelPos,
    pub pen_info: Option<PenInfo>,
}

pub struct Combination {
    keys: Vec<Keycode>,
    repeatable: bool,
}

impl Combination {
    // Vec::with_capacity is not const yet :(
    pub const INACTIVE: Combination = Combination {
        keys: Vec::new(),
        repeatable: false,
    };

    pub fn repeatable(self) -> Combination {
        Combination {
            repeatable: true,
            ..self
        }
    }
}

impl std::fmt::Debug for Combination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.keys)
    }
}

impl From<Keycode> for Combination {
    fn from(key: Keycode) -> Self {
        Combination {
            keys: vec![key.normalize_mirrored()],
            repeatable: false,
        }
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
#[rustfmt::skip]
pub enum Keycode {
    Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, Key0, A, B, C, D, E, F, G, H, I, J, K, L,
    M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, Escape, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11,
    F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24, Snapshot, Scroll, Pause,
    Insert, Home, Delete, End, PageDown, PageUp, Left, Up, Right, Down, Back, Return, Space,
    Compose, Caret, Numlock, Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6,
    Numpad7, Numpad8, Numpad9, NumpadAdd, NumpadDivide, NumpadDecimal, NumpadComma, NumpadEnter,
    NumpadEquals, NumpadMultiply, NumpadSubtract, AbntC1, AbntC2, Apostrophe, Apps, Asterisk, At,
    Ax, Backslash, Calculator, Capital, Colon, Comma, Convert, Equals, Grave, Kana, Kanji, LAlt,
    LBracket, LControl, LShift, LWin, Mail, MediaSelect, MediaStop, Minus, Mute, MyComputer,
    NavigateForward, NavigateBackward, NextTrack, NoConvert, OEM102, Period, PlayPause, Plus,
    Power, PrevTrack, RAlt, RBracket, RControl, RShift, RWin, Semicolon, Slash, Sleep, Stop, Sysrq,
    Tab, Underline, Unlabeled, VolumeDown, VolumeUp, Wake, WebBack, WebFavorites, WebForward,
    WebHome, WebRefresh, WebSearch, WebStop, Yen, Copy, Paste, Cut
}

impl Keycode {
    pub fn normalize_mirrored(self) -> Keycode {
        use Keycode::*;

        macro_rules! normalize {
            ($($variant:ident => $normal:ident),* $(,)?) => {
                match self {
                    $($variant => $normal,)*
                    _ => self,
                }
            };
        }

        normalize!(
            RControl => LControl,
            RShift => LShift,
            RAlt => LAlt,
            RWin => LWin,
        )
    }

    pub fn modifier(&self) -> bool {
        use Keycode::*;
        matches!(self.normalize_mirrored(), LControl | LShift | LAlt | LWin)
    }
}

impl std::ops::BitOr for Keycode {
    type Output = Combination;

    fn bitor(self, rhs: Self) -> Self::Output {
        Combination {
            keys: vec![self.normalize_mirrored(), rhs.normalize_mirrored()],
            repeatable: false,
        }
    }
}

impl std::ops::BitOr<Keycode> for Combination {
    type Output = Combination;
    fn bitor(mut self, rhs: Keycode) -> Self::Output {
        self.keys.push(rhs.normalize_mirrored());
        self
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Other(usize),
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ElementState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy)]
pub enum KeyState {
    Downstroke,
    Held,
    Upstroke,
    Released,
}

impl KeyState {
    pub fn is_down(&self) -> bool {
        use KeyState::*;
        matches!(self, Downstroke | Held)
    }

    pub fn just_pressed(&self) -> bool {
        use KeyState::*;
        matches!(self, Downstroke)
    }

    pub fn just_released(&self) -> bool {
        use KeyState::*;
        matches!(self, Upstroke)
    }

    pub fn edge(&self) -> bool {
        use KeyState::*;
        matches!(self, Upstroke | Downstroke)
    }

    pub fn next(&self) -> KeyState {
        use KeyState::*;
        match self {
            Upstroke => Released,
            Released => Released,
            Downstroke => Held,
            Held => Held,
        }
    }
}

#[derive(Default, Debug)]
pub struct InputHandler {
    pub(super) keys: HashMap<Keycode, KeyState>,
    buttons: HashMap<MouseButton, KeyState>,
    cursor_pos: PixelPos,
}

fn cycle_state(key_state: KeyState, element_state: ElementState) -> KeyState {
    match (key_state, element_state) {
        (KeyState::Released, ElementState::Pressed) => KeyState::Downstroke,
        (KeyState::Released, ElementState::Released) => KeyState::Released,
        (KeyState::Downstroke, ElementState::Pressed) => KeyState::Held,
        (KeyState::Downstroke, ElementState::Released) => KeyState::Upstroke,
        (KeyState::Held, ElementState::Pressed) => KeyState::Held,
        (KeyState::Held, ElementState::Released) => KeyState::Upstroke,
        (KeyState::Upstroke, ElementState::Pressed) => KeyState::Downstroke,

        // this state edge doesn't make any sense but it's happened when I double-tap the trackpad
        // on my laptop
        (KeyState::Upstroke, ElementState::Released) => KeyState::Released,
    }
}

impl InputHandler {
    pub(super) fn handle_mouse_move(&mut self, cursor_pos: PixelPos) {
        self.cursor_pos = cursor_pos;
    }

    pub(super) fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        let button_state = self.buttons.entry(button).or_insert(KeyState::Released);
        let next_state = cycle_state(*button_state, state);
        *button_state = next_state;
    }

    pub fn cursor_pos(&self) -> PixelPos {
        self.cursor_pos
    }

    pub fn button_down(&mut self, button: MouseButton) -> bool {
        self.buttons.contains_key(&button) && self.buttons[&button].is_down()
    }

    pub fn button_just_pressed(&mut self, button: MouseButton) -> bool {
        self.buttons.contains_key(&button) && self.buttons[&button].just_pressed()
    }

    pub fn button_just_released(&mut self, button: MouseButton) -> bool {
        self.buttons.contains_key(&button) && self.buttons[&button].just_released()
    }

    pub(super) fn handle_key(&mut self, key: Keycode, state: ElementState) {
        let key = key.normalize_mirrored();
        let key_state = self.keys.entry(key).or_insert(KeyState::Released);
        let next_state = cycle_state(*key_state, state);
        *key_state = next_state;
    }

    pub fn is_down(&self, key: Keycode) -> bool {
        let key = key.normalize_mirrored();
        self.keys.contains_key(&key) && self.keys[&key].is_down()
    }

    pub fn just_pressed(&self, key: Keycode) -> bool {
        let key = key.normalize_mirrored();
        self.keys.contains_key(&key) && self.keys[&key].just_pressed()
    }

    pub fn just_released(&self, key: Keycode) -> bool {
        let key = key.normalize_mirrored();
        self.keys.contains_key(&key) && self.keys[&key].just_released()
    }

    pub fn shift(&self) -> bool {
        use Keycode::{LShift, RShift};
        self.is_down(LShift) || self.is_down(RShift)
    }

    pub fn control(&self) -> bool {
        use Keycode::{LControl, RControl};
        self.is_down(LControl) || self.is_down(RControl)
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.buttons.clear();
    }

    pub fn combo_just_pressed(&self, combo: &Combination) -> bool {
        (combo
            .keys
            .iter()
            .filter(|key| !key.modifier())
            .any(|key| self.just_pressed(*key))
            || combo.repeatable)
            && combo.keys.iter().all(|key| self.is_down(*key))
    }

    pub(super) fn pump_key_state(&mut self) {
        self.keys
            .values_mut()
            .chain(self.buttons.values_mut())
            .filter(|state| state.edge())
            .for_each(|value| *value = value.next());
    }
}
