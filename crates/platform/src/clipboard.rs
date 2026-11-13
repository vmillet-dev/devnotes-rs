//! What the clipboard holds besides the text egui hands a field: a page's HTML, read when a Note
//! is pasted into, and the text as it is. A clipboard that cannot be opened, or holds neither,
//! answers `None`.

use std::sync::Arc;

pub trait Clipboard {
    fn html(&self) -> Option<String>;

    /// As it is held: what the window hands a field has lost its CRLFs.
    fn text(&self) -> Option<String>;
}

/// The system's, through `arboard`.
pub struct System;

impl Clipboard for System {
    fn html(&self) -> Option<String> {
        arboard::Clipboard::new().ok()?.get().html().ok()
    }

    fn text(&self) -> Option<String> {
        arboard::Clipboard::new().ok()?.get_text().ok()
    }
}

pub fn system() -> Arc<dyn Clipboard + Send + Sync> {
    Arc::new(System)
}

/// An empty clipboard, until the shell hands the system's: a test never reads the machine's.
pub struct NoClipboard;

impl Clipboard for NoClipboard {
    fn html(&self) -> Option<String> {
        None
    }

    fn text(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_empty_clipboard_holds_nothing() {
        assert_eq!(NoClipboard.html(), None);
        assert_eq!(NoClipboard.text(), None);
    }
}
