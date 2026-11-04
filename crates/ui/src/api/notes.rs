//! Notes: the view the canvas draws.

use engine::notes::model::{DisplayNote, NoteDraft, NotePatch};
use engine::notes::priority::Priority;
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

pub fn pin(backend: &Backend, ids: Vec<String>, pinned: bool) -> Pending<()> {
    backend.db(move |engine| notes::set_pinned(engine, ids, pinned).map(drop))
}

/// Triaging is not editing: `updated_at` stays where it was.
pub fn prioritise(backend: &Backend, ids: Vec<String>, priority: Priority) -> Pending<()> {
    backend.db(move |engine| notes::set_priority(engine, ids, priority).map(drop))
}

/// Out of their folders, into another space.
pub fn move_to(backend: &Backend, ids: Vec<String>, space_id: String) -> Pending<()> {
    backend.db(move |engine| notes::move_notes(engine, ids, space_id).map(drop))
}

/// Into the trash, which keeps them thirty days.
pub fn delete(backend: &Backend, ids: Vec<String>) -> Pending<()> {
    backend.db(move |engine| notes::delete_notes(engine, ids).map(drop))
}

/// The copy, under the title the interface words.
pub fn duplicate(backend: &Backend, id: String, title: String) -> Pending<DisplayNote> {
    backend.db(move |engine| notes::duplicate_note(engine, id, title))
}

/// Through `normalize_tags`, the one keeper of what a tag is.
pub fn tag(backend: &Backend, ids: Vec<String>, tag: String) -> Pending<()> {
    backend.db(move |engine| notes::tag_notes(engine, ids, vec![tag]).map(drop))
}

pub fn untag(backend: &Backend, ids: Vec<String>, tag: String) -> Pending<()> {
    backend.db(move |engine| notes::remove_tag(engine, ids, tag).map(drop))
}
