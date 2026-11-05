//! Drawings on their way to a file: each says how it went, once it is written or given up.

use std::sync::Arc;

use i18n::{Message, keys};
use platform::dialog::FileDialog;

use crate::api;
use crate::api::notes::Exported;
use crate::backend::{Backend, Pending};
use crate::notes::DrawingExport;
use crate::services::errors::ErrorNotifier;
use crate::services::status::StatusNotifier;

#[derive(Default)]
pub struct DrawingExports {
    writing: Vec<Pending<Exported>>,
}

impl DrawingExports {
    pub fn ask(
        &mut self,
        backend: &Backend,
        dialog: &Arc<dyn FileDialog + Send + Sync>,
        export: DrawingExport,
    ) {
        let writing = api::notes::export_drawing(backend, Arc::clone(dialog), export);
        self.writing.push(writing);
    }

    pub fn update(&mut self, status: &mut StatusNotifier, errors: &mut ErrorNotifier) {
        self.writing.retain_mut(|pending| {
            let Some(answer) = pending.poll() else {
                return true;
            };
            match answer {
                Ok(Exported::Saved(name)) => {
                    status.report(Message::new(keys::drawing::EXPORTED).with("name", name));
                }
                Ok(Exported::Cancelled) => {}
                Ok(Exported::Unwritable) => {
                    errors.notify(Message::new(keys::tools::files::UNWRITABLE));
                }
                Err(error) => errors.report(keys::drawing::EXPORT_FAILED, &error),
            }
            false
        });
    }
}
