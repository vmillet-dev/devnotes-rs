//! What was sent: the workspace shows it in place of the tabs while it is open. Read again each
//! time a send ends, and after it is emptied. Rust masks before sealing: nothing here sees more.

use chrono::{DateTime, Utc};
use engine::http::history::{HistoryDay, HistoryDraft, HistoryEntry};
use i18n::keys;

use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::state::Revision;
use crate::state::query::QueryCell;
use crate::state::undo::UNDO_WINDOW;

/// What the page asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    Select(String),
    Delete(String),
    Undo,
    DismissUndo,
    AskClear,
    CancelClear,
    Clear,
    /// `true` once the question over a tab's changes was answered.
    Reopen(bool),
    CancelReplace,
}

/// Back in the request it came from, or a draft of its own.
pub enum Reopened {
    Onto {
        request_id: String,
        draft: HistoryDraft,
    },
    Draft {
        name: String,
        draft: HistoryDraft,
    },
}

enum Reopening {
    /// The send to rebuild as a draft should the request have gone meanwhile.
    Onto {
        request_id: String,
        entry_id: String,
        sent_at: DateTime<Utc>,
        pending: Pending<Option<HistoryDraft>>,
    },
    Draft {
        sent_at: DateTime<Utc>,
        pending: Pending<HistoryDraft>,
    },
}

/// A draft is named after the send and its time.
fn named(draft: HistoryDraft, sent_at: DateTime<Utc>) -> Reopened {
    let time = sent_at.with_timezone(&chrono::Local).format("%H:%M");
    Reopened::Draft {
        name: format!("{} · {time}", draft.name),
        draft,
    }
}

#[derive(Default)]
pub struct History {
    read: Revision,
    days: QueryCell<(Revision, Revision), Vec<HistoryDay>>,
    selected: Option<String>,
    entry: QueryCell<String, HistoryEntry>,
    deleting: Vec<Pending<HistoryEntry>>,
    /// The send just deleted, for as long as its undo is offered, and until when.
    removed: Option<(HistoryEntry, f64)>,
    restoring: Option<Pending<()>>,
    counting: Option<Pending<u32>>,
    confirming: Option<u32>,
    clearing: Option<Pending<u32>>,
    /// The name of the request whose changes a reopening would replace.
    replacing: Option<String>,
    reopening: Option<Reopening>,
}

impl History {
    /// Once a frame while the area is shown. Answers how many sends « Clear » removed, and what
    /// a reopening rebuilt.
    pub fn update(
        &mut self,
        (ctx, backend): (&egui::Context, &Backend),
        (open, revision, now, tz_offset): (bool, Revision, f64, i32),
        errors: &mut ErrorNotifier,
    ) -> (Option<u32>, Option<Reopened>) {
        self.adopt_deletions(now, errors);
        if let Some((_, until)) = self.removed {
            if now >= until {
                self.removed = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(until - now));
            }
        }
        let cleared = self.adopt_clearing(errors);
        let reopened = self.adopt_reopening(backend, errors);
        if open {
            self.days.want((self.read, revision));
            if self.days.update(|_| api::http::history(backend, tz_offset))
                && let Some(error) = self.days.error()
            {
                errors.report(keys::http::history::FAILED, error);
            }
        }
        if let Some(id) = &self.selected {
            self.entry.want(id.clone());
            if self
                .entry
                .update(|id| api::http::history_entry(backend, id))
                && let Some(error) = self.entry.error()
            {
                errors.report(keys::http::history::FAILED, error);
            }
        }
        (cleared, reopened)
    }

    fn adopt_deletions(&mut self, now: f64, errors: &mut ErrorNotifier) {
        let mut removed = None;
        self.deleting.retain_mut(|deleting| {
            let Some(answer) = deleting.poll() else {
                return true;
            };
            match answer {
                Ok(entry) => removed = Some(entry),
                Err(error) => errors.report(keys::http::history::FAILED, &error),
            }
            false
        });
        if let Some(entry) = removed {
            if self.selected.as_deref() == Some(entry.item.id.as_str()) {
                self.selected = None;
            }
            self.removed = Some((entry, now + UNDO_WINDOW));
            self.read.bump();
        }
        if let Some(pending) = &mut self.restoring
            && let Some(answer) = pending.poll()
        {
            self.restoring = None;
            match answer {
                Ok(()) => self.read.bump(),
                Err(error) => errors.report(keys::errors::UNDO_FAILED, &error),
            }
        }
    }

    fn adopt_clearing(&mut self, errors: &mut ErrorNotifier) -> Option<u32> {
        if let Some(pending) = &mut self.counting
            && let Some(answer) = pending.poll()
        {
            self.counting = None;
            match answer {
                Ok(count) => self.confirming = Some(count),
                Err(error) => errors.report(keys::http::history::FAILED, &error),
            }
        }
        let answer = self.clearing.as_mut()?.poll()?;
        self.clearing = None;
        match answer {
            Ok(count) => {
                self.selected = None;
                self.read.bump();
                Some(count)
            }
            Err(error) => {
                errors.report(keys::http::history::FAILED, &error);
                None
            }
        }
    }

    fn adopt_reopening(
        &mut self,
        backend: &Backend,
        errors: &mut ErrorNotifier,
    ) -> Option<Reopened> {
        match self.reopening.as_mut()? {
            Reopening::Onto {
                request_id,
                entry_id,
                sent_at,
                pending,
            } => {
                let answer = pending.poll()?;
                let (request_id, entry_id, sent_at) =
                    (request_id.clone(), entry_id.clone(), *sent_at);
                self.reopening = None;
                match answer {
                    Ok(Some(draft)) => return Some(Reopened::Onto { request_id, draft }),
                    Ok(None) => self.reopen_draft(backend, entry_id, sent_at),
                    Err(error) => errors.report(keys::http::history::FAILED, &error),
                }
                None
            }
            Reopening::Draft { sent_at, pending } => {
                let answer = pending.poll()?;
                let sent_at = *sent_at;
                self.reopening = None;
                match answer {
                    Ok(draft) => Some(named(draft, sent_at)),
                    Err(error) => {
                        errors.report(keys::http::history::FAILED, &error);
                        None
                    }
                }
            }
        }
    }

    pub fn ask(&mut self, backend: &Backend, asked: Asked) {
        match asked {
            Asked::Select(id) => {
                self.selected = Some(id);
                self.replacing = None;
            }
            Asked::Delete(id) => self.deleting.push(api::http::delete_history(backend, id)),
            Asked::Undo => {
                if let Some((entry, _)) = self.removed.take() {
                    self.restoring = Some(api::http::restore_history(backend, entry));
                }
            }
            Asked::DismissUndo => self.removed = None,
            Asked::AskClear => self.counting = Some(api::http::count_history(backend)),
            Asked::CancelClear => self.confirming = None,
            Asked::Clear => {
                self.confirming = None;
                self.clearing = Some(api::http::clear_history(backend));
            }
            Asked::CancelReplace => self.replacing = None,
            // The state above holds the tabs and the tree it takes to decide.
            Asked::Reopen(_) => {}
        }
    }

    pub fn ask_replace(&mut self, name: String) {
        self.replacing = Some(name);
    }

    pub fn reopen_onto(&mut self, backend: &Backend, request_id: String, entry: &HistoryEntry) {
        self.replacing = None;
        let id = entry.item.id.clone();
        self.reopening = Some(Reopening::Onto {
            request_id,
            entry_id: id.clone(),
            sent_at: entry.item.sent_at,
            pending: api::http::history_onto(backend, id),
        });
    }

    pub fn reopen_draft(&mut self, backend: &Backend, id: String, sent_at: DateTime<Utc>) {
        self.replacing = None;
        self.reopening = Some(Reopening::Draft {
            sent_at,
            pending: api::http::history_draft(backend, id),
        });
    }

    pub fn days(&self) -> &[HistoryDay] {
        self.days.value().map_or(&[], Vec::as_slice)
    }

    pub fn is_loaded(&self) -> bool {
        self.days.value().is_some()
    }

    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// The entry chosen, once read: the one before it stays no longer than its selection.
    pub fn entry(&self) -> Option<&HistoryEntry> {
        let selected = self.selected.as_deref()?;
        self.entry.value().filter(|entry| entry.item.id == selected)
    }

    pub fn removed(&self) -> Option<&HistoryEntry> {
        self.removed.as_ref().map(|(entry, _)| entry)
    }

    pub fn confirming(&self) -> Option<u32> {
        self.confirming
    }

    pub fn replacing(&self) -> Option<&str> {
        self.replacing.as_deref()
    }
}
