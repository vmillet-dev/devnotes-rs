//! The globals and the environments, and which one is active. The choice is the library's; one
//! deleted since it was chosen sends with the globals alone.

use engine::http::environments::{Environment, HttpEnvironments};

use crate::api;
use crate::backend::Backend;
use crate::services::preferences::Preferences;
use crate::state::Revision;
use crate::state::query::QueryCell;

/// In the library's preferences: the environment a request is sent with belongs to its
/// collections.
pub const ACTIVE_KEY: &str = "devnotes.notes.http.environment";

#[derive(Default)]
pub struct Environments {
    sets: QueryCell<Revision, HttpEnvironments>,
    active: Option<String>,
}

impl Environments {
    pub fn restore(&mut self, preferences: &Preferences) {
        self.active = preferences
            .get(ACTIVE_KEY)
            .filter(|id| !id.is_empty())
            .map(str::to_owned);
    }

    pub fn remember(&self, preferences: &mut Preferences) {
        preferences.set(ACTIVE_KEY, self.active.as_deref().unwrap_or(""));
    }

    pub fn update(&mut self, backend: &Backend, revision: Revision) {
        self.sets.want(revision);
        self.sets.update(|_| api::http::environments(backend));
    }

    pub fn all(&self) -> &[Environment] {
        self.sets
            .value()
            .map_or(&[], |sets| sets.environments.as_slice())
    }

    pub fn active(&self) -> Option<&Environment> {
        let id = self.active.as_deref()?;
        self.all().iter().find(|known| known.id == id)
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active().map(|active| active.id.as_str())
    }

    pub fn choose(&mut self, id: Option<String>) {
        self.active = id;
    }
}
