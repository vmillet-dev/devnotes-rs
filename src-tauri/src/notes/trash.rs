use chrono::{DateTime, TimeDelta, Utc};
use serde::Serialize;
use specta::Type;

use super::model::Note;
use super::store;
use crate::attachments;
use crate::db::{Db, lock};
use crate::error::StorageError;

/// Single threshold: the panel shows the deadline the purge applies.
pub const RETENTION: TimeDelta = TimeDelta::days(30);

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrashedNote {
    #[serde(flatten)]
    pub note: Note,
    pub deleted_at: DateTime<Utc>,
    /// Derived, never stored: retention can change between versions.
    pub purge_at: DateTime<Utc>,
}

pub fn purge_at(deleted_at: DateTime<Utc>) -> DateTime<Utc> {
    deleted_at + RETENTION
}

pub fn is_expired(deleted_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    deleted_at <= expiry_cutoff(now)
}

/// A note deleted at or before this instant has outlived `RETENTION` by `now`.
pub fn expiry_cutoff(now: DateTime<Utc>) -> DateTime<Utc> {
    now - RETENTION
}

pub fn trashed(note: Note, deleted_at: DateTime<Utc>) -> TrashedNote {
    TrashedNote {
        note,
        deleted_at,
        purge_at: purge_at(deleted_at),
    }
}

/// ⚠️ The attachment file names are collected before the `DELETE`: afterwards the cascade
/// has taken the records that carried them, and the files are orphaned until the next
/// startup sweep.
pub fn purge(db: &Db, ids: Vec<String>) -> Result<usize, StorageError> {
    if ids.is_empty() {
        return Ok(0);
    }

    let mut connection = lock(db)?;
    let directory = attachments::directory(&connection);
    let files = attachments::store::stored_names_of(&mut connection, &ids)?;
    let purged = store::trash::purge(&mut connection, &ids)?;
    drop(connection);

    attachments::remove_files(&directory, &files);

    Ok(purged)
}

pub fn purge_expired(db: &Db) -> Result<(), StorageError> {
    let expired = {
        let mut connection = lock(db)?;
        store::trash::expired_ids(&mut connection, Utc::now())?
    };

    purge(db, expired)?;

    Ok(())
}

/// What makes retention hold even if nobody opens the trash. Never fatal.
pub fn sweep_at_startup(db: &Db) {
    if let Err(error) = purge_expired(db) {
        log::warn!("Expired trash not purged: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachments::model::Attachment;
    use crate::db::iso8601;
    use crate::notes::fixtures::note;

    fn at(iso: &str) -> DateTime<Utc> {
        iso8601::parse(iso).unwrap()
    }

    /// A note trashed long ago, with an attachment whose file sits in the library.
    fn a_trashed_note_with_a_file() -> (Db, String, std::path::PathBuf) {
        let mut library = crate::db::open_in_memory().unwrap();
        let space_id = crate::spaces::store::create(&mut library, "Perso")
            .unwrap()
            .id;
        let note = Note { space_id, ..note() };
        store::insert_imported(&mut library, &note).unwrap();

        let attachment = Attachment {
            id: "a-1".to_string(),
            note_id: note.id.clone(),
            file_name: "capture.png".to_string(),
            mime_type: "image/png".to_string(),
            byte_size: 3,
            created_at: at("2020-01-01T00:00:00.000Z"),
        };
        let directory = attachments::directory(&library);
        std::fs::create_dir_all(&directory).unwrap();
        let file = directory.join(attachment.stored_name());
        std::fs::write(&file, b"png").unwrap();
        let (db, vault) = library.split();
        attachments::store::create(db, vault, &attachment).unwrap();
        store::trash::trash(&mut library, &note.id, at("2020-01-01T00:00:00.000Z")).unwrap();

        (std::sync::Mutex::new(Some(library)), note.id, file)
    }

    #[test]
    fn purging_a_note_removes_the_files_of_its_attachments() {
        let (db, id, file) = a_trashed_note_with_a_file();

        assert_eq!(purge(&db, vec![id]).unwrap(), 1);

        assert!(!file.exists());
        std::fs::remove_dir_all(file.parent().unwrap()).ok();
    }

    #[test]
    fn the_startup_sweep_purges_what_outlived_its_retention() {
        let (db, _, file) = a_trashed_note_with_a_file();

        sweep_at_startup(&db);

        assert!(!file.exists());
        assert!(
            store::trash::list_trashed(&mut crate::db::lock(&db).unwrap())
                .unwrap()
                .is_empty()
        );
        std::fs::remove_dir_all(file.parent().unwrap()).ok();
    }

    #[test]
    fn a_note_deleted_today_expires_thirty_days_later() {
        assert_eq!(
            purge_at(at("2026-07-25T09:00:00.000Z")),
            at("2026-08-24T09:00:00.000Z")
        );
    }

    #[test]
    fn the_last_day_still_belongs_to_the_user() {
        let deleted = at("2026-07-25T09:00:00.000Z");

        assert!(!is_expired(deleted, at("2026-08-24T08:59:59.999Z")));
        assert!(is_expired(deleted, at("2026-08-24T09:00:00.000Z")));
    }
}
