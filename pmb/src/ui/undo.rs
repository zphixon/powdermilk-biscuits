use slotmap::DefaultKey;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    DrawStroke(DefaultKey),
    EraseStroke(DefaultKey),
}

#[derive(Debug)]
pub struct UndoStack {
    buffer: Vec<Action>,
    cursor: usize,
    saved: usize,
}

impl UndoStack {
    pub fn new() -> Self {
        UndoStack {
            buffer: Vec::new(),
            cursor: 0,
            saved: 0,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }

    pub fn at_saved_state(&self) -> bool {
        self.cursor == self.saved
    }

    pub fn set_saved_state(&mut self) {
        self.saved = self.cursor;
    }

    #[must_use]
    pub fn last(&self) -> Option<Action> {
        if self.cursor == 0 {
            return None;
        }

        self.buffer.get(self.cursor - 1).copied()
    }

    pub fn push(&mut self, action: Action) {
        if self.cursor == self.buffer.len() {
            tracing::debug!("append");
            // [a, b, c, d]
            //           ^ c=4
            // [a, b, c, d, e]
            //              ^ c=5
            self.buffer.push(action);
        } else if 1 <= self.cursor && self.cursor < self.buffer.len() {
            tracing::debug!("split off");
            // [a, b, c, d]
            //        ^ c=3
            // [a, b, e]
            //        ^ c=3
            let _old = self.buffer.split_off(self.cursor);
            self.buffer.push(action);
        } else {
            tracing::debug!("replace");
            // [a, b, c, d]
            //  ^ c=1
            // [e]
            //  ^ c=1
            self.buffer = vec![action];
        }

        self.cursor = self.buffer.len();
    }

    #[must_use]
    pub fn undo(&mut self) -> Option<Action> {
        let last = self.last();
        if self.cursor > 0 {
            self.cursor -= 1;
        }
        last
    }

    #[must_use]
    pub fn redo(&mut self) -> Option<Action> {
        if self.cursor < self.buffer.len() {
            self.cursor += 1;
            self.last()
        } else {
            None
        }
    }
}

#[test]
fn undo_stack() {
    let mut sm = slotmap::SlotMap::new();
    let mut stack = UndoStack::new();

    let a1 = sm.insert(());
    stack.push(Action::DrawStroke(a1));
    assert_eq!(stack.last(), Some(Action::DrawStroke(a1)));

    let a2 = sm.insert(());
    stack.push(Action::DrawStroke(a2));
    assert_eq!(stack.last(), Some(Action::DrawStroke(a2)));

    let to_be_undone = stack.undo();
    assert_eq!(to_be_undone, Some(Action::DrawStroke(a2)));
    assert_eq!(stack.last(), Some(Action::DrawStroke(a1)));

    let to_be_undone = stack.undo();
    assert_eq!(to_be_undone, Some(Action::DrawStroke(a1)));
    assert_eq!(stack.last(), None);

    let to_be_redone = stack.redo();
    assert_eq!(to_be_redone, Some(Action::DrawStroke(a1)));
    assert_eq!(stack.last(), Some(Action::DrawStroke(a1)));

    let to_be_undone = stack.undo();
    assert_eq!(to_be_undone, Some(Action::DrawStroke(a1)));
    assert_eq!(stack.last(), None);

    let to_be_redone = stack.redo();
    assert_eq!(to_be_redone, Some(Action::DrawStroke(a1)));
    assert_eq!(stack.last(), Some(Action::DrawStroke(a1)));

    let to_be_redone = stack.redo();
    assert_eq!(to_be_redone, Some(Action::DrawStroke(a2)));
    assert_eq!(stack.last(), Some(Action::DrawStroke(a2)));

    let _undone = stack.undo();
    let _undone = stack.undo();
    let _undone = stack.undo();
    let _undone = stack.undo();
    let a3 = sm.insert(());
    stack.push(Action::DrawStroke(a3));
    assert_eq!(stack.last(), Some(Action::DrawStroke(a3)));
}
