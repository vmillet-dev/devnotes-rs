//! The note open in the editor. An existing note is read whole by id, never from a preview, and
//! read again whenever the revision moves: a pin, a filing or an undo made elsewhere shows at
//! once. A new note is a draft that writes nothing until it is worth keeping, then is created
//! once. The editor's fields are drafts of their own, written when they are left and when the
//! editor closes; a field that did not move is not sent (the reference's `UNCHANGED`).

use std::sync::Arc;

use engine::notes::checklist::ChecklistItem;
use engine::notes::drawing::DrawingPictures;
use engine::notes::kind::NoteKind;
use engine::notes::language::Language;
use engine::notes::model::{DisplayNote, Note, NoteDraft, NoteLifecycle, NotePatch};
use engine::notes::priority::Priority;
use i18n::keys;

use super::{QueryCell, Revision};
use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;

/// What a body field holds once it is left: a note's text, or a list's items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Content(String),
    Items(Vec<ChecklistItem>),
}

/// The fields left in an opening; `None` leaves a field as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Edit {
    pub title: Option<String>,
    pub source: Option<String>,
    pub body: Option<Body>,
    pub tags: Option<Vec<String>>,
    pub lifecycle: Option<NoteLifecycle>,
    pub language: Option<Language>,
}

impl Edit {
    fn merge(&mut self, later: Edit) {
        let Edit {
            title,
            source,
            body,
            tags,
            lifecycle,
            language,
        } = later;
        self.title = title.or(self.title.take());
        self.source = source.or(self.source.take());
        self.body = body.or(self.body.take());
        self.tags = tags.or(self.tags.take());
        self.lifecycle = lifecycle.or(self.lifecycle.take());
        self.language = language.or(self.language.take());
    }

    fn is_empty(&self) -> bool {
        *self == Edit::default()
    }

    /// Only what differs from `note`: a field left as it was makes no round trip.
    fn changed_from(self, note: &Note) -> Edit {
        let body = self.body.filter(|body| match body {
            Body::Content(content) => *content != note.content,
            Body::Items(items) => *items != note.items,
        });
        Edit {
            title: self.title.filter(|title| *title != note.title),
            source: self.source.filter(|source| *source != note.source),
            body,
            tags: self.tags.filter(|tags| *tags != note.tags),
            lifecycle: self
                .lifecycle
                .filter(|lifecycle| *lifecycle != note.lifecycle),
            language: self.language.filter(|language| *language != note.language),
        }
    }

    fn into_patch(self) -> NotePatch {
        let (content, items) = match self.body {
            Some(Body::Content(content)) => (Some(content), None),
            Some(Body::Items(items)) => (None, Some(items)),
            None => (None, None),
        };
        NotePatch {
            title: self.title,
            source: self.source,
            content,
            items,
            tags: self.tags,
            lifecycle: self.lifecycle,
            language: self.language,
            ..NotePatch::default()
        }
    }

    /// Onto a draft, which has no row to write it to yet.
    fn apply_to(self, note: &mut Note) {
        if let Some(title) = self.title {
            note.title = title;
        }
        if let Some(source) = self.source {
            note.source = source;
        }
        match self.body {
            Some(Body::Content(content)) => note.content = content,
            Some(Body::Items(items)) => note.items = items,
            None => {}
        }
        if let Some(tags) = self.tags {
            note.tags = api::notes::normalize_tags(&tags);
        }
        if let Some(lifecycle) = self.lifecycle {
            note.lifecycle = lifecycle;
        }
        if let Some(language) = self.language {
            note.language = language;
        }
    }
}

/// What the editor shows: a draft held here until it is worth keeping, or a note read by id.
pub struct Opened {
    /// Bumped by each opening and by a conversion: the fields drawn take their drafts again.
    pub session: u64,
    /// While new, nothing of it is written; kept until the note it became is read.
    draft: Option<DisplayNote>,
    /// The note, read again at each revision; its id once the draft is written.
    read: QueryCell<(String, Revision), DisplayNote>,
    id: Option<String>,
    /// A body put back: the fields start again from the next read.
    reseed: bool,
}

impl Opened {
    /// What the fields compare with; `None` while it is read. A draft just written stays
    /// until its first read answers.
    pub fn note(&self) -> Option<&DisplayNote> {
        self.read.value().or(self.draft.as_ref())
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn is_draft(&self) -> bool {
        self.id.is_none()
    }
}

/// A drawing's pictures on their way: read to see whether it has any, then taken. They go to
/// the note once it has an id: a new drawing's first ones follow the write that makes it.
struct Picturing {
    session: u64,
    id: Option<String>,
    step: PictureStep,
}

enum PictureStep {
    Checking {
        has: Pending<Option<String>>,
        scene: (String, Arc<drawing::Palette>),
    },
    Taking(Pending<DrawingPictures>),
    Taken(DrawingPictures),
}

/// A draft being written, and what was left while it was: sent once it has an id.
struct Creating {
    session: u64,
    pending: Pending<DisplayNote>,
    waiting: Edit,
}

#[derive(Default)]
pub struct EditorState {
    open: Option<Opened>,
    sessions: u64,
    creating: Vec<Creating>,
    writes: Vec<Pending<DisplayNote>>,
    picturing: Vec<Picturing>,
    /// Pictures being written, by note.
    pictured: Vec<(String, Pending<DisplayNote>)>,
    /// The notes whose pictures were written since the last frame asked.
    redrawn: Vec<String>,
}

impl EditorState {
    pub fn open(&mut self, id: String) {
        self.sessions += 1;
        self.open = Some(Opened {
            session: self.sessions,
            draft: None,
            read: QueryCell::default(),
            id: Some(id),
            reseed: false,
        });
    }

    /// A new note of `kind` in `space`, filed in `folder`: nothing is written yet.
    pub fn draft(&mut self, kind: NoteKind, space: String, folder: Option<String>) {
        self.sessions += 1;
        let note = Note {
            id: String::new(),
            space_id: space,
            folder_id: folder,
            title: String::new(),
            language: Language::Txt,
            content: String::new(),
            source: String::new(),
            tags: Vec::new(),
            pinned: false,
            created_at: chrono::DateTime::UNIX_EPOCH,
            updated_at: chrono::DateTime::UNIX_EPOCH,
            lifecycle: NoteLifecycle::Permanent,
            kind,
            priority: Priority::None,
            items: Vec::new(),
            placeholder_values: std::collections::BTreeMap::new(),
        };
        self.open = Some(Opened {
            session: self.sessions,
            draft: Some(api::notes::decorate(note)),
            read: QueryCell::default(),
            id: None,
            reseed: false,
        });
    }

    /// Named already, so worth keeping: written at once, the editor open on it.
    pub fn draft_titled(
        &mut self,
        backend: &Backend,
        (kind, space, folder): (NoteKind, String, Option<String>),
        title: String,
    ) {
        self.draft(kind, space, folder);
        let edit = Edit {
            title: Some(title),
            ..Edit::default()
        };
        self.save(backend, self.sessions, edit);
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

    /// The note open, by id: `None` while it is a draft.
    pub fn open_id(&self) -> Option<&str> {
        self.open.as_ref().and_then(Opened::id)
    }

    /// The fields take their drafts again, from the note as it now is: a conversion rewrote it.
    pub fn reseed(&mut self, id: &str) {
        if let Some(open) = self.open.as_mut().filter(|open| open.id() == Some(id)) {
            self.sessions += 1;
            open.session = self.sessions;
        }
    }

    /// A body put back under the fields: they start again once the note is read again.
    pub fn reseed_when_read(&mut self, id: &str) {
        if let Some(open) = self.open.as_mut().filter(|open| open.id() == Some(id)) {
            open.reseed = true;
        }
    }

    /// The fields left in `session`. A draft takes them here, and is written once they make
    /// it worth keeping; a field that did not move is dropped.
    pub fn save(&mut self, backend: &Backend, session: u64, edit: Edit) {
        if edit.is_empty() {
            return;
        }
        if let Some(creating) = self
            .creating
            .iter_mut()
            .find(|creating| creating.session == session)
        {
            creating.waiting.merge(edit);
            return;
        }
        let Some(open) = self.open.as_mut().filter(|open| open.session == session) else {
            return;
        };
        if open.id.is_none()
            && let Some(draft) = &open.draft
        {
            let mut note = draft.note.clone();
            edit.apply_to(&mut note);
            self.keep_draft(backend, note);
            return;
        }
        let (Some(id), Some(note)) = (open.id.clone(), open.note()) else {
            return;
        };
        let edit = edit.changed_from(&note.note);
        if !edit.is_empty() {
            self.writes
                .push(api::notes::update(backend, id, edit.into_patch()));
        }
    }

    /// A body formatted on save: into the opening if it is still open, by id once it is closed.
    pub fn save_body(&mut self, backend: &Backend, session: u64, id: String, body: String) {
        let edit = Edit {
            body: Some(Body::Content(body)),
            ..Edit::default()
        };
        if self
            .open
            .as_ref()
            .is_some_and(|open| open.session == session)
        {
            self.save(backend, session, edit);
        } else {
            self.writes
                .push(api::notes::update(backend, id, edit.into_patch()));
        }
    }

    /// The pictures of the drawing left in `session`. `missing` takes them only if it has none.
    pub fn picture(
        &mut self,
        backend: &Backend,
        session: u64,
        (content, palette): (String, Arc<drawing::Palette>),
        missing: bool,
    ) {
        let id = self
            .open
            .as_ref()
            .filter(|open| open.session == session)
            .and_then(|open| open.id.clone());
        let step = match id.clone().filter(|_| missing) {
            Some(id) => PictureStep::Checking {
                has: api::notes::drawing_thumbnail(backend, id),
                scene: (content, palette),
            },
            None if missing => return,
            None => PictureStep::Taking(api::notes::picture_drawing(backend, content, palette)),
        };
        self.picturing.push(Picturing { session, id, step });
    }

    /// What came back goes on to its next step; what is taken is written once the note has a row.
    fn advance_pictures(&mut self, backend: &Backend, errors: &mut ErrorNotifier) {
        let mut writes = Vec::new();
        self.picturing.retain_mut(|entry| {
            let step = std::mem::replace(
                &mut entry.step,
                PictureStep::Taken(DrawingPictures::default()),
            );
            entry.step = match step {
                PictureStep::Checking { mut has, scene } => match has.poll() {
                    None => PictureStep::Checking { has, scene },
                    Some(Ok(None)) => {
                        PictureStep::Taking(api::notes::picture_drawing(backend, scene.0, scene.1))
                    }
                    Some(_) => return false,
                },
                PictureStep::Taking(mut pending) => match pending.poll() {
                    None => PictureStep::Taking(pending),
                    Some(Ok(pictures)) => PictureStep::Taken(pictures),
                    Some(Err(error)) => {
                        errors.report(keys::errors::NOTE_SAVE_FAILED, &error);
                        return false;
                    }
                },
                taken @ PictureStep::Taken(_) => taken,
            };
            if let (PictureStep::Taken(pictures), Some(id)) = (&entry.step, &entry.id) {
                writes.push((id.clone(), pictures.clone()));
                return false;
            }
            true
        });
        for (id, pictures) in writes {
            let patch = NotePatch {
                pictures: Some(pictures),
                ..NotePatch::default()
            };
            let writing = api::notes::update(backend, id.clone(), patch);
            self.pictured.push((id, writing));
        }
    }

    /// The notes whose pictures changed: a card holds what it read before them.
    pub fn take_redrawn(&mut self) -> Vec<String> {
        std::mem::take(&mut self.redrawn)
    }

    /// Pinned, prioritised, moved or filed while a draft: it changes here, written with the
    /// rest once worth keeping. `false` when the note open is not a draft.
    pub fn change_draft(&mut self, backend: &Backend, change: impl FnOnce(&mut Note)) -> bool {
        let Some(draft) = self
            .open
            .as_ref()
            .filter(|open| open.is_draft())
            .and_then(|open| open.draft.as_ref())
        else {
            return false;
        };
        let mut note = draft.note.clone();
        change(&mut note);
        self.keep_draft(backend, note);
        true
    }

    /// Written as it is: a file attached to it makes it worth keeping.
    pub fn write_draft(&mut self, backend: &Backend) {
        let Some(open) = self.open.as_ref().filter(|open| open.is_draft()) else {
            return;
        };
        let session = open.session;
        let writing = self.creating.iter().any(|it| it.session == session);
        if let (Some(draft), false) = (&open.draft, writing) {
            let pending = api::notes::create(backend, new_note(&draft.note));
            self.creating.push(Creating {
                session,
                pending,
                waiting: Edit::default(),
            });
        }
    }

    /// A draft with nothing worth keeping only changes kind: there is no text to convert.
    pub fn convert_draft(&mut self, into: NoteKind) {
        let Some(open) = self.open.as_mut().filter(|open| open.is_draft()) else {
            return;
        };
        if let Some(draft) = &mut open.draft
            && !worth_keeping(&draft.note)
        {
            draft.note.kind = into;
            draft.note.language = Language::Txt;
            draft.note.items.clear();
            self.sessions += 1;
            open.session = self.sessions;
        }
    }

    /// The draft as it now is; written when worth keeping.
    fn keep_draft(&mut self, backend: &Backend, note: Note) {
        let Some(open) = self.open.as_mut() else {
            return;
        };
        // One creation per draft: what changes while it is written stays on screen only.
        let writing = self
            .creating
            .iter()
            .any(|creating| creating.session == open.session);
        if worth_keeping(&note) && !writing {
            let pending = api::notes::create(backend, new_note(&note));
            self.creating.push(Creating {
                session: open.session,
                pending,
                waiting: Edit::default(),
            });
        }
        open.draft = Some(api::notes::decorate(note));
    }

    pub fn update(&mut self, backend: &Backend, revision: Revision, errors: &mut ErrorNotifier) {
        if let Some(open) = &mut self.open
            && let Some(id) = &open.id
        {
            open.read.want((id.clone(), revision));
            let read = open
                .read
                .update(|(id, _)| api::notes::get(backend, id.clone()));
            if read && std::mem::take(&mut open.reseed) {
                // A session number of its own, past every one given: `sessions` follows below.
                open.session = self.sessions + 1;
            }
            if open.read.value().is_some() {
                open.draft = None;
            } else if open.draft.is_none()
                && let Some(error) = open.read.error()
            {
                errors.report(keys::errors::NOTE_READ_FAILED, error);
                self.open = None;
            }
        }
        if let Some(open) = &self.open {
            self.sessions = self.sessions.max(open.session);
        }
    }

    /// The writes that answered; each one moves the revision.
    pub fn adopt(
        &mut self,
        backend: &Backend,
        revision: &mut Revision,
        errors: &mut ErrorNotifier,
    ) {
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
                    let waiting = waiting.changed_from(&note.note);
                    if !waiting.is_empty() {
                        self.writes.push(api::notes::update(
                            backend,
                            note.id.clone(),
                            waiting.into_patch(),
                        ));
                    }
                    for entry in &mut self.picturing {
                        if entry.session == session {
                            entry.id = Some(note.id.clone());
                        }
                    }
                    if let Some(open) = self.open.as_mut().filter(|open| open.session == session) {
                        open.id = Some(note.id.clone());
                    }
                }
                Err(error) => {
                    self.picturing.retain(|entry| entry.session != session);
                    errors.report(keys::errors::NOTE_CREATE_FAILED, &error);
                }
            }
        }
        self.advance_pictures(backend, errors);
        let redrawn = &mut self.redrawn;
        self.pictured
            .retain_mut(|(id, pending)| match pending.poll() {
                None => true,
                Some(Ok(_)) => {
                    redrawn.push(std::mem::take(id));
                    false
                }
                Some(Err(error)) => {
                    errors.report(keys::errors::NOTE_SAVE_FAILED, &error);
                    false
                }
            });
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

/// The reference's `isWorthSaving`: an untouched draft closed is a note nobody asked for.
fn worth_keeping(note: &Note) -> bool {
    !note.title.trim().is_empty()
        || !note.content.trim().is_empty()
        || !note.source.trim().is_empty()
        || !note.tags.is_empty()
        || !note.items.is_empty()
        || note.pinned
        || note.priority != Priority::None
        || matches!(note.lifecycle, NoteLifecycle::Expires { .. })
}

fn new_note(note: &Note) -> NoteDraft {
    NoteDraft {
        space_id: note.space_id.clone(),
        folder_id: note.folder_id.clone(),
        title: note.title.clone(),
        language: note.language,
        content: note.content.clone(),
        source: note.source.clone(),
        tags: note.tags.clone(),
        pinned: note.pinned,
        lifecycle: note.lifecycle.clone(),
        kind: note.kind,
        priority: note.priority,
        items: note.items.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> Note {
        let mut state = EditorState::default();
        state.draft(NoteKind::Snippet, "space".into(), None);
        state.opened().unwrap().note().unwrap().note.clone()
    }

    #[test]
    fn an_untouched_draft_is_not_worth_keeping() {
        let mut note = blank();
        note.title = "  ".into();
        note.content = "\n".into();
        assert!(!worth_keeping(&note));
        note.priority = Priority::High;
        assert!(worth_keeping(&note));
    }

    #[test]
    fn a_field_left_as_it_was_is_not_sent() {
        let mut note = blank();
        note.title = "psql".into();
        note.tags = vec!["db".into()];
        let edit = Edit {
            title: Some("psql".into()),
            tags: Some(vec!["db".into()]),
            source: Some("Ops".into()),
            ..Edit::default()
        };

        let changed = edit.changed_from(&note);

        assert_eq!(
            changed,
            Edit {
                source: Some("Ops".into()),
                ..Edit::default()
            }
        );
    }

    #[test]
    fn a_later_field_wins_and_an_earlier_one_stays() {
        let mut first = Edit {
            title: Some("a".into()),
            source: Some("Ops".into()),
            ..Edit::default()
        };
        first.merge(Edit {
            title: Some("b".into()),
            ..Edit::default()
        });

        assert_eq!(first.title.as_deref(), Some("b"));
        assert_eq!(first.source.as_deref(), Some("Ops"));
    }
}
