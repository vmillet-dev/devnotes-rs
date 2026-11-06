//! The changelog baked into the binary.

use engine::changelog::{self, model::ChangelogRelease};

use crate::backend::{Backend, Pending};

pub fn changelog(backend: &Backend) -> Pending<Vec<ChangelogRelease>> {
    backend.compute(|_| Ok(changelog::app_changelog()))
}
