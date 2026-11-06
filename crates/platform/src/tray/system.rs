//! The icon itself, which only a desktop can answer for: the OS checklist's (plan § 8.5).

use std::sync::Arc;

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use super::{Asked, CAPTURE, Labels, NEW_NOTE, OPEN, PALETTE, Picture, QUIT, Tray, asked_by};

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

    use super::super::{Labels, NoTray, Picture, Tray};
    use super::{build, replace_menu};

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

    use super::super::{Labels, Picture, Tray};
    use super::{build, replace_menu};

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
