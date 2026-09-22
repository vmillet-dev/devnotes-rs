//! The bodies kept beside a note. The retention itself lives in `notes::revision` —
//! this module only knows how to write one, read one, and prune the rest.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db::iso8601;
use crate::db::schema::note_revisions;
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

/// The kept bodies of one note, newest first.
///
/// ⚠️ Ordered by `rowid`, not by `taken_at`. The editor commits the title, the source
/// and the body back to back, so two revisions can share a millisecond — and the
/// tiebreak was a random UUID, which put the older one first about half the time.
/// Insertion order is what "newest" means here, and `rowid` *is* insertion order.
fn rows(
    connection: &mut SqliteConnection,
    note_id: &str,
) -> Result<Vec<RevisionRow>, StorageError> {
    Ok(note_revisions::table
        .filter(note_revisions::note_id.eq(note_id))
        .order(diesel::dsl::sql::<diesel::sql_types::Bool>("rowid DESC"))
        .select(RevisionRow::as_select())
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
        // ⚠️ Narrowed to the note as well as the id: an id comes from the front end, and
        // one revision must not be reachable through another note's history.
        .filter(note_revisions::note_id.eq(note_id))
        .select(RevisionRow::as_select())
        .first(connection)
        .optional()?
    else {
        return Ok(None);
    };

    Ok(Some(vault.open(&row.content)?))
}

/// Keeps `content` beside the note, unless the newest kept body already is it.
///
/// ⚠️ It is the body **before** the edit that is worth keeping: the version that worked.
/// Callers pass what the row holds, not what the patch carries.
///
/// ⚠️ Skipped when the newest revision already holds this exact text. The editor commits
/// on blur *and* on every closing path, so an editing session produces several writes of
/// the same body — without this, every one of them would push a duplicate and rotate a
/// genuinely different version out of the cap.
pub fn record(
    connection: &mut SqliteConnection,
    vault: &Vault,
    note_id: &str,
    content: &str,
    now: DateTime<Utc>,
) -> Result<(), StorageError> {
    let kept = rows(connection, note_id)?;
    if let Some(newest) = kept.first()
        && vault.open(&newest.content)? == content
    {
        return Ok(());
    }

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

/// ⚠️ The **oldest** go. `kept` is what was there before the insert, newest first, so
/// everything from `KEEP - 1` on is what the new one pushes past the cap.
fn prune(
    connection: &mut SqliteConnection,
    note_id: &str,
    kept: &[RevisionRow],
) -> Result<(), StorageError> {
    let stale: Vec<&String> = kept.iter().skip(KEEP - 1).map(|row| &row.id).collect();
    if stale.is_empty() {
        return Ok(());
    }

    diesel::delete(
        note_revisions::table
            .filter(note_revisions::note_id.eq(note_id))
            .filter(note_revisions::id.eq_any(stale)),
    )
    .execute(connection)?;

    Ok(())
}
