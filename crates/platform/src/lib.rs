//! What DevNotes asks of the operating system, behind safe APIs (D7): where its files are, where
//! its window was, and whether another launch already holds the profile. Neither egui nor the
//! core is named here.

pub mod instance;
pub mod paths;
pub mod window;
