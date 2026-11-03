//! Notes: the view the canvas draws.

use engine::notes;
use engine::notes::view::{NotesQuery, NotesView};

use crate::backend::{Backend, Pending};

/// Reads the index, not the database: still the db queue, which owns both (D11).
pub fn query(backend: &Backend, query: NotesQuery) -> Pending<NotesView> {
    backend.db(move |engine| notes::query_notes(engine, query))
}
