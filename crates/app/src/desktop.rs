//! What the binary starts on the desktop before the window: only a desktop answers for it, the
//! OS checklist's (plan § 8.5).

use platform::tray::{NoTray, Picture, Tray};
use ui::services::preferences::Preferences;
use ui::services::settings::Settings;

/// Here, on the thread the window's loop will run on: on Windows that loop serves the icon.
pub(crate) fn start_tray(preferences: &Preferences, inbox: ui::inputs::Inbox) -> Box<dyn Tray> {
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
