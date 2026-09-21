#![allow(clippy::needless_pass_by_value)]

pub mod board;
pub mod model;
pub mod store;

use std::collections::HashMap;

use chrono::Utc;
use tauri::State;

use crate::count::saturating_u32 as count;
use crate::db::{Db, lock};
use crate::error::AppError;
use crate::notes::view::{NoteFilter, NotesQuery};
use crate::{attachments, notes};
use board::{BoardLayout, BoardQuery, BoardView, CardPlacement, ZonePlacement};
use model::{Folder, FolderColour, FolderDraft, NoteFiling};

/// The second way to look at a space: folders as zones, their notes inside them, the
/// loose ones beside them.
///
/// ⚠️ It reads the whole space and marks what matches rather than narrowing — the quick
/// filter, the rails and the search all **dim** on the board. Reflowing the survivors into
/// a list would throw away the spatial memory the board exists for.
///
/// ⚠️ `apply_folders` deliberately does not run: a chip naming the zone a card already
/// sits in is noise, and a loose card has no folder to name.
#[tauri::command(async)]
#[specta::specta]
pub fn board_view(query: BoardQuery, db: State<'_, Db>) -> Result<BoardView, AppError> {
    let mut connection = lock(&db)?;

    let folders = store::list(&mut connection, Some(&query.space_id))?;
    let (notes, facets) = notes::store::fetch(&mut connection, &whole_space(&query))?;

    let mut note_counts: HashMap<String, usize> = HashMap::new();
    let mut loose_ids: Vec<String> = Vec::new();
    for note in &notes {
        match &note.folder_id {
            Some(id) => *note_counts.entry(id.clone()).or_default() += 1,
            None => loose_ids.push(note.id.clone()),
        }
    }

    let folder_ids: Vec<String> = folders.iter().map(|folder| folder.id.clone()).collect();
    let (frames, positions) = store::board::geometry(
        &mut connection,
        &query.space_id,
        &folder_ids,
        &note_counts,
        &loose_ids,
    )?;

    let counts = attachments::store::counts(&mut connection)?;
    let globals = notes::store::global_placeholder_values(&mut connection)?;

    let mut view = board::build(notes, folders, &frames, &positions, facets, &query);
    for entry in view
        .zones
        .iter_mut()
        .flat_map(|zone| &mut zone.notes)
        .chain(&mut view.loose)
    {
        entry.note.attachment_count = counts.get(&entry.note.id).copied().unwrap_or(0);
        crate::notes::model::apply_global_defaults(&mut entry.note, &globals);
    }

    Ok(view)
}

/// ⚠️ Everything neutral but the space: the board decides what matches in Rust, on the
/// whole space, because it dims rather than narrows.
fn whole_space(query: &BoardQuery) -> NotesQuery {
    NotesQuery {
        space_id: Some(query.space_id.clone()),
        folder_id: None,
        search: String::new(),
        filter: NoteFilter::All,
        tags: Vec::new(),
        languages: Vec::new(),
        now: query.now,
        tz_offset_minutes: 0,
        pinned_first: true,
    }
}

/// Where the zones and the loose cards ended up, written as one batch behind the front
/// end's debounce.
///
/// ⚠️ One command and one transaction for the whole gesture: a drag that ends outside the
/// window, or an application that quits mid-gesture, must not leave half a board behind.
/// Filing is **not** here — membership comes from [`file_notes`], which answers what it
/// changed so the undo can put it back.
#[tauri::command(async)]
#[specta::specta]
pub fn save_board_layout(
    zones: Vec<ZonePlacement>,
    cards: Vec<CardPlacement>,
    db: State<'_, Db>,
) -> Result<(), AppError> {
    let mut connection = lock(&db)?;

    Ok(store::board::save_layout(&mut connection, &zones, &cards)?)
}

/// `None` = every space, like [`crate::notes::view::NotesQuery::space_id`].
#[tauri::command(async)]
#[specta::specta]
pub fn list_folders(space_id: Option<String>, db: State<'_, Db>) -> Result<Vec<Folder>, AppError> {
    let mut connection = lock(&db)?;

    Ok(store::list(&mut connection, space_id.as_deref())?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn create_folder(draft: FolderDraft, db: State<'_, Db>) -> Result<Folder, AppError> {
    let name = draft.validated_name()?;

    let mut connection = lock(&db)?;

    Ok(store::create(
        &mut connection,
        &draft.space_id,
        &name,
        Utc::now(),
    )?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn rename_folder(id: String, name: String, db: State<'_, Db>) -> Result<Folder, AppError> {
    let name = model::validated_name(&name)?;

    let mut connection = lock(&db)?;

    Ok(store::rename(&mut connection, &id, &name)?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn recolour_folder(
    id: String,
    colour: FolderColour,
    db: State<'_, Db>,
) -> Result<Folder, AppError> {
    let mut connection = lock(&db)?;

    Ok(store::recolour(&mut connection, &id, colour)?)
}

/// ⚠️ No refuge argument, unlike [`crate::spaces::delete_space`]: the notes come out
/// loose, and "no folder" is a legitimate state rather than data loss.
#[tauri::command(async)]
#[specta::specta]
pub fn delete_folder(id: String, db: State<'_, Db>) -> Result<(), AppError> {
    let mut connection = lock(&db)?;

    Ok(store::delete(&mut connection, &id)?)
}

/// A batch, like [`crate::notes::move_notes`]: the selection bar files a whole selection,
/// and a drop on the board is a batch of one. `folderId` absent unfiles.
#[tauri::command(async)]
#[specta::specta]
pub fn file_notes(
    ids: Vec<String>,
    folder_id: Option<String>,
    db: State<'_, Db>,
) -> Result<Vec<NoteFiling>, AppError> {
    let mut connection = lock(&db)?;

    Ok(store::file_many(
        &mut connection,
        &ids,
        folder_id.as_deref(),
        Utc::now(),
    )?)
}

/// The undo of [`file_notes`]: each note goes back to the folder it left, or back to
/// being loose.
#[tauri::command(async)]
#[specta::specta]
pub fn file_notes_back(filings: Vec<NoteFiling>, db: State<'_, Db>) -> Result<u32, AppError> {
    let mut connection = lock(&db)?;

    Ok(count(store::restore_filings(&mut connection, &filings)?))
}

/// Puts one space's board back in order: zones in reading order, each at the height its
/// contents need, the loose cards flowing underneath.
///
/// ⚠️ It answers the layout it **replaced**, not the one it wrote. The new one arrives
/// with the reload the front end does anyway; this is the only moment the old one still
/// exists, and a tidy-up overwrites sizes chosen by hand — the one board gesture that
/// cannot be walked back by dragging.
#[tauri::command(async)]
#[specta::specta]
pub fn arrange_board(space_id: String, db: State<'_, Db>) -> Result<BoardLayout, AppError> {
    let mut connection = lock(&db)?;

    Ok(store::board::arrange(&mut connection, &space_id)?)
}

/// The undo of [`arrange_board`]: every zone and every loose card back where it was.
#[tauri::command(async)]
#[specta::specta]
pub fn restore_board_layout(layout: BoardLayout, db: State<'_, Db>) -> Result<(), AppError> {
    let mut connection = lock(&db)?;

    Ok(store::board::save_layout(
        &mut connection,
        &layout.zones,
        &layout.cards,
    )?)
}
