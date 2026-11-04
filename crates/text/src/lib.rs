//! The editing core under every editor that is not a one-line field (plan § 3.8): a rope with a
//! selection, undo grouped by typing, edits by grapheme, word and line, and search. It knows no
//! rule of the notes (D10): what to decorate, which unit indents a language, which token
//! comments a line are given by the caller. No egui here: the view is `ui`'s `EditorView`.
//!
//! Positions are `char` offsets into the rope, as `ropey` counts them; the caret moves by
//! grapheme, so it never stops inside « é » written as `e` and a combining accent.

mod buffer;
mod diff;
pub mod format;
pub mod highlight;
mod history;
mod indent;
mod search;
mod shield;
mod words;

pub use ropey::Rope;

pub use buffer::{Buffer, Direction, Edited, Selection, Unit};
pub use diff::changed_lines;
pub use indent::{TextEdit, indent, outdent, rewrite};
pub use search::{COUNT_CAP, Count, Query, find};
pub use shield::{Shielded, shield_fields, unshield_fields};
pub use words::count_words;
