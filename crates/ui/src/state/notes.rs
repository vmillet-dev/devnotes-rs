//! The notes' one writer. Writes are not optimistic: the core persists, its answer is adopted,
//! and the revision is bumped so that every view that reads notes asks again.

use engine::notes::checklist::ChecklistItem;
use engine::notes::model::{DisplayNote, NotePatch};
use engine::notes::priority::Priority;
use i18n::{Key, keys};

use super::Revision;
use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;

#[derive(Default)]
pub struct NotesState {
    writes: Vec<(Key, Pending<()>)>,
    saves: Vec<Pending<DisplayNote>>,
    /// Opened in the editor once made, ready to be renamed.
    duplicates: Vec<Pending<DisplayNote>>,
    /// Read whole, then copied: a card carries only its first lines.
    copies: Vec<Pending<DisplayNote>>,
}

/// What a frame's answers ask of the rest of the window.
#[derive(Default)]
pub struct Answered {
    /// A copy made: the editor opens on it.
    pub open: Option<String>,
    /// A note read for the clipboard.
    pub copied: Vec<DisplayNote>,
}

impl NotesState {
    /// A todo list's items, the whole list: a position is all the identity an item has.
    pub fn set_items(&mut self, backend: &Backend, id: String, items: Vec<ChecklistItem>) {
        let patch = NotePatch {
            items: Some(items),
            ..NotePatch::default()
        };
        self.saves.push(api::notes::update(backend, id, patch));
    }

    pub fn pin(&mut self, backend: &Backend, id: String, pinned: bool) {
        let pending = api::notes::pin(backend, vec![id], pinned);
        self.writes.push((keys::errors::NOTE_SAVE_FAILED, pending));
    }

    pub fn prioritise(&mut self, backend: &Backend, id: String, priority: Priority) {
        let pending = api::notes::prioritise(backend, vec![id], priority);
        self.writes.push((keys::errors::NOTE_SAVE_FAILED, pending));
    }

    pub fn file(&mut self, backend: &Backend, id: String, folder: Option<String>) {
        let pending = api::folders::file(backend, vec![id], folder);
        self.writes.push((keys::errors::NOTE_MOVE_FAILED, pending));
    }

    pub fn move_to(&mut self, backend: &Backend, id: String, space: String) {
        let pending = api::notes::move_to(backend, vec![id], space);
        self.writes.push((keys::errors::NOTE_MOVE_FAILED, pending));
    }

    pub fn delete(&mut self, backend: &Backend, id: String) {
        let pending = api::notes::delete(backend, vec![id]);
        self.writes
            .push((keys::errors::NOTE_DELETE_FAILED, pending));
    }

    pub fn duplicate(&mut self, backend: &Backend, id: String, title: String) {
        self.duplicates
            .push(api::notes::duplicate(backend, id, title));
    }

    pub fn copy(&mut self, backend: &Backend, id: String) {
        self.copies.push(api::notes::get(backend, id));
    }

    /// The answers that arrived: each one written moves the revision.
    pub fn update(&mut self, revision: &mut Revision, errors: &mut ErrorNotifier) -> Answered {
        let mut answered = Answered::default();
        self.writes
            .retain_mut(|(failed, pending)| match pending.poll() {
                None => true,
                Some(Ok(())) => {
                    revision.bump();
                    false
                }
                Some(Err(error)) => {
                    errors.report(*failed, &error);
                    false
                }
            });
        let saved = |failed: Key,
                     revision: &mut Revision,
                     errors: &mut ErrorNotifier,
                     pending: &mut Pending<DisplayNote>| {
            let answer = pending.poll()?;
            match answer {
                Ok(note) => {
                    revision.bump();
                    Some(Some(note))
                }
                Err(error) => {
                    errors.report(failed, &error);
                    Some(None)
                }
            }
        };
        self.saves.retain_mut(|pending| {
            saved(keys::errors::NOTE_SAVE_FAILED, revision, errors, pending).is_none()
        });
        self.duplicates.retain_mut(|pending| {
            match saved(
                keys::errors::NOTE_DUPLICATE_FAILED,
                revision,
                errors,
                pending,
            ) {
                None => true,
                Some(made) => {
                    answered.open = made.map(|note| note.id.clone()).or(answered.open.take());
                    false
                }
            }
        });
        self.copies.retain_mut(|pending| match pending.poll() {
            None => true,
            Some(Ok(note)) => {
                answered.copied.push(note);
                false
            }
            Some(Err(error)) => {
                errors.report(keys::errors::NOTE_READ_FAILED, &error);
                false
            }
        });
        answered
    }
}
