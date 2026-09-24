//! The names on disk: what a library's directory holds, and what the profile holds beside
//! the libraries.
//!
//! ⚠️ Each is the address of something already written on someone's machine: renaming one
//! sends an installed copy looking for files still there under the old name.

use chrono::{DateTime, NaiveDateTime, Utc};

/// Spelled once, so the sidecars below cannot drift from it.
macro_rules! database {
    () => {
        "devnotes.sqlite3"
    };
}

pub(crate) const DATABASE: &str = database!();

/// What SQLite keeps beside the database under WAL; they travel with it.
pub(crate) const DATABASE_SIDECARS: [&str; 2] =
    [concat!(database!(), "-wal"), concat!(database!(), "-shm")];

/// The library's key, wrapped under the passphrase.
pub(crate) const KEY_FILE: &str = "vault.json";

pub(crate) const ATTACHMENTS: &str = "attachments";

/// The rolling copies taken at launch.
pub(crate) const BACKUPS: &str = "backups";

/// A library that would not open, set aside with what could be rescued from it.
pub(crate) const DAMAGED: &str = "damaged";

/// A library whose passphrase was forgotten, set aside whole.
pub(crate) const ARCHIVED: &str = "archived";

/// The library a restore replaced, kept so the gesture can be undone by hand.
pub(crate) const REPLACED: &str = "replaced";

/// The library's own preferences and the application's, told apart by their directory.
pub(crate) const PREFERENCES: &str = "preferences.json";

/// Every file a library is made of.
pub(crate) const LIBRARY_FILES: [&str; 4] = [
    DATABASE,
    DATABASE_SIDECARS[0],
    DATABASE_SIDECARS[1],
    KEY_FILE,
];

/// Every directory a library accumulates.
pub(crate) const LIBRARY_DIRECTORIES: [&str; 5] =
    [ATTACHMENTS, BACKUPS, DAMAGED, ARCHIVED, REPLACED];

/// The list of libraries, at the profile root and owned by none of them.
pub(crate) const REGISTRY: &str = "libraries.json";

/// Where the libraries live, one directory each.
pub(crate) const LIBRARIES: &str = "libraries";

/// The decrypted copies handed to other programs, at the profile root so one sweep catches
/// every library's.
pub(crate) const PLAINTEXT_COPIES: &str = "open";

/// Colons are legal in an instant and not in a Windows path.
const STAMP: &str = "%Y-%m-%d_%H-%M-%S";

/// The name of a timestamped directory — a backup, a library set aside.
pub(crate) fn stamp(at: DateTime<Utc>) -> String {
    at.format(STAMP).to_string()
}

pub(crate) fn parse_stamp(name: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(name, STAMP)
        .ok()
        .map(|at| at.and_utc())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sidecars_are_named_after_the_database() {
        assert_eq!(
            DATABASE_SIDECARS,
            ["devnotes.sqlite3-wal", "devnotes.sqlite3-shm"]
        );
    }

    #[test]
    fn a_stamp_reads_back_as_the_instant_it_was_written_from() {
        let at = crate::db::iso8601::parse("2026-07-25T09:12:34.000Z").unwrap();

        assert_eq!(stamp(at), "2026-07-25_09-12-34");
        assert_eq!(parse_stamp(&stamp(at)), Some(at));
        assert_eq!(parse_stamp("not a stamp"), None);
    }
}
