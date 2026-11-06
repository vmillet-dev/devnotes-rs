//! The icon of the notification area and its menu. Its labels arrive translated: the language is
//! the interface's. A desktop without a tray answers `exists() == false`, and the window then
//! closes as usual: hidden, nothing could call it back.

mod system;

pub use system::system;

/// What the icon and its menu ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    Reveal,
    NewNote,
    Capture,
    Palette,
    Quit,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Labels {
    /// The application's name, shown over the icon.
    pub tooltip: String,
    pub open: String,
    pub new_note: String,
    pub capture: String,
    pub palette: String,
    pub quit: String,
}

/// Straight RGBA, as the window's icon is decoded.
pub struct Picture {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub trait Tray {
    fn exists(&self) -> bool;
    fn set_labels(&self, labels: &Labels);
    /// At the exit: an icon left behind stays in the area until the mouse passes over it.
    fn remove(&self);
}

/// A desktop without a tray, and the tests that have no use for one.
pub struct NoTray;

impl Tray for NoTray {
    fn exists(&self) -> bool {
        false
    }

    fn set_labels(&self, _labels: &Labels) {}

    fn remove(&self) {}
}

const OPEN: &str = "open";
const NEW_NOTE: &str = "new-note";
const CAPTURE: &str = "capture";
const PALETTE: &str = "palette";
const QUIT: &str = "quit";

fn asked_by(item: &str) -> Option<Asked> {
    match item {
        OPEN => Some(Asked::Reveal),
        NEW_NOTE => Some(Asked::NewNote),
        CAPTURE => Some(Asked::Capture),
        PALETTE => Some(Asked::Palette),
        QUIT => Some(Asked::Quit),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_entry_of_the_menu_asks_its_own_thing() {
        assert_eq!(asked_by(OPEN), Some(Asked::Reveal));
        assert_eq!(asked_by(NEW_NOTE), Some(Asked::NewNote));
        assert_eq!(asked_by(CAPTURE), Some(Asked::Capture));
        assert_eq!(asked_by(PALETTE), Some(Asked::Palette));
        assert_eq!(asked_by(QUIT), Some(Asked::Quit));
        assert_eq!(asked_by("separator"), None);
    }

    #[test]
    fn without_a_tray_nothing_can_call_the_window_back() {
        assert!(!NoTray.exists());
    }
}
