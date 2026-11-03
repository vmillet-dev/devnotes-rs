//! The two `preferences.json`, read and written on the db queue: two writes made in a row land
//! in that order, and none waits on the UI thread.

use engine::error::{AppError, StorageError};
use engine::{layout, libraries};

use crate::backend::{Backend, Pending};
use crate::services::preferences::Preferences;

/// The open library's own: read again each time a library opens.
pub fn library(backend: &Backend) -> Pending<Preferences> {
    backend.db(|engine| {
        let directory = libraries::open_library_directory(engine)?;
        Ok(Preferences::read(&directory.join(layout::PREFERENCES)))
    })
}

pub fn write_library(backend: &Backend, preferences: Preferences) -> Pending<()> {
    backend.db(move |engine| {
        let directory = libraries::open_library_directory(engine)?;
        written(&preferences, &directory.join(layout::PREFERENCES))
    })
}

pub fn write_application(backend: &Backend, preferences: Preferences) -> Pending<()> {
    let file = backend.engine().paths().profile.join(layout::PREFERENCES);
    backend.db(move |_| written(&preferences, &file))
}

fn written(preferences: &Preferences, file: &std::path::Path) -> Result<(), AppError> {
    preferences
        .write(file)
        .map_err(|error| StorageError::File(format!("{}: {error}", file.display())).into())
}
