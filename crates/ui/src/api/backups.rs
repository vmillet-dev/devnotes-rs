//! The copies kept beside the library.

use engine::backup::{self, copies::Backup};

use crate::backend::{Backend, Pending};

pub fn list(backend: &Backend) -> Pending<Vec<Backup>> {
    backend.db(backup::list_backups)
}

/// The person's setting, handed to the core before an unlock reads it.
pub fn keep_launch_copies(backend: &Backend, wanted: bool) {
    backend.engine().keep_launch_copies(wanted);
}

/// Closes the library and puts the copy back: the folder the replaced one went to.
pub fn restore(backend: &Backend, id: String) -> Pending<String> {
    backend.db(move |engine| backup::restore_backup(engine, &id))
}
