//! What the clipboard holds besides the text egui hands a field: a page's HTML, read when a Note
//! is pasted into. A clipboard that cannot be opened, or holds no HTML, answers `None`.

use std::sync::Arc;

pub trait Clipboard {
    fn html(&self) -> Option<String>;
}

/// The system's, through `arboard`.
pub struct System;

impl Clipboard for System {
    fn html(&self) -> Option<String> {
        arboard::Clipboard::new().ok()?.get().html().ok()
    }
}

pub fn system() -> Arc<dyn Clipboard + Send + Sync> {
    Arc::new(System)
}
