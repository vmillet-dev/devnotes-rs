//! The bodies kept beside the note open (the reference's `NoteRevisionsStore`). Going back to one
//! is irreversible — the current text and every newer version go — so a row opens a preview of
//! what would change, and only the preview restores.

use engine::notes::model::DisplayNote;
use engine::notes::revision::{DiffLine, Revision as Kept};
use i18n::keys;

use super::{QueryCell, Revision};
use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;

/// A kept body opened for a look, against the text going back to it would replace.
#[derive(Debug, Clone)]
pub struct Preview {
    pub kept: Kept,
    /// How many kept bodies are newer: they go, with the current text.
    pub newer: usize,
    pub lines: Vec<DiffLine>,
}

#[derive(Default)]
pub struct RevisionsState {
    /// The note whose bodies are listed, read again at each revision: a save keeps one more.
    list: QueryCell<(String, Revision), Vec<Kept>>,
    note: Option<String>,
    open: bool,
    preview: Option<Preview>,
    comparing: Option<(String, usize, Pending<Vec<DiffLine>>)>,
    restoring: Option<Pending<DisplayNote>>,
}

impl RevisionsState {
    /// The note the editor shows; `None` for a draft or a todo list, which keep no body.
    pub fn follow(&mut self, note: Option<&str>) {
        if self.note.as_deref() != note {
            *self = Self {
                note: note.map(str::to_owned),
                ..Self::default()
            };
        }
    }

    pub fn kept(&self) -> &[Kept] {
        self.list.value().map_or(&[], Vec::as_slice)
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn preview(&self) -> Option<&Preview> {
        self.preview.as_ref()
    }

    pub fn is_restoring(&self) -> bool {
        self.restoring.is_some()
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        self.preview = None;
    }

    pub fn close_preview(&mut self) {
        self.preview = None;
    }

    /// The kept body `id` compared with the current text.
    pub fn look(&mut self, backend: &Backend, id: &str) {
        let (Some(note), Some(newer)) = (
            self.note.clone(),
            self.kept().iter().position(|kept| kept.id == id),
        ) else {
            return;
        };
        let pending = api::notes::revision_diff(backend, note, id.to_owned());
        self.comparing = Some((id.to_owned(), newer, pending));
    }

    /// Back to the body being previewed.
    pub fn restore(&mut self, backend: &Backend) {
        if let (Some(note), Some(preview)) = (&self.note, &self.preview)
            && self.restoring.is_none()
        {
            let id = preview.kept.id.clone();
            self.restoring = Some(api::notes::restore_revision(backend, note.clone(), id));
        }
    }

    /// The answers that arrived; the note restored, when one was.
    pub fn update(
        &mut self,
        backend: &Backend,
        revision: &mut Revision,
        errors: &mut ErrorNotifier,
    ) -> Option<String> {
        if let Some(note) = &self.note {
            self.list.want((note.clone(), *revision));
            let answered = self
                .list
                .update(|(id, _)| api::notes::list_revisions(backend, id.clone()));
            if answered && let Some(error) = self.list.error() {
                errors.report(keys::errors::REVISIONS_LIST_FAILED, error);
            }
        }
        let compared = self
            .comparing
            .as_mut()
            .and_then(|(_, _, pending)| pending.poll());
        if let Some(answer) = compared
            && let Some((id, newer, _)) = self.comparing.take()
        {
            match answer {
                Ok(lines) => {
                    let kept = self.kept().iter().find(|kept| kept.id == id).cloned();
                    self.preview = kept.map(|kept| Preview { kept, newer, lines });
                }
                Err(error) => errors.report(keys::errors::REVISION_DIFF_FAILED, &error),
            }
        }
        let answer = self.restoring.as_mut()?.poll()?;
        self.restoring = None;
        match answer {
            Ok(note) => {
                self.preview = None;
                revision.bump();
                Some(note.id.clone())
            }
            Err(error) => {
                errors.report(keys::errors::REVISION_RESTORE_FAILED, &error);
                None
            }
        }
    }
}
