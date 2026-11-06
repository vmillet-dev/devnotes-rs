//! What leaves the HTTP area to be kept: an answer's body in a file, or a text as a note, with
//! where it went said on the status line.

use std::path::PathBuf;
use std::sync::Arc;

use engine::http::send::{Exchange, NoteText};
use engine::notes::kind::NoteKind;
use engine::notes::language::Language;
use engine::notes::model::{DisplayNote, NoteDraft, NoteLifecycle};
use engine::notes::priority::Priority;
use i18n::{Message, keys};
use platform::dialog::FileDialog;

use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;

/// What the dialog keeps as a note; where it goes is asked there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteAsked {
    pub title: String,
    pub language: Language,
    pub content: String,
    /// The line under the title, which says what is kept.
    pub what: i18n::Key,
    /// Proposed in the tags' field: generated code is filed under `http`.
    pub tags: String,
    pub source: String,
    /// Said in the dialog, whose button then reads « Save anyway »: a password.
    pub warning: Option<i18n::Key>,
}

/// An answer's text on its way to a note: Rust masks what the request sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseNote {
    pub send_id: Option<String>,
    pub exchange: Box<Exchange>,
    pub text: String,
    pub title: String,
    pub language: Language,
    pub source: String,
}

/// Where the dialog settled it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    pub title: String,
    pub space_id: String,
    pub folder_id: Option<String>,
    pub tags: Vec<String>,
    /// « Work › Scripts », for the line that says where it went.
    pub place: String,
}

struct Masking {
    asked: ResponseNote,
    pending: Pending<NoteText>,
}

enum SavingFile {
    Choosing(String, Pending<Option<PathBuf>>),
    Writing(String, Pending<bool>),
}

#[derive(Default)]
pub struct Kept {
    masking: Option<Masking>,
    asked: Option<NoteAsked>,
    creating: Option<(Pending<DisplayNote>, String)>,
    saving_file: Option<SavingFile>,
}

/// What a frame ended: a message for the status line, and whether a note was made.
#[derive(Default)]
pub struct KeptAnswer {
    pub said: Option<Message>,
    pub note_made: bool,
}

impl Kept {
    pub fn asked(&self) -> Option<&NoteAsked> {
        self.asked.as_ref()
    }

    pub fn ask_note(&mut self, asked: NoteAsked) {
        self.asked = Some(asked);
    }

    pub fn ask_response_note(&mut self, backend: &Backend, asked: ResponseNote) {
        let pending = api::http::response_note(
            backend,
            asked.send_id.clone(),
            (*asked.exchange).clone(),
            asked.text.clone(),
        );
        self.masking = Some(Masking { asked, pending });
    }

    pub fn cancel_note(&mut self) {
        self.asked = None;
    }

    pub fn save_note(&mut self, backend: &Backend, placement: Placement) {
        let Some(asked) = self.asked.take() else {
            return;
        };
        let draft = NoteDraft {
            space_id: placement.space_id,
            folder_id: placement.folder_id,
            title: placement.title,
            language: asked.language,
            content: asked.content,
            source: asked.source,
            tags: placement.tags,
            pinned: false,
            lifecycle: NoteLifecycle::Permanent,
            kind: NoteKind::default(),
            priority: Priority::default(),
            items: Vec::new(),
        };
        self.creating = Some((api::notes::create(backend, draft), placement.place));
    }

    pub fn save_file(
        &mut self,
        backend: &Backend,
        dialog: &Arc<dyn FileDialog + Send + Sync>,
        send_id: String,
        name: String,
    ) {
        let dialog = Arc::clone(dialog);
        let pending = backend.compute(move |_| Ok(dialog.save(&name)));
        self.saving_file = Some(SavingFile::Choosing(send_id, pending));
    }

    pub fn update(&mut self, backend: &Backend, errors: &mut ErrorNotifier) -> KeptAnswer {
        let mut answer = KeptAnswer::default();
        self.adopt_masking(errors);
        if let Some((pending, place)) = &mut self.creating
            && let Some(created) = pending.poll()
        {
            let place = place.clone();
            self.creating = None;
            match created {
                Ok(note) => {
                    answer.note_made = true;
                    answer.said = Some(
                        Message::new(keys::tools::SAVED)
                            .with("title", note.title.clone())
                            .with("place", place),
                    );
                }
                Err(error) => errors.report(keys::errors::NOTE_CREATE_FAILED, &error),
            }
        }
        if let Some(said) = self.adopt_file(backend, errors) {
            answer.said = Some(said);
        }
        answer
    }

    fn adopt_masking(&mut self, errors: &mut ErrorNotifier) {
        let Some(masking) = &mut self.masking else {
            return;
        };
        let Some(answer) = masking.pending.poll() else {
            return;
        };
        let Some(Masking { asked, .. }) = self.masking.take() else {
            return;
        };
        match answer {
            Ok(note) => {
                self.asked = Some(NoteAsked {
                    title: asked.title,
                    language: asked.language,
                    content: note.text,
                    what: if note.masked {
                        keys::http::response::NOTE_WHAT_MASKED
                    } else {
                        keys::http::response::NOTE_WHAT
                    },
                    tags: String::new(),
                    source: asked.source,
                    warning: None,
                });
            }
            Err(error) => errors.report(keys::http::FAILED, &error),
        }
    }

    fn adopt_file(&mut self, backend: &Backend, errors: &mut ErrorNotifier) -> Option<Message> {
        match self.saving_file.as_mut()? {
            SavingFile::Choosing(id, pending) => {
                let answer = pending.poll()?;
                let id = id.clone();
                self.saving_file = None;
                match answer {
                    Ok(Some(path)) => {
                        let shown = path.display().to_string();
                        let pending = api::http::save_response(backend, id, shown.clone());
                        self.saving_file = Some(SavingFile::Writing(shown, pending));
                    }
                    Ok(None) => {}
                    Err(error) => errors.report(keys::http::response::SAVE_FAILED, &error),
                }
                None
            }
            SavingFile::Writing(path, pending) => {
                let answer = pending.poll()?;
                let path = path.clone();
                self.saving_file = None;
                match answer {
                    Ok(true) => Some(Message::new(keys::http::response::SAVED).with("path", path)),
                    Ok(false) => Some(Message::new(keys::http::response::SAVE_FAILED)),
                    Err(error) => {
                        errors.report(keys::http::response::SAVE_FAILED, &error);
                        None
                    }
                }
            }
        }
    }
}
