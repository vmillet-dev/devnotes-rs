//! The libraries panel of the File menu: which there are, and what is done to them. Opening one
//! is the gate's: it closes what is open and comes back asking for the other phrase.

use engine::libraries::registry::{LibraryEntry, Registry};
use i18n::{Message, keys};

use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::services::status::StatusNotifier;

/// Where a switch is going, kept while the question about unsaved requests is out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Goal {
    Open(String),
    Create(String),
}

enum Work {
    Renaming(Pending<()>),
    Deleting { name: String, done: Pending<()> },
}

#[derive(Default)]
pub struct LibrariesState {
    registry: Option<Registry>,
    asked: Option<Pending<Registry>>,
    work: Option<Work>,
    /// Named before it runs: a second click on the same button is the guard a double click
    /// defeats, and this erases a whole library.
    pending_deletion: Option<String>,
    goal: Option<Goal>,
}

impl LibrariesState {
    /// Read when the panel opens, and after anything that changes it.
    pub fn load(&mut self, backend: &Backend) {
        self.asked = Some(api::libraries::list(backend));
    }

    pub fn libraries(&self) -> &[LibraryEntry] {
        self.registry
            .as_ref()
            .map_or(&[], |registry| registry.libraries.as_slice())
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.registry
            .as_ref()
            .is_some_and(|registry| registry.open.as_deref() == Some(id))
    }

    pub fn is_working(&self) -> bool {
        self.work.is_some()
    }

    pub fn pending_deletion(&self) -> Option<&str> {
        self.pending_deletion.as_deref()
    }

    pub fn rename(&mut self, backend: &Backend, id: String, name: String) {
        if !self.is_working() {
            self.work = Some(Work::Renaming(api::libraries::rename(backend, id, name)));
        }
    }

    /// Proposes only. Neither the open one, whose files sit under a live connection, nor the
    /// last, which would leave the gate with nothing to offer; the core refuses both as well.
    pub fn ask_to_delete(&mut self, id: String) {
        if !self.is_open(&id) && self.libraries().len() > 1 {
            self.pending_deletion = Some(id);
        }
    }

    pub fn dismiss_deletion(&mut self) {
        self.pending_deletion = None;
    }

    pub fn confirm_deletion(&mut self, backend: &Backend) {
        if self.is_working() {
            return;
        }
        let Some(id) = self.pending_deletion.clone() else {
            return;
        };
        let name = self
            .libraries()
            .iter()
            .find(|entry| entry.id == id)
            .map_or_else(String::new, |entry| entry.name.clone());
        let done = api::libraries::delete(backend, id);
        self.work = Some(Work::Deleting { name, done });
    }

    pub fn hold(&mut self, goal: Goal) {
        self.goal = Some(goal);
    }

    pub fn take_goal(&mut self) -> Option<Goal> {
        self.goal.take()
    }

    pub fn update(
        &mut self,
        backend: &Backend,
        status: &mut StatusNotifier,
        errors: &mut ErrorNotifier,
    ) {
        if let Some(answer) = self.asked.as_mut().and_then(Pending::poll) {
            self.asked = None;
            match answer {
                Ok(registry) => self.registry = Some(registry),
                Err(error) => errors.report(keys::errors::LIBRARIES_LIST_FAILED, &error),
            }
        }
        let answer = match &mut self.work {
            Some(Work::Renaming(done) | Work::Deleting { done, .. }) => done.poll(),
            None => None,
        };
        let Some(answer) = answer else {
            return;
        };
        let finished = self.work.take();
        match answer {
            Ok(()) => {
                if let Some(Work::Deleting { name, .. }) = finished {
                    self.pending_deletion = None;
                    status.report(Message::new(keys::libraries::DELETED).with("name", name));
                }
                self.load(backend);
            }
            Err(error) => errors.report(keys::errors::LIBRARY_ACTION_FAILED, &error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    struct Bench {
        backend: Backend,
        _profile: tempfile::TempDir,
        libraries: LibrariesState,
        status: StatusNotifier,
        errors: ErrorNotifier,
    }

    impl Bench {
        fn new() -> Self {
            let (backend, profile) = testing::unlocked();
            let mut bench = Self {
                backend,
                _profile: profile,
                libraries: LibrariesState::default(),
                status: StatusNotifier::default(),
                errors: ErrorNotifier::default(),
            };
            bench.libraries.load(&bench.backend);
            bench.settle();
            bench
        }

        /// Until the answers, and the list read again behind them, are in.
        fn settle(&mut self) {
            while self.libraries.is_working() || self.libraries.asked.is_some() {
                self.backend.settle();
                self.libraries
                    .update(&self.backend, &mut self.status, &mut self.errors);
            }
        }

        fn add(&mut self, name: &str) -> String {
            let name = name.to_owned();
            let mut made = self
                .backend
                .db(move |engine| engine::libraries::create_library(engine, name));
            self.backend.settle();
            let id = made.poll().unwrap().unwrap().id;
            self.libraries.load(&self.backend);
            self.settle();
            id
        }

        fn open_id(&self) -> String {
            self.libraries
                .libraries()
                .iter()
                .find(|entry| self.libraries.is_open(&entry.id))
                .unwrap()
                .id
                .clone()
        }
    }

    #[test]
    fn a_library_is_renamed_and_listed_under_its_new_name() {
        let mut bench = Bench::new();
        let id = bench.open_id();

        bench
            .libraries
            .rename(&bench.backend, id, "Maison".to_owned());
        bench.settle();

        assert_eq!(bench.libraries.libraries()[0].name, "Maison");
    }

    #[test]
    fn neither_the_open_library_nor_the_last_is_offered_for_deletion() {
        let mut bench = Bench::new();
        let open = bench.open_id();

        bench.libraries.ask_to_delete(open.clone());
        assert_eq!(bench.libraries.pending_deletion(), None);

        bench.add("Boulot");
        bench.libraries.ask_to_delete(open);
        assert_eq!(bench.libraries.pending_deletion(), None);
    }

    #[test]
    fn a_deletion_is_named_then_confirmed_and_said() {
        let mut bench = Bench::new();
        let other = bench.add("Boulot");

        bench.libraries.ask_to_delete(other.clone());
        assert_eq!(bench.libraries.pending_deletion(), Some(other.as_str()));
        assert_eq!(bench.libraries.libraries().len(), 2);

        bench.libraries.confirm_deletion(&bench.backend);
        bench.settle();

        assert_eq!(bench.libraries.libraries().len(), 1);
        assert_eq!(bench.libraries.pending_deletion(), None);
        let (_, said) = bench.status.latest().unwrap();
        assert_eq!(said.key, keys::libraries::DELETED);
    }

    #[test]
    fn a_refused_rename_says_so_and_the_names_stay() {
        let mut bench = Bench::new();
        let before = bench.libraries.libraries()[0].name.clone();

        bench
            .libraries
            .rename(&bench.backend, "gone".to_owned(), "Maison".to_owned());
        bench.settle();

        assert!(bench.errors.notice().is_some());
        assert_eq!(bench.libraries.libraries()[0].name, before);
    }
}
