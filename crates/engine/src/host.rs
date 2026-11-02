//! What the core asks of the shell it runs under: where its files are, and a way to tell the
//! interface what happened while nobody was asking.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::http::console::ConsoleEvent;
use crate::http::runner::RunEvent;
use crate::http::websocket::WebsocketEvent;

/// Resolved once by the shell, before the engine exists.
#[derive(Debug, Clone)]
pub struct Paths {
    /// The registry, `libraries/`, the application's `preferences.json`.
    pub profile: PathBuf,
    /// What ships beside the binary: the identity tool's lists.
    pub resources: PathBuf,
}

/// What the core says unasked: a socket's traffic, a script's console, a run's steps.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    Socket(WebsocketEvent),
    Console(ConsoleEvent),
    Run(RunEvent),
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: EngineEvent);
}

/// Keeps what it is told, for a test to read.
#[derive(Default)]
pub struct RecordingSink(Mutex<Vec<EngineEvent>>);

impl RecordingSink {
    pub fn events(&self) -> Vec<EngineEvent> {
        self.0
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default()
    }
}

impl EventSink for RecordingSink {
    fn emit(&self, event: EngineEvent) {
        if let Ok(mut events) = self.0.lock() {
            events.push(event);
        }
    }
}
