//! The changelog and the documentation baked into the binary.

use engine::changelog::{self, model::ChangelogRelease, model::ChangelogSection};
use engine::docs::{self, model::Index, model::Page, search::Hit};
use i18n::Language;

use crate::backend::{Backend, Pending};

pub fn changelog(backend: &Backend) -> Pending<Vec<ChangelogRelease>> {
    backend.compute(|_| Ok(changelog::app_changelog()))
}

/// What the updater hands over, read with the file's grammar: a release's few lines, on the frame.
pub fn release_notes(markdown: &str) -> Vec<ChangelogSection> {
    changelog::release_notes(markdown)
}

pub fn docs_index() -> &'static Index {
    docs::docs_index()
}

/// A page of a few kilobytes, read once: on the frame.
pub fn docs_page(language: Language, id: &str) -> Option<&'static Page> {
    docs::docs_page(language.code(), id)
}

/// Eleven pages already read: on the frame, when the words change.
pub fn search_docs(language: Language, query: &str) -> Vec<Hit> {
    docs::search::search_docs(language.code(), query)
}
