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
use model::{Folder, FolderColour, FolderDraft, FolderMove, Lineage, NoteFiling};

/// The second way to look at a space, one level at a time: the root's sub-folders as zones,
/// each gathering what is under it, and what is filed in the root itself free beside them. It
/// reads the whole space and marks what matches rather than narrowing. A card names its folder
/// only when it comes from below its zone — a chip naming the zone it sits in is noise.
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

        let lineage = Lineage::of(&folders);
        let root = query.root_id.as_deref();
        let zones: Vec<(String, bool)> = lineage
            .children(root)
            .iter()
            .map(|id| (id.clone(), !lineage.children(Some(id)).is_empty()))
            .collect();
        let (frames, positions) = store::board::geometry(
            &mut connection,
            &query.space_id,
            &zones,
            &Occupancy::on(&notes, &lineage, root),
        )?;

        let mut decorations = notes::store::decorations(&mut connection)?;
        decorations.folders = store::by_id(&mut connection, Some(&query.space_id))?;
        // The dimming and the layout read nothing more: every other command waits on this lock.
        drop(connection);

        let mut view = board::build(notes, &folders, &frames, &positions, facets, &query);
        decorations.apply(view.notes_mut());
        view.keep_chips_from_below();

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
    root_id: Option<String>,
    zones: Vec<ZonePlacement>,
    cards: Vec<CardPlacement>,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::board::save_layout(
            &mut connection,
            root_id.as_deref(),
            &zones,
            &cards,
        )?)
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
            draft.parent_id.as_deref(),
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

/// Under another folder of its space, `None` at its top, with everything it holds. The answer
/// is where it was: what [`move_folder_back`] puts back.
#[tauri::command]
#[specta::specta]
pub async fn move_folder<R: Runtime>(
    id: String,
    parent_id: Option<String>,
    app: AppHandle<R>,
) -> Result<FolderMove, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::move_to(&mut connection, &id, parent_id.as_deref())?)
    })
    .await
}

/// The undo of [`move_folder`]; `false` when its old parent is gone or no longer a place for it.
#[tauri::command]
#[specta::specta]
pub async fn move_folder_back<R: Runtime>(
    was: FolderMove,
    app: AppHandle<R>,
) -> Result<bool, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::move_back(&mut connection, &was)?)
    })
    .await
}

/// No refuge, unlike [`crate::spaces::delete_space`]: what it held goes to its parent, or
/// loose at the top of the space.
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

/// Puts one board back in order — the space's, or a folder's, `rootId` — as far as `scope`
/// allows, and answers the layout it replaced: the only moment the old one still exists, for an
/// undo.
#[tauri::command]
#[specta::specta]
pub async fn arrange_board<R: Runtime>(
    space_id: String,
    root_id: Option<String>,
    scope: BoardScope,
    app: AppHandle<R>,
) -> Result<BoardArrangement, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::board::arrange(
            &mut connection,
            &space_id,
            root_id.as_deref(),
            scope,
        )?)
    })
    .await
}
