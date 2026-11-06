//! What the binary starts on the desktop before the window: only a desktop answers for it, the
//! OS checklist's (plan § 8.5).

use std::sync::Arc;

use engine::app_info::METADATA;
use platform::autostart::Autostart;
use platform::dialog::FileDialog;
use platform::shortcuts::Shortcuts;
use platform::tray::{NoTray, Picture, Tray};
use ui::inputs::{Inbox, Input};
use ui::services::preferences::Preferences;
use ui::services::settings::Settings;

/// Who sends a toast on Windows.
const IDENTIFIER: &str = "com.devnotes.app";

/// ⚠️ Here, on the thread the window's loop will run on: on Windows that loop serves the icon
/// and the keys.
pub(crate) fn start(preferences: &Preferences, inbox: &Inbox) -> ui::Desktop {
    ui::Desktop {
        tray: start_tray(preferences, inbox.clone()),
        shortcuts: take_shortcuts(inbox.clone()),
        autostart: autostart(),
        notifier: platform::notify::system(METADATA.name, IDENTIFIER),
        dialog: dialog(inbox.clone()),
    }
}

fn dialog(inbox: Inbox) -> Arc<dyn FileDialog + Send + Sync> {
    platform::dialog::system(move || inbox.post(Input::NoFileDialog))
}

fn start_tray(preferences: &Preferences, inbox: Inbox) -> Box<dyn Tray> {
    let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../icons/32x32.png")) else {
        return Box::new(NoTray);
    };
    let picture = Picture {
        rgba: icon.rgba,
        width: icon.width,
        height: icon.height,
    };
    let language = Settings::read(preferences).locale.language();
    platform::tray::system(
        picture,
        &ui::services::tray::labels(language),
        move |asked| {
            inbox.post(ui::inputs::Input::Tray(asked));
        },
    )
}

#[cfg(not(feature = "e2e"))]
fn take_shortcuts(inbox: Inbox) -> Box<dyn Shortcuts> {
    platform::shortcuts::system(move |key| inbox.post(ui::inputs::Input::Shortcut(key)))
}

/// A scenario presses no key of the machine (`--action` stands for it), and one held by a
/// person's own copy would open every run on a banner.
#[cfg(feature = "e2e")]
fn take_shortcuts(_inbox: Inbox) -> Box<dyn Shortcuts> {
    Box::new(platform::shortcuts::NoShortcuts)
}

#[cfg(not(feature = "e2e"))]
fn autostart() -> Arc<dyn Autostart + Send + Sync> {
    platform::autostart::system()
}

/// A scenario flips the switch: it must not leave an entry in the session of whoever runs it.
#[cfg(feature = "e2e")]
fn autostart() -> Arc<dyn Autostart + Send + Sync> {
    Arc::new(platform::autostart::NoAutostart)
}
