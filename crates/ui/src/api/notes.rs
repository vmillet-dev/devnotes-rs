//! Notes: the view the canvas draws.

use engine::notes::model::{DisplayNote, NoteDraft, NotePatch};
use engine::notes::view::{NotesQuery, NotesView};
use engine::{notes, spaces};

use crate::backend::{Backend, Pending};
use crate::services::samples::Samples;

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

/// The first launch's samples, unless the library already has a space (a library emptied of
/// its notes is not a new one). Answers whether it seeded.
pub fn seed_samples(backend: &Backend, samples: Samples) -> Pending<bool> {
    backend.db(move |engine| {
        if !spaces::list_spaces(engine)?.is_empty() {
            return Ok(false);
        }
        notes::seed_samples(engine, samples.space, samples.folders, samples.notes)?;
        Ok(true)
    })
}

/// The whole note, body included: opening one never starts from a preview.
pub fn get(backend: &Backend, id: String) -> Pending<DisplayNote> {
    backend.db(move |engine| notes::get_note(engine, id))
}

/// A new note, once it is worth keeping.
pub fn create(backend: &Backend, draft: NoteDraft) -> Pending<DisplayNote> {
    backend.db(move |engine| notes::create_note(engine, draft))
}
