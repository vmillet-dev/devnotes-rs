//! The commands themselves, called as the interface calls them: through a Tauri app on the mock
//! runtime, the library in its `Db`. What they add to the store is what these check: the lock,
//! the validation before it, and the code each failure crosses the bridge as.
//!
//! Not here: the commands that reach the profile (the gate, the registry, the copies) or a
//! plugin (the opener, the clipboard). The mock app has neither a profile of its own nor them.

use std::future::Future;
use std::path::Path;

use devnotes_lib::db::{Db, Library, open, open_in_memory, test_vault};
use devnotes_lib::error::{AppError, ErrorCode};
use devnotes_lib::spaces::model::SpaceDraft;
use devnotes_lib::spaces::{create_space, list_spaces};
use tauri::test::{MockRuntime, mock_app};
use tauri::{App, AppHandle, Manager};

#[path = "../common/mod.rs"]
mod common;

mod attachments;
mod folders;
mod notes;
mod spaces;
mod transfer;

/// The app is kept, not only its handle: the state goes with it.
struct Session(App<MockRuntime>);

impl Session {
    fn with(library: Option<Library>) -> Self {
        let app = mock_app();
        app.manage(Db::new(library));
        Self(app)
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
        std::fs::create_dir_all(devnotes_lib::attachments::files::directory(&library)).unwrap();

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
