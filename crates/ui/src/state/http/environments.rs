//! The globals and the environments, and which one is active. The choice is the library's; one
//! deleted since it was chosen sends with the globals alone. A table typed in keeps its rows as
//! typed until they are written, once the keys rest; every other write is read back.

use engine::folders::model::FolderColour;
use engine::http::environments::{
    Environment, EnvironmentPatch, HttpEnvironments, Variable, VariableKind,
};
use i18n::keys;

use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::services::preferences::Preferences;
use crate::state::Revision;
use crate::state::query::QueryCell;

/// In the library's preferences: the environment a request is sent with belongs to its
/// collections.
pub const ACTIVE_KEY: &str = "devnotes.notes.http.environment";

/// How long the keys rest before a table is written, in seconds.
const SAVE_AFTER: f64 = 0.4;

/// The set the page shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Target {
    #[default]
    Globals,
    Environment(String),
}

/// What the page asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    Select(Target),
    Create(String),
    Rename {
        id: String,
        name: String,
    },
    Recolour {
        id: String,
        colour: FolderColour,
    },
    Protect {
        id: String,
        protect: bool,
    },
    Duplicate {
        id: String,
        name: String,
    },
    Delete(String),
    Use(String),
    /// The rows as typed in the set shown.
    Rows(Vec<Variable>),
}

/// The rows as typed, until they are read back.
struct Typed {
    target: Target,
    rows: Vec<Variable>,
    /// When the last key was typed, in seconds.
    at: f64,
    /// They have left: what is read next holds them.
    writing: bool,
}

#[derive(Default)]
pub struct Environments {
    sets: QueryCell<(Revision, Revision), HttpEnvironments>,
    /// Bumped by a write of this page: the tree need not be read again for it.
    written: Revision,
    active: Option<String>,
    target: Target,
    typed: Option<Typed>,
    /// Each answers the set to show once written, if another.
    writes: Vec<Pending<Option<String>>>,
    /// The tab whose send waits for a yes, under a protected environment.
    asking: Option<String>,
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

    pub fn update(
        &mut self,
        (ctx, backend): (&egui::Context, &Backend),
        (revision, now): (Revision, f64),
        errors: &mut ErrorNotifier,
    ) {
        self.adopt_writes(errors);
        self.write_when_rested(ctx, backend, now);
        self.sets.want((revision, self.written));
        let read = self.sets.update(|_| api::http::environments(backend));
        if let Some(error) = self.sets.error().filter(|_| read) {
            errors.report(keys::http::environments::FAILED, error);
        }
        let held = self.writes.is_empty() && !self.sets.is_loading();
        if held && self.typed.as_ref().is_some_and(|typed| typed.writing) {
            self.typed = None;
        }
    }

    fn adopt_writes(&mut self, errors: &mut ErrorNotifier) {
        let mut shown = None;
        let mut done = false;
        self.writes.retain_mut(|write| {
            let Some(answer) = write.poll() else {
                return true;
            };
            match answer {
                Ok(id) => shown = shown.take().or(id),
                Err(error) => errors.report(keys::http::environments::FAILED, &error),
            }
            done = true;
            false
        });
        if done {
            self.written.bump();
        }
        if let Some(id) = shown {
            self.target = Target::Environment(id);
        }
    }

    fn write_when_rested(&mut self, ctx: &egui::Context, backend: &Backend, now: f64) {
        let Some(typed) = self.typed.as_ref().filter(|typed| !typed.writing) else {
            return;
        };
        let due = typed.at + SAVE_AFTER;
        if now < due {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(due - now));
            return;
        }
        self.flush(backend);
    }

    /// What is still waiting is written now: the page is left, another set chosen.
    pub fn flush(&mut self, backend: &Backend) {
        let Some(typed) = self.typed.as_mut().filter(|typed| !typed.writing) else {
            return;
        };
        typed.writing = true;
        let rows = typed.rows.clone();
        let write = match &typed.target {
            Target::Globals => api::http::save_globals(backend, rows),
            Target::Environment(id) => {
                let patch = EnvironmentPatch {
                    variables: Some(rows),
                    ..EnvironmentPatch::default()
                };
                api::http::save_environment(backend, id.clone(), patch)
            }
        };
        self.writes.push(write);
    }

    pub fn ask(&mut self, backend: &Backend, asked: Asked, now: f64) {
        let patch =
            |patch: EnvironmentPatch, id: String| api::http::save_environment(backend, id, patch);
        match asked {
            Asked::Select(target) => {
                self.flush(backend);
                self.target = target;
            }
            Asked::Create(name) => {
                let write = api::http::create_environment(backend, name);
                self.writes.push(write);
            }
            Asked::Rename { id, name } => {
                let change = EnvironmentPatch {
                    name: Some(name),
                    ..EnvironmentPatch::default()
                };
                self.writes.push(patch(change, id));
            }
            Asked::Recolour { id, colour } => {
                let change = EnvironmentPatch {
                    colour: Some(colour),
                    ..EnvironmentPatch::default()
                };
                self.writes.push(patch(change, id));
            }
            Asked::Protect { id, protect } => {
                let change = EnvironmentPatch {
                    protected: Some(protect),
                    ..EnvironmentPatch::default()
                };
                self.writes.push(patch(change, id));
            }
            Asked::Duplicate { id, name } => {
                self.flush(backend);
                let write = api::http::duplicate_environment(backend, id, name);
                self.writes.push(write);
            }
            Asked::Delete(id) => self.delete(backend, id),
            Asked::Use(id) => self.choose(Some(id)),
            Asked::Rows(rows) => {
                self.typed = Some(Typed {
                    target: self.target.clone(),
                    rows,
                    at: now,
                    writing: false,
                });
            }
        }
    }

    fn delete(&mut self, backend: &Backend, id: String) {
        let gone = Target::Environment(id.clone());
        if self
            .typed
            .as_ref()
            .is_some_and(|typed| typed.target == gone)
        {
            self.typed = None;
        }
        if self.target == gone {
            self.target = Target::Globals;
        }
        if self.active.as_deref() == Some(id.as_str()) {
            self.active = None;
        }
        self.writes.push(api::http::delete_environment(backend, id));
    }

    /// A name nothing sets, given a row in the active environment — the globals with none — its
    /// value to type.
    pub fn define(&mut self, backend: &Backend, name: String, now: f64) {
        self.flush(backend);
        self.target = self.active().map_or(Target::Globals, |active| {
            Target::Environment(active.id.clone())
        });
        if self.rows().iter().any(|row| row.name == name) {
            return;
        }
        let mut rows = self.rows().to_vec();
        rows.push(Variable {
            name,
            kind: VariableKind::Text,
            ..Variable::default()
        });
        self.ask(backend, Asked::Rows(rows), now);
    }

    pub fn all(&self) -> &[Environment] {
        self.sets
            .value()
            .map_or(&[], |sets| sets.environments.as_slice())
    }

    pub fn globals(&self) -> &[Variable] {
        self.sets
            .value()
            .map_or(&[], |sets| sets.globals.as_slice())
    }

    pub fn is_loaded(&self) -> bool {
        self.sets.value().is_some()
    }

    pub fn active(&self) -> Option<&Environment> {
        let id = self.active.as_deref()?;
        self.all().iter().find(|known| known.id == id)
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active().map(|active| active.id.as_str())
    }

    pub fn choose(&mut self, id: Option<String>) {
        self.asking = None;
        self.active = id;
    }

    pub fn target(&self) -> &Target {
        &self.target
    }

    /// The environment the page shows; `None` while it shows the globals.
    pub fn shown(&self) -> Option<&Environment> {
        match &self.target {
            Target::Globals => None,
            Target::Environment(id) => self.all().iter().find(|known| known.id == *id),
        }
    }

    /// What the page's table shows: the rows as typed, until they are read back.
    pub fn rows(&self) -> &[Variable] {
        if let Some(typed) = self
            .typed
            .as_ref()
            .filter(|typed| typed.target == self.target)
        {
            return &typed.rows;
        }
        self.shown()
            .map_or_else(|| self.globals(), |shown| shown.variables.as_slice())
    }

    pub fn asking(&self) -> Option<&str> {
        self.asking.as_deref()
    }

    pub fn ask_before(&mut self, key: &str) {
        self.asking = Some(key.to_owned());
    }

    pub fn stop_asking(&mut self) {
        self.asking = None;
    }
}
