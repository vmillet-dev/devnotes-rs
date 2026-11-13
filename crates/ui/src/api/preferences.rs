//! The two `preferences.json`, read and written on the db queue: two writes made in a row land
//! in that order, and none waits on the UI thread.

use engine::error::{AppError, StorageError};
use engine::libraries::registry;
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

/// A key gone from the file of the library the gate stands before: none is open to answer for
/// its directory.
pub fn forget_in_closed_library(backend: &Backend, key: &'static str) -> Pending<()> {
    let profile = backend.engine().paths().profile.clone();
    backend.db(move |_| {
        let file = registry::open_directory_in(&profile)?.join(layout::PREFERENCES);
        let mut preferences = Preferences::read(&file);
        preferences.remove(key);
        written(&preferences, &file)
    })
}

pub fn write_application(backend: &Backend, preferences: Preferences) -> Pending<()> {
    let file = backend.engine().paths().profile.join(layout::PREFERENCES);
    backend.db(move |_| written(&preferences, &file))
}

/// Logged when it fails: most callers let the answer go, and a setting lost must leave a trace.
fn written(preferences: &Preferences, file: &std::path::Path) -> Result<(), AppError> {
    preferences.write(file).map_err(|error| {
        let error: AppError = StorageError::File(format!("{}: {error}", file.display())).into();
        log::warn!("The preferences were not written: {}", error.for_log());
        error
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::testing;

    /// What the log was told, for the test that reads it: no other test here sets a logger.
    struct Heard(Mutex<Vec<String>>);

    impl log::Log for Heard {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            self.0.lock().unwrap().push(record.args().to_string());
        }

        fn flush(&self) {}
    }

    static HEARD: Heard = Heard(Mutex::new(Vec::new()));

    #[test]
    fn a_preference_write_that_fails_is_logged() {
        let _ = log::set_logger(&HEARD);
        log::set_max_level(log::LevelFilter::Warn);
        let (backend, _profile) = testing::unlocked();
        let directory =
            engine::db::run_here(backend.engine(), libraries::open_library_directory).unwrap();
        // A folder where the file goes: no write can take its place.
        let taken = directory.join(layout::PREFERENCES);
        std::fs::create_dir_all(taken.join("inside")).unwrap();

        let mut written = write_library(&backend, Preferences::default());
        backend.settle();

        assert!(written.poll().unwrap().is_err());
        let heard = HEARD.0.lock().unwrap();
        assert!(
            heard.iter().any(|line| line.contains("preferences")),
            "{heard:?}"
        );
    }
}
