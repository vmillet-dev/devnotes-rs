//! The JSON visualiser's model: a text in, what to draw out. It takes a text and not a note,
//! so an HTTP response is explored the same way. YAML, TOML and XML are read into the same
//! model, each through a reader of its own (`read`).

pub(crate) mod layout;
pub mod model;
pub mod parse;
mod position;
pub mod read;
mod toml;
pub(crate) mod xml;
pub(crate) mod yaml;
