//! The icon of the notification area and its menu. Its labels arrive translated: the language is
//! the interface's. A desktop without a tray answers `exists() == false`, and the window then
//! closes as usual: hidden, nothing could call it back.

use std::sync::Arc;

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

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

type Listener = Arc<dyn Fn(Asked) + Send + Sync>;

/// ⚠️ Called on the window's thread: on Windows the icon lives on the thread whose message loop
/// serves it, the one winit runs.
pub fn system(
    picture: Picture,
    labels: &Labels,
    on_asked: impl Fn(Asked) + Send + Sync + 'static,
) -> Box<dyn Tray> {
    listen(Arc::new(on_asked));
    native::start(picture, labels)
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

fn listen(on_asked: Listener) {
    let on_menu = Arc::clone(&on_asked);
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if let Some(asked) = asked_by(event.id().as_ref()) {
            on_menu(asked);
        }
    }));
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            on_asked(Asked::Reveal);
        }
    }));
}

fn menu(labels: &Labels) -> Result<Menu, tray_icon::menu::Error> {
    let item = |id: &str, label: &str| MenuItem::with_id(id, label, true, None);
    Menu::with_items(&[
        &item(OPEN, &labels.open),
        &item(NEW_NOTE, &labels.new_note),
        &item(CAPTURE, &labels.capture),
        &item(PALETTE, &labels.palette),
        &PredefinedMenuItem::separator(),
        &item(QUIT, &labels.quit),
    ])
}

fn build(picture: Picture, labels: &Labels) -> Result<TrayIcon, String> {
    let icon = Icon::from_rgba(picture.rgba, picture.width, picture.height)
        .map_err(|error| error.to_string())?;
    let menu = menu(labels).map_err(|error| error.to_string())?;
    TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip(&labels.tooltip)
        .with_menu(Box::new(menu))
        // The menu stays on the right click, where Windows expects it.
        .with_menu_on_left_click(false)
        .build()
        .map_err(|error| error.to_string())
}

fn replace_menu(tray: &TrayIcon, labels: &Labels) {
    match menu(labels) {
        Ok(menu) => tray.set_menu(Some(Box::new(menu))),
        Err(error) => log::warn!("The tray menu was not updated: {error}"),
    }
}

#[cfg(not(target_os = "linux"))]
mod native {
    use std::cell::RefCell;

    use tray_icon::TrayIcon;

    use super::{Labels, NoTray, Picture, Tray, build, replace_menu};

    struct System(RefCell<Option<TrayIcon>>);

    impl Tray for System {
        fn exists(&self) -> bool {
            self.0.borrow().is_some()
        }

        fn set_labels(&self, labels: &Labels) {
            if let Some(tray) = self.0.borrow().as_ref() {
                replace_menu(tray, labels);
            }
        }

        fn remove(&self) {
            self.0.borrow_mut().take();
        }
    }

    pub(super) fn start(picture: Picture, labels: &Labels) -> Box<dyn Tray> {
        match build(picture, labels) {
            Ok(tray) => Box::new(System(RefCell::new(Some(tray)))),
            Err(error) => {
                log::warn!("No system tray: {error}");
                Box::new(NoTray)
            }
        }
    }
}

/// GTK's thread, the only one of the process (D29): the icon, its menu and their loop live on
/// it, and the rest of the application reaches them by message.
#[cfg(target_os = "linux")]
mod native {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::time::Duration;

    use gtk::glib::{self, ControlFlow};

    use super::{Labels, Picture, Tray, build, replace_menu};

    /// How often the loop reads its messages. A change of language, or the exit: never in a hurry.
    const READ_EVERY: Duration = Duration::from_millis(200);

    enum Message {
        Labels(Labels),
        Remove,
    }

    struct System {
        /// Stays false when the thread dies: the indicator library is opened by a panic-prone
        /// load, and a desktop without it has no tray.
        exists: Arc<AtomicBool>,
        messages: Sender<Message>,
    }

    impl Tray for System {
        fn exists(&self) -> bool {
            self.exists.load(Ordering::Relaxed)
        }

        fn set_labels(&self, labels: &Labels) {
            let _ = self.messages.send(Message::Labels(labels.clone()));
        }

        fn remove(&self) {
            self.exists.store(false, Ordering::Relaxed);
            let _ = self.messages.send(Message::Remove);
        }
    }

    pub(super) fn start(picture: Picture, labels: &Labels) -> Box<dyn Tray> {
        let labels = labels.clone();
        let exists = Arc::new(AtomicBool::new(false));
        let (messages, inbox) = mpsc::channel();
        let shown = Arc::clone(&exists);
        let started = std::thread::Builder::new()
            .name("tray".to_owned())
            .spawn(move || serve(picture, &labels, inbox, &shown));
        if let Err(error) = started {
            log::warn!("No system tray: {error}");
        }
        Box::new(System { exists, messages })
    }

    fn serve(picture: Picture, labels: &Labels, inbox: Receiver<Message>, exists: &AtomicBool) {
        if let Err(error) = gtk::init() {
            log::warn!("No system tray: {error}");
            return;
        }
        let tray = match build(picture, labels) {
            Ok(tray) => tray,
            Err(error) => {
                log::warn!("No system tray: {error}");
                return;
            }
        };
        exists.store(true, Ordering::Relaxed);
        glib::timeout_add_local(READ_EVERY, move || {
            for message in inbox.try_iter() {
                match message {
                    Message::Labels(labels) => replace_menu(&tray, &labels),
                    Message::Remove => {
                        gtk::main_quit();
                        return ControlFlow::Break;
                    }
                }
            }
            ControlFlow::Continue
        });
        gtk::main();
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
