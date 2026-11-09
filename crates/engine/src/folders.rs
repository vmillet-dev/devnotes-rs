#![allow(clippy::needless_pass_by_value)]

pub mod board;
pub mod model;
pub mod store;

use chrono::Utc;

use crate::Engine;
use crate::count::saturating_u32 as count;
use crate::db::lock;
use crate::error::AppError;
use crate::index::Changed;

use board::{
    BoardArrangement, BoardQuery, BoardScope, BoardView, CardPlacement, Occupancy, PreviewLines,
    ZonePlacement,
};
use model::{Folder, FolderColour, FolderDraft, FolderMove, Lineage, NoteFiling};

/// The second way to look at a space, one level at a time: the root's sub-folders as zones,
/// each gathering what is under it, and what is filed in the root itself free beside them. It
/// reads the whole space and marks what matches rather than narrowing. A card names its folder
/// only when it comes from below its zone — a chip naming the zone it sits in is noise.
pub fn board_view(engine: &Engine, query: BoardQuery) -> Result<BoardView, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    let folders = store::list(&mut connection, Some(&query.space_id))?;
    let mut index = engine.index()?;
    index.ready(&mut connection)?;
    let (entries, facets) = index.select(&query.whole_space(), None);
    let notes: Vec<_> = entries.iter().map(|entry| entry.note.clone()).collect();
    let decorations = index.decorations();
    drop(index);

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
        query.preview_lines.card_height(),
    )?;

    drop(connection);

    let mut view = board::build(notes, &folders, &frames, &positions, facets, &query);
    decorations.apply(view.notes_mut());
    view.keep_chips_from_below();

    Ok(view)
}

/// Where the zones and the loose cards ended up, as one batch and one transaction: a gesture
/// cut short must not leave half a board. Filing is not here (see [`file_notes`]). This is
/// also the undo of [`arrange_board`], with the layout that one answered.
pub fn save_board_layout(
    engine: &Engine,
    root_id: Option<String>,
    zones: Vec<ZonePlacement>,
    cards: Vec<CardPlacement>,
) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::board::save_layout(
        &mut connection,
        root_id.as_deref(),
        &zones,
        &cards,
    )?)
}

/// `None` = every space, like [`crate::notes::view::NotesQuery::space_id`].
pub fn list_folders(engine: &Engine, space_id: Option<String>) -> Result<Vec<Folder>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::list(&mut connection, space_id.as_deref())?)
}

pub fn create_folder(engine: &Engine, draft: FolderDraft) -> Result<Folder, AppError> {
    engine.write(|db| {
        let name = draft.validated_name()?;

        let mut connection = lock(db)?;

        let answer = store::create(
            &mut connection,
            &draft.space_id,
            draft.parent_id.as_deref(),
            &name,
            Utc::now(),
        )?;
        let changed = Changed {
            folders: true,
            ..Changed::default()
        };
        Ok((answer, changed))
    })
}

pub fn rename_folder(engine: &Engine, id: String, name: String) -> Result<Folder, AppError> {
    engine.write(|db| {
        let name = model::validated_name(&name)?;

        let mut connection = lock(db)?;

        let answer = store::rename(&mut connection, &id, &name)?;
        let changed = Changed {
            folders: true,
            ..Changed::default()
        };
        Ok((answer, changed))
    })
}

pub fn recolour_folder(
    engine: &Engine,
    id: String,
    colour: FolderColour,
) -> Result<Folder, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = store::recolour(&mut connection, &id, colour)?;
        let changed = Changed {
            folders: true,
            ..Changed::default()
        };
        Ok((answer, changed))
    })
}

/// Under another folder of its space, `None` at its top, with everything it holds. The answer
/// is where it was: what [`move_folder_back`] puts back.
pub fn move_folder(
    engine: &Engine,
    id: String,
    parent_id: Option<String>,
) -> Result<FolderMove, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = store::move_to(&mut connection, &id, parent_id.as_deref())?;
        let changed = Changed {
            folders: true,
            ..Changed::default()
        };
        Ok((answer, changed))
    })
}

/// The undo of [`move_folder`]; `false` when its old parent is gone or no longer a place for it.
pub fn move_folder_back(engine: &Engine, was: FolderMove) -> Result<bool, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = store::move_back(&mut connection, &was)?;
        let changed = Changed {
            folders: true,
            ..Changed::default()
        };
        Ok((answer, changed))
    })
}

/// No refuge, unlike [`crate::spaces::delete_space`]: what it held goes to its parent, or
/// loose at the top of the space. `lines` sizes the rows the parent's zone grows by.
pub fn delete_folder(engine: &Engine, id: String, lines: PreviewLines) -> Result<(), AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        store::delete(&mut connection, &id, lines.card_height())?;
        let changed = Changed::everything();
        Ok(((), changed))
    })
}

/// A batch, like [`crate::notes::move_notes`]: the selection bar files a whole selection,
/// and a drop on the board is a batch of one. `folderId` absent unfiles. `lines` sizes the
/// rows a zone grows by to show what was filed into it.
pub fn file_notes(
    engine: &Engine,
    ids: Vec<String>,
    folder_id: Option<String>,
    lines: PreviewLines,
) -> Result<Vec<NoteFiling>, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = store::file_many(
            &mut connection,
            &ids,
            folder_id.as_deref(),
            Utc::now(),
            lines.card_height(),
        )?;
        let changed = Changed::notes(ids.clone());
        Ok((answer, changed))
    })
}

/// The undo of [`file_notes`]: each note goes back to the folder it left, or to loose.
pub fn file_notes_back(
    engine: &Engine,
    filings: Vec<NoteFiling>,
    lines: PreviewLines,
) -> Result<u32, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = count(store::restore_filings(
            &mut connection,
            &filings,
            lines.card_height(),
        )?);
        let changed = Changed::notes(filings.iter().map(|one| one.note_id.clone()));
        Ok((answer, changed))
    })
}

/// Puts one board back in order — the space's, or a folder's, `rootId` — as far as `scope`
/// allows, and answers the layout it replaced: the only moment the old one still exists, for an
/// undo.
pub fn arrange_board(
    engine: &Engine,
    space_id: String,
    root_id: Option<String>,
    scope: BoardScope,
    lines: PreviewLines,
) -> Result<BoardArrangement, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::board::arrange(
        &mut connection,
        &space_id,
        root_id.as_deref(),
        scope,
        lines.card_height(),
    )?)
}
