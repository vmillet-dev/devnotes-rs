//! The changelog baked into the binary.

use engine::changelog::{self, model::ChangelogRelease, model::ChangelogSection};

use crate::backend::{Backend, Pending};

pub fn changelog(backend: &Backend) -> Pending<Vec<ChangelogRelease>> {
    backend.compute(|_| Ok(changelog::app_changelog()))
}

/// What the updater hands over, read with the file's grammar: a release's few lines, on the frame.
pub fn release_notes(markdown: &str) -> Vec<ChangelogSection> {
    changelog::release_notes(markdown.to_owned())
}
