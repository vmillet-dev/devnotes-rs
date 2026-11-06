//! Import, export and share. The file dialogs block until answered: off the frame, on the pool.

use std::path::PathBuf;
use std::sync::Arc;

use engine::transfer::{
    self,
    model::{ExportReport, ExportScope, ImportReport},
};
use platform::dialog::FileDialog;

use crate::backend::{Backend, Pending};

/// `json` stays on the way in: an export written before the archive is still importable.
const READABLE: [&str; 2] = ["devnotes", "json"];

pub fn pick_bundle(
    backend: &Backend,
    dialog: Arc<dyn FileDialog + Send + Sync>,
) -> Pending<Option<PathBuf>> {
    backend.compute(move |_| Ok(dialog.open_of(&READABLE)))
}

pub fn choose_destination(
    backend: &Backend,
    dialog: Arc<dyn FileDialog + Send + Sync>,
    name: String,
) -> Pending<Option<PathBuf>> {
    backend.compute(move |_| Ok(dialog.save(&name)))
}

pub fn count(backend: &Backend, scope: ExportScope) -> Pending<u32> {
    backend.db(move |engine| transfer::count_export(engine, scope))
}

pub fn export(
    backend: &Backend,
    path: String,
    scope: ExportScope,
    passphrase: Option<String>,
) -> Pending<ExportReport> {
    backend.db(move |engine| transfer::export_notes(engine, path, scope, passphrase))
}

pub fn is_protected(backend: &Backend, path: String) -> Pending<bool> {
    backend.compute(move |_| transfer::export_is_protected(path))
}

pub fn import(
    backend: &Backend,
    path: String,
    passphrase: Option<String>,
) -> Pending<ImportReport> {
    backend.db(move |engine| transfer::import_notes(engine, path, passphrase))
}

pub fn share(backend: &Backend, ids: Vec<String>) -> Pending<String> {
    backend.db(move |engine| transfer::share_notes(engine, ids))
}
