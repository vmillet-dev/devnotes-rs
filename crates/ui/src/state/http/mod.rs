//! The HTTP area's state: the collections as the rail shows them. Every write is persisted, then
//! the tree read again: the core renumbers what a move or a copy touched, and the interface never
//! guesses the result.

pub mod moves;
pub mod tabs;
pub mod tree;

use std::collections::BTreeSet;

use engine::http::VariablesRequest;
use engine::http::model::{HttpContents, HttpItem, HttpTree, RequestDocument, RequestKind};
use engine::http::send::ImpliedHeader;
use engine::http::variables::UsedVariable;
use i18n::{Message, keys};

use super::{QueryCell, Revision};
use crate::api;
use crate::api::http::Written;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::services::preferences::Preferences;
use moves::{KeyMove, RailMove, Refusal, Where};
use tree::RailRow;

/// In the library's preferences: which folds were left closed belongs to its collections.
pub const COLLAPSED_KEY: &str = "devnotes.notes.http.collapsed";

/// How long a move made with the keys stays marked, in seconds: as long as its fade.
pub const MARK_SECONDS: f64 = 2.0;

/// A write the rail asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Write {
    CreateCollection(String),
    /// At the collection's root when `folder_id` is `None`.
    CreateFolder {
        collection_id: String,
        folder_id: Option<String>,
        name: String,
    },
    CreateRequest {
        collection_id: String,
        folder_id: Option<String>,
        name: String,
        kind: RequestKind,
    },
    Rename {
        item: HttpItem,
        name: String,
    },
    Duplicate {
        item: HttpItem,
        name: String,
    },
    Delete(HttpItem),
    Move(RailMove),
}

/// A move made with the keys, waiting for the tree that says where it landed.
struct Landing {
    item: HttpItem,
    key: KeyMove,
    name: String,
}

struct Writing {
    pending: Pending<Written>,
    landing: Option<Landing>,
}

/// A move on screen: the row, what now holds it, and since when (egui's time).
#[derive(Debug, Clone, PartialEq)]
pub struct Mark {
    pub id: String,
    pub parent_id: Option<String>,
    pub since: f64,
}

/// What the editor's answers depend on: the draft in front, where it sits, what it inherits.
#[derive(Debug, Clone, PartialEq)]
struct Question {
    kind: RequestKind,
    document: RequestDocument,
    place: Option<tabs::Place>,
    revision: Revision,
}

/// A deletion asked: what it takes with it, read before the question is shown.
pub struct Deleting {
    pub item: HttpItem,
    pending: Option<Pending<HttpContents>>,
    pub contents: Option<HttpContents>,
}

#[derive(Default)]
pub struct HttpState {
    /// Bumped by the area's own writes: the notes are not asked again for a request renamed.
    revision: Revision,
    tree: QueryCell<Revision, HttpTree>,
    collapsed: BTreeSet<String>,
    /// Laid out again when the tree or a fold changes, not every frame: 2,000 rows.
    rows: Vec<RailRow>,
    writes: Vec<Writing>,
    /// Said once the tree after the write is read.
    landing: Option<(Revision, Landing)>,
    mark: Option<Mark>,
    /// The last sentence the rail's live region says, and how many were said before it.
    said: Option<(u32, Message)>,
    deleting: Option<Deleting>,
    tabs: tabs::Tabs,
    /// What the request in front sends unasked, and where its `{{variables}}` resolve from:
    /// Rust composes the request, so Rust says; the last answer stays while the next is read.
    implied: QueryCell<Question, Vec<ImpliedHeader>>,
    used: QueryCell<Question, Vec<UsedVariable>>,
    /// The folds or the tabs changed: the library's preferences are written.
    unremembered: bool,
}

impl HttpState {
    /// The folds and the tabs, read again when a library opens.
    pub fn restore(&mut self, preferences: &Preferences) {
        self.tabs.restore(preferences);
        let stored = preferences.get(COLLAPSED_KEY).unwrap_or("[]");
        self.collapsed = serde_json::from_str::<Vec<serde_json::Value>>(stored)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|id| id.as_str().map(str::to_owned))
            .collect();
        self.lay_out();
    }

    /// Once a frame, with the library open: the tree is read the first time the area asks.
    pub fn update(&mut self, backend: &Backend, now: f64, errors: &mut ErrorNotifier) {
        for id in self.adopt_writes(errors) {
            self.tabs.open(backend, id);
        }
        let answered = self.tabs.update(backend, errors);
        if answered.saved {
            self.revision.bump();
        }
        self.unremembered |= answered.changed_preferences;
        if let Some(deleting) = &mut self.deleting
            && let Some(answer) = deleting.pending.as_mut().and_then(Pending::poll)
        {
            deleting.pending = None;
            match answer {
                Ok(contents) => deleting.contents = Some(contents),
                Err(error) => {
                    errors.report(keys::http::FAILED, &error);
                    self.deleting = None;
                }
            }
        }
        self.tree.want(self.revision);
        if self.tree.update(|_| api::http::tree(backend)) {
            if let Some(error) = self.tree.error() {
                errors.report(keys::http::FAILED, error);
            }
            self.lay_out();
            self.land(now);
            if self.tree.value().is_some() {
                self.tabs
                    .follow(self.tree.value().unwrap_or(&HttpTree::default()));
                self.unremembered = true;
            }
        }
        self.ask_editor(backend);
        if self
            .mark
            .as_ref()
            .is_some_and(|mark| now - mark.since > MARK_SECONDS)
        {
            self.mark = None;
        }
    }

    /// The requests the writes created, for their tabs.
    fn adopt_writes(&mut self, errors: &mut ErrorNotifier) -> Vec<String> {
        let mut landed = None;
        let mut wrote = false;
        let mut created = Vec::new();
        self.writes.retain_mut(|writing| {
            let Some(answer) = writing.pending.poll() else {
                return true;
            };
            match answer {
                Ok(made) => {
                    wrote = true;
                    created.extend(made);
                    if let Some(landing) = writing.landing.take() {
                        landed = Some(landing);
                    }
                }
                Err(error) => errors.report(keys::http::FAILED, &error),
            }
            false
        });
        if wrote {
            self.revision.bump();
        }
        if let Some(landing) = landed {
            self.landing = Some((self.revision, landing));
        }
        created
    }

    /// The headers sent unasked and the variables used, for the draft in front.
    fn ask_editor(&mut self, backend: &Backend) {
        let Some(tab) = self.tabs.active() else {
            return;
        };
        let question = Question {
            kind: tab.draft.kind,
            document: tab.draft.document.clone(),
            place: tab.place.clone(),
            revision: self.revision,
        };
        self.implied.want(question.clone());
        self.implied.update(|question| {
            let place = question.place.map_or((None, None), |place| {
                (Some(place.collection_id), place.folder_id)
            });
            api::http::implied_headers(backend, (question.kind, question.document), place)
        });
        self.used.want(question);
        self.used.update(|question| {
            let place = question.place;
            let request = VariablesRequest {
                kind: question.kind,
                document: question.document,
                collection_id: place.as_ref().map(|place| place.collection_id.clone()),
                folder_id: place.and_then(|place| place.folder_id),
                environment_id: None,
            };
            api::http::used_variables(backend, request)
        });
    }

    pub fn implied(&self) -> &[ImpliedHeader] {
        self.implied.value().map_or(&[], Vec::as_slice)
    }

    pub fn used(&self) -> &[UsedVariable] {
        self.used.value().map_or(&[], Vec::as_slice)
    }

    /// A move made with the keys, read from the tree the core answered — never the move asked.
    fn land(&mut self, now: f64) {
        use keys::http::rail::moved;

        if self
            .landing
            .as_ref()
            .is_none_or(|(revision, _)| self.tree.answered() != Some(revision))
        {
            return;
        }
        let Some((_, landing)) = self.landing.take() else {
            return;
        };
        let Some(landed) = moves::landing(self.tree(), &landing.item, landing.key) else {
            return;
        };
        self.mark = Some(Mark {
            id: landing.item.id.clone(),
            parent_id: landed.parent_id,
            since: now,
        });
        let key = match landed.at {
            Where::Into => moved::INTO,
            Where::Root => moved::ROOT,
            Where::Before => moved::BEFORE,
            Where::After => moved::AFTER,
        };
        let message = Message::new(key)
            .with("name", landing.name.as_str())
            .with("other", landed.other.as_str());
        self.say(message);
    }

    fn say(&mut self, message: Message) {
        let beat = self.said.as_ref().map_or(0, |(beat, _)| beat + 1);
        self.said = Some((beat, message));
    }

    pub fn tree(&self) -> &HttpTree {
        static EMPTY: HttpTree = HttpTree {
            collections: Vec::new(),
        };
        self.tree.value().unwrap_or(&EMPTY)
    }

    pub fn rows(&self) -> &[RailRow] {
        &self.rows
    }

    /// Read at least once: until then an empty tree says nothing about what the library holds.
    pub fn loaded(&self) -> bool {
        self.tree.value().is_some()
    }

    pub fn mark(&self) -> Option<&Mark> {
        self.mark.as_ref()
    }

    pub fn said(&self) -> Option<&(u32, Message)> {
        self.said.as_ref()
    }

    pub fn deleting(&self) -> Option<&Deleting> {
        self.deleting.as_ref()
    }

    pub fn tabs(&self) -> &tabs::Tabs {
        &self.tabs
    }

    /// What the tabs are asked; each one is remembered.
    pub fn tabs_mut(&mut self) -> &mut tabs::Tabs {
        self.unremembered = true;
        &mut self.tabs
    }

    /// Folded or unfolded, and remembered in the library's preferences.
    pub fn toggle(&mut self, id: &str) {
        if !self.collapsed.remove(id) {
            self.collapsed.insert(id.to_owned());
        }
        self.unremembered = true;
        self.lay_out();
    }

    /// Unfolds what would hide a row just made or moved there.
    fn unfold(&mut self, id: &str) {
        if self.collapsed.contains(id) {
            self.toggle(id);
        }
    }

    /// The folds and the tabs into the library's preferences, when they changed since: `true`
    /// when the file is to be written.
    pub fn remember(&mut self, preferences: &mut Preferences) -> bool {
        if !std::mem::take(&mut self.unremembered) {
            return false;
        }
        let ids: Vec<&String> = self.collapsed.iter().collect();
        preferences.set(
            COLLAPSED_KEY,
            &serde_json::to_string(&ids).unwrap_or_default(),
        );
        self.tabs.remember(preferences);
        true
    }

    pub fn write(&mut self, backend: &Backend, write: Write) {
        let pending = match write {
            Write::CreateCollection(name) => api::http::create_collection(backend, name),
            Write::CreateFolder {
                collection_id,
                folder_id,
                name,
            } => {
                self.unfold(folder_id.as_deref().unwrap_or(&collection_id));
                api::http::create_folder(backend, collection_id, folder_id, name)
            }
            Write::CreateRequest {
                collection_id,
                folder_id,
                name,
                kind,
            } => {
                self.unfold(folder_id.as_deref().unwrap_or(&collection_id));
                api::http::create_request(backend, (collection_id, folder_id), name, kind)
            }
            Write::Rename { item, name } => api::http::rename(backend, item, name),
            Write::Duplicate { item, name } => api::http::duplicate(backend, item, name),
            Write::Delete(item) => {
                self.deleting = None;
                api::http::delete(backend, item)
            }
            Write::Move(railmove) => self.apply_move(backend, railmove),
        };
        self.writes.push(Writing {
            pending,
            landing: None,
        });
    }

    fn apply_move(&mut self, backend: &Backend, railmove: RailMove) -> Pending<Written> {
        match railmove {
            RailMove::Reorder { id, index } => api::http::reorder_collection(backend, id, index),
            RailMove::Move { item, place } => {
                if let Some(folder) = &place.folder_id {
                    self.unfold(folder);
                }
                api::http::move_item(backend, item, place)
            }
        }
    }

    /// `Alt` and an arrow: moved, then shown and said once the tree is read again; or said why
    /// not.
    pub fn key_move(&mut self, backend: &Backend, (item, name): (HttpItem, String), key: KeyMove) {
        let Some(railmove) = moves::key_move(self.tree(), &item, key) else {
            use keys::http::rail::moved::refused;
            let key = match moves::refusal(&item, key) {
                Refusal::First => refused::FIRST,
                Refusal::Last => refused::LAST,
                Refusal::NoFolderAbove => refused::NO_FOLDER_ABOVE,
                Refusal::AtRoot => refused::AT_ROOT,
                Refusal::Collection => refused::COLLECTION,
            };
            self.say(Message::new(key).with("name", name.as_str()));
            return;
        };
        let pending = self.apply_move(backend, railmove);
        self.writes.push(Writing {
            pending,
            landing: Some(Landing { item, key, name }),
        });
    }

    /// The question comes once Rust has counted what goes with it.
    pub fn ask_delete(&mut self, backend: &Backend, item: HttpItem) {
        let pending = api::http::contents(backend, item.clone());
        self.deleting = Some(Deleting {
            item,
            pending: Some(pending),
            contents: None,
        });
    }

    pub fn cancel_delete(&mut self) {
        self.deleting = None;
    }

    fn lay_out(&mut self) {
        self.rows = tree::rail_rows(self.tree(), &self.collapsed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folds_are_read_back_from_the_preferences() {
        let mut preferences = Preferences::default();
        let mut state = HttpState::default();

        state.toggle("a");
        state.toggle("b");
        state.toggle("a");
        assert!(state.remember(&mut preferences));
        assert!(!state.remember(&mut preferences));
        let mut read = HttpState::default();
        read.restore(&preferences);

        assert_eq!(preferences.get(COLLAPSED_KEY), Some(r#"["b"]"#));
        assert_eq!(read.collapsed, BTreeSet::from(["b".to_owned()]));
    }

    #[test]
    fn a_damaged_list_of_folds_is_none() {
        let mut preferences = Preferences::default();
        preferences.set(COLLAPSED_KEY, "{not json");
        let mut state = HttpState::default();

        state.restore(&preferences);

        assert!(state.collapsed.is_empty());
    }
}
