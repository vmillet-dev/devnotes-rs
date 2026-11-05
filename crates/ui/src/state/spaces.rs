//! The library's spaces, and which one narrows the canvas. `None` is a choice — every space —
//! and not a waiting state. Writes are not optimistic: each answer bumps the revision, and the
//! list is asked again with it. The folders inside them are `state::folders`'.

use engine::spaces::model::{Space, SpaceContents};
use i18n::{Message, keys};

use super::{QueryCell, Revision};
use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::services::preferences::Preferences;

/// The library's, like everything under `devnotes.notes.`: an id means nothing elsewhere.
pub const ACTIVE_SPACE_KEY: &str = "devnotes.notes.activeSpace";

enum Write {
    /// Shown once made: its id arrives with the answer.
    Made(Pending<Space>),
    Changed(Pending<Space>),
    /// Its refuge is shown once it is gone: the notes have just landed there.
    Deleted {
        pending: Pending<()>,
        refuge: String,
    },
}

struct Writing {
    write: Write,
    /// Said when it fails, with the name typed.
    failed: Message,
}

#[derive(Default)]
pub struct SpacesState {
    spaces: QueryCell<Revision, Vec<Space>>,
    chosen: Option<String>,
    writes: Vec<Writing>,
    /// The space whose deletion is being considered, and what it would move and take.
    counted: Option<String>,
    contents: QueryCell<(String, Revision), SpaceContents>,
}

impl SpacesState {
    /// The space the library was left on, from its preferences.
    pub fn restore(&mut self, preferences: &Preferences) {
        self.chosen = preferences
            .get(ACTIVE_SPACE_KEY)
            .filter(|id| !id.is_empty())
            .map(str::to_owned);
    }

    /// The choice into `preferences`, every other key kept.
    pub fn remember(&self, preferences: &mut Preferences) {
        preferences.set(ACTIVE_SPACE_KEY, self.chosen.as_deref().unwrap_or_default());
    }

    /// Once a frame, with the library open. `true` when a write that came back changed the space
    /// shown: the caller leaves the folder and remembers it.
    pub fn update(
        &mut self,
        backend: &Backend,
        revision: &mut Revision,
        errors: &mut ErrorNotifier,
    ) -> bool {
        let mut shown = Vec::new();
        self.writes.retain_mut(|writing| {
            let answer = match &mut writing.write {
                Write::Made(pending) => pending
                    .poll()
                    .map(|answer| answer.map(|space| Some(space.id))),
                Write::Changed(pending) => pending.poll().map(|answer| answer.map(|_| None)),
                Write::Deleted { pending, refuge } => pending
                    .poll()
                    .map(|answer| answer.map(|()| Some(refuge.clone()))),
            };
            match answer {
                None => true,
                Some(Ok(show)) => {
                    revision.bump();
                    shown.extend(show);
                    false
                }
                Some(Err(error)) => {
                    errors.report_message(writing.failed.clone(), &error);
                    false
                }
            }
        });
        let moved = !shown.is_empty();
        if let Some(space) = shown.pop() {
            self.chosen = Some(space);
        }
        self.spaces.want(*revision);
        if self.spaces.update(|_| api::spaces::list(backend))
            && let Some(error) = self.spaces.error()
        {
            errors.report(keys::errors::SPACES_LOAD_FAILED, error);
        }
        if let Some(id) = &self.counted {
            self.contents.want((id.clone(), *revision));
            self.contents
                .update(|(id, _)| api::spaces::contents(backend, id));
        }
        moved
    }

    /// Pinned first, then by name. Empty until the first answer.
    pub fn spaces(&self) -> &[Space] {
        self.spaces.value().map_or(&[], Vec::as_slice)
    }

    /// An id no space answers to is « every space », once the list is known: a space deleted
    /// elsewhere hides no note.
    pub fn active_space(&self) -> Option<&str> {
        let chosen = self.chosen.as_deref()?;
        let Some(spaces) = self.spaces.value() else {
            return Some(chosen);
        };
        spaces
            .iter()
            .any(|space| space.id == chosen)
            .then_some(chosen)
    }

    /// The space whose deletion is considered, and what it holds once counted.
    pub fn contents(&self, id: &str) -> Option<&SpaceContents> {
        (self.counted.as_deref() == Some(id))
            .then(|| self.contents.value())
            .flatten()
    }

    /// The caller leaves the folder: the row means the space itself.
    pub fn choose_space(&mut self, id: Option<String>) {
        self.chosen = id;
    }

    pub fn count(&mut self, id: String) {
        if self.counted.as_deref() != Some(id.as_str()) {
            self.contents = QueryCell::default();
            self.counted = Some(id);
        }
    }

    pub fn create_space(&mut self, backend: &Backend, name: &str) {
        let Some(name) = trimmed(name) else { return };
        let failed = Message::new(keys::errors::SPACE_CREATE_FAILED).with("name", name.as_str());
        let pending = api::spaces::create(backend, name);
        self.push(Write::Made(pending), failed);
    }

    pub fn rename_space(&mut self, backend: &Backend, id: String, name: &str) {
        let Some(name) = trimmed(name) else { return };
        let unchanged = self
            .spaces()
            .iter()
            .any(|space| space.id == id && space.name == name);
        if unchanged {
            return;
        }
        let failed = Message::new(keys::errors::SPACE_RENAME_FAILED).with("name", name.as_str());
        self.push(
            Write::Changed(api::spaces::rename(backend, id, name)),
            failed,
        );
    }

    pub fn pin_space(&mut self, backend: &Backend, id: String, pinned: bool) {
        let failed = Message::new(keys::errors::SPACE_PIN_FAILED);
        self.push(
            Write::Changed(api::spaces::pin(backend, id, pinned)),
            failed,
        );
    }

    /// The refuge is then shown: the notes have just landed there.
    pub fn delete_space(&mut self, backend: &Backend, id: String, refuge: String) {
        if id == refuge {
            return;
        }
        let failed = Message::new(keys::errors::SPACE_DELETE_FAILED);
        let write = Write::Deleted {
            pending: api::spaces::delete(backend, id, refuge.clone()),
            refuge,
        };
        self.push(write, failed);
    }

    fn push(&mut self, write: Write, failed: Message) {
        self.writes.push(Writing { write, failed });
    }
}

fn trimmed(name: &str) -> Option<String> {
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}
