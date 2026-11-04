//! The trash, loaded when its panel opens: a permanent query would run on every deletion. What
//! the core matched is what is shown; restoring all reaches what the search found.

use std::time::Duration;

use engine::notes::trash::TrashedNote;
use i18n::keys;

use super::Revision;
use super::notes_query::DEBOUNCE;
use crate::api;
use crate::backend::{Backend, Pending};
use crate::notes::Trash;
use crate::services::errors::ErrorNotifier;

#[derive(Default)]
pub struct TrashState {
    open: bool,
    search: String,
    /// When the search typed is asked, on egui's clock.
    due: Option<f64>,
    loading: Option<Pending<Vec<TrashedNote>>>,
    notes: Vec<TrashedNote>,
    writes: Vec<Pending<u32>>,
}

impl TrashState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn notes(&self) -> &[TrashedNote] {
        &self.notes
    }

    /// Nothing answered yet since it opened.
    pub fn is_loading(&self) -> bool {
        self.loading.is_some() && self.notes.is_empty()
    }

    pub fn search(&self) -> &str {
        &self.search
    }

    pub fn searching(&self) -> bool {
        !self.search.trim().is_empty()
    }

    pub fn apply(&mut self, asked: Trash, backend: &Backend, now: f64) {
        match asked {
            Trash::Open => self.open(backend),
            Trash::Close => self.close(),
            Trash::Search(typed) => {
                self.search = typed;
                self.due = Some(now + DEBOUNCE);
            }
            Trash::Restore(id) => self.writes.push(api::notes::restore(backend, vec![id])),
            Trash::Purge(id) => self.writes.push(api::notes::purge(backend, vec![id])),
            // The way out of a mistaken bulk deletion.
            Trash::RestoreAll => {
                self.due = None;
                let search = self.search.trim().to_owned();
                self.writes.push(api::notes::restore_trash(backend, search));
            }
            Trash::Empty => self.writes.push(api::notes::empty_trash(backend)),
        }
    }

    fn open(&mut self, backend: &Backend) {
        self.open = true;
        self.search.clear();
        self.notes.clear();
        self.load(backend);
    }

    fn close(&mut self) {
        self.open = false;
        self.due = None;
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        backend: &Backend,
        revision: &mut Revision,
        errors: &mut ErrorNotifier,
    ) {
        if let Some(due) = self.due {
            let time = ctx.input(|input| input.time);
            if time >= due {
                self.due = None;
                self.load(backend);
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(due - time));
            }
        }
        if let Some(answer) = self.loading.as_mut().and_then(Pending::poll) {
            self.loading = None;
            match answer {
                Ok(notes) => self.notes = notes,
                Err(error) => errors.report(keys::errors::TRASH_LOAD_FAILED, &error),
            }
        }
        let mut written = false;
        self.writes.retain_mut(|pending| match pending.poll() {
            None => true,
            Some(Ok(_)) => {
                written = true;
                false
            }
            Some(Err(error)) => {
                errors.report(keys::errors::TRASH_ACTION_FAILED, &error);
                false
            }
        });
        if written {
            revision.bump();
            if self.open {
                self.load(backend);
            }
        }
    }

    /// The core purges what the retention caught up with before it answers.
    fn load(&mut self, backend: &Backend) {
        let search = self.search.trim().to_owned();
        self.loading = Some(api::notes::list_trash(backend, search));
    }
}
