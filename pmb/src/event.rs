use crate::graphics::PixelPos;

#[derive(Clone, Copy)]
pub struct PenInfo {
    pub barrel: bool,
    pub inverted: bool,
    pub eraser: bool,
}

#[derive(PartialEq)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

pub struct Touch {
    pub force: Option<f64>,
    pub phase: TouchPhase,
    pub location: PixelPos,
    pub pen_info: Option<PenInfo>,
}
