#![allow(clippy::needless_pass_by_value)]

use super::run;
use engine::error::AppError;
use engine::notes::calc;
use engine::notes::convert;
use engine::notes::drawing;
use engine::notes::kind;
use engine::notes::language::Language;
use engine::notes::model;
use engine::notes::model::DisplayNote;
use engine::notes::model::NoteDraft;
use engine::notes::model::NotePatch;
use engine::notes::model::SampleNote;
use engine::notes::model::TagUsage;
use engine::notes::placeholder;
use engine::notes::priority::Priority;
use engine::notes::revision::DiffLine;
use engine::notes::revision::Revision;
use engine::notes::trash::TrashedNote;
use engine::notes::view::NotesQuery;
use engine::notes::view::NotesView;
use engine::{Engine, notes};
use std::collections::BTreeMap;
use std::sync::Arc;
use tauri::State;

/// No command returns the raw list: it would invite re-filtering on the front end.
#[tauri::command]
#[specta::specta]
pub async fn query_notes(
    query: NotesQuery,
    engine: State<'_, Arc<Engine>>,
) -> Result<NotesView, AppError> {
    run(&engine, move |engine| notes::query_notes(engine, query)).await
}

/// The whole note. A list sends previews ([`DisplayNote::truncated`]), so the editor and
/// every copy of a long body read it here.
#[tauri::command]
#[specta::specta]
pub async fn get_note(id: String, engine: State<'_, Arc<Engine>>) -> Result<DisplayNote, AppError> {
    run(&engine, move |engine| notes::get_note(engine, id)).await
}

/// The drawings a Note can embed, by title: their titles alone are opened.
#[tauri::command]
#[specta::specta]
pub async fn list_drawings(
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<drawing::DrawingRef>, AppError> {
    run(&engine, notes::list_drawings).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_note(
    draft: NoteDraft,
    engine: State<'_, Arc<Engine>>,
) -> Result<DisplayNote, AppError> {
    run(&engine, move |engine| notes::create_note(engine, draft)).await
}

/// What a paste into an empty Note is. Asked by the front end before it writes: typing keeps
/// a Note prose, and only a paste of code turns it into a snippet with a language.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
#[specta::specta]
pub fn detect_language(content: String) -> Language {
    notes::detect_language(content)
}

/// A Note cut into slides, with the mind map of its headings: what « Présenter » shows. The
/// draft is sent rather than the stored body, so what is presented is what is on screen.
#[tauri::command]
#[specta::specta]
pub async fn present_note(markdown: String) -> Result<model::Presentation, AppError> {
    notes::present_note(markdown).await
}

/// A Calcul note's sheet, every line computed, as it is typed: no lock, and nothing stored.
/// `today` is the local day of the window, as the notes' sections reckon it.
#[tauri::command]
#[specta::specta]
pub async fn evaluate_calc(
    sheet: String,
    today: chrono::NaiveDate,
) -> Result<calc::CalcSheet, AppError> {
    notes::evaluate_calc(sheet, today).await
}

/// Where the fields of a text lie, for the code field to mark them: what a field is stays here.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
#[specta::specta]
pub fn field_spans(content: String) -> Vec<placeholder::FieldSpan> {
    notes::field_spans(content)
}

/// A copy to start another note from, attachments included and history left behind. The
/// title is the front end's: its suffix is a translation.
#[tauri::command]
#[specta::specta]
pub async fn duplicate_note(
    id: String,
    title: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<DisplayNote, AppError> {
    run(&engine, move |engine| {
        notes::duplicate_note(engine, id, title)
    })
    .await
}

/// The first launch: the space, its folders and the notes in one transaction, since a space
/// standing alone reads as "already seeded" for good. The strings stay on the front end, with
/// the translations. Answers the space it made, which the front end opens on.
#[tauri::command]
#[specta::specta]
pub async fn seed_samples(
    space_name: String,
    folders: Vec<String>,
    notes: Vec<SampleNote>,
    engine: State<'_, Arc<Engine>>,
) -> Result<engine::spaces::model::Space, AppError> {
    run(&engine, move |engine| {
        notes::seed_samples(engine, space_name, folders, notes)
    })
    .await
}

/// The note rewritten as `into`, with the body it had: its undo is that body written back.
#[tauri::command]
#[specta::specta]
pub async fn convert_note(
    id: String,
    into: kind::NoteKind,
    engine: State<'_, Arc<Engine>>,
) -> Result<convert::NoteConversion, AppError> {
    run(&engine, move |engine| notes::convert_note(engine, id, into)).await
}

/// The bodies kept beside a note, newest first.
///
/// Instants and sizes, never the bodies.
#[tauri::command]
#[specta::specta]
pub async fn list_revisions(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<Revision>, AppError> {
    run(&engine, move |engine| notes::list_revisions(engine, id)).await
}

/// What going back to a kept body would change, line by line, against the current text.
#[tauri::command]
#[specta::specta]
pub async fn revision_diff(
    id: String,
    revision_id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<DiffLine>, AppError> {
    run(&engine, move |engine| {
        notes::revision_diff(engine, id, revision_id)
    })
    .await
}

/// Goes back to a kept body, dropping it and every body kept after it from the history.
///
/// Leaves `updated_at` alone: putting something back is not editing it.
#[tauri::command]
#[specta::specta]
pub async fn restore_revision(
    id: String,
    revision_id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<DisplayNote, AppError> {
    run(&engine, move |engine| {
        notes::restore_revision(engine, id, revision_id)
    })
    .await
}

/// Moves to the trash: the note comes back through [`restore_notes`] for
/// [`trash::RETENTION`].
#[tauri::command]
#[specta::specta]
pub async fn delete_note(id: String, engine: State<'_, Arc<Engine>>) -> Result<(), AppError> {
    run(&engine, move |engine| notes::delete_note(engine, id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_notes(
    ids: Vec<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| notes::delete_notes(engine, ids)).await
}

#[tauri::command]
#[specta::specta]
pub async fn restore_notes(
    ids: Vec<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| notes::restore_notes(engine, ids)).await
}

/// Purges first: the trash must never show a note a restart would erase. `search` narrows it
/// the way the canvas search does, folded; empty lists everything.
#[tauri::command]
#[specta::specta]
pub async fn list_trash(
    search: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<TrashedNote>, AppError> {
    run(&engine, move |engine| notes::list_trash(engine, search)).await
}

/// What the trash lists for `search` back where it was: the way out of a mistaken bulk deletion.
#[tauri::command]
#[specta::specta]
pub async fn restore_trash(
    search: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| notes::restore_trash(engine, search)).await
}

#[tauri::command]
#[specta::specta]
pub async fn purge_notes(
    ids: Vec<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| notes::purge_notes(engine, ids)).await
}

#[tauri::command]
#[specta::specta]
pub async fn empty_trash(engine: State<'_, Arc<Engine>>) -> Result<u32, AppError> {
    run(&engine, notes::empty_trash).await
}

#[tauri::command]
#[specta::specta]
pub async fn move_notes(
    ids: Vec<String>,
    space_id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<model::NotePlacement>, AppError> {
    run(&engine, move |engine| {
        notes::move_notes(engine, ids, space_id)
    })
    .await
}

/// The undo of [`move_notes`]: each note goes back to the space it left.
#[tauri::command]
#[specta::specta]
pub async fn move_notes_back(
    placements: Vec<model::NotePlacement>,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| {
        notes::move_notes_back(engine, placements)
    })
    .await
}

/// Normalized here as everywhere else, or a typed `#urgent` would not join `urgent`.
#[tauri::command]
#[specta::specta]
pub async fn tag_notes(
    ids: Vec<String>,
    tags: Vec<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<model::NoteTag>, AppError> {
    run(&engine, move |engine| notes::tag_notes(engine, ids, tags)).await
}

/// The undo of [`tag_notes`]: exactly the pairs it added, and nothing wider.
#[tauri::command]
#[specta::specta]
pub async fn untag_notes(
    pairs: Vec<model::NoteTag>,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| notes::untag_notes(engine, pairs)).await
}

/// Answers the pairs it took off: normalized like every tag, so `#Urgent` finds `urgent`.
#[tauri::command]
#[specta::specta]
pub async fn remove_tag(
    ids: Vec<String>,
    tag: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<model::NoteTag>, AppError> {
    run(&engine, move |engine| notes::remove_tag(engine, ids, tag)).await
}

/// The undo of [`remove_tag`]: exactly the pairs it took off.
#[tauri::command]
#[specta::specta]
pub async fn restore_tags(
    pairs: Vec<model::NoteTag>,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| notes::restore_tags(engine, pairs)).await
}

/// Answers the notes whose pin changed; undone by the same call, the other way, over those.
#[tauri::command]
#[specta::specta]
pub async fn set_pinned(
    ids: Vec<String>,
    pinned: bool,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<String>, AppError> {
    run(&engine, move |engine| {
        notes::set_pinned(engine, ids, pinned)
    })
    .await
}

/// Answers each note whose priority moved, with the one it had; `updated_at` stays.
#[tauri::command]
#[specta::specta]
pub async fn set_priority(
    ids: Vec<String>,
    priority: Priority,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<model::NotePriority>, AppError> {
    run(&engine, move |engine| {
        notes::set_priority(engine, ids, priority)
    })
    .await
}

/// The undo of [`set_priority`]: each note gets back the priority it had.
#[tauri::command]
#[specta::specta]
pub async fn restore_priorities(
    previous: Vec<model::NotePriority>,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| {
        notes::restore_priorities(engine, previous)
    })
    .await
}

/// What a corpus-wide tag action is about to touch, asked before it runs.
#[tauri::command]
#[specta::specta]
pub async fn count_notes_tagged(
    tags: Vec<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| {
        notes::count_notes_tagged(engine, tags)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_tags(engine: State<'_, Arc<Engine>>) -> Result<Vec<TagUsage>, AppError> {
    run(&engine, notes::list_tags).await
}

/// Renames one tag or merges several: renaming onto an existing tag is a merge anyway,
/// since a note cannot carry one twice.
#[tauri::command]
#[specta::specta]
pub async fn rename_tags(
    tags: Vec<String>,
    into: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| {
        notes::rename_tags(engine, tags, into)
    })
    .await
}

/// A list rather than one tag at a time: one round trip per tag is one lock per tag.
#[tauri::command]
#[specta::specta]
pub async fn delete_tags(
    tags: Vec<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| notes::delete_tags(engine, tags)).await
}

/// A command of its own rather than a `NotePatch` field, so `updated_at` stays put: filling a
/// field is not editing the note.
#[tauri::command]
#[specta::specta]
pub async fn set_placeholder_values(
    id: String,
    values: BTreeMap<String, String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<DisplayNote, AppError> {
    run(&engine, move |engine| {
        notes::set_placeholder_values(engine, id, values)
    })
    .await
}

/// No note identifier: the palette fills an unsaved draft as readily as a saved note.
/// The database is read only for the global variables.
#[tauri::command]
#[specta::specta]
pub async fn fill_placeholders(
    content: String,
    values: BTreeMap<String, String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<String, AppError> {
    run(&engine, move |engine| {
        notes::fill_placeholders(engine, content, values)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_global_placeholders(
    engine: State<'_, Arc<Engine>>,
) -> Result<BTreeMap<String, String>, AppError> {
    run(&engine, move |engine| {
        notes::list_global_placeholders(engine)
    })
    .await
}

/// Stores the whole set: what is not sent is what the user removed.
#[tauri::command]
#[specta::specta]
pub async fn set_global_placeholders(
    values: BTreeMap<String, String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<BTreeMap<String, String>, AppError> {
    run(&engine, move |engine| {
        notes::set_global_placeholders(engine, values)
    })
    .await
}

/// A Dessin's small picture, for its card, which asks for it once it is drawn: a list sends
/// none, so that a search opens no picture. `None` when there is none to show.
#[tauri::command]
#[specta::specta]
pub async fn drawing_thumbnail(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Option<String>, AppError> {
    run(&engine, move |engine| notes::drawing_thumbnail(engine, id)).await
}

/// A Dessin's large picture, for a Note that embeds it by its id.
#[tauri::command]
#[specta::specta]
pub async fn drawing_picture(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Option<String>, AppError> {
    run(&engine, move |engine| notes::drawing_picture(engine, id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_note(
    id: String,
    patch: NotePatch,
    engine: State<'_, Arc<Engine>>,
) -> Result<DisplayNote, AppError> {
    run(&engine, move |engine| notes::update_note(engine, id, patch)).await
}
