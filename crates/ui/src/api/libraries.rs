//! The registry of libraries: which there are, and which is open.

use engine::libraries::{self, registry::Registry};

use crate::backend::{Backend, Pending};

pub fn list(backend: &Backend) -> Pending<Registry> {
    backend.db(|engine| Ok(libraries::list_libraries(engine)))
}

/// Another library, locked: the gate asks for its phrase next.
pub fn open(backend: &Backend, id: String) -> Pending<()> {
    backend.db(move |engine| libraries::open_library(engine, id))
}

/// Creating opens it, in one gesture: the gate then asks for a phrase, as on a first launch.
pub fn create_and_open(backend: &Backend, name: String) -> Pending<()> {
    backend.db(move |engine| {
        let entry = libraries::create_library(engine, name)?;
        libraries::open_library(engine, entry.id)
    })
}

pub fn rename(backend: &Backend, id: String, name: String) -> Pending<()> {
    backend.db(move |engine| libraries::rename_library(engine, id, name))
}

pub fn delete(backend: &Backend, id: String) -> Pending<()> {
    backend.db(move |engine| libraries::delete_library(engine, id))
}
