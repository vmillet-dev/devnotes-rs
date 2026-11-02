//! The commands the front end calls: one line each, over the engine's service functions. A data
//! function runs on the engine's db queue, a network one on its runtime, a tool off the window.
//!
//! ⚠️ Their doc comments are the engine's, copied: `bindings.ts` carries them to the front end.

pub mod attachments;
pub mod backup;
pub mod changelog;
pub mod folders;
pub mod http;
pub mod json;
pub mod libraries;
pub mod notes;
pub mod recovery;
pub mod spaces;
pub mod tools;
pub mod transfer;
pub mod vault;

use std::sync::Arc;

use engine::Engine;
use engine::error::AppError;

async fn run<T, F>(engine: &Arc<Engine>, job: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&Engine) -> Result<T, AppError> + Send + 'static,
{
    engine::db::run(Arc::clone(engine), job).await
}
