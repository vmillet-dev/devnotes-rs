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
    purge_at(deleted_at) <= now
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
    use crate::db::iso8601;

    fn at(iso: &str) -> DateTime<Utc> {
        iso8601::parse(iso).unwrap()
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
