//! A Dessin (plan § 3.9.2): its body is the scene Fabric stored, `{ version, scene }`, a stored
//! and exported format that does not change. This crate reads and writes the part of it the
//! editor and the pictures use, and keeps every other field as it was read. No egui, no engine.

pub mod edit;
pub mod geometry;
pub mod marks;
pub mod picture;
pub mod scene;
pub mod text;

pub use edit::{Editor, Frame, Grip, Tool};
pub use geometry::{Box, Ends, Point};
pub use marks::{Fill, Mark, marks};
pub use picture::{Format, Palette, Pictures, export, pictures, svg};
pub use scene::{Drawing, Hue, Kind, Object, PathCommand};
pub use text::{Fonts, wrap};
