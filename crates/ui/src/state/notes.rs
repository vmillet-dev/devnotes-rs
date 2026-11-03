//! The notes' one writer. Writes are not optimistic: the core persists, its answer is adopted,
//! and the revision is bumped so that every view that reads notes asks again.

use engine::notes::checklist::ChecklistItem;
use engine::notes::model::{DisplayNote, NotePatch};
use i18n::keys;

use super::Revision;
use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;

#[derive(Default)]
pub struct NotesState {
    writes: Vec<Pending<DisplayNote>>,
}

impl NotesState {
    /// A todo list's items, the whole list: a position is all the identity an item has.
    pub fn set_items(&mut self, backend: &Backend, id: String, items: Vec<ChecklistItem>) {
        let patch = NotePatch {
            items: Some(items),
            ..NotePatch::default()
        };
        self.writes.push(api::notes::update(backend, id, patch));
    }

    /// The answers that arrived: each one written moves the revision.
    pub fn update(&mut self, revision: &mut Revision, errors: &mut ErrorNotifier) {
        self.writes.retain_mut(|pending| match pending.poll() {
            None => true,
            Some(Ok(_)) => {
                revision.bump();
                false
            }
            Some(Err(error)) => {
                errors.report(keys::errors::NOTE_SAVE_FAILED, &error);
                false
            }
        });
    }
}
