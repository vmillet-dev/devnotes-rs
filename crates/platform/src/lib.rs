//! What DevNotes asks of the operating system, behind safe APIs (D7): where its files are, where
//! its window was, whether another launch already holds the profile, whether the system asks
//! for less motion, which file a dialog chose, the icon of the notification area, the keys taken
//! from the whole machine, the session's autostart, toasts, links and updates. Neither egui nor
//! the core is named here.

pub mod autostart;
pub mod clipboard;
pub mod desktop;
pub mod dialog;
pub mod instance;
pub mod links;
pub mod motion;
pub mod notify;
pub mod paths;
pub mod shortcuts;
pub mod tray;
pub mod updater;
pub mod window;
