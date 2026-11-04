//! What DevNotes asks of the operating system, behind safe APIs (D7): where its files are, where
//! its window was, whether another launch already holds the profile, and whether the system
//! asks for less motion, and which file a dialog chose. Neither egui nor the core is named here.

pub mod desktop;
pub mod dialog;
pub mod instance;
pub mod motion;
pub mod paths;
pub mod window;
