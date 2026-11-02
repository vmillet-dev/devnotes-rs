use std::fs;

use engine::backup::{restore_backup, take_restored_aside};
use engine::db::open;
use engine::vault::file::create;
use engine::vault::key::Cost;

use super::Session;

const COPY: &str = "2026-07-25_09-00-00";

/// The page a restore reloads asks where the replaced library went, and is told once.
#[test]
fn a_restore_keeps_where_the_library_went_for_the_page_it_reloads() {
    let scratch = tempfile::tempdir().unwrap();
    let directory = scratch.path();
    let vault = create(directory, "a passphrase", Cost::FOR_TESTS).unwrap();
    let library = open(&directory.join("devnotes.sqlite3"), vault).unwrap();
    // A copy as the launch writes one: the database and its key file, under its stamp.
    let copy = directory.join("backups").join(COPY);
    fs::create_dir_all(&copy).unwrap();
    for name in ["devnotes.sqlite3", "vault.json"] {
        fs::copy(directory.join(name), copy.join(name)).unwrap();
    }
    let session = Session::with(Some(library));

    let aside = session
        .call(|e| restore_backup(e, COPY.to_string()))
        .unwrap();

    assert!(
        directory
            .join(&aside)
            .starts_with(directory.join("replaced"))
    );
    assert_eq!(session.call(take_restored_aside), Some(aside));
    assert_eq!(session.call(take_restored_aside), None);
}
