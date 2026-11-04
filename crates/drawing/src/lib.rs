//! A Dessin (plan § 3.9.2): its body is the scene Fabric stored, `{ version, scene }`, a stored
//! and exported format that does not change. This crate reads and writes the part of it the
//! editor and the pictures use, and keeps every other field as it was read. No egui, no engine.

pub mod geometry;
pub mod scene;

pub use geometry::{Box, Ends, Point};
pub use scene::{Drawing, Hue, Kind, Object, PathCommand};
