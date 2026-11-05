//! The egui interface of DevNotes: the application frame, its state and its screens. It
//! calls the core directly, in the same process (plan § 3.3).

pub mod action;
pub mod api;
mod app;
pub mod backend;
mod banners;
pub mod gate;
pub mod http;
pub mod inputs;
pub mod keys;
pub mod notes;
pub mod palette;
pub mod services;
pub mod state;
#[cfg(test)]
mod testing;
pub mod theme;
mod titlebar;
pub mod widgets;

pub use app::{DevNotesApp, Startup};
