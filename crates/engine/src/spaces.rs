#![allow(clippy::needless_pass_by_value)]

pub mod model;
pub mod store;

use crate::Engine;
use crate::db::lock;
use crate::error::AppError;
use crate::index::Changed;
use model::{Space, SpaceContents, SpaceDraft};

pub fn list_spaces(engine: &Engine) -> Result<Vec<Space>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::list(&mut connection)?)
}

pub fn create_space(engine: &Engine, draft: SpaceDraft) -> Result<Space, AppError> {
    let db = &engine.db;
    let name = draft.validated_name()?;

    let mut connection = lock(db)?;

    Ok(store::create(&mut connection, &name)?)
}

pub fn rename_space(engine: &Engine, id: String, draft: SpaceDraft) -> Result<Space, AppError> {
    let db = &engine.db;
    let name = draft.validated_name()?;

    let mut connection = lock(db)?;

    Ok(store::rename(&mut connection, &id, &name)?)
}

pub fn pin_space(engine: &Engine, id: String, pinned: bool) -> Result<Space, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::set_pinned(&mut connection, &id, pinned)?)
}

/// What deleting the space would move and take, for the confirmation that comes first.
pub fn count_space_contents(engine: &Engine, id: String) -> Result<SpaceContents, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::contents(&mut connection, &id)?)
}

pub fn delete_space(engine: &Engine, id: String, target_space_id: String) -> Result<(), AppError> {
    engine.write(|db| {
        // A space as its own refuge would see its notes swept away by the cascade right
        // after the transfer.
        model::validate_move_target(&id, &target_space_id)?;

        let mut connection = lock(db)?;

        store::delete(&mut connection, &id, &target_space_id)?;
        let changed = Changed::everything();
        Ok(((), changed))
    })
}
