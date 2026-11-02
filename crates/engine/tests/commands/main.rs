//! The commands themselves, called as the interface calls them: through a Tauri app on the mock
//! runtime, the library in its `Db`. What they add to the store is what these check: the lock,
//! the validation before it, and the code each failure crosses the bridge as.
//!
//! Not here: the commands that reach the profile (the gate, the registry, the launch copy) or a
//! plugin (the opener, the clipboard). The mock app has neither a profile of its own nor them.

use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use engine::db::{Library, open, open_in_memory, test_vault};
use engine::error::{AppError, ErrorCode};
use engine::http::console::HTTP_CONSOLE_EVENT;
use engine::http::runner::HTTP_RUN_EVENT;
use engine::http::websocket::WEBSOCKET_EVENT;
use engine::spaces::model::SpaceDraft;
use engine::spaces::{create_space, list_spaces};
use engine::{Engine, EngineEvent, EventSink, Paths};
use tauri::test::{MockRuntime, mock_app};
use tauri::{App, AppHandle, Emitter, Manager};

#[path = "../common/mod.rs"]
mod common;

mod attachments;
mod backups;
mod folders;
mod http;
mod http_environments;
mod http_runner;
mod http_scripts;
mod http_send;
mod notes;
mod profile;
mod spaces;
mod transfer;
mod websocket;

/// The app is kept, not only its handle: the state goes with it. Beside it, the profile the
/// engine was given.
struct Session(
    App<MockRuntime>,
    #[allow(dead_code, reason = "held so the directory outlives the engine")] tempfile::TempDir,
);

/// What the engine says, emitted on the mock app under the names the shell gives it.
struct Emitted(AppHandle<MockRuntime>);

impl EventSink for Emitted {
    fn emit(&self, event: EngineEvent) {
        let _ = match event {
            EngineEvent::Socket(event) => self.0.emit(WEBSOCKET_EVENT, event),
            EngineEvent::Console(event) => self.0.emit(HTTP_CONSOLE_EVENT, event),
            EngineEvent::Run(event) => self.0.emit(HTTP_RUN_EVENT, event),
        };
    }
}

impl Session {
    fn with(library: Option<Library>) -> Self {
        let profile = tempfile::tempdir().unwrap();
        let paths = Paths {
            profile: profile.path().to_path_buf(),
            resources: profile.path().to_path_buf(),
        };
        let app = mock_app();
        let engine = Engine::new(paths, Arc::new(Emitted(app.handle().clone())));
        *engine.db().lock().unwrap() = library;
        app.manage(Arc::new(engine));

        Self(app, profile)
    }

    fn open() -> Self {
        Self::with(Some(open_in_memory().unwrap()))
    }

    fn locked() -> Self {
        Self::with(None)
    }

    /// A library on disk, for the commands that write beside it.
    fn on_disk(directory: &Path) -> Self {
        std::fs::create_dir_all(directory).unwrap();
        let library = open(&directory.join("notes.db"), test_vault().unwrap()).unwrap();
        std::fs::create_dir_all(engine::attachments::files::directory(&library)).unwrap();

        Self::with(Some(library))
    }

    fn call<T, F>(&self, command: impl FnOnce(AppHandle<MockRuntime>) -> F) -> T
    where
        F: Future<Output = T>,
    {
        tauri::async_runtime::block_on(command(self.0.handle().clone()))
    }

    fn space(&self, name: &str) -> String {
        let draft = SpaceDraft {
            name: name.to_string(),
        };

        self.call(|app| create_space(draft, app)).unwrap().id
    }

    fn spaces(&self) -> Vec<String> {
        self.call(list_spaces)
            .unwrap()
            .into_iter()
            .map(|space| space.name)
            .collect()
    }
}

fn code<T: std::fmt::Debug>(answer: Result<T, AppError>) -> ErrorCode {
    answer.unwrap_err().code
}
