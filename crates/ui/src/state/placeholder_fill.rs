//! A note with `{{fields}}` copied: the form asks for them first. What a field is, and what an
//! empty one is worth, is `notes::placeholder`'s; here are fill, copy, keep. A global variable
//! only proposes a value: it reaches the form as the default and is never stored.

use std::collections::BTreeMap;

use engine::notes::model::DisplayNote;
use i18n::{Message, keys};

use super::Revision;
use crate::api;
use crate::backend::{Backend, Pending};
use crate::notes::Fill;
use crate::services::errors::ErrorNotifier;

#[derive(Default)]
pub struct FillState {
    /// Read whole: a card carries only its first lines.
    target: Option<DisplayNote>,
    filling: Vec<Pending<String>>,
    keeping: Vec<Pending<DisplayNote>>,
    /// The editor's body filled, for its opening.
    preview: Option<(u64, String)>,
    previewing: Option<(u64, Pending<String>)>,
}

/// What the frame's answers ask of the window.
#[derive(Default)]
pub struct Filled {
    /// Text for the clipboard, and what to say about it.
    pub copied: Vec<(String, Message)>,
}

impl FillState {
    pub fn target(&self) -> Option<&DisplayNote> {
        self.target.as_ref()
    }

    pub fn is_open(&self) -> bool {
        self.target.is_some()
    }

    pub fn open(&mut self, note: DisplayNote) {
        self.target = Some(note);
    }

    /// `Some` for « copy as is »: the text, tokens and all, which needs no round trip.
    pub fn apply(&mut self, asked: Fill, backend: &Backend) -> Option<(String, Message)> {
        let note = self.target.take()?;
        match asked {
            Fill::Cancel => None,
            Fill::Raw => Some((note.content.clone(), copied_note(&note))),
            Fill::Submit(values) => {
                self.filling.push(api::notes::fill(
                    backend,
                    note.content.clone(),
                    values.clone(),
                ));
                self.keeping.push(api::notes::set_field_values(
                    backend,
                    note.id.clone(),
                    values,
                ));
                None
            }
        }
    }

    /// The editor's panel: what it holds is written, its filled text copied or shown.
    pub fn keep(&mut self, backend: &Backend, id: String, values: BTreeMap<String, String>) {
        self.keeping
            .push(api::notes::set_field_values(backend, id, values));
    }

    pub fn copy(&mut self, backend: &Backend, content: String, values: BTreeMap<String, String>) {
        self.filling
            .push(api::notes::fill(backend, content, values));
    }

    /// A newer value takes the place of the one on its way.
    pub fn ask_preview(
        &mut self,
        backend: &Backend,
        session: u64,
        content: String,
        values: BTreeMap<String, String>,
    ) {
        self.previewing = Some((session, api::notes::fill(backend, content, values)));
    }

    /// The body filled, once computed for this opening.
    pub fn preview(&self, session: u64) -> Option<&str> {
        let (of, text) = self.preview.as_ref()?;
        (*of == session).then_some(text.as_str())
    }

    pub fn update(&mut self, revision: &mut Revision, errors: &mut ErrorNotifier) -> Filled {
        let mut filled = Filled::default();
        if let Some((session, pending)) = &mut self.previewing
            && let Some(answer) = pending.poll()
        {
            match answer {
                Ok(text) => self.preview = Some((*session, text)),
                Err(error) => errors.report(keys::errors::NOTE_READ_FAILED, &error),
            }
            self.previewing = None;
        }
        self.filling.retain_mut(|pending| match pending.poll() {
            None => true,
            Some(Ok(text)) => {
                filled
                    .copied
                    .push((text, Message::new(keys::placeholders::COPIED_FILLED)));
                false
            }
            Some(Err(error)) => {
                errors.report(keys::errors::NOTE_READ_FAILED, &error);
                false
            }
        });
        // Filling a field is not editing the note: `updated_at` stays, the revision moves.
        self.keeping.retain_mut(|pending| match pending.poll() {
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
        filled
    }
}

/// « “title” copied », or the untitled version.
pub fn copied_note(note: &DisplayNote) -> Message {
    if note.title.is_empty() {
        Message::new(keys::notes::COPIED_UNTITLED)
    } else {
        Message::new(keys::notes::COPIED_NOTE).with("title", note.title.as_str())
    }
}
