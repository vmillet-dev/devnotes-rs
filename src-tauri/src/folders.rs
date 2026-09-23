#![allow(clippy::needless_pass_by_value)]

pub mod board;
pub mod model;
pub mod store;

use chrono::Utc;
use tauri::State;

use crate::count::saturating_u32 as count;
use crate::db::{Db, lock};
use crate::error::AppError;

use crate::notes;
use board::{
    BoardArrangement, BoardQuery, BoardScope, BoardView, CardPlacement, Occupancy, ZonePlacement,
};
use model::{Folder, FolderColour, FolderDraft, NoteFiling};

/// The second way to look at a space: folders as zones, their notes inside them, the
/// loose ones beside them.
///
/// ⚠️ It reads the whole space and marks what matches rather than narrowing — the quick
/// filter, the rails and the search all **dim** on the board. Reflowing the survivors into
/// a list would throw away the spatial memory the board exists for.
///
/// ⚠️ No folder chips: a chip naming the zone a card already sits in is noise, and a loose
/// card has no folder to name.
#[tauri::command(async)]
#[specta::specta]
pub fn board_view(query: BoardQuery, db: State<'_, Db>) -> Result<BoardView, AppError> {
    let mut connection = lock(&db)?;

    let folders = store::list(&mut connection, Some(&query.space_id))?;
    let (notes, facets) = notes::store::fetch(&mut connection, &query.whole_space())?;

    let folder_ids: Vec<String> = folders.iter().map(|folder| folder.id.clone()).collect();
    let (frames, positions) = store::board::geometry(
        &mut connection,
        &query.space_id,
        &folder_ids,
        &Occupancy::of(&notes),
    )?;

    let mut view = board::build(notes, folders, &frames, &positions, facets, &query);
    notes::store::decorations(&mut connection)?.apply(view.notes_mut());

    Ok(view)
}

/// Where the zones and the loose cards ended up, written as one batch behind the front
/// end's debounce.
///
/// ⚠️ One command and one transaction for the whole gesture: a drag that ends outside the
/// window, or an application that quits mid-gesture, must not leave half a board behind.
/// Filing is **not** here — membership comes from [`file_notes`], which answers what it
/// changed so the undo can put it back. The undo of [`arrange_board`] is this command too,
/// with the layout that one answered.
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

/// Puts one space's board back in order, as far as `scope` allows: the loose cards alone,
/// or the zones with them.
///
/// ⚠️ It answers the layout it **replaced**, not the one it wrote. The new one arrives
/// with the reload the front end does anyway; this is the only moment the old one still
/// exists, and [`board::BoardScope::Everything`] overwrites sizes chosen by hand — the one
/// board gesture that cannot be walked back by dragging.
#[tauri::command(async)]
#[specta::specta]
pub fn arrange_board(
    space_id: String,
    scope: BoardScope,
    db: State<'_, Db>,
) -> Result<BoardArrangement, AppError> {
    let mut connection = lock(&db)?;

    Ok(store::board::arrange(&mut connection, &space_id, scope)?)
}
