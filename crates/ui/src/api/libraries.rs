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
