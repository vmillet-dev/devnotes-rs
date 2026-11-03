//! The egui interface of DevNotes: the application frame, its state and its screens. It
//! calls the core directly, in the same process (plan § 3.3).

pub mod api;
mod app;
pub mod backend;
pub mod inputs;
#[cfg(test)]
mod testing;
pub mod theme;

pub use app::{DevNotesApp, Startup};
