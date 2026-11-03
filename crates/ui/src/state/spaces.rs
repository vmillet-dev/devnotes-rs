//! The library's spaces and folders, and which of them narrows the canvas. `None` is a choice —
//! every space, every folder — and not a waiting state. Writes are not optimistic: each answer
//! bumps the revision, and both lists are asked again with it.

use engine::folders::model::{Folder, FolderColour};
use engine::spaces::model::{Space, SpaceContents};
use i18n::{Message, keys};

use super::{QueryCell, Revision};
use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::services::preferences::Preferences;

/// The library's, like everything under `devnotes.notes.`: an id means nothing elsewhere.
pub const ACTIVE_SPACE_KEY: &str = "devnotes.notes.activeSpace";
pub const ACTIVE_FOLDER_KEY: &str = "devnotes.notes.activeFolder";

/// What the canvas shows once a write is in: a space made or taking the notes in, a folder's
/// parent once it is gone.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Then {
    Stay,
    Show {
        space: Option<String>,
        folder: Option<String>,
    },
    /// Up to `parent` if `folder` was the one shown.
    LeaveFolder {
        folder: String,
        parent: Option<String>,
    },
}

enum Write {
    /// Shown once made: its id arrives with the answer.
    SpaceMade(Pending<Space>),
    Space(Pending<Space>),
    Folder(Pending<Folder>),
    Done(Pending<()>),
}

struct Writing {
    write: Write,
    /// Said when it fails, with the name typed.
    failed: Message,
    then: Then,
}

#[derive(Default)]
pub struct SpacesState {
    spaces: QueryCell<Revision, Vec<Space>>,
    folders: QueryCell<Revision, Vec<Folder>>,
    chosen_space: Option<String>,
    chosen_folder: Option<String>,
    writes: Vec<Writing>,
    /// The space whose deletion is being considered, and what it would move and take.
    counted: Option<String>,
    contents: QueryCell<(String, Revision), SpaceContents>,
}

impl SpacesState {
    /// The space and the folder the library was left on, from its preferences.
    pub fn restore(&mut self, preferences: &Preferences) {
        let remembered = |key| {
            preferences
                .get(key)
                .filter(|id| !id.is_empty())
                .map(str::to_owned)
        };
        self.chosen_space = remembered(ACTIVE_SPACE_KEY);
        self.chosen_folder = remembered(ACTIVE_FOLDER_KEY);
    }

    /// The choice into `preferences`, every other key kept.
    pub fn remember(&self, preferences: &mut Preferences) {
        preferences.set(
            ACTIVE_SPACE_KEY,
            self.chosen_space.as_deref().unwrap_or_default(),
        );
        preferences.set(
            ACTIVE_FOLDER_KEY,
            self.chosen_folder.as_deref().unwrap_or_default(),
        );
    }

    /// Once a frame, with the library open. `true` when what is shown was changed by a write
    /// that came back: the caller remembers it.
    pub fn update(
        &mut self,
        backend: &Backend,
        revision: &mut Revision,
        errors: &mut ErrorNotifier,
    ) -> bool {
        let mut moved = false;
        let mut done = Vec::new();
        self.writes.retain_mut(|writing| {
            let answer = match &mut writing.write {
                Write::SpaceMade(pending) => pending.poll().map(|answer| {
                    answer.map(|space| Then::Show {
                        space: Some(space.id),
                        folder: None,
                    })
                }),
                Write::Space(pending) => pending
                    .poll()
                    .map(|answer| answer.map(|_| writing.then.clone())),
                Write::Folder(pending) => pending
                    .poll()
                    .map(|answer| answer.map(|_| writing.then.clone())),
                Write::Done(pending) => pending
                    .poll()
                    .map(|answer| answer.map(|()| writing.then.clone())),
            };
            match answer {
                None => true,
                Some(Ok(then)) => {
                    done.push(then);
                    false
                }
                Some(Err(error)) => {
                    errors.report_message(writing.failed.clone(), &error);
                    false
                }
            }
        });
        for then in done {
            revision.bump();
            moved |= self.follow(then);
        }
        self.spaces.want(*revision);
        self.folders.want(*revision);
        let spaces_changed = self.spaces.update(|_| api::spaces::list(backend));
        let folders_changed = self.folders.update(|_| api::folders::list(backend));
        if spaces_changed && let Some(error) = self.spaces.error() {
            errors.report(keys::errors::SPACES_LOAD_FAILED, error);
        }
        if folders_changed && let Some(error) = self.folders.error() {
            errors.report(keys::errors::FOLDERS_LOAD_FAILED, error);
        }
        if let Some(id) = &self.counted {
            self.contents.want((id.clone(), *revision));
            self.contents
                .update(|(id, _)| api::spaces::contents(backend, id));
        }
        moved
    }

    fn follow(&mut self, then: Then) -> bool {
        match then {
            Then::Stay => false,
            Then::Show { space, folder } => {
                self.chosen_space = space;
                self.chosen_folder = folder;
                true
            }
            Then::LeaveFolder { folder, parent } => {
                if self.chosen_folder.as_deref() != Some(folder.as_str()) {
                    return false;
                }
                self.chosen_folder = parent;
                true
            }
        }
    }

    /// Pinned first, then by name. Empty until the first answer.
    pub fn spaces(&self) -> &[Space] {
        self.spaces.value().map_or(&[], Vec::as_slice)
    }

    /// Every space's, in creation order.
    pub fn folders(&self) -> &[Folder] {
        self.folders.value().map_or(&[], Vec::as_slice)
    }

    /// An id no space answers to is « every space », once the list is known: a space deleted
    /// elsewhere hides no note.
    pub fn active_space(&self) -> Option<&str> {
        let chosen = self.chosen_space.as_deref()?;
        let Some(spaces) = self.spaces.value() else {
            return Some(chosen);
        };
        spaces
            .iter()
            .any(|space| space.id == chosen)
            .then_some(chosen)
    }

    /// A folder of another space than the one shown is « every folder ».
    pub fn active_folder(&self) -> Option<&str> {
        let chosen = self.chosen_folder.as_deref()?;
        let Some(folders) = self.folders.value() else {
            return Some(chosen);
        };
        let space = self.active_space();
        folders
            .iter()
            .any(|folder| folder.id == chosen && space.is_none_or(|space| folder.space_id == space))
            .then_some(chosen)
    }

    /// The space whose deletion is considered, and what it holds once counted.
    pub fn contents(&self, id: &str) -> Option<&SpaceContents> {
        (self.counted.as_deref() == Some(id))
            .then(|| self.contents.value())
            .flatten()
    }

    /// Choosing a space leaves the folder: the row means the space itself.
    pub fn choose_space(&mut self, id: Option<String>) {
        self.chosen_space = id;
        self.chosen_folder = None;
    }

    /// ⚠️ The space first: a folder is shown inside the space it belongs to.
    pub fn choose_folder(&mut self, id: &str) {
        let Some(space) = self
            .folders()
            .iter()
            .find(|folder| folder.id == id)
            .map(|folder| folder.space_id.clone())
        else {
            return;
        };
        self.chosen_space = Some(space);
        self.chosen_folder = Some(id.to_owned());
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
        self.push(Write::SpaceMade(pending), failed, Then::Stay);
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
            Write::Space(api::spaces::rename(backend, id, name)),
            failed,
            Then::Stay,
        );
    }

    pub fn pin_space(&mut self, backend: &Backend, id: String, pinned: bool) {
        let failed = Message::new(keys::errors::SPACE_PIN_FAILED);
        self.push(
            Write::Space(api::spaces::pin(backend, id, pinned)),
            failed,
            Then::Stay,
        );
    }

    /// The refuge is then shown: the notes have just landed there.
    pub fn delete_space(&mut self, backend: &Backend, id: String, refuge: String) {
        if id == refuge {
            return;
        }
        let failed = Message::new(keys::errors::SPACE_DELETE_FAILED);
        let then = Then::Show {
            space: Some(refuge.clone()),
            folder: None,
        };
        self.push(
            Write::Done(api::spaces::delete(backend, id, refuge)),
            failed,
            then,
        );
    }

    pub fn create_folder(
        &mut self,
        backend: &Backend,
        space_id: String,
        parent_id: Option<String>,
        name: &str,
    ) {
        let Some(name) = trimmed(name) else { return };
        let failed = Message::new(keys::errors::FOLDER_CREATE_FAILED).with("name", name.as_str());
        let pending = api::folders::create(backend, space_id, parent_id, name);
        self.push(Write::Folder(pending), failed, Then::Stay);
    }

    pub fn rename_folder(&mut self, backend: &Backend, id: String, name: &str) {
        let Some(name) = trimmed(name) else { return };
        let unchanged = self
            .folders()
            .iter()
            .any(|folder| folder.id == id && folder.name == name);
        if unchanged {
            return;
        }
        let failed = Message::new(keys::errors::FOLDER_RENAME_FAILED).with("name", name.as_str());
        self.push(
            Write::Folder(api::folders::rename(backend, id, name)),
            failed,
            Then::Stay,
        );
    }

    pub fn recolour_folder(&mut self, backend: &Backend, id: String, colour: FolderColour) {
        let failed = Message::new(keys::errors::FOLDER_RECOLOUR_FAILED);
        let pending = api::folders::recolour(backend, id, colour);
        self.push(Write::Folder(pending), failed, Then::Stay);
    }

    /// What it held goes up a level, and so does the canvas if it was showing it.
    pub fn delete_folder(&mut self, backend: &Backend, id: String) {
        let parent = self
            .folders()
            .iter()
            .find(|folder| folder.id == id)
            .and_then(|folder| folder.parent_id.clone());
        let failed = Message::new(keys::errors::FOLDER_DELETE_FAILED);
        let then = Then::LeaveFolder {
            folder: id.clone(),
            parent,
        };
        self.push(Write::Done(api::folders::delete(backend, id)), failed, then);
    }

    fn push(&mut self, write: Write, failed: Message, then: Then) {
        self.writes.push(Writing {
            write,
            failed,
            then,
        });
    }
}

fn trimmed(name: &str) -> Option<String> {
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}
