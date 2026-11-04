//! The tags of the whole library, managed: a tag that drifts drifts everywhere. Nothing runs
//! before saying what it touches — the notes counted by the core, distinctly: a note carrying two
//! of the tags is one note.

use std::collections::BTreeSet;

use engine::notes::model::TagUsage;
use i18n::keys;

use super::Revision;
use crate::api;
use crate::backend::{Backend, Pending};
use crate::notes::Tags;
use crate::services::errors::ErrorNotifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Rename,
    /// Renaming several onto one: the only kind nothing can undo.
    Merge,
    Delete,
}

/// A change waiting to be confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagChange {
    pub kind: ChangeKind,
    pub tags: Vec<String>,
    /// Empty for a deletion.
    pub into: String,
    pub notes: u32,
}

#[derive(Default)]
pub struct TagsState {
    open: bool,
    loading: Option<Pending<Vec<TagUsage>>>,
    tags: Vec<TagUsage>,
    selected: BTreeSet<String>,
    pending: Option<TagChange>,
    /// Counted before it is proposed: a count that cannot be read proposes nothing.
    proposing: Option<(TagChange, Pending<u32>)>,
    writes: Vec<Pending<u32>>,
}

impl TagsState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn is_loading(&self) -> bool {
        self.loading.is_some() && self.tags.is_empty()
    }

    pub fn tags(&self) -> &[TagUsage] {
        &self.tags
    }

    pub fn is_selected(&self, tag: &str) -> bool {
        self.selected.contains(tag)
    }

    pub fn selected(&self) -> usize {
        self.selected.len()
    }

    pub fn pending(&self) -> Option<&TagChange> {
        self.pending.as_ref()
    }

    pub fn apply(&mut self, asked: Tags, backend: &Backend) {
        match asked {
            Tags::Open => {
                self.open = true;
                self.selected.clear();
                self.pending = None;
                self.load(backend);
            }
            Tags::Close => {
                self.open = false;
                self.pending = None;
                self.proposing = None;
            }
            // Another selection withdraws what was proposed about the last one.
            Tags::Toggle(tag) => {
                self.pending = None;
                if !self.selected.remove(&tag) {
                    self.selected.insert(tag);
                }
            }
            // Renaming onto an existing tag is a merge: a note cannot carry one twice.
            Tags::Rename(into) => {
                let into = into.trim().to_owned();
                if into.is_empty() {
                    return;
                }
                let kind = if self.selected.len() == 1 {
                    ChangeKind::Rename
                } else {
                    ChangeKind::Merge
                };
                self.propose(backend, kind, into);
            }
            Tags::Delete => self.propose(backend, ChangeKind::Delete, String::new()),
            Tags::Cancel => self.pending = None,
            Tags::Confirm => {
                let Some(change) = self.pending.take() else {
                    return;
                };
                self.writes.push(match change.kind {
                    ChangeKind::Delete => api::notes::delete_tags(backend, change.tags),
                    ChangeKind::Rename | ChangeKind::Merge => {
                        api::notes::rename_tags(backend, change.tags, change.into)
                    }
                });
            }
        }
    }

    fn propose(&mut self, backend: &Backend, kind: ChangeKind, into: String) {
        if self.selected.is_empty() {
            return;
        }
        let tags: Vec<String> = self.selected.iter().cloned().collect();
        let counting = api::notes::count_tagged(backend, tags.clone());
        let change = TagChange {
            kind,
            tags,
            into,
            notes: 0,
        };
        self.proposing = Some((change, counting));
    }

    pub fn update(
        &mut self,
        backend: &Backend,
        revision: &mut Revision,
        errors: &mut ErrorNotifier,
    ) {
        if let Some(answer) = self.loading.as_mut().and_then(Pending::poll) {
            self.loading = None;
            match answer {
                Ok(tags) => self.tags = tags,
                Err(error) => errors.report(keys::errors::TAGS_LOAD_FAILED, &error),
            }
        }
        if let Some(answer) = self
            .proposing
            .as_mut()
            .and_then(|(_, counting)| counting.poll())
        {
            let (change, _) = self.proposing.take().expect("polled just now");
            match answer {
                Ok(notes) => self.pending = Some(TagChange { notes, ..change }),
                Err(error) => errors.report(keys::errors::TAG_ACTION_FAILED, &error),
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
                errors.report(keys::errors::TAG_ACTION_FAILED, &error);
                false
            }
        });
        // Retagging rewrites the library: the rails and the cards are both stale.
        if written {
            revision.bump();
            self.selected.clear();
            self.pending = None;
            if self.open {
                self.load(backend);
            }
        }
    }

    fn load(&mut self, backend: &Backend) {
        self.loading = Some(api::notes::list_tags(backend));
    }
}
