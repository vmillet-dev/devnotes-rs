//! A Note's document (plan § 3.9.1, points 1–2): blocks and marked text, read from Markdown by
//! `pulldown-cmark` with the core's options and written back byte for byte as the reference's
//! editor wrote it (`golden/markdown`), its typed text escaped only where it would read as
//! syntax (`golden/markdown-escape`). No egui here: the editor and the reader are `ui`'s.
//!
//! Offsets are `char`s, as the text editor counts them.

pub mod edit;
mod escape;
mod history;
mod html;
pub mod inlines;
mod model;
mod read;
pub mod rules;
pub mod table;
pub mod tree;
mod write;

pub use escape::escape;
pub use history::History;
pub use html::{formatted, from_html};
pub use model::{Block, Cell, Document, Inline, Item, ListKind, Marks, Position, Selection};
pub use read::{OPTIONS, read};
pub use write::write;
