#![allow(clippy::needless_pass_by_value)]

pub mod checklist;
pub(crate) mod duplicate;
pub mod language;
pub(crate) mod markdown;
pub mod model;
pub mod placeholder;
pub mod revision;
pub mod store;
pub mod trash;
pub mod view;

use std::collections::BTreeMap;

use chrono::Utc;
use tauri::{AppHandle, Runtime};

use crate::count::saturating_u32 as count;
use crate::db::{Library, blocking, lock};
use crate::error::{AppError, StorageError};
use crate::folders;
use crate::spaces::model::SpaceDraft;
use language::Language;
use model::{DisplayNote, NoteDraft, NotePatch, SampleNote, TagUsage};
use revision::{DiffLine, Revision};
use trash::TrashedNote;
use view::{NotesQuery, NotesView};

/// No command returns the raw list: it would invite re-filtering on the front end.
#[tauri::command]
#[specta::specta]
pub async fn query_notes<R: Runtime>(
    query: NotesQuery,
    app: AppHandle<R>,
) -> Result<NotesView, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;
        let (notes, facets) = store::fetch(&mut connection, &query)?;

        let mut decorations = store::decorations(&mut connection)?;
        if query.shows_folder_chips() {
            decorations.folders =
                folders::store::by_id(&mut connection, query.space_id.as_deref())?;
        }
        // The search and the sections read nothing more: every other command waits on this lock.
        drop(connection);

        let mut view = view::build(notes, facets, &query);
        decorations.apply(view.notes_mut());

        Ok(view)
    })
    .await
}

/// The whole note. A list sends previews ([`DisplayNote::truncated`]), so the editor and
/// every copy of a long body read it here.
#[tauri::command]
#[specta::specta]
pub async fn get_note<R: Runtime>(id: String, app: AppHandle<R>) -> Result<DisplayNote, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;
        let note = store::get(&mut connection, &id)?;

        Ok(display(&mut connection, note)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_note<R: Runtime>(
    draft: NoteDraft,
    app: AppHandle<R>,
) -> Result<DisplayNote, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;
        let note = store::create(&mut connection, draft, Utc::now())?;

        Ok(display(&mut connection, note)?)
    })
    .await
}

/// What a paste into an empty Text note is. Asked by the front end before it writes: typing
/// keeps a note Text, and only a paste of code gives it a language.
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn detect_language(content: String) -> Language {
    language::from_content(&content)
}

/// A copy to start another note from, attachments included and history left behind. The
/// title is the front end's: its suffix is a translation.
#[tauri::command]
#[specta::specta]
pub async fn duplicate_note<R: Runtime>(
    id: String,
    title: String,
    app: AppHandle<R>,
) -> Result<DisplayNote, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;
        let note = duplicate::duplicate(&mut connection, &id, title, Utc::now())?;

        Ok(display(&mut connection, note)?)
    })
    .await
}

/// The first launch: the space, its folders and the notes in one transaction, since a space
/// standing alone reads as "already seeded" for good. The strings stay on the front end, with
/// the translations. Answers the space it made, which the front end opens on.
#[tauri::command]
#[specta::specta]
pub async fn seed_samples<R: Runtime>(
    space_name: String,
    folders: Vec<String>,
    notes: Vec<SampleNote>,
    app: AppHandle<R>,
) -> Result<crate::spaces::model::Space, AppError> {
    blocking(app, move |_, db| {
        let name = SpaceDraft { name: space_name }.validated_name()?;
        let folder_names = folders
            .iter()
            .map(|folder| crate::folders::model::validated_name(folder))
            .collect::<Result<Vec<_>, _>>()?;

        let mut connection = lock(db)?;

        Ok(store::seed(
            &mut connection,
            &name,
            &folder_names,
            notes,
            Utc::now(),
        )?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn update_note<R: Runtime>(
    id: String,
    patch: NotePatch,
    app: AppHandle<R>,
) -> Result<DisplayNote, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;
        let note = store::update(&mut connection, &id, &patch, Utc::now())?;

        Ok(display(&mut connection, note)?)
    })
    .await
}

/// The bodies kept beside a note, newest first.
///
/// Instants and sizes, never the bodies.
#[tauri::command]
#[specta::specta]
pub async fn list_revisions<R: Runtime>(
    id: String,
    app: AppHandle<R>,
) -> Result<Vec<Revision>, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        let (connection, vault) = connection.split();

        Ok(store::revisions::list(connection, vault, &id)?)
    })
    .await
}

/// What going back to a kept body would change, line by line, against the current text.
#[tauri::command]
#[specta::specta]
pub async fn revision_diff<R: Runtime>(
    id: String,
    revision_id: String,
    app: AppHandle<R>,
) -> Result<Vec<DiffLine>, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;
        let (connection, vault) = connection.split();

        let Some((current, version)) =
            store::revisions::compare(connection, vault, &id, &revision_id)?
        else {
            return Err(StorageError::RevisionNotFound(revision_id).into());
        };

        Ok(revision::diff(&current, &version))
    })
    .await
}

/// Goes back to a kept body, dropping it and every body kept after it from the history.
///
/// Leaves `updated_at` alone: putting something back is not editing it.
#[tauri::command]
#[specta::specta]
pub async fn restore_revision<R: Runtime>(
    id: String,
    revision_id: String,
    app: AppHandle<R>,
) -> Result<DisplayNote, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;
        let note = store::restore_revision(&mut connection, &id, &revision_id)?;

        Ok(display(&mut connection, note)?)
    })
    .await
}

/// Moves to the trash: the note comes back through [`restore_notes`] for
/// [`trash::RETENTION`].
#[tauri::command]
#[specta::specta]
pub async fn delete_note<R: Runtime>(id: String, app: AppHandle<R>) -> Result<(), AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::trash::trash(&mut connection, &id, Utc::now())?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_notes<R: Runtime>(
    ids: Vec<String>,
    app: AppHandle<R>,
) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(count(store::trash::trash_many(
            &mut connection,
            &ids,
            Utc::now(),
        )?))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn restore_notes<R: Runtime>(
    ids: Vec<String>,
    app: AppHandle<R>,
) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(count(store::trash::restore_many(&mut connection, &ids)?))
    })
    .await
}

/// Purges first: the trash must never show a note a restart would erase.
#[tauri::command]
#[specta::specta]
pub async fn list_trash<R: Runtime>(app: AppHandle<R>) -> Result<Vec<TrashedNote>, AppError> {
    blocking(app, move |_, db| {
        trash::purge_expired(db)?;

        let mut connection = lock(db)?;

        Ok(store::trash::list_trashed(&mut connection)?
            .into_iter()
            .map(|(note, deleted_at)| trash::trashed(note, deleted_at))
            .collect())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn purge_notes<R: Runtime>(ids: Vec<String>, app: AppHandle<R>) -> Result<u32, AppError> {
    blocking(app, move |_, db| Ok(count(trash::purge(db, ids)?))).await
}

#[tauri::command]
#[specta::specta]
pub async fn empty_trash<R: Runtime>(app: AppHandle<R>) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let ids = {
            let mut connection = lock(db)?;
            store::trash::trashed_ids(&mut connection)?
        };

        Ok(count(trash::purge(db, ids)?))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn move_notes<R: Runtime>(
    ids: Vec<String>,
    space_id: String,
    app: AppHandle<R>,
) -> Result<Vec<model::NotePlacement>, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::move_many(
            &mut connection,
            &ids,
            &space_id,
            Utc::now(),
        )?)
    })
    .await
}

/// The undo of [`move_notes`]: each note goes back to the space it left.
#[tauri::command]
#[specta::specta]
pub async fn move_notes_back<R: Runtime>(
    placements: Vec<model::NotePlacement>,
    app: AppHandle<R>,
) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(count(store::restore_placements(
            &mut connection,
            &placements,
        )?))
    })
    .await
}

/// Normalized here as everywhere else, or a typed `#urgent` would not join `urgent`.
#[tauri::command]
#[specta::specta]
pub async fn tag_notes<R: Runtime>(
    ids: Vec<String>,
    tags: Vec<String>,
    app: AppHandle<R>,
) -> Result<Vec<model::NoteTag>, AppError> {
    blocking(app, move |_, db| {
        let normalized = model::normalize_tags(&tags);

        let mut connection = lock(db)?;

        Ok(store::tag_many(
            &mut connection,
            &ids,
            &normalized,
            Utc::now(),
        )?)
    })
    .await
}

/// The undo of [`tag_notes`]: exactly the pairs it added, and nothing wider.
#[tauri::command]
#[specta::specta]
pub async fn untag_notes<R: Runtime>(
    pairs: Vec<model::NoteTag>,
    app: AppHandle<R>,
) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(count(store::untag_many(&mut connection, &pairs)?))
    })
    .await
}

/// What a corpus-wide tag action is about to touch, asked before it runs.
#[tauri::command]
#[specta::specta]
pub async fn count_notes_tagged<R: Runtime>(
    tags: Vec<String>,
    app: AppHandle<R>,
) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let normalized = model::normalize_tags(&tags);

        let mut connection = lock(db)?;

        Ok(count(store::count_notes_tagged(
            &mut connection,
            &normalized,
        )?))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_tags<R: Runtime>(app: AppHandle<R>) -> Result<Vec<TagUsage>, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::tag_usage(&mut connection)?
            .into_iter()
            .map(|(tag, notes)| TagUsage {
                tag,
                note_count: count(notes),
            })
            .collect())
    })
    .await
}

/// Renames one tag or merges several: renaming onto an existing tag is a merge anyway,
/// since a note cannot carry one twice.
#[tauri::command]
#[specta::specta]
pub async fn rename_tags<R: Runtime>(
    tags: Vec<String>,
    into: String,
    app: AppHandle<R>,
) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let (sources, target) = model::retagging(&tags, &into)?;

        let mut connection = lock(db)?;

        Ok(count(store::retag(&mut connection, &sources, &target)?))
    })
    .await
}

/// A list rather than one tag at a time: one round trip per tag is one lock per tag.
#[tauri::command]
#[specta::specta]
pub async fn delete_tags<R: Runtime>(
    tags: Vec<String>,
    app: AppHandle<R>,
) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let tags = model::normalize_tags(&tags);

        let mut connection = lock(db)?;

        Ok(count(store::drop_tags(&mut connection, &tags)?))
    })
    .await
}

/// A command of its own rather than a `NotePatch` field, so `updated_at` stays put: filling a
/// field is not editing the note.
#[tauri::command]
#[specta::specta]
pub async fn set_placeholder_values<R: Runtime>(
    id: String,
    values: BTreeMap<String, String>,
    app: AppHandle<R>,
) -> Result<DisplayNote, AppError> {
    blocking(app, move |_, db| {
        let retained = placeholder::normalize_values(values);

        let mut connection = lock(db)?;
        let note = store::set_placeholder_values(&mut connection, &id, &retained)?;

        Ok(display(&mut connection, note)?)
    })
    .await
}

/// No note identifier: the palette fills an unsaved draft as readily as a saved note.
/// The database is read only for the global variables.
#[tauri::command]
#[specta::specta]
pub async fn fill_placeholders<R: Runtime>(
    content: String,
    values: BTreeMap<String, String>,
    app: AppHandle<R>,
) -> Result<String, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;
        let globals = store::global_placeholder_values(&mut connection)?;

        Ok(placeholder::fill(
            &content,
            &placeholder::resolve(&globals, &values),
        ))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_global_placeholders<R: Runtime>(
    app: AppHandle<R>,
) -> Result<BTreeMap<String, String>, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::global_placeholder_values(&mut connection)?)
    })
    .await
}

/// Stores the whole set: what is not sent is what the user removed.
#[tauri::command]
#[specta::specta]
pub async fn set_global_placeholders<R: Runtime>(
    values: BTreeMap<String, String>,
    app: AppHandle<R>,
) -> Result<BTreeMap<String, String>, AppError> {
    blocking(app, move |_, db| {
        let retained = placeholder::normalize_values(values);

        let mut connection = lock(db)?;
        store::replace_global_placeholder_values(&mut connection, &retained)?;

        Ok(retained)
    })
    .await
}

/// What only the database knows, each a query of its own: hence not in [`model::decorate`].
fn display(connection: &mut Library, note: model::Note) -> Result<DisplayNote, StorageError> {
    let mut decorated = model::decorate_now(note);
    store::decorations(connection)?.apply([&mut decorated]);

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
