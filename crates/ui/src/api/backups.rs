//! The copies kept beside the library.

use engine::backup::{self, copies::Backup};

use crate::backend::{Backend, Pending};

pub fn list(backend: &Backend) -> Pending<Vec<Backup>> {
    backend.db(backup::list_backups)
}

/// Closes the library and puts the copy back: the folder the replaced one went to.
pub fn restore(backend: &Backend, id: String) -> Pending<String> {
    backend.db(move |engine| backup::restore_backup(engine, id))
}
