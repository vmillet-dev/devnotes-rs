//! The note open in the stand-in editor — plain text, a development tool removed with phase 4's
//! editors (D4). An existing note is read whole by id, never from a preview; a new one is a
//! draft that writes nothing until it is worth keeping, then is created once. Every field is
//! written when it is left and when the editor closes; answers are adopted, never assumed.

use engine::notes::checklist::ChecklistItem;
use engine::notes::kind::NoteKind;
use engine::notes::language::Language;
use engine::notes::model::{DisplayNote, NoteDraft, NoteLifecycle, NotePatch};
use engine::notes::priority::Priority;
use i18n::keys;

use super::Revision;
use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;

/// What precedes a done task in the plain text.
pub const DONE: &str = "[x] ";

/// What a field holds once it is left: the body of a note, or a list's items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Content(String),
    Items(Vec<ChecklistItem>),
}

/// A list's tasks as plain text: one a line, `[x] ` before a done one.
pub fn text_of(kind: NoteKind, content: &str, items: &[ChecklistItem]) -> String {
    if kind != NoteKind::Checklist {
        return content.to_owned();
    }
    items
        .iter()
        .map(|item| {
            if item.done {
                format!("{DONE}{}", item.text)
            } else {
                item.text.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The text read back: a list's lines are its tasks, the empty ones dropped.
pub fn body_of(kind: NoteKind, text: &str) -> Body {
    if kind != NoteKind::Checklist {
        return Body::Content(text.to_owned());
    }
    let items = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| match line.strip_prefix(DONE) {
            Some(text) => ChecklistItem {
                text: text.to_owned(),
                done: true,
            },
            None => ChecklistItem {
                text: line.to_owned(),
                done: false,
            },
        })
        .collect();
    Body::Items(items)
}

/// What the editor shows, as last saved: what its fields are compared with.
pub struct Opened {
    /// Bumped by each opening: a field left in another opening is never written here.
    pub session: u64,
    /// `None` while a new note is a draft.
    id: Option<String>,
    pub kind: NoteKind,
    pub title: String,
    pub text: String,
    /// Where a draft is created.
    place: (String, Option<String>),
    reading: Option<Pending<DisplayNote>>,
}

impl Opened {
    /// Read, or a draft: its fields can be drawn.
    pub fn is_ready(&self) -> bool {
        self.reading.is_none()
    }

    fn adopt(&mut self, note: &DisplayNote) {
        self.id = Some(note.id.clone());
        self.kind = note.kind;
        self.title.clone_from(&note.title);
        self.text = text_of(note.kind, &note.content, &note.items);
    }
}

/// A draft being created, and what was left while it was: sent once it has an id.
struct Creating {
    session: u64,
    pending: Pending<DisplayNote>,
    waiting: Changes,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Changes {
    title: Option<String>,
    body: Option<Body>,
}

impl Changes {
    fn merge(&mut self, later: Changes) {
        if later.title.is_some() {
            self.title = later.title;
        }
        if later.body.is_some() {
            self.body = later.body;
        }
    }

    fn is_empty(&self) -> bool {
        self.title.is_none() && self.body.is_none()
    }
}

#[derive(Default)]
pub struct EditorState {
    open: Option<Opened>,
    sessions: u64,
    creating: Vec<Creating>,
    writes: Vec<Pending<DisplayNote>>,
}

impl EditorState {
    pub fn open(&mut self, backend: &Backend, id: String) {
        let reading = api::notes::get(backend, id.clone());
        self.sessions += 1;
        self.open = Some(Opened {
            session: self.sessions,
            id: Some(id),
            kind: NoteKind::Snippet,
            title: String::new(),
            text: String::new(),
            place: (String::new(), None),
            reading: Some(reading),
        });
    }

    /// A new note of `kind` in `space`, filed in `folder`: nothing is written yet.
    pub fn draft(&mut self, kind: NoteKind, space: String, folder: Option<String>) {
        self.sessions += 1;
        self.open = Some(Opened {
            session: self.sessions,
            id: None,
            kind,
            title: String::new(),
            text: String::new(),
            place: (space, folder),
            reading: None,
        });
    }

    pub fn close(&mut self) {
        self.open = None;
    }

    pub fn opened(&self) -> Option<&Opened> {
        self.open.as_ref()
    }

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// The fields left in `session`; `None` leaves a field as it is.
    pub fn save(
        &mut self,
        backend: &Backend,
        session: u64,
        title: Option<String>,
        body: Option<Body>,
    ) {
        let changes = Changes { title, body };
        if changes.is_empty() {
            return;
        }
        if let Some(creating) = self
            .creating
            .iter_mut()
            .find(|creating| creating.session == session)
        {
            creating.waiting.merge(changes);
            return;
        }
        let Some(open) = self.open.as_ref().filter(|open| open.session == session) else {
            return;
        };
        match &open.id {
            Some(id) => self.writes.push(update(backend, id.clone(), changes)),
            None if worth_keeping(&changes) => {
                let pending = api::notes::create(backend, new_note(open, changes));
                self.creating.push(Creating {
                    session,
                    pending,
                    waiting: Changes::default(),
                });
            }
            None => {}
        }
    }

    pub fn update(
        &mut self,
        backend: &Backend,
        revision: &mut Revision,
        errors: &mut ErrorNotifier,
    ) {
        if let Some(open) = &mut self.open
            && let Some(reading) = &mut open.reading
            && let Some(answer) = reading.poll()
        {
            open.reading = None;
            match answer {
                Ok(note) => open.adopt(&note),
                Err(error) => {
                    errors.report(keys::errors::NOTE_READ_FAILED, &error);
                    self.open = None;
                }
            }
        }
        let mut created = Vec::new();
        self.creating
            .retain_mut(|creating| match creating.pending.poll() {
                None => true,
                Some(answer) => {
                    created.push((
                        creating.session,
                        std::mem::take(&mut creating.waiting),
                        answer,
                    ));
                    false
                }
            });
        for (session, waiting, answer) in created {
            match answer {
                Ok(note) => {
                    revision.bump();
                    if !waiting.is_empty() {
                        self.writes.push(update(backend, note.id.clone(), waiting));
                    }
                    if let Some(open) = self.open.as_mut().filter(|open| open.session == session) {
                        open.adopt(&note);
                    }
                }
                Err(error) => errors.report(keys::errors::NOTE_CREATE_FAILED, &error),
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
                && open.id.as_deref() == Some(note.id.as_str())
            {
                open.adopt(&note);
            }
        }
    }
}

/// A title or a body: an untouched draft closed is a note nobody asked for.
fn worth_keeping(changes: &Changes) -> bool {
    let titled = changes
        .title
        .as_ref()
        .is_some_and(|title| !title.trim().is_empty());
    let written = match &changes.body {
        Some(Body::Content(content)) => !content.trim().is_empty(),
        Some(Body::Items(items)) => !items.is_empty(),
        None => false,
    };
    titled || written
}

fn update(backend: &Backend, id: String, changes: Changes) -> Pending<DisplayNote> {
    let (content, items) = match changes.body {
        Some(Body::Content(content)) => (Some(content), None),
        Some(Body::Items(items)) => (None, Some(items)),
        None => (None, None),
    };
    let patch = NotePatch {
        title: changes.title,
        content,
        items,
        ..NotePatch::default()
    };
    api::notes::update(backend, id, patch)
}

fn new_note(open: &Opened, changes: Changes) -> NoteDraft {
    let (content, items) = match changes.body {
        Some(Body::Content(content)) => (content, Vec::new()),
        Some(Body::Items(items)) => (String::new(), items),
        None => (String::new(), Vec::new()),
    };
    NoteDraft {
        space_id: open.place.0.clone(),
        folder_id: open.place.1.clone(),
        title: changes.title.unwrap_or_default(),
        language: Language::Txt,
        content,
        source: String::new(),
        tags: Vec::new(),
        pinned: false,
        lifecycle: NoteLifecycle::Permanent,
        kind: open.kind,
        priority: Priority::None,
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_is_one_task_a_line_a_done_one_marked() {
        let Body::Items(items) = body_of(NoteKind::Checklist, "milk\n[x] bread\n\n  \neggs") else {
            panic!("a list reads as items");
        };

        let read: Vec<_> = items
            .iter()
            .map(|item| (item.text.as_str(), item.done))
            .collect();
        assert_eq!(read, [("milk", false), ("bread", true), ("eggs", false)]);
        assert_eq!(
            text_of(NoteKind::Checklist, "", &items),
            "milk\n[x] bread\neggs"
        );
    }

    #[test]
    fn a_note_reads_as_its_text() {
        assert_eq!(
            body_of(NoteKind::Note, "# Release\n"),
            Body::Content("# Release\n".into())
        );
    }

    #[test]
    fn an_untouched_draft_is_not_worth_keeping() {
        assert!(!worth_keeping(&Changes {
            title: Some("  ".into()),
            body: Some(Body::Content("\n".into())),
        }));
        assert!(worth_keeping(&Changes {
            title: None,
            body: Some(Body::Items(vec![ChecklistItem {
                text: "milk".into(),
                done: false,
            }])),
        }));
    }
}
