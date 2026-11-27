pub mod model;
pub mod store;

use chrono::Utc;

use crate::Engine;
use crate::count::saturating_u32 as count;
use crate::db::lock;
use crate::error::AppError;
use crate::index::Changed;
use crate::notes::model::HeldNotes;
use model::{Space, SpaceContents, SpaceDraft};

pub fn list_spaces(engine: &Engine) -> Result<Vec<Space>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::list(&mut connection)?)
}

pub fn create_space(engine: &Engine, draft: &SpaceDraft) -> Result<Space, AppError> {
    let db = &engine.db;
    let name = draft.validated_name()?;

    let mut connection = lock(db)?;

    Ok(store::create(&mut connection, &name)?)
}

pub fn rename_space(engine: &Engine, id: &str, draft: &SpaceDraft) -> Result<Space, AppError> {
    let db = &engine.db;
    let name = draft.validated_name()?;

    let mut connection = lock(db)?;

    Ok(store::rename(&mut connection, id, &name)?)
}

pub fn pin_space(engine: &Engine, id: &str, pinned: bool) -> Result<Space, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::set_pinned(&mut connection, id, pinned)?)
}

/// What deleting the space would move and take, for the confirmation that comes first.
pub fn count_space_contents(engine: &Engine, id: &str) -> Result<SpaceContents, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::contents(&mut connection, id)?)
}

/// Its notes go to `target_space_id`, out of any folder; with `HeldNotes::Trash` they go to the
/// trash as well, and a restore brings them back there. Answers how many live notes it moved.
pub fn delete_space(
    engine: &Engine,
    id: &str,
    target_space_id: &str,
    held: HeldNotes,
) -> Result<u32, AppError> {
    engine.write(|db| {
        // A space as its own refuge would see its notes swept away by the cascade right
        // after the transfer.
        model::validate_move_target(id, target_space_id)?;

        let mut connection = lock(db)?;

        let trashed_at = (held == HeldNotes::Trash).then(Utc::now);
        let moved = store::delete(&mut connection, id, target_space_id, trashed_at)?;
        let changed = Changed::everything();
        Ok((count(moved), changed))
    })
}
