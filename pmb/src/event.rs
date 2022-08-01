use crate::graphics::PixelPos;

#[derive(Clone, Copy, Debug)]
pub struct PenInfo {
    pub barrel: bool,
    pub inverted: bool,
    pub eraser: bool,
}

#[derive(PartialEq, Debug)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

#[derive(Debug)]
pub struct Touch {
    pub force: Option<f64>,
    pub phase: TouchPhase,
    pub location: PixelPos,
    pub pen_info: Option<PenInfo>,
}
