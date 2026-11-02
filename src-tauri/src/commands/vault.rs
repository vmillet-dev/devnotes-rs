#![allow(clippy::needless_pass_by_value)]

use super::run;
use engine::error::AppError;
use engine::vault::PassphraseChange;
use engine::vault::VaultState;
use engine::{Engine, index, vault};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn vault_state(engine: State<'_, Arc<Engine>>) -> Result<VaultState, AppError> {
    run(&engine, vault::vault_state).await
}

/// The first launch. Refuses a library that already has a key file: that file is the only
/// way into the notes beside it.
#[tauri::command]
#[specta::specta]
pub async fn create_vault(
    passphrase: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    let passphrase = vault::secret(passphrase);
    run(&engine, move |engine| {
        vault::create_vault(engine, passphrase)
    })
    .await?;
    tauri::async_runtime::spawn(index::fill(Arc::clone(&engine)));
    Ok(())
}

/// Off the main thread: deriving the key is slow on purpose, and would freeze the window over
/// every attempt.
#[tauri::command]
#[specta::specta]
pub async fn unlock_vault(
    passphrase: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    let passphrase = vault::secret(passphrase);
    run(&engine, move |engine| {
        vault::unlock_vault(engine, passphrase)
    })
    .await?;
    // The index fills behind the gate, a slice a job: the canvas does not wait for it.
    tauri::async_runtime::spawn(index::fill(Arc::clone(&engine)));
    Ok(())
}

/// A new phrase over the same library, from the preferences panel.
///
/// Not a re-encryption: the key the notes are sealed with is rewrapped, and the library stays
/// open on it.
#[tauri::command]
#[specta::specta]
pub async fn change_passphrase(
    current: String,
    next: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<PassphraseChange, AppError> {
    let (current, next) = (vault::secret(current), vault::secret(next));
    run(&engine, move |engine| {
        vault::change_passphrase(engine, current, next)
    })
    .await
}
