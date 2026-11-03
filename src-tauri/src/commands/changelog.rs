#![allow(clippy::needless_pass_by_value)]

use engine::changelog;
use engine::changelog::model::ChangelogRelease;
use engine::changelog::model::ChangelogSection;

#[tauri::command(async)]
#[specta::specta]
pub fn app_changelog() -> Vec<ChangelogRelease> {
    changelog::app_changelog()
}

/// The notes the updater hands over, read with the same grammar as the file.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn release_notes(markdown: String) -> Vec<ChangelogSection> {
    changelog::release_notes(markdown)
}
