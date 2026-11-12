//! Spaces: the rail's top level, and where a note lives.

use engine::spaces::{self, model::Space, model::SpaceContents, model::SpaceDraft};

use crate::backend::{Backend, Pending};

/// Pinned first, then by name: the core's order.
pub fn list(backend: &Backend) -> Pending<Vec<Space>> {
    backend.db(spaces::list_spaces)
}

pub fn create(backend: &Backend, name: String) -> Pending<Space> {
    backend.db(move |engine| spaces::create_space(engine, SpaceDraft { name }))
}

pub fn rename(backend: &Backend, id: String, name: String) -> Pending<Space> {
    backend.db(move |engine| spaces::rename_space(engine, id, SpaceDraft { name }))
}

pub fn pin(backend: &Backend, id: String, pinned: bool) -> Pending<Space> {
    backend.db(move |engine| spaces::pin_space(engine, id, pinned))
}

/// What deleting it would move and take, said before the first click.
pub fn contents(backend: &Backend, id: String) -> Pending<SpaceContents> {
    backend.db(move |engine| spaces::count_space_contents(engine, id))
}

/// Its notes go to `refuge` first: the space leaves with whatever nobody takes in.
pub fn delete(backend: &Backend, id: String, refuge: String) -> Pending<()> {
    backend.db(move |engine| spaces::delete_space(engine, id, refuge))
}
