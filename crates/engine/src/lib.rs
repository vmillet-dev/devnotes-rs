// A module of service functions is `pub`; the rest is `pub(crate)`, for `unreachable_pub`.
pub mod attachments;
pub mod backup;
pub mod changelog;
pub mod db;
pub mod error;
pub mod folders;
pub mod host;
pub mod http;
pub mod index;
pub mod json;
pub mod libraries;
pub mod notes;
pub mod recovery;
pub mod spaces;
pub mod tools;
pub mod transfer;
pub mod vault;

pub mod app_info;
pub(crate) mod closed_enum;
pub(crate) mod count;
pub(crate) mod durable;
pub mod layout;
pub(crate) mod name;

use std::sync::{Arc, Mutex, MutexGuard};

use error::StorageError;

pub use host::{EngineEvent, EventSink, Paths, RecordingSink};

/// The core's state, created once by the shell and shared behind an `Arc`.
pub struct Engine {
    pub(crate) queue: db::queue::DbQueue,
    pub(crate) db: db::Db,
    /// Behind its own lock, always taken after the library's.
    pub(crate) index: Mutex<index::NoteIndex>,
    pub(crate) restored: backup::Restored,
    pub(crate) sending: http::send::Sending,
    pub(crate) runs: http::runner::Runs,
    pub(crate) sockets: http::websocket::Sockets,
    pub(crate) paths: Paths,
    pub(crate) events: Arc<dyn EventSink>,
    /// The shell's: sends and sockets run on it.
    pub(crate) runtime: tokio::runtime::Handle,
}

impl Engine {
    /// Starts the db thread; no library is open: `vault::unlock` opens one.
    pub fn new(paths: Paths, events: Arc<dyn EventSink>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            queue: db::queue::DbQueue::start(),
            db: db::Db::new(None),
            index: Mutex::new(index::NoteIndex::default()),
            restored: backup::Restored::default(),
            sending: http::send::Sending::default(),
            runs: http::runner::Runs::default(),
            sockets: http::websocket::Sockets::default(),
            paths,
            events,
            runtime,
        }
    }

    /// For the tests, which open a library of their own.
    pub fn db(&self) -> &db::Db {
        &self.db
    }

    pub(crate) fn index(&self) -> Result<MutexGuard<'_, index::NoteIndex>, StorageError> {
        self.index.lock().map_err(|_| StorageError::Unavailable)
    }

    /// The one way to write what the index holds: `job` answers its value and what it changed,
    /// and the index reads that again once `job` has returned, its transactions committed. A
    /// refresh that fails leaves an index the next read builds whole, never a stale one.
    pub(crate) fn write<T>(
        &self,
        job: impl FnOnce(&db::Db) -> Result<(T, index::Changed), error::AppError>,
    ) -> Result<T, error::AppError> {
        let (value, changed) = job(&self.db)?;
        let mut library = db::lock(&self.db)?;
        let mut index = self.index()?;
        if let Err(error) = index.refresh(&mut library, &changed) {
            log::warn!("The notes index is read again from scratch: {error}");
            index.clear();
        }
        Ok(value)
    }

    /// Whether the index holds what the open library does: see `NoteIndex::matches`.
    #[doc(hidden)]
    pub fn index_matches_library(&self) -> Result<(), String> {
        let mut library = db::lock(&self.db).map_err(|error| error.to_string())?;
        let index = self.index().map_err(|error| error.to_string())?;
        index.matches(&mut library)
    }

    /// Wipes the index: the application is leaving, and its memory with it.
    pub fn wipe_index(&self) {
        if let Ok(mut index) = self.index.lock() {
            index.clear();
        }
    }

    /// How many notes the index holds, for the tests.
    #[doc(hidden)]
    pub fn indexed(&self) -> usize {
        self.index.lock().map_or(0, |index| index.len())
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    /// What runs for the library about to be closed, whose writes would go nowhere: the sockets,
    /// the runs and the sends. A send that still ends writes nothing (`Library::number`).
    pub(crate) fn let_go_of_the_network(&self) {
        if let Ok(mut index) = self.index.lock() {
            index.clear();
        }
        self.stop_network();
    }

    /// The sockets, the runs and the sends, as the window goes.
    pub fn stop_network(&self) {
        self.sockets.close_all();
        self.runs.cancel_all();
        self.sending.cancel_all();
    }

    pub(crate) fn emit(&self, event: EngineEvent) {
        self.events.emit(event);
    }
}

/// The launch chores, run after the unlock: nothing can be read before it, and none of
/// them is fatal.
pub(crate) fn sweep(engine: &Engine) {
    let db = &engine.db;

    // ⚠️ The copy before the sweeps: it is worth most holding what retention will purge.
    backup::take(&engine.paths, db);

    notes::trash::sweep_at_startup(db);
    http::history::sweep_at_startup(db);
    if let Err(error) = attachments::files::sweep_orphan_files(db) {
        log::warn!("Orphan attachment files not swept: {error}");
    }
    // The decrypted copies `open_attachment` wrote: another application may still hold
    // them on close, so they go at the next launch at the latest.
    attachments::sealed::sweep_plaintext(&engine.paths);
}

/// Started with this first argument, the application is a script host and nothing else: no
/// window, no library (`http::scripts::child`).
pub use http::scripts::child::ARG as SCRIPT_HOST_ARG;

/// The script host's `main`: its exit code.
pub fn serve_scripts() -> i32 {
    http::scripts::child::serve()
}
