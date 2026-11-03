#![allow(clippy::needless_pass_by_value)]

use super::run;
use engine::error::AppError;
use engine::folders::board::BoardArrangement;
use engine::folders::board::BoardQuery;
use engine::folders::board::BoardScope;
use engine::folders::board::BoardView;
use engine::folders::board::CardPlacement;
use engine::folders::board::ZonePlacement;
use engine::folders::model::Folder;
use engine::folders::model::FolderColour;
use engine::folders::model::FolderDraft;
use engine::folders::model::FolderMove;
use engine::folders::model::NoteFiling;
use engine::{Engine, folders};
use std::sync::Arc;
use tauri::State;

/// The second way to look at a space, one level at a time: the root's sub-folders as zones,
/// each gathering what is under it, and what is filed in the root itself free beside them. It
/// reads the whole space and marks what matches rather than narrowing. A card names its folder
/// only when it comes from below its zone — a chip naming the zone it sits in is noise.
#[tauri::command]
#[specta::specta]
pub async fn board_view(
    query: BoardQuery,
    engine: State<'_, Arc<Engine>>,
) -> Result<BoardView, AppError> {
    run(&engine, move |engine| folders::board_view(engine, query)).await
}

/// Where the zones and the loose cards ended up, as one batch and one transaction: a gesture
/// cut short must not leave half a board. Filing is not here (see [`file_notes`]). This is
/// also the undo of [`arrange_board`], with the layout that one answered.
#[tauri::command]
#[specta::specta]
pub async fn save_board_layout(
    root_id: Option<String>,
    zones: Vec<ZonePlacement>,
    cards: Vec<CardPlacement>,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    run(&engine, move |engine| {
        folders::save_board_layout(engine, root_id, zones, cards)
    })
    .await
}

/// `None` = every space, like [`engine::notes::view::NotesQuery::space_id`].
#[tauri::command]
#[specta::specta]
pub async fn list_folders(
    space_id: Option<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<Folder>, AppError> {
    run(&engine, move |engine| {
        folders::list_folders(engine, space_id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_folder(
    draft: FolderDraft,
    engine: State<'_, Arc<Engine>>,
) -> Result<Folder, AppError> {
    run(&engine, move |engine| folders::create_folder(engine, draft)).await
}

#[tauri::command]
#[specta::specta]
pub async fn rename_folder(
    id: String,
    name: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Folder, AppError> {
    run(&engine, move |engine| {
        folders::rename_folder(engine, id, name)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn recolour_folder(
    id: String,
    colour: FolderColour,
    engine: State<'_, Arc<Engine>>,
) -> Result<Folder, AppError> {
    run(&engine, move |engine| {
        folders::recolour_folder(engine, id, colour)
    })
    .await
}

/// Under another folder of its space, `None` at its top, with everything it holds. The answer
/// is where it was: what [`move_folder_back`] puts back.
#[tauri::command]
#[specta::specta]
pub async fn move_folder(
    id: String,
    parent_id: Option<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<FolderMove, AppError> {
    run(&engine, move |engine| {
        folders::move_folder(engine, id, parent_id)
    })
    .await
}

/// The undo of [`move_folder`]; `false` when its old parent is gone or no longer a place for it.
#[tauri::command]
#[specta::specta]
pub async fn move_folder_back(
    was: FolderMove,
    engine: State<'_, Arc<Engine>>,
) -> Result<bool, AppError> {
    run(&engine, move |engine| {
        folders::move_folder_back(engine, was)
    })
    .await
}

/// No refuge, unlike [`engine::spaces::delete_space`]: what it held goes to its parent, or
/// loose at the top of the space.
#[tauri::command]
#[specta::specta]
pub async fn delete_folder(id: String, engine: State<'_, Arc<Engine>>) -> Result<(), AppError> {
    run(&engine, move |engine| folders::delete_folder(engine, id)).await
}

/// A batch, like [`engine::notes::move_notes`]: the selection bar files a whole selection,
/// and a drop on the board is a batch of one. `folderId` absent unfiles.
#[tauri::command]
#[specta::specta]
pub async fn file_notes(
    ids: Vec<String>,
    folder_id: Option<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<NoteFiling>, AppError> {
    run(&engine, move |engine| {
        folders::file_notes(engine, ids, folder_id)
    })
    .await
}

/// The undo of [`file_notes`]: each note goes back to the folder it left, or to loose.
#[tauri::command]
#[specta::specta]
pub async fn file_notes_back(
    filings: Vec<NoteFiling>,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| {
        folders::file_notes_back(engine, filings)
    })
    .await
}

/// Puts one board back in order — the space's, or a folder's, `rootId` — as far as `scope`
/// allows, and answers the layout it replaced: the only moment the old one still exists, for an
/// undo.
#[tauri::command]
#[specta::specta]
pub async fn arrange_board(
    space_id: String,
    root_id: Option<String>,
    scope: BoardScope,
    engine: State<'_, Arc<Engine>>,
) -> Result<BoardArrangement, AppError> {
    run(&engine, move |engine| {
        folders::arrange_board(engine, space_id, root_id, scope)
    })
    .await
}
