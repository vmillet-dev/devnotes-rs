//! The changelog behind « What's new », parsed off the frame the first time it is asked for.

use engine::changelog::model::ChangelogRelease;

use crate::api;
use crate::backend::{Backend, Pending};

#[derive(Default)]
pub struct HelpState {
    asked: Option<Pending<Vec<ChangelogRelease>>>,
    releases: Option<Vec<ChangelogRelease>>,
    failed: bool,
}

impl HelpState {
    pub fn load(&mut self, backend: &Backend) {
        if self.releases.is_none() && self.asked.is_none() {
            self.failed = false;
            self.asked = Some(api::help::changelog(backend));
        }
    }

    pub fn is_loading(&self) -> bool {
        self.asked.is_some()
    }

    pub fn has_failed(&self) -> bool {
        self.failed
    }

    pub fn releases(&self) -> &[ChangelogRelease] {
        self.releases.as_deref().unwrap_or_default()
    }

    pub fn update(&mut self) {
        let Some(answer) = self.asked.as_mut().and_then(Pending::poll) else {
            return;
        };
        self.asked = None;
        match answer {
            Ok(releases) => self.releases = Some(releases),
            Err(_) => self.failed = true,
        }
    }
}
