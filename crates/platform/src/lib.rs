//! What DevNotes asks of the operating system, behind safe APIs (D7): where its files are, where
//! its window was, whether another launch already holds the profile, whether the system asks
//! for less motion, which file a dialog chose, the icon of the notification area and the keys
//! taken from the whole machine. Neither egui nor the core is named here.

pub mod clipboard;
pub mod desktop;
pub mod dialog;
pub mod instance;
pub mod motion;
pub mod paths;
pub mod shortcuts;
pub mod tray;
pub mod window;
