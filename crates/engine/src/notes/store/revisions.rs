//! The bodies kept beside a note: write one, read one, prune the rest. The retention rules
//! live in `notes::revision`.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db::iso8601;
use crate::db::schema::{note_revisions, notes};
use crate::error::StorageError;
use crate::notes::revision::{KEEP, Revision, describe};
use crate::vault::key::Vault;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = note_revisions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct RevisionRow {
    id: String,
    note_id: String,
    /// Sealed, like the column it copies.
    content: String,
    taken_at: String,
}

/// ⚠️ Newest first by `rowid`, not `taken_at`: two revisions can share a millisecond, and
/// `rowid` is insertion order.
fn newest_first() -> diesel::expression::SqlLiteral<diesel::sql_types::Bool> {
    diesel::dsl::sql::<diesel::sql_types::Bool>("rowid DESC")
}

/// The kept bodies of one note, newest first, sealed.
fn rows(
    connection: &mut SqliteConnection,
    note_id: &str,
) -> Result<Vec<RevisionRow>, StorageError> {
    Ok(note_revisions::table
        .filter(note_revisions::note_id.eq(note_id))
        .order(newest_first())
        .select(RevisionRow::as_select())
        .load(connection)?)
}

/// Their ids alone, in the same order: what the rotation needs, without twenty bodies.
fn ids(connection: &mut SqliteConnection, note_id: &str) -> Result<Vec<String>, StorageError> {
    Ok(note_revisions::table
        .filter(note_revisions::note_id.eq(note_id))
        .order(newest_first())
        .select(note_revisions::id)
        .load(connection)?)
}

/// What the panel draws: instants and sizes, never the bodies.
pub fn list(
    connection: &mut SqliteConnection,
    vault: &Vault,
    note_id: &str,
) -> Result<Vec<Revision>, StorageError> {
    rows(connection, note_id)?
        .into_iter()
        .map(|row| {
            let content = vault.open(&row.content)?;
            let taken_at = iso8601::parse(&row.taken_at).map_err(|_| StorageError::CorruptRow {
                id: row.id.clone(),
                field: "takenAt",
            })?;

            Ok(describe(row.id, taken_at, &content))
        })
        .collect()
}

/// One kept body, opened. `None` when the id names nothing of this note's.
pub fn content_of(
    connection: &mut SqliteConnection,
    vault: &Vault,
    note_id: &str,
    revision_id: &str,
) -> Result<Option<String>, StorageError> {
    let Some(row) = note_revisions::table
        .filter(note_revisions::id.eq(revision_id))
        // Narrowed to the note as well: the id comes from the front end, and one note's
        // history must not reach another's revision.
        .filter(note_revisions::note_id.eq(note_id))
        .select(RevisionRow::as_select())
        .first(connection)
        .optional()?
    else {
        return Ok(None);
    };

    Ok(Some(vault.open(&row.content)?))
}

/// The note's current body and one of its kept ones, opened, for the preview to compare.
/// `None` when either is missing, or the revision is another note's.
pub fn compare(
    connection: &mut SqliteConnection,
    vault: &Vault,
    note_id: &str,
    revision_id: &str,
) -> Result<Option<(String, String)>, StorageError> {
    let Some(current) = notes::table
        .find(note_id)
        .filter(notes::deleted_at.is_null())
        .select(notes::content)
        .first::<String>(connection)
        .optional()?
    else {
        return Ok(None);
    };
    let Some(version) = content_of(connection, vault, note_id, revision_id)? else {
        return Ok(None);
    };

    Ok(Some((vault.open(&current)?, version)))
}

/// Forgets `revision_id` and every body kept after it, in insertion order.
pub fn discard_from(
    connection: &mut SqliteConnection,
    note_id: &str,
    revision_id: &str,
) -> Result<(), StorageError> {
    let kept = ids(connection, note_id)?;
    let Some(at) = kept.iter().position(|id| id == revision_id) else {
        return Ok(());
    };
    let newer = &kept[..=at];

    diesel::delete(
        note_revisions::table
            .filter(note_revisions::note_id.eq(note_id))
            .filter(note_revisions::id.eq_any(newer)),
    )
    .execute(connection)?;

    Ok(())
}

/// Keeps `content` — the body as it was before the edit — unless the newest kept body already
/// is it: the editor commits the same body on blur and on every closing path, and each copy
/// would rotate a different version out of the cap.
pub fn record(
    connection: &mut SqliteConnection,
    vault: &Vault,
    note_id: &str,
    content: &str,
    now: DateTime<Utc>,
) -> Result<(), StorageError> {
    let newest: Option<String> = note_revisions::table
        .filter(note_revisions::note_id.eq(note_id))
        .order(newest_first())
        .select(note_revisions::content)
        .first(connection)
        .optional()?;
    if let Some(newest) = newest
        && vault.open(&newest)? == content
    {
        return Ok(());
    }
    let kept = ids(connection, note_id)?;

    diesel::insert_into(note_revisions::table)
        .values(RevisionRow {
            id: uuid::Uuid::new_v4().to_string(),
            note_id: note_id.to_string(),
            content: vault.seal(content)?,
            taken_at: iso8601::format(now),
        })
        .execute(connection)?;

    prune(connection, note_id, &kept)?;

    Ok(())
}

/// The oldest go: `kept` is newest first, so everything from `KEEP - 1` on is past the cap.
fn prune(
    connection: &mut SqliteConnection,
    note_id: &str,
    kept: &[String],
) -> Result<(), StorageError> {
    let Some(stale) = kept.get(KEEP - 1..) else {
        return Ok(());
    };

    diesel::delete(
        note_revisions::table
            .filter(note_revisions::note_id.eq(note_id))
            .filter(note_revisions::id.eq_any(stale)),
    )
    .execute(connection)?;

    Ok(())
}
