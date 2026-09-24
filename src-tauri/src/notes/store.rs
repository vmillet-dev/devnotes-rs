pub mod related;
pub mod revisions;
pub mod trash;

use std::collections::{BTreeMap, HashMap, HashSet};

use diesel::prelude::*;
use uuid::Uuid;

use chrono::{DateTime, TimeDelta, Utc};

use super::model::{Note, NoteDraft, NoteLifecycle, NotePatch, NotePlacement, NoteTag, SampleNote};
use super::revision;
use super::view::{Decorations, Facets, NoteFilter, NotesQuery};
use crate::db::schema::{global_placeholders, note_tags, notes};
use crate::db::{Library, iso8601};
use crate::error::StorageError;
use crate::folders::store as folders;
use crate::spaces::model::Space;
use crate::spaces::store as spaces;
use crate::vault::key::Vault;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = notes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(super) struct NoteRow {
    id: String,
    space_id: String,
    title: String,
    language: String,
    content: String,
    source: String,
    pinned: bool,
    created_at: String,
    updated_at: String,
    lifecycle_kind: String,
    lifecycle_expires_at: Option<String>,
    kind: String,
    folder_id: Option<String>,
}

/// Not a `TryFrom`: opening a row needs the key. An unreadable date fails the read — only
/// [`iso8601::format`] writes these columns — while language and `kind` degrade, since a newer
/// version may write a value this build does not know. ⚠️ A value that will not open never
/// degrades: a wrong key must stop the read, not hand back plausible emptiness.
impl NoteRow {
    fn open(row: Self, vault: &Vault) -> Result<Note, StorageError> {
        let instant = |field: &'static str, value: &str| {
            iso8601::parse(value).map_err(|_| StorageError::CorruptRow {
                id: row.id.clone(),
                field,
            })
        };

        // The schema's `CHECK` makes `("expires", None)` unreachable.
        let lifecycle = match (row.lifecycle_kind.as_str(), &row.lifecycle_expires_at) {
            ("expires", Some(at)) => NoteLifecycle::Expires {
                at: instant("lifecycleExpiresAt", at)?,
            },
            _ => NoteLifecycle::Permanent,
        };

        Ok(Note {
            created_at: instant("createdAt", &row.created_at)?,
            updated_at: instant("updatedAt", &row.updated_at)?,
            language: row.language.parse().unwrap_or_default(),
            kind: row.kind.parse().unwrap_or_default(),
            title: vault.open(&row.title)?,
            content: vault.open(&row.content)?,
            source: vault.open(&row.source)?,
            id: row.id,
            space_id: row.space_id,
            folder_id: row.folder_id,
            tags: Vec::new(),
            items: Vec::new(),
            placeholder_values: BTreeMap::new(),
            pinned: row.pinned,
            lifecycle,
        })
    }

    /// What SQL filters, orders or groups on stays in the clear; what a reader would want is
    /// sealed.
    fn seal(note: &Note, vault: &Vault) -> Result<Self, StorageError> {
        let (lifecycle_kind, lifecycle_expires_at) = lifecycle_columns(&note.lifecycle);

        Ok(Self {
            id: note.id.clone(),
            space_id: note.space_id.clone(),
            title: vault.seal(&note.title)?,
            language: note.language.to_string(),
            content: vault.seal(&note.content)?,
            source: vault.seal(&note.source)?,
            pinned: note.pinned,
            created_at: iso8601::format(note.created_at),
            updated_at: iso8601::format(note.updated_at),
            lifecycle_kind: lifecycle_kind.to_string(),
            lifecycle_expires_at,
            kind: note.kind.to_string(),
            folder_id: note.folder_id.clone(),
        })
    }
}

fn lifecycle_columns(lifecycle: &NoteLifecycle) -> (&'static str, Option<String>) {
    match lifecycle {
        NoteLifecycle::Permanent => ("permanent", None),
        NoteLifecycle::Expires { at } => ("expires", Some(iso8601::format(*at))),
    }
}

/// What an update writes.
///
/// ⚠️ Every column is optional so an untouched one gets no assignment: writing all of them
/// back would make permanent the fallback an older build read for a value it does not know.
/// `updated_at` is not optional: refreshing it is what the patch path is for.
#[derive(AsChangeset)]
#[diesel(table_name = notes)]
struct NoteChanges {
    space_id: Option<String>,
    title: Option<String>,
    language: Option<String>,
    content: Option<String>,
    source: Option<String>,
    pinned: Option<bool>,
    updated_at: String,
    lifecycle_kind: Option<String>,
    /// Twice optional: the outer layer skips the column, the inner one is the `NULL` a
    /// permanent note needs written.
    #[allow(clippy::option_option)]
    lifecycle_expires_at: Option<Option<String>>,
    kind: Option<String>,
    /// Twice optional too, written only by a move between spaces: filing has its own command.
    #[allow(clippy::option_option)]
    folder_id: Option<Option<String>>,
}

impl NoteChanges {
    /// The columns `after` moved away from `before`. Only those are sealed.
    fn between(before: &Note, after: &Note, vault: &Vault) -> Result<Self, StorageError> {
        let sealed = |moved: bool, text: &str| moved.then(|| vault.seal(text)).transpose();
        let lifecycle_moved = after.lifecycle != before.lifecycle;
        let (lifecycle_kind, lifecycle_expires_at) = lifecycle_columns(&after.lifecycle);

        Ok(Self {
            space_id: (after.space_id != before.space_id).then(|| after.space_id.clone()),
            title: sealed(after.title != before.title, &after.title)?,
            language: (after.language != before.language).then(|| after.language.to_string()),
            content: sealed(after.content != before.content, &after.content)?,
            source: sealed(after.source != before.source, &after.source)?,
            pinned: (after.pinned != before.pinned).then_some(after.pinned),
            updated_at: iso8601::format(after.updated_at),
            lifecycle_kind: lifecycle_moved.then(|| lifecycle_kind.to_string()),
            lifecycle_expires_at: lifecycle_moved.then_some(lifecycle_expires_at),
            kind: (after.kind != before.kind).then(|| after.kind.to_string()),
            folder_id: (after.folder_id != before.folder_id).then(|| after.folder_id.clone()),
        })
    }
}

/// Every row or none: a value that will not open stops the read.
fn open_all(rows: Vec<NoteRow>, vault: &Vault) -> Result<Vec<Note>, StorageError> {
    rows.into_iter()
        .map(|row| NoteRow::open(row, vault))
        .collect()
}

/// Opens the rows and attaches their side tables.
fn opened(
    connection: &mut Library,
    rows: Vec<NoteRow>,
    space_id: Option<&str>,
) -> Result<Vec<Note>, StorageError> {
    let (db, vault) = connection.split();
    let mut notes = open_all(rows, vault)?;
    related::attach_related(db, vault, &mut notes, space_id)?;

    Ok(notes)
}

pub(super) fn notes_of_space(
    space_id: &str,
) -> diesel::helper_types::Filter<
    diesel::helper_types::Select<notes::table, notes::id>,
    diesel::dsl::Eq<notes::space_id, String>,
> {
    notes::table
        .select(notes::id)
        .filter(notes::space_id.eq(space_id.to_string()))
}

/// The value is sealed, the name is not: rows are found by it, and a variable called `host`
/// says little without what it holds.
pub fn global_placeholder_values(
    connection: &mut Library,
) -> Result<BTreeMap<String, String>, StorageError> {
    let (db, vault) = connection.split();

    global_placeholders::table
        .select((global_placeholders::name, global_placeholders::value))
        .order(global_placeholders::name.asc())
        .load::<(String, String)>(db)?
        .into_iter()
        .map(|(name, value)| Ok((name, vault.open(&value)?)))
        .collect::<Result<BTreeMap<_, _>, StorageError>>()
}

pub fn replace_global_placeholder_values(
    connection: &mut Library,
    values: &BTreeMap<String, String>,
) -> Result<(), StorageError> {
    connection.transaction(|connection, vault| {
        diesel::delete(global_placeholders::table).execute(connection)?;

        if !values.is_empty() {
            let rows: Vec<_> = values
                .iter()
                .map(|(name, value)| {
                    Ok((
                        global_placeholders::name.eq(name),
                        global_placeholders::value.eq(vault.seal(value)?),
                    ))
                })
                .collect::<Result<Vec<_>, StorageError>>()?;
            diesel::insert_into(global_placeholders::table)
                .values(rows)
                .execute(connection)?;
        }

        Ok(())
    })
}

/// Scoped to the space, not to the current filter: see `NotesView`.
fn facets(connection: &mut Library, space_id: Option<&str>) -> Result<Facets, StorageError> {
    let mut tags = note_tags::table
        .inner_join(notes::table)
        .filter(notes::deleted_at.is_null())
        .select(note_tags::tag)
        .distinct()
        .order(note_tags::tag.asc())
        .into_boxed();
    let mut languages = notes::table
        .filter(notes::deleted_at.is_null())
        .select(notes::language)
        .distinct()
        .order(notes::language.asc())
        .into_boxed();

    if let Some(id) = space_id {
        tags = tags.filter(notes::space_id.eq(id.to_string()));
        languages = languages.filter(notes::space_id.eq(id.to_string()));
    }

    Ok(Facets {
        tags: tags.load::<String>(connection.db())?,
        // A stored language this build does not know has no facet to offer.
        languages: languages
            .load::<String>(connection.db())?
            .iter()
            .filter_map(|language| language.parse().ok())
            .collect(),
    })
}

/// Coarse criteria only; `view::build` takes over for search and sections.
pub fn fetch(
    connection: &mut Library,
    request: &NotesQuery,
) -> Result<(Vec<Note>, Facets), StorageError> {
    let mut query = notes::table
        .filter(notes::deleted_at.is_null())
        .select(NoteRow::as_select())
        .into_boxed();

    if let Some(space_id) = &request.space_id {
        query = query.filter(notes::space_id.eq(space_id.clone()));
    }

    if let Some(folder_id) = &request.folder_id {
        query = query.filter(notes::folder_id.eq(folder_id.clone()));
    }

    match request.filter {
        NoteFilter::All => {}
        NoteFilter::Pinned => query = query.filter(notes::pinned.eq(true)),
        NoteFilter::Untriaged => query = query.filter(notes::lifecycle_kind.eq("expires")),
    }

    if !request.languages.is_empty() {
        let selected: Vec<String> = request.languages.iter().map(ToString::to_string).collect();
        query = query.filter(notes::language.eq_any(selected));
    }

    let selected_tags = request.selected_tags();
    if !selected_tags.is_empty() {
        query = query.filter(
            notes::id.eq_any(
                note_tags::table
                    .select(note_tags::note_id)
                    .filter(note_tags::tag.eq_any(selected_tags)),
            ),
        );
    }

    // `updated_at` although the sections group on `created_at`: the section says when a note
    // was born, the order within it which one moved last.
    let rows = query
        .order((notes::updated_at.desc(), notes::id.asc()))
        .load::<NoteRow>(connection.db())?;
    let notes = opened(connection, rows, request.space_id.as_deref())?;

    Ok((notes, facets(connection, request.space_id.as_deref())?))
}

/// One note, whole: what the editor opens on.
pub fn get(connection: &mut Library, id: &str) -> Result<Note, StorageError> {
    let (connection, vault) = connection.split();

    find(connection, vault, id)?.ok_or_else(|| StorageError::NoteNotFound(id.to_string()))
}

fn find(
    connection: &mut SqliteConnection,
    vault: &Vault,
    id: &str,
) -> Result<Option<Note>, StorageError> {
    let Some(row) = notes::table
        .find(id)
        .filter(notes::deleted_at.is_null())
        .select(NoteRow::as_select())
        .first::<NoteRow>(connection)
        .optional()?
    else {
        return Ok(None);
    };

    let tags = related::tags_of(connection, id)?;
    let items = related::items_of(connection, vault, id)?;
    let placeholder_values = related::placeholder_values_of(connection, vault, id)?;

    Ok(Some(Note {
        tags,
        items,
        placeholder_values,
        ..NoteRow::open(row, vault)?
    }))
}

pub fn create(
    connection: &mut Library,
    draft: NoteDraft,
    now: DateTime<Utc>,
) -> Result<Note, StorageError> {
    connection.transaction(|connection, vault| create_in(connection, vault, draft, now))
}

/// For a caller already inside a transaction: the first launch writes a space and its notes
/// as one.
pub(crate) fn create_in(
    connection: &mut SqliteConnection,
    vault: &Vault,
    draft: NoteDraft,
    now: DateTime<Utc>,
) -> Result<Note, StorageError> {
    if !spaces::exists(connection, &draft.space_id)? {
        return Err(StorageError::SpaceNotFound(draft.space_id));
    }

    let mut note = draft.into_note(Uuid::new_v4().to_string(), now);

    diesel::insert_into(notes::table)
        .values(NoteRow::seal(&note, vault)?)
        .execute(connection)?;
    let written = std::mem::take(&mut note.tags);
    note.tags = related::replace_tags(connection, &note.id, &written)?;
    related::replace_items(connection, vault, &note.id, &note.items)?;

    Ok(note)
}

/// The first launch, as one write.
///
/// One transaction for the space, the folders and the notes: a space standing without them
/// reads as "already seeded" to both of the front end's guards, for ever. A draft's own
/// `space_id` is replaced — the space does not exist when the front end composes them.
pub fn seed(
    connection: &mut Library,
    space_name: &str,
    folder_names: &[String],
    notes: Vec<SampleNote>,
    now: DateTime<Utc>,
) -> Result<Space, StorageError> {
    connection.transaction(|connection, vault| {
        let space = spaces::create_in(connection, vault, space_name)?;

        // A millisecond apart, forward: `folders::list` orders `created_at` ascending and
        // breaks ties on a random UUID.
        let mut folder_ids: Vec<String> = Vec::with_capacity(folder_names.len());
        for (index, name) in folder_names.iter().enumerate() {
            let at = now + TimeDelta::milliseconds(i64::try_from(index).unwrap_or(0));
            folder_ids.push(folders::create_in(connection, vault, &space.id, name, at)?.id);
        }

        // And backward for the notes: the canvas orders `updated_at` descending.
        for (index, note) in notes.into_iter().enumerate() {
            let at = now - TimeDelta::milliseconds(i64::try_from(index).unwrap_or(0));
            let folder_id = note
                .folder
                .and_then(|index| folder_ids.get(usize::try_from(index).unwrap_or(usize::MAX)))
                .cloned();

            create_in(
                connection,
                vault,
                NoteDraft {
                    space_id: space.id.clone(),
                    folder_id,
                    ..note.draft
                },
                at,
            )?;
        }

        Ok(space)
    })
}

pub fn update(
    connection: &mut Library,
    id: &str,
    patch: &NotePatch,
    now: DateTime<Utc>,
) -> Result<Note, StorageError> {
    connection.transaction(|connection, vault| {
        let Some(mut note) = find(connection, vault, id)? else {
            return Err(StorageError::NoteNotFound(id.to_string()));
        };

        if let Some(space_id) = &patch.space_id
            && !spaces::exists(connection, space_id)?
        {
            return Err(StorageError::SpaceNotFound(space_id.clone()));
        }

        // Compared against the note as read, not the patch: `apply` moves more than it is
        // handed — a new body re-detects the language.
        let before = note.clone();
        patch.apply(&mut note, now);

        // Before the update, in the same transaction: what is kept is the body as it was.
        if revision::worth_keeping(&before, &note) {
            revisions::record(connection, vault, &note.id, &before.content, now)?;
        }

        diesel::update(notes::table.find(&note.id))
            .set(NoteChanges::between(&before, &note, vault)?)
            .execute(connection)?;

        if patch.tags.is_some() {
            let written = std::mem::take(&mut note.tags);
            note.tags = related::replace_tags(connection, &note.id, &written)?;
        }

        if patch.items.is_some() {
            related::replace_items(connection, vault, &note.id, &note.items)?;
        }

        Ok(note)
    })
}

/// Goes back to a kept body, which leaves the history with every body kept after it. Nothing
/// is kept of the text it replaces: the preview is the guard, not an undo. Leaves
/// `updated_at` alone.
pub fn restore_revision(
    connection: &mut Library,
    id: &str,
    revision_id: &str,
) -> Result<Note, StorageError> {
    connection.transaction(|connection, vault| {
        let Some(mut note) = find(connection, vault, id)? else {
            return Err(StorageError::NoteNotFound(id.to_string()));
        };

        let Some(content) = revisions::content_of(connection, vault, id, revision_id)? else {
            return Err(StorageError::RevisionNotFound(revision_id.to_string()));
        };

        revisions::discard_from(connection, id, revision_id)?;

        diesel::update(notes::table.find(id))
            .set(notes::content.eq(vault.seal(&content)?))
            .execute(connection)?;

        note.content = content;

        Ok(note)
    })
}

/// Leaves `updated_at` alone: filling a field is not editing the note.
pub fn set_placeholder_values(
    connection: &mut Library,
    id: &str,
    values: &BTreeMap<String, String>,
) -> Result<Note, StorageError> {
    connection.transaction(|connection, vault| {
        let Some(mut note) = find(connection, vault, id)? else {
            return Err(StorageError::NoteNotFound(id.to_string()));
        };

        related::replace_placeholder_values(connection, vault, id, values)?;
        note.placeholder_values = values.clone();

        Ok(note)
    })
}

/// Answers where each note came from, which is what putting the move back needs.
pub fn move_many(
    connection: &mut Library,
    ids: &[String],
    space_id: &str,
    now: DateTime<Utc>,
) -> Result<Vec<NotePlacement>, StorageError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    connection.transaction(|connection, _vault| {
        if !spaces::exists(connection, space_id)? {
            return Err(StorageError::SpaceNotFound(space_id.to_string()));
        }

        // Read before the update, which erases where each note came from.
        let moved: Vec<NotePlacement> = notes::table
            .filter(notes::id.eq_any(ids))
            .filter(notes::deleted_at.is_null())
            .filter(notes::space_id.ne(space_id))
            .select((notes::id, notes::space_id))
            .load::<(String, String)>(connection)?
            .into_iter()
            .map(|(note_id, space_id)| NotePlacement { note_id, space_id })
            .collect();

        let touched: Vec<&String> = moved.iter().map(|placement| &placement.note_id).collect();
        diesel::update(notes::table.filter(notes::id.eq_any(touched)))
            .set((
                notes::space_id.eq(space_id),
                // The folder stays behind with its space, as it does through a patch.
                notes::folder_id.eq(None::<String>),
                notes::updated_at.eq(iso8601::format(now)),
            ))
            .execute(connection)?;

        Ok(moved)
    })
}

/// Puts moved notes back where they were, leaving `updated_at` alone: undoing is not editing.
pub fn restore_placements(
    connection: &mut Library,
    placements: &[NotePlacement],
) -> Result<usize, StorageError> {
    if placements.is_empty() {
        return Ok(0);
    }

    connection.transaction(|connection, _vault| {
        let mut by_space: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for placement in placements {
            by_space
                .entry(placement.space_id.as_str())
                .or_default()
                .push(placement.note_id.as_str());
        }

        let mut restored = 0;
        for (space_id, ids) in by_space {
            // A space dropped since the move is not an error: the rest still goes back.
            if !spaces::exists(connection, space_id)? {
                continue;
            }

            restored += diesel::update(
                notes::table
                    .filter(notes::id.eq_any(&ids))
                    .filter(notes::deleted_at.is_null()),
            )
            .set(notes::space_id.eq(space_id))
            .execute(connection)?;
        }

        Ok(restored)
    })
}

/// Answers the pairs it actually added, not how many notes it looked at.
pub fn tag_many(
    connection: &mut Library,
    ids: &[String],
    tags: &[String],
    now: DateTime<Utc>,
) -> Result<Vec<NoteTag>, StorageError> {
    if ids.is_empty() || tags.is_empty() {
        return Ok(Vec::new());
    }

    connection.transaction(|connection, _vault| {
        let targets = notes::table
            .filter(notes::id.eq_any(ids))
            .filter(notes::deleted_at.is_null())
            .select(notes::id)
            .load::<String>(connection)?;

        // ⚠️ What the notes already carry, read before inserting: the undo strips exactly what
        // was added. `note_tags.tag` is `NOCASE`, which SQLite folds over ASCII only — the
        // comparison `to_ascii_lowercase` makes below.
        let carried: HashSet<(String, String)> = note_tags::table
            .filter(note_tags::note_id.eq_any(&targets))
            .filter(note_tags::tag.eq_any(tags))
            .select((note_tags::note_id, note_tags::tag))
            .load::<(String, String)>(connection)?
            .into_iter()
            .map(|(id, name)| (id, name.to_ascii_lowercase()))
            .collect();

        let mut added = Vec::new();
        for note_id in &targets {
            for tag in tags {
                let held = carried.contains(&(note_id.clone(), tag.to_ascii_lowercase()));
                if !held {
                    added.push(NoteTag {
                        note_id: note_id.clone(),
                        tag: tag.clone(),
                    });
                }
            }
        }

        if added.is_empty() {
            return Ok(added);
        }

        let rows: Vec<_> = added
            .iter()
            .map(|pair| {
                (
                    note_tags::note_id.eq(&pair.note_id),
                    note_tags::tag.eq(&pair.tag),
                )
            })
            .collect();

        diesel::insert_or_ignore_into(note_tags::table)
            .values(rows)
            .execute(connection)?;

        // Only what gained something moves up the canvas.
        let touched: Vec<&String> = added.iter().map(|pair| &pair.note_id).collect();
        diesel::update(notes::table.filter(notes::id.eq_any(touched)))
            .set(notes::updated_at.eq(iso8601::format(now)))
            .execute(connection)?;

        Ok(added)
    })
}

/// Removes exactly these pairs, the undo of [`tag_many`], leaving `updated_at` alone.
pub fn untag_many(connection: &mut Library, pairs: &[NoteTag]) -> Result<usize, StorageError> {
    if pairs.is_empty() {
        return Ok(0);
    }

    let mut by_tag: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for pair in pairs {
        by_tag.entry(&pair.tag).or_default().push(&pair.note_id);
    }

    connection.transaction(|connection, _vault| {
        let mut removed = 0;
        for (tag, note_ids) in by_tag {
            removed += diesel::delete(
                note_tags::table
                    .filter(note_tags::tag.eq(tag))
                    .filter(note_tags::note_id.eq_any(note_ids)),
            )
            .execute(connection)?;
        }

        Ok(removed)
    })
}

/// How many notes — the trash included — carry at least one of these tags.
///
/// Counted distinctly, not summed per tag: a note carrying two of them is one note.
pub fn count_notes_tagged(
    connection: &mut Library,
    tags: &[String],
) -> Result<usize, StorageError> {
    if tags.is_empty() {
        return Ok(0);
    }

    carrying(connection.db(), tags)
}

/// The counters and the global values. The folder chips depend on the view: the caller's.
pub fn decorations(connection: &mut Library) -> Result<Decorations, StorageError> {
    Ok(Decorations {
        attachment_counts: crate::attachments::store::counts(connection)?,
        folders: HashMap::new(),
        globals: global_placeholder_values(connection)?,
    })
}

pub fn tag_usage(connection: &mut Library) -> Result<Vec<(String, i64)>, StorageError> {
    Ok(note_tags::table
        .inner_join(notes::table)
        .filter(notes::deleted_at.is_null())
        .group_by(note_tags::tag)
        .select((note_tags::tag, diesel::dsl::count_star()))
        .order(note_tags::tag.asc())
        .load::<(String, i64)>(connection.db())?)
}

/// Leaves `updated_at` alone: a corpus-wide rename would float up notes nobody reopened.
pub fn retag(
    connection: &mut Library,
    sources: &[String],
    target: &str,
) -> Result<usize, StorageError> {
    if sources.is_empty() {
        return Ok(0);
    }

    connection.transaction(|connection, _vault| {
        let renamed = note_tags::table
            .filter(note_tags::tag.eq_any(sources))
            .select(note_tags::note_id)
            .distinct()
            .load::<String>(connection)?;

        // ⚠️ The target is swept with the sources, then rewritten: the key is `NOCASE`, so a
        // pure case correction (`auth` → `Auth`) would otherwise be a no-op.
        let mut holders = renamed.clone();
        holders.extend(
            note_tags::table
                .filter(note_tags::tag.eq(target))
                .select(note_tags::note_id)
                .load::<String>(connection)?,
        );
        holders.sort();
        holders.dedup();

        diesel::delete(
            note_tags::table.filter(note_tags::tag.eq_any(sources).or(note_tags::tag.eq(target))),
        )
        .execute(connection)?;

        let rows: Vec<_> = holders
            .iter()
            .map(|note_id| (note_tags::note_id.eq(note_id), note_tags::tag.eq(target)))
            .collect();
        diesel::insert_into(note_tags::table)
            .values(rows)
            .execute(connection)?;

        Ok(renamed.len())
    })
}

/// Answers the number of notes, like the confirmation before it, not the number of rows.
pub fn drop_tags(connection: &mut Library, tags: &[String]) -> Result<usize, StorageError> {
    if tags.is_empty() {
        return Ok(0);
    }

    connection.transaction(|connection, _vault| {
        let holders = carrying(connection, tags)?;
        diesel::delete(note_tags::table.filter(note_tags::tag.eq_any(tags))).execute(connection)?;

        Ok(holders)
    })
}

/// Trashed notes included: a note restored after a rename comes back under the rail's name.
fn carrying(connection: &mut SqliteConnection, tags: &[String]) -> Result<usize, StorageError> {
    let counted: i64 = note_tags::table
        .filter(note_tags::tag.eq_any(tags))
        .select(diesel::dsl::count(note_tags::note_id).aggregate_distinct())
        .first(connection)?;

    Ok(usize::try_from(counted).unwrap_or(0))
}

/// Export only: no command hands this list to the front, which would re-filter it.
pub fn all(connection: &mut Library, space_id: Option<&str>) -> Result<Vec<Note>, StorageError> {
    let mut query = notes::table
        .filter(notes::deleted_at.is_null())
        .select(NoteRow::as_select())
        .into_boxed();

    if let Some(space_id) = space_id {
        query = query.filter(notes::space_id.eq(space_id.to_string()));
    }

    let rows = query
        .order((notes::created_at.asc(), notes::id.asc()))
        .load::<NoteRow>(connection.db())?;
    opened(connection, rows, space_id)
}

pub fn by_ids(connection: &mut Library, ids: &[String]) -> Result<Vec<Note>, StorageError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let rows = notes::table
        .filter(notes::id.eq_any(ids))
        .filter(notes::deleted_at.is_null())
        .select(NoteRow::as_select())
        .order((notes::created_at.asc(), notes::id.asc()))
        .load::<NoteRow>(connection.db())?;
    opened(connection, rows, None)
}

pub fn insert_imported(connection: &mut Library, note: &Note) -> Result<bool, StorageError> {
    connection.transaction(|connection, vault| insert_imported_in(connection, vault, note))
}

/// For a caller already inside a transaction: an import is one transaction for the whole
/// file. Written as given; [`Note::normalized`] is the caller's.
pub(crate) fn insert_imported_in(
    connection: &mut SqliteConnection,
    vault: &Vault,
    note: &Note,
) -> Result<bool, StorageError> {
    let taken = notes::table
        .find(&note.id)
        .select(notes::id)
        .first::<String>(connection)
        .optional()?
        .is_some();
    if taken {
        return Ok(false);
    }

    diesel::insert_into(notes::table)
        .values(NoteRow::seal(note, vault)?)
        .execute(connection)?;
    related::replace_tags(connection, &note.id, &note.tags)?;
    related::replace_items(connection, vault, &note.id, &note.items)?;
    related::replace_placeholder_values(connection, vault, &note.id, &note.placeholder_values)?;

    Ok(true)
}
