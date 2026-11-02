//! The service functions, called as the shell calls them, on an engine over a temporary
//! profile. What they add to the store is what these check: the lock, the validation before it,
//! and the code each failure answers.
//!
//! Not here: what reaches the profile (the gate, the registry, the launch copy), tested where
//! it is written.

use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use engine::db::{Library, open, open_in_memory, test_vault};
use engine::error::{AppError, ErrorCode};
use engine::spaces::model::SpaceDraft;
use engine::spaces::{create_space, list_spaces};
use engine::{Engine, Paths, RecordingSink};

#[path = "../common/mod.rs"]
mod common;

mod attachments;
mod backups;
mod concurrency;
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

/// An engine, what it said, the runtime its sends run on and the profile it was given.
struct Session {
    engine: Arc<Engine>,
    heard: Arc<RecordingSink>,
    runtime: tokio::runtime::Runtime,
    _profile: tempfile::TempDir,
}

impl Session {
    fn with(library: Option<Library>) -> Self {
        let profile = tempfile::tempdir().unwrap();
        let paths = Paths {
            profile: profile.path().to_path_buf(),
            resources: profile.path().to_path_buf(),
        };
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let heard = Arc::new(RecordingSink::default());
        let engine = Engine::new(paths, Arc::clone(&heard) as _, runtime.handle().clone());
        *engine.db().lock().unwrap() = library;

        Self {
            engine: Arc::new(engine),
            heard,
            runtime,
            _profile: profile,
        }
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

    /// A data function, on this thread as if it were the db thread: one at a time.
    fn call<T>(&self, job: impl FnOnce(&Engine) -> T) -> T {
        engine::db::run_here(&self.engine, job)
    }

    /// A network function, on the engine's runtime.
    fn wait<F: Future>(&self, job: impl FnOnce(Arc<Engine>) -> F) -> F::Output {
        self.runtime.block_on(job(Arc::clone(&self.engine)))
    }

    fn space(&self, name: &str) -> String {
        let draft = SpaceDraft {
            name: name.to_string(),
        };

        self.call(|e| create_space(e, draft)).unwrap().id
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
