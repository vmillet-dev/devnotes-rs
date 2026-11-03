//! Notes: the view the canvas draws.

use engine::notes;
use engine::notes::model::{DisplayNote, NotePatch};
use engine::notes::view::{NotesQuery, NotesView};

use crate::backend::{Backend, Pending};

/// Reads the index, not the database: still the db queue, which owns both (D11).
pub fn query(backend: &Backend, query: NotesQuery) -> Pending<NotesView> {
    backend.db(move |engine| notes::query_notes(engine, query))
}

/// A Dessin's small picture as a PNG data URI; `None` when it has none.
pub fn drawing_thumbnail(backend: &Backend, id: String) -> Pending<Option<String>> {
    backend.db(move |engine| notes::drawing_thumbnail(engine, id))
}

/// A field set to `None` is left as it is.
pub fn update(backend: &Backend, id: String, patch: NotePatch) -> Pending<DisplayNote> {
    backend.db(move |engine| notes::update_note(engine, id, patch))
}
