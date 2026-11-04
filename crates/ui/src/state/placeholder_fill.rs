//! A note with `{{fields}}` copied: the form asks for them first. What a field is, and what an
//! empty one is worth, is `notes::placeholder`'s; here are fill, copy, keep. A global variable
//! only proposes a value: it reaches the form as the default and is never stored.

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

    pub fn update(&mut self, revision: &mut Revision, errors: &mut ErrorNotifier) -> Filled {
        let mut filled = Filled::default();
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
