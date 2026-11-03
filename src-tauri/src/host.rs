//! What the engine asks of Tauri: the paths it resolves, and its events under the names the
//! front end listens to.

use engine::http::console::HTTP_CONSOLE_EVENT;
use engine::http::runner::HTTP_RUN_EVENT;
use engine::http::websocket::WEBSOCKET_EVENT;
use engine::{EngineEvent, EventSink, Paths};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub(crate) fn paths<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Paths> {
    Ok(Paths {
        profile: app.path().app_data_dir()?,
        resources: app.path().resource_dir()?,
    })
}

pub(crate) struct TauriHost<R: Runtime>(pub(crate) AppHandle<R>);

impl<R: Runtime> EventSink for TauriHost<R> {
    fn emit(&self, event: EngineEvent) {
        let sent = match event {
            EngineEvent::Socket(event) => self.0.emit(WEBSOCKET_EVENT, event),
            EngineEvent::Console(event) => self.0.emit(HTTP_CONSOLE_EVENT, event),
            EngineEvent::Run(event) => self.0.emit(HTTP_RUN_EVENT, event),
            // The front end does not wait on the index: a query finishes it.
            EngineEvent::IndexProgress { .. } | EngineEvent::IndexReady => return,
        };
        if let Err(error) = sent {
            log::warn!("An engine event did not reach the window: {error}");
        }
    }
}
