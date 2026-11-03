//! The egui interface of DevNotes: the application frame, its state and its screens. It
//! calls the core directly, in the same process (plan § 3.3).

mod app;
pub mod inputs;
mod theme;

pub use app::{DevNotesApp, Startup};
