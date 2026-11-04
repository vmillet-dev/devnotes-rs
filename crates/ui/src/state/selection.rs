//! What the canvas holds apart from the notes: the card a first `Delete` armed. The focus itself
//! is egui's, on the card's button.

#[derive(Default)]
pub struct SelectionState {
    armed: Option<String>,
}

impl SelectionState {
    pub fn armed(&self) -> Option<&str> {
        self.armed.as_deref()
    }

    /// `true` when `id` was armed already: the second press, which deletes.
    pub fn arm(&mut self, id: &str) -> bool {
        if self.armed.as_deref() == Some(id) {
            self.armed = None;
            return true;
        }
        self.armed = Some(id.to_owned());
        false
    }

    /// `true` when something was armed: Escape's first rung.
    pub fn disarm(&mut self) -> bool {
        self.armed.take().is_some()
    }
}
