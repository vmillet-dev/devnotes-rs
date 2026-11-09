//! The library's folders, and which one is open. `None` is a choice — every folder — and not a
//! waiting state. Opening a folder is choosing it, from the rail, the switcher or a zone's title
//! on the board. Writes are not optimistic: each answer bumps the revision, and the list is asked
//! again with it.

use engine::folders::board::PreviewLines;
use engine::folders::model::{Folder, FolderColour, FolderMove};
use i18n::{Message, keys};

use super::{QueryCell, Revision};
use crate::api;
use crate::api::undo::Reversible;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::services::preferences::Preferences;

/// The library's, like everything under `devnotes.notes.`: an id means nothing elsewhere.
pub const ACTIVE_FOLDER_KEY: &str = "devnotes.notes.activeFolder";

enum Write {
    Folder(Pending<Folder>),
    /// What it held goes up a level, and so does the canvas if it was showing it: to `parent`.
    Deleted {
        pending: Pending<()>,
        folder: String,
        parent: Option<String>,
    },
    /// Offered back by the undo bar, named in its words.
    Moved {
        pending: Pending<FolderMove>,
        name: String,
        into: Option<String>,
    },
}

struct Writing {
    write: Write,
    /// Said when it fails, with the name typed.
    failed: Message,
}

/// What a frame's answers ask of the rest of the window.
#[derive(Default)]
pub struct Answered {
    /// The open folder changed: the caller remembers it.
    pub moved: bool,
    /// A folder moved, for the undo bar.
    pub done: Vec<Reversible>,
}

#[derive(Default)]
pub struct FoldersState {
    folders: QueryCell<Revision, Vec<Folder>>,
    chosen: Option<String>,
    writes: Vec<Writing>,
    /// The palette's « New folder » waits for its name.
    naming: bool,
}

impl FoldersState {
    pub fn restore(&mut self, preferences: &Preferences) {
        self.chosen = preferences
            .get(ACTIVE_FOLDER_KEY)
            .filter(|id| !id.is_empty())
            .map(str::to_owned);
    }

    pub fn remember(&self, preferences: &mut Preferences) {
        preferences.set(
            ACTIVE_FOLDER_KEY,
            self.chosen.as_deref().unwrap_or_default(),
        );
    }

    /// Once a frame, with the library open.
    pub fn update(
        &mut self,
        backend: &Backend,
        revision: &mut Revision,
        errors: &mut ErrorNotifier,
    ) -> Answered {
        let mut answered = Answered::default();
        let mut left = Vec::new();
        self.writes.retain_mut(|writing| {
            let answer = match &mut writing.write {
                Write::Folder(pending) => pending.poll().map(|answer| answer.map(|_| None)),
                Write::Deleted {
                    pending,
                    folder,
                    parent,
                } => pending
                    .poll()
                    .map(|answer| answer.map(|()| Some((folder.clone(), parent.clone())))),
                Write::Moved {
                    pending,
                    name,
                    into,
                } => pending.poll().map(|answer| {
                    answer.map(|was| {
                        answered.done.push(Reversible::FolderMove {
                            was,
                            name: name.clone(),
                            into: into.clone(),
                        });
                        None
                    })
                }),
            };
            match answer {
                None => true,
                Some(Ok(leaving)) => {
                    revision.bump();
                    left.extend(leaving);
                    false
                }
                Some(Err(error)) => {
                    errors.report_message(writing.failed.clone(), &error);
                    false
                }
            }
        });
        for (folder, parent) in left {
            if self.chosen.as_deref() == Some(folder.as_str()) {
                self.chosen = parent;
                answered.moved = true;
            }
        }
        self.folders.want(*revision);
        if self.folders.update(|_| api::folders::list(backend))
            && let Some(error) = self.folders.error()
        {
            errors.report(keys::errors::FOLDERS_LOAD_FAILED, error);
        }
        answered
    }

    /// Every space's, in creation order.
    pub fn list(&self) -> &[Folder] {
        self.folders.value().map_or(&[], Vec::as_slice)
    }

    pub fn find(&self, id: &str) -> Option<&Folder> {
        self.list().iter().find(|folder| folder.id == id)
    }

    /// A folder of another space than `space`, the one shown, is « every folder ».
    pub fn active(&self, space: Option<&str>) -> Option<&str> {
        let chosen = self.chosen.as_deref()?;
        let Some(folders) = self.folders.value() else {
            return Some(chosen);
        };
        folders
            .iter()
            .any(|folder| folder.id == chosen && space.is_none_or(|space| folder.space_id == space))
            .then_some(chosen)
    }

    /// Whether `id` holds folders: its board is then a board, and not a flat grid.
    pub fn has_children(&self, id: &str) -> bool {
        self.list()
            .iter()
            .any(|folder| folder.parent_id.as_deref() == Some(id))
    }

    /// Opened, in its own space: answers that space, which the caller shows.
    pub fn choose(&mut self, id: &str) -> Option<String> {
        let space = self.find(id)?.space_id.clone();
        self.chosen = Some(id.to_owned());
        Some(space)
    }

    /// Out of every folder: the space itself.
    pub fn leave(&mut self) {
        self.chosen = None;
    }

    /// Across every space a folder has nowhere to go: nothing is asked.
    pub fn ask_name(&mut self, space: Option<&str>) {
        self.naming = space.is_some();
    }

    pub fn naming(&self) -> bool {
        self.naming
    }

    pub fn dismiss_naming(&mut self) {
        self.naming = false;
    }

    pub fn create(
        &mut self,
        backend: &Backend,
        space_id: String,
        parent_id: Option<String>,
        name: &str,
    ) {
        let Some(name) = trimmed(name) else { return };
        let failed = Message::new(keys::errors::FOLDER_CREATE_FAILED).with("name", name.as_str());
        let pending = api::folders::create(backend, space_id, parent_id, name);
        self.push(Write::Folder(pending), failed);
    }

    pub fn rename(&mut self, backend: &Backend, id: String, name: &str) {
        let Some(name) = trimmed(name) else { return };
        let unchanged = self
            .list()
            .iter()
            .any(|folder| folder.id == id && folder.name == name);
        if unchanged {
            return;
        }
        let failed = Message::new(keys::errors::FOLDER_RENAME_FAILED).with("name", name.as_str());
        self.push(
            Write::Folder(api::folders::rename(backend, id, name)),
            failed,
        );
    }

    pub fn recolour(&mut self, backend: &Backend, id: String, colour: FolderColour) {
        let failed = Message::new(keys::errors::FOLDER_RECOLOUR_FAILED);
        let pending = api::folders::recolour(backend, id, colour);
        self.push(Write::Folder(pending), failed);
    }

    /// What it held goes up a level, and so does the canvas if it was showing it.
    pub fn delete(&mut self, backend: &Backend, id: String, lines: PreviewLines) {
        let parent = self.find(&id).and_then(|folder| folder.parent_id.clone());
        let failed = Message::new(keys::errors::FOLDER_DELETE_FAILED);
        let write = Write::Deleted {
            pending: api::folders::delete(backend, id.clone(), lines),
            folder: id,
            parent,
        };
        self.push(write, failed);
    }

    /// Under `parent_id`, or at the top of its space for `None`, with everything it holds. The
    /// rules are `Lineage`'s, in the core; a move it refuses is said, and changes nothing.
    pub fn move_folder(&mut self, backend: &Backend, id: String, parent_id: Option<String>) {
        let Some(name) = self.find(&id).map(|folder| folder.name.clone()) else {
            return;
        };
        let into = parent_id
            .as_deref()
            .and_then(|parent| self.find(parent))
            .map(|parent| parent.name.clone());
        let failed = Message::new(keys::errors::FOLDER_MOVE_FAILED);
        let write = Write::Moved {
            pending: api::folders::move_to(backend, id, parent_id),
            name,
            into,
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
