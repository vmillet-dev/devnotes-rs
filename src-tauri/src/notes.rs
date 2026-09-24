#![allow(clippy::needless_pass_by_value)]

pub mod checklist;
pub mod language;
pub mod model;
pub mod placeholder;
pub mod revision;
pub mod store;
pub mod trash;
pub mod view;

use std::collections::BTreeMap;

use chrono::Utc;
use tauri::State;

use crate::attachments;
use crate::count::saturating_u32 as count;
use crate::db::{Db, Library, lock};
use crate::error::{AppError, StorageError};
use crate::folders;
use crate::spaces::model::SpaceDraft;
use model::{DisplayNote, NoteDraft, NotePatch, SampleNote, TagUsage};
use revision::{DiffLine, Revision};
use trash::TrashedNote;
use view::{NotesQuery, NotesView};

/// ⚠️ No command returns the raw list: it would invite re-filtering on the front end.
#[tauri::command(async)]
#[specta::specta]
pub fn query_notes(query: NotesQuery, db: State<'_, Db>) -> Result<NotesView, AppError> {
    let mut connection = lock(&db)?;
    let (notes, facets) = store::fetch(&mut connection, &query)?;
    let counts = attachments::store::counts(&mut connection)?;

    let globals = store::global_placeholder_values(&mut connection)?;

    let mut view = view::build(notes, facets, &query);
    view::apply_attachment_counts(&mut view, &counts);

    // ⚠️ Not inside an opened folder: a chip naming the folder every card is already in is
    // noise, and the breadcrumb above says it once. Same reason the board resolves none.
    if query.folder_id.is_none() {
        let folders = folders::store::by_id(&mut connection, query.space_id.as_deref())?;
        view::apply_folders(&mut view, &folders);
    }

    view::apply_global_defaults(&mut view, &globals);

    Ok(view)
}

#[tauri::command(async)]
#[specta::specta]
pub fn create_note(draft: NoteDraft, db: State<'_, Db>) -> Result<DisplayNote, AppError> {
    let mut connection = lock(&db)?;
    let note = store::create(&mut connection, draft, Utc::now())?;

    Ok(display(&mut connection, note)?)
}

/// The first launch, and the only command that writes a space and notes at once.
///
/// ⚠️ One transaction, because six round trips were six chances to be killed halfway:
/// a space with nothing in it reads as "already seeded" to both of the front end's
/// guards, and the canvas stays empty for the life of that install. The strings stay on
/// the front end, where the translations are — only the atomicity comes from here.
///
/// Answers the space it made: with exactly one, "all spaces" is a distinction without a
/// difference, and the front end opens on it rather than on a board it cannot show.
#[tauri::command(async)]
#[specta::specta]
pub fn seed_samples(
    space_name: String,
    folders: Vec<String>,
    notes: Vec<SampleNote>,
    db: State<'_, Db>,
) -> Result<crate::spaces::model::Space, AppError> {
    let name = SpaceDraft { name: space_name }.validated_name()?;
    let folder_names = folders
        .iter()
        .map(|folder| crate::folders::model::validated_name(folder))
        .collect::<Result<Vec<_>, _>>()?;

    let mut connection = lock(&db)?;

    Ok(store::seed(
        &mut connection,
        &name,
        &folder_names,
        notes,
        Utc::now(),
    )?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn update_note(
    id: String,
    patch: NotePatch,
    db: State<'_, Db>,
) -> Result<DisplayNote, AppError> {
    let mut connection = lock(&db)?;
    let note = store::update(&mut connection, &id, &patch, Utc::now())?;

    Ok(display(&mut connection, note)?)
}

/// The bodies kept beside a note, newest first.
///
/// ⚠️ Metadata only: instants and sizes, never the bodies themselves. Twenty of them is
/// what makes this table big, and a list that carried them would send the whole history
/// across to draw twenty dates.
#[tauri::command(async)]
#[specta::specta]
pub fn list_revisions(id: String, db: State<'_, Db>) -> Result<Vec<Revision>, AppError> {
    let mut connection = lock(&db)?;

    let (connection, vault) = connection.split();

    Ok(store::revisions::list(connection, vault, &id)?)
}

/// What going back to a kept body would change, line by line, against the current text.
#[tauri::command(async)]
#[specta::specta]
pub fn revision_diff(
    id: String,
    revision_id: String,
    db: State<'_, Db>,
) -> Result<Vec<DiffLine>, AppError> {
    let mut connection = lock(&db)?;
    let (connection, vault) = connection.split();

    let Some((current, version)) = store::revisions::compare(connection, vault, &id, &revision_id)?
    else {
        return Err(StorageError::RevisionNotFound(revision_id).into());
    };

    Ok(revision::diff(&current, &version))
}

/// Goes back to a kept body, dropping it and every body kept after it from the history.
///
/// ⚠️ `updated_at` is **not** touched. Putting something back is not editing it — the
/// same line `restore_notes`, `move_notes_back`, `untag_notes` and
/// `set_placeholder_values` already hold — and the canvas sorts on that column.
#[tauri::command(async)]
#[specta::specta]
pub fn restore_revision(
    id: String,
    revision_id: String,
    db: State<'_, Db>,
) -> Result<DisplayNote, AppError> {
    let mut connection = lock(&db)?;
    let note = store::restore_revision(&mut connection, &id, &revision_id)?;

    Ok(display(&mut connection, note)?)
}

/// Moves to the trash: the note comes back through [`restore_notes`] for
/// [`trash::RETENTION`].
#[tauri::command(async)]
#[specta::specta]
pub fn delete_note(id: String, db: State<'_, Db>) -> Result<(), AppError> {
    let mut connection = lock(&db)?;

    Ok(store::trash::trash(&mut connection, &id, Utc::now())?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn delete_notes(ids: Vec<String>, db: State<'_, Db>) -> Result<u32, AppError> {
    let mut connection = lock(&db)?;

    Ok(count(store::trash::trash_many(
        &mut connection,
        &ids,
        Utc::now(),
    )?))
}

#[tauri::command(async)]
#[specta::specta]
pub fn restore_notes(ids: Vec<String>, db: State<'_, Db>) -> Result<u32, AppError> {
    let mut connection = lock(&db)?;

    Ok(count(store::trash::restore_many(&mut connection, &ids)?))
}

/// Purges first: the trash must never show a note a restart would erase.
#[tauri::command(async)]
#[specta::specta]
pub fn list_trash(db: State<'_, Db>) -> Result<Vec<TrashedNote>, AppError> {
    trash::purge_expired(&db)?;

    let mut connection = lock(&db)?;

    Ok(store::trash::list_trashed(&mut connection)?
        .into_iter()
        .map(|(note, deleted_at)| trash::trashed(note, deleted_at))
        .collect())
}

#[tauri::command(async)]
#[specta::specta]
pub fn purge_notes(ids: Vec<String>, db: State<'_, Db>) -> Result<u32, AppError> {
    Ok(count(trash::purge(&db, ids)?))
}

#[tauri::command(async)]
#[specta::specta]
pub fn empty_trash(db: State<'_, Db>) -> Result<u32, AppError> {
    let ids = {
        let mut connection = lock(&db)?;
        store::trash::trashed_ids(&mut connection)?
    };

    Ok(count(trash::purge(&db, ids)?))
}

#[tauri::command(async)]
#[specta::specta]
pub fn move_notes(
    ids: Vec<String>,
    space_id: String,
    db: State<'_, Db>,
) -> Result<Vec<model::NotePlacement>, AppError> {
    let mut connection = lock(&db)?;

    Ok(store::move_many(
        &mut connection,
        &ids,
        &space_id,
        Utc::now(),
    )?)
}

/// The undo of [`move_notes`]: each note goes back to the space it left.
#[tauri::command(async)]
#[specta::specta]
pub fn move_notes_back(
    placements: Vec<model::NotePlacement>,
    db: State<'_, Db>,
) -> Result<u32, AppError> {
    let mut connection = lock(&db)?;

    Ok(count(store::restore_placements(
        &mut connection,
        &placements,
    )?))
}

/// Normalized here as everywhere else, or a typed `#urgent` would not join `urgent`.
#[tauri::command(async)]
#[specta::specta]
pub fn tag_notes(
    ids: Vec<String>,
    tags: Vec<String>,
    db: State<'_, Db>,
) -> Result<Vec<model::NoteTag>, AppError> {
    let normalized = model::normalize_tags(&tags);

    let mut connection = lock(&db)?;

    Ok(store::tag_many(
        &mut connection,
        &ids,
        &normalized,
        Utc::now(),
    )?)
}

/// The undo of [`tag_notes`]: exactly the pairs it added, and nothing wider.
#[tauri::command(async)]
#[specta::specta]
pub fn untag_notes(pairs: Vec<model::NoteTag>, db: State<'_, Db>) -> Result<u32, AppError> {
    let mut connection = lock(&db)?;

    Ok(count(store::untag_many(&mut connection, &pairs)?))
}

/// What a corpus-wide tag action is about to touch, asked before it runs.
#[tauri::command(async)]
#[specta::specta]
pub fn count_notes_tagged(tags: Vec<String>, db: State<'_, Db>) -> Result<u32, AppError> {
    let normalized = model::normalize_tags(&tags);

    let mut connection = lock(&db)?;

    Ok(count(store::count_notes_tagged(
        &mut connection,
        &normalized,
    )?))
}

#[tauri::command(async)]
#[specta::specta]
pub fn list_tags(db: State<'_, Db>) -> Result<Vec<TagUsage>, AppError> {
    let mut connection = lock(&db)?;

    Ok(store::tag_usage(&mut connection)?
        .into_iter()
        .map(|(tag, notes)| TagUsage {
            tag,
            note_count: count(notes),
        })
        .collect())
}

/// Renames one tag or merges several: renaming onto an existing tag is a merge anyway,
/// since a note cannot carry one twice.
#[tauri::command(async)]
#[specta::specta]
pub fn rename_tags(tags: Vec<String>, into: String, db: State<'_, Db>) -> Result<u32, AppError> {
    let (sources, target) = model::retagging(&tags, &into)?;

    let mut connection = lock(&db)?;

    Ok(count(store::retag(&mut connection, &sources, &target)?))
}

/// A list rather than one tag at a time: one round trip per tag is one lock per tag.
#[tauri::command(async)]
#[specta::specta]
pub fn delete_tags(tags: Vec<String>, db: State<'_, Db>) -> Result<u32, AppError> {
    let tags = model::normalize_tags(&tags);

    let mut connection = lock(&db)?;

    Ok(count(store::drop_tags(&mut connection, &tags)?))
}

/// ⚠️ A command of its own rather than a `NotePatch` field: filling a field is not
/// editing the note, so `updated_at` stays put — the canvas sorts on it.
#[tauri::command(async)]
#[specta::specta]
pub fn set_placeholder_values(
    id: String,
    values: BTreeMap<String, String>,
    db: State<'_, Db>,
) -> Result<DisplayNote, AppError> {
    let retained = placeholder::normalize_values(values);

    let mut connection = lock(&db)?;
    let note = store::set_placeholder_values(&mut connection, &id, &retained)?;

    Ok(display(&mut connection, note)?)
}

/// No note identifier: the palette fills an unsaved draft as readily as a saved note.
/// The database is read only for the global variables.
#[tauri::command(async)]
#[specta::specta]
pub fn fill_placeholders(
    content: String,
    values: BTreeMap<String, String>,
    db: State<'_, Db>,
) -> Result<String, AppError> {
    let mut connection = lock(&db)?;
    let globals = store::global_placeholder_values(&mut connection)?;

    Ok(placeholder::fill(
        &content,
        &placeholder::resolve(&globals, &values),
    ))
}

#[tauri::command(async)]
#[specta::specta]
pub fn list_global_placeholders(db: State<'_, Db>) -> Result<BTreeMap<String, String>, AppError> {
    let mut connection = lock(&db)?;

    Ok(store::global_placeholder_values(&mut connection)?)
}

/// Stores the whole set: what is not sent is what the user removed.
#[tauri::command(async)]
#[specta::specta]
pub fn set_global_placeholders(
    values: BTreeMap<String, String>,
    db: State<'_, Db>,
) -> Result<BTreeMap<String, String>, AppError> {
    let retained = placeholder::normalize_values(values);

    let mut connection = lock(&db)?;
    store::replace_global_placeholder_values(&mut connection, &retained)?;

    Ok(retained)
}

/// What only the database knows. Both are queries of their own, which is why neither
/// lives in [`model::decorate`].
fn display(connection: &mut Library, note: model::Note) -> Result<DisplayNote, StorageError> {
    let mut decorated = model::decorate_now(note);
    decorated.attachment_count = attachments::store::count_for(connection, &decorated.id)?;
    model::apply_global_defaults(
        &mut decorated,
        &store::global_placeholder_values(connection)?,
    );

    Ok(decorated)
}

/// Reference note for the feature tests: a field added to [`model::Note`] is declared
/// here rather than in every module that builds one.
#[cfg(test)]
pub(crate) mod fixtures {
    use std::collections::BTreeMap;

    use chrono::{DateTime, Utc};

    use super::checklist::NoteKind;
    use super::language::Language;
    use super::model::{Note, NoteLifecycle};
    use crate::db::iso8601;

    pub(crate) const NOW: &str = "2026-07-25T09:00:00.000Z";

    pub(crate) fn at(iso: &str) -> DateTime<Utc> {
        iso8601::parse(iso).expect("tests write valid instants")
    }

    pub(crate) fn note() -> Note {
        Note {
            id: "n-1".to_string(),
            space_id: "s-1".to_string(),
            folder_id: None,
            title: "Title".to_string(),
            language: Language::Txt,
            content: "Content".to_string(),
            source: String::new(),
            tags: vec!["auth".to_string()],
            pinned: false,
            created_at: at(NOW),
            updated_at: at(NOW),
            lifecycle: NoteLifecycle::Permanent,
            kind: NoteKind::Snippet,
            items: Vec::new(),
            placeholder_values: BTreeMap::new(),
        }
    }
}
