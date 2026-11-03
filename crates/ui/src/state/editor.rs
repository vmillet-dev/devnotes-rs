//! The note open in the stand-in editor — plain text, a development tool removed with phase 4's
//! editors (D4). It is read whole by id, never from a preview, and written when a field is left
//! and when it closes. Writes are not optimistic: the answer is adopted, the revision bumped.

use engine::notes::checklist::ChecklistItem;
use engine::notes::model::{DisplayNote, NotePatch};
use i18n::keys;

use super::Revision;
use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;

/// What a field holds once it is left: the body of a note, or a list's items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Content(String),
    Items(Vec<ChecklistItem>),
}

#[derive(Default)]
pub struct EditorState {
    open: Option<Opened>,
    writes: Vec<Pending<DisplayNote>>,
}

struct Opened {
    id: String,
    reading: Option<Pending<DisplayNote>>,
    note: Option<DisplayNote>,
}

impl EditorState {
    pub fn open(&mut self, backend: &Backend, id: String) {
        let reading = api::notes::get(backend, id.clone());
        self.open = Some(Opened {
            id,
            reading: Some(reading),
            note: None,
        });
    }

    pub fn close(&mut self) {
        self.open = None;
    }

    /// The note open, once read.
    pub fn note(&self) -> Option<&DisplayNote> {
        self.open.as_ref().and_then(|open| open.note.as_ref())
    }

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// `None` leaves a field as it is.
    pub fn save(
        &mut self,
        backend: &Backend,
        id: String,
        title: Option<String>,
        body: Option<Body>,
    ) {
        if title.is_none() && body.is_none() {
            return;
        }
        let (content, items) = match body {
            Some(Body::Content(content)) => (Some(content), None),
            Some(Body::Items(items)) => (None, Some(items)),
            None => (None, None),
        };
        let patch = NotePatch {
            title,
            content,
            items,
            ..NotePatch::default()
        };
        self.writes.push(api::notes::update(backend, id, patch));
    }

    pub fn update(&mut self, revision: &mut Revision, errors: &mut ErrorNotifier) {
        if let Some(open) = &mut self.open
            && let Some(reading) = &mut open.reading
            && let Some(answer) = reading.poll()
        {
            open.reading = None;
            match answer {
                Ok(note) => open.note = Some(note),
                Err(error) => {
                    errors.report(keys::errors::NOTE_READ_FAILED, &error);
                    self.open = None;
                }
            }
        }
        let mut saved = Vec::new();
        self.writes.retain_mut(|pending| match pending.poll() {
            None => true,
            Some(Ok(note)) => {
                saved.push(note);
                false
            }
            Some(Err(error)) => {
                errors.report(keys::errors::NOTE_SAVE_FAILED, &error);
                false
            }
        });
        for note in saved {
            revision.bump();
            if let Some(open) = &mut self.open
                && open.id == note.id
            {
                open.note = Some(note);
            }
        }
    }
}
