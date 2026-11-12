//! Folders, nested inside one space.

use engine::folders::{
    self,
    board::PreviewLines,
    model::{Folder, FolderColour, FolderDraft, FolderMove},
};

use super::undo::Reversible;
use crate::backend::{Backend, Pending};

/// Every space's: the rail draws the whole library. In creation order.
pub fn list(backend: &Backend) -> Pending<Vec<Folder>> {
    backend.db(|engine| folders::list_folders(engine, None))
}

pub fn create(
    backend: &Backend,
    space_id: String,
    parent_id: Option<String>,
    name: String,
) -> Pending<Folder> {
    let draft = FolderDraft {
        space_id,
        name,
        parent_id,
    };
    backend.db(move |engine| folders::create_folder(engine, draft))
}

pub fn rename(backend: &Backend, id: String, name: String) -> Pending<Folder> {
    backend.db(move |engine| folders::rename_folder(engine, id, name))
}

pub fn recolour(backend: &Backend, id: String, colour: FolderColour) -> Pending<Folder> {
    backend.db(move |engine| folders::recolour_folder(engine, id, colour))
}

/// What it holds goes up a level.
pub fn delete(backend: &Backend, id: String, lines: PreviewLines) -> Pending<()> {
    backend.db(move |engine| folders::delete_folder(engine, id, lines))
}

/// Under `parent_id`, or at the top of its space, with what it holds: answers where it was.
pub fn move_to(backend: &Backend, id: String, parent_id: Option<String>) -> Pending<FolderMove> {
    backend.db(move |engine| folders::move_folder(engine, id, parent_id))
}

/// What siblings are ordered by: their name, case and accents aside. Pure, under a
/// microsecond (D12).
pub fn sort_key(name: &str) -> String {
    engine::notes::view::fold(name)
}

/// Into `folder`, or out of every folder for `None`.
pub fn file(
    backend: &Backend,
    ids: Vec<String>,
    folder: Option<String>,
    lines: PreviewLines,
) -> Pending<Reversible> {
    backend.db(move |engine| folders::file_notes(engine, ids, folder, lines).map(Reversible::File))
}
