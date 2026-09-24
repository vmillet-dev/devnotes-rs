//! The retention, and what a purge does to the attachment files, live in `notes::trash`: here
//! `deleted_at` decides. ⚠️ Every read elsewhere filters on `deleted_at IS NULL`, or a trashed
//! note comes back editable without saying it is on borrowed time.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use super::{NoteRow, related};
use crate::db::Library;
use crate::db::iso8601;
use crate::db::schema::notes;
use crate::error::StorageError;
use crate::notes::model::Note;
use crate::notes::trash;

/// Stamps `deleted_at`; the row survives for [`trash::RETENTION`] and [`purge`] erases. `delete`
/// is the word `spaces::store` and `attachments::store` use for an irreversible one.
pub fn trash(connection: &mut Library, id: &str, now: DateTime<Utc>) -> Result<(), StorageError> {
    if trash_many(connection, std::slice::from_ref(&id.to_string()), now)? == 0 {
        return Err(StorageError::NoteNotFound(id.to_string()));
    }

    Ok(())
}

/// Returns what actually moved: an id gone stale fails nothing but itself.
pub fn trash_many(
    connection: &mut Library,
    ids: &[String],
    now: DateTime<Utc>,
) -> Result<usize, StorageError> {
    if ids.is_empty() {
        return Ok(0);
    }

    Ok(diesel::update(
        notes::table
            .filter(notes::id.eq_any(ids))
            .filter(notes::deleted_at.is_null()),
    )
    .set(notes::deleted_at.eq(iso8601::format(now)))
    .execute(connection.db())?)
}

/// Leaves `updated_at` alone: the note comes back where it was.
pub fn restore_many(connection: &mut Library, ids: &[String]) -> Result<usize, StorageError> {
    if ids.is_empty() {
        return Ok(0);
    }

    Ok(diesel::update(
        notes::table
            .filter(notes::id.eq_any(ids))
            .filter(notes::deleted_at.is_not_null()),
    )
    .set(notes::deleted_at.eq(None::<String>))
    .execute(connection.db())?)
}

pub fn list_trashed(connection: &mut Library) -> Result<Vec<(Note, DateTime<Utc>)>, StorageError> {
    let rows = notes::table
        .filter(notes::deleted_at.is_not_null())
        .select((NoteRow::as_select(), notes::deleted_at))
        .order((notes::deleted_at.desc(), notes::id.asc()))
        .load::<(NoteRow, Option<String>)>(connection.db())?;

    let mut grouped = related::trashed_tags(connection)?;
    let vault = connection.vault();
    rows.into_iter()
        .map(|(row, deleted_at)| {
            let id = row.id.clone();
            let raw = deleted_at.unwrap_or_default();
            let deleted_at = iso8601::parse(&raw).map_err(|_| StorageError::CorruptRow {
                id: id.clone(),
                field: "deletedAt",
            })?;

            Ok((
                Note {
                    tags: grouped.remove(&id).unwrap_or_default(),
                    ..NoteRow::open(row, vault)?
                },
                deleted_at,
            ))
        })
        .collect()
}

/// Separate from [`purge`] so the caller can erase the attached files first.
pub fn expired_ids(
    connection: &mut Library,
    now: DateTime<Utc>,
) -> Result<Vec<String>, StorageError> {
    // Compared as text: `db::iso8601` always writes milliseconds, so the column orders in time.
    let cutoff = iso8601::format(trash::expiry_cutoff(now));

    Ok(notes::table
        .filter(notes::deleted_at.le(cutoff))
        .select(notes::id)
        .load::<String>(connection.db())?)
}

pub fn trashed_ids(connection: &mut Library) -> Result<Vec<String>, StorageError> {
    Ok(notes::table
        .filter(notes::deleted_at.is_not_null())
        .select(notes::id)
        .load::<String>(connection.db())?)
}

/// Permanent; tags and attachments leave by cascade (`PRAGMA foreign_keys`). Restricted to
/// trashed notes, so nothing short-circuits the reprieve.
pub fn purge(connection: &mut Library, ids: &[String]) -> Result<usize, StorageError> {
    if ids.is_empty() {
        return Ok(0);
    }

    Ok(diesel::delete(
        notes::table
            .filter(notes::id.eq_any(ids))
            .filter(notes::deleted_at.is_not_null()),
    )
    .execute(connection.db())?)
}
