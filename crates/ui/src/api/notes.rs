//! Notes: the view the canvas draws.

use engine::notes::model::{DisplayNote, NoteDraft, NotePatch, TagUsage};
use engine::notes::priority::Priority;
use engine::notes::trash::TrashedNote;
use engine::notes::view::{NotesQuery, NotesView};
use engine::{notes, spaces};

use super::undo::Reversible;
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

pub fn pin(backend: &Backend, ids: Vec<String>, pinned: bool) -> Pending<Reversible> {
    backend.db(move |engine| {
        notes::set_pinned(engine, ids, pinned).map(|ids| Reversible::Pin { ids, pinned })
    })
}

/// Triaging is not editing: `updated_at` stays where it was.
pub fn prioritise(backend: &Backend, ids: Vec<String>, priority: Priority) -> Pending<Reversible> {
    backend.db(move |engine| notes::set_priority(engine, ids, priority).map(Reversible::Priority))
}

/// Out of their folders, into another space.
pub fn move_to(backend: &Backend, ids: Vec<String>, space_id: String) -> Pending<Reversible> {
    backend.db(move |engine| notes::move_notes(engine, ids, space_id).map(Reversible::Move))
}

/// Into the trash, which keeps them thirty days.
pub fn delete(backend: &Backend, ids: Vec<String>) -> Pending<Reversible> {
    backend.db(move |engine| {
        let count = notes::delete_notes(engine, ids.clone())?;
        Ok(Reversible::Deletion { ids, count })
    })
}

/// What the search matches, as `view::fold` reads it; the expired purged first.
pub fn list_trash(backend: &Backend, search: String) -> Pending<Vec<TrashedNote>> {
    backend.db(move |engine| notes::list_trash(engine, search))
}

/// `updated_at` stays where it was: putting back is not editing.
pub fn restore(backend: &Backend, ids: Vec<String>) -> Pending<u32> {
    backend.db(move |engine| notes::restore_notes(engine, ids))
}

/// What the same search finds, or the whole trash.
pub fn restore_trash(backend: &Backend, search: String) -> Pending<u32> {
    backend.db(move |engine| notes::restore_trash(engine, search))
}

/// Only notes already in the trash: nothing shortens the retention.
pub fn purge(backend: &Backend, ids: Vec<String>) -> Pending<u32> {
    backend.db(move |engine| notes::purge_notes(engine, ids))
}

pub fn empty_trash(backend: &Backend) -> Pending<u32> {
    backend.db(notes::empty_trash)
}

/// Every tag of the library, with the notes carrying it; read from the index.
pub fn list_tags(backend: &Backend) -> Pending<Vec<TagUsage>> {
    backend.db(notes::list_tags)
}

/// The notes carrying any of `tags`, each once: what a change would touch.
pub fn count_tagged(backend: &Backend, tags: Vec<String>) -> Pending<u32> {
    backend.db(move |engine| notes::count_notes_tagged(engine, tags))
}

/// Onto an existing tag, a merge. `updated_at` stays: a retag is aimed at no note.
pub fn rename_tags(backend: &Backend, tags: Vec<String>, into: String) -> Pending<u32> {
    backend.db(move |engine| notes::rename_tags(engine, tags, into))
}

pub fn delete_tags(backend: &Backend, tags: Vec<String>) -> Pending<u32> {
    backend.db(move |engine| notes::delete_tags(engine, tags))
}

/// The copy, under the title the interface words.
pub fn duplicate(backend: &Backend, id: String, title: String) -> Pending<DisplayNote> {
    backend.db(move |engine| notes::duplicate_note(engine, id, title))
}

/// Through `normalize_tags`, the one keeper of what a tag is.
pub fn tag(backend: &Backend, ids: Vec<String>, tag: String) -> Pending<Reversible> {
    backend.db(move |engine| notes::tag_notes(engine, ids, vec![tag]).map(Reversible::Tag))
}

pub fn untag(backend: &Backend, ids: Vec<String>, tag: String) -> Pending<Reversible> {
    backend.db(move |engine| notes::remove_tag(engine, ids, tag).map(Reversible::Untag))
}
