#![allow(clippy::needless_pass_by_value)]

pub mod board;
pub mod model;
pub mod store;

use chrono::Utc;
use tauri::{AppHandle, Runtime};

use crate::count::saturating_u32 as count;
use crate::db::{blocking, lock};
use crate::error::AppError;

use crate::notes;
use board::{
    BoardArrangement, BoardQuery, BoardScope, BoardView, CardPlacement, Occupancy, ZonePlacement,
};
use model::{Folder, FolderColour, FolderDraft, NoteFiling};

/// The second way to look at a space: folders as zones, their notes inside, the loose ones
/// beside them. It reads the whole space and marks what matches rather than narrowing, and
/// draws no folder chips — a chip naming the zone a card sits in is noise.
#[tauri::command]
#[specta::specta]
pub async fn board_view<R: Runtime>(
    query: BoardQuery,
    app: AppHandle<R>,
) -> Result<BoardView, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        let folders = store::list(&mut connection, Some(&query.space_id))?;
        let (notes, facets) = notes::store::fetch(&mut connection, &query.whole_space())?;

        let folder_ids: Vec<String> = folders.iter().map(|folder| folder.id.clone()).collect();
        let (frames, positions) = store::board::geometry(
            &mut connection,
            &query.space_id,
            &folder_ids,
            &Occupancy::of(&notes),
        )?;

        let decorations = notes::store::decorations(&mut connection)?;
        // The dimming and the layout read nothing more: every other command waits on this lock.
        drop(connection);

        let mut view = board::build(notes, folders, &frames, &positions, facets, &query);
        decorations.apply(view.notes_mut());

        Ok(view)
    })
    .await
}

/// Where the zones and the loose cards ended up, as one batch and one transaction: a gesture
/// cut short must not leave half a board. Filing is not here (see [`file_notes`]). This is
/// also the undo of [`arrange_board`], with the layout that one answered.
#[tauri::command]
#[specta::specta]
pub async fn save_board_layout<R: Runtime>(
    zones: Vec<ZonePlacement>,
    cards: Vec<CardPlacement>,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::board::save_layout(&mut connection, &zones, &cards)?)
    })
    .await
}

/// `None` = every space, like [`crate::notes::view::NotesQuery::space_id`].
#[tauri::command]
#[specta::specta]
pub async fn list_folders<R: Runtime>(
    space_id: Option<String>,
    app: AppHandle<R>,
) -> Result<Vec<Folder>, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::list(&mut connection, space_id.as_deref())?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_folder<R: Runtime>(
    draft: FolderDraft,
    app: AppHandle<R>,
) -> Result<Folder, AppError> {
    blocking(app, move |_, db| {
        let name = draft.validated_name()?;

        let mut connection = lock(db)?;

        Ok(store::create(
            &mut connection,
            &draft.space_id,
            &name,
            Utc::now(),
        )?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn rename_folder<R: Runtime>(
    id: String,
    name: String,
    app: AppHandle<R>,
) -> Result<Folder, AppError> {
    blocking(app, move |_, db| {
        let name = model::validated_name(&name)?;

        let mut connection = lock(db)?;

        Ok(store::rename(&mut connection, &id, &name)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn recolour_folder<R: Runtime>(
    id: String,
    colour: FolderColour,
    app: AppHandle<R>,
) -> Result<Folder, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::recolour(&mut connection, &id, colour)?)
    })
    .await
}

/// No refuge, unlike [`crate::spaces::delete_space`]: the notes come out loose, a legitimate
/// state.
#[tauri::command]
#[specta::specta]
pub async fn delete_folder<R: Runtime>(id: String, app: AppHandle<R>) -> Result<(), AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::delete(&mut connection, &id)?)
    })
    .await
}

/// A batch, like [`crate::notes::move_notes`]: the selection bar files a whole selection,
/// and a drop on the board is a batch of one. `folderId` absent unfiles.
#[tauri::command]
#[specta::specta]
pub async fn file_notes<R: Runtime>(
    ids: Vec<String>,
    folder_id: Option<String>,
    app: AppHandle<R>,
) -> Result<Vec<NoteFiling>, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::file_many(
            &mut connection,
            &ids,
            folder_id.as_deref(),
            Utc::now(),
        )?)
    })
    .await
}

/// The undo of [`file_notes`]: each note goes back to the folder it left, or to loose.
#[tauri::command]
#[specta::specta]
pub async fn file_notes_back<R: Runtime>(
    filings: Vec<NoteFiling>,
    app: AppHandle<R>,
) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(count(store::restore_filings(&mut connection, &filings)?))
    })
    .await
}

/// Puts one space's board back in order, as far as `scope` allows, and answers the layout it
/// replaced: the only moment the old one still exists, for an undo.
#[tauri::command]
#[specta::specta]
pub async fn arrange_board<R: Runtime>(
    space_id: String,
    scope: BoardScope,
    app: AppHandle<R>,
) -> Result<BoardArrangement, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::board::arrange(&mut connection, &space_id, scope)?)
    })
    .await
}
