//! The open requests, one tab each. A tab edits a draft beside what Rust holds, and saving writes
//! the draft whole; a request begun from « New request » is a draft alone until saved, like a new
//! note.
//!
//! ⚠️ A saved request's name and place are the tree's: the rail renames, moves and deletes there,
//! and a tab follows each tree that is read. A tab that kept the name it was opened with showed
//! the old one after a rename from the rail — and its next save wrote it back over the new one.

use engine::http::environments::Variable;
use engine::http::graphql::GraphqlDocument;
use engine::http::model::{
    Extraction, HttpMethod, HttpRequest, HttpRequestDraft, HttpRequestPatch, HttpTree, KeyValue,
    NewPlace, RequestAuth, RequestBody, RequestDocument, RequestKind,
};
use engine::http::query::QuerySide;
use engine::http::transport::TransportSettings;

use super::bodies::{self, BodiesLeft, BodyKind};
use super::tree;
use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::services::preferences::Preferences;

/// In the library's preferences: which requests were open, and which one was in front.
pub const TABS_KEY: &str = "devnotes.notes.http.tabs";

/// What a tab edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub name: String,
    pub kind: RequestKind,
    pub method: HttpMethod,
    pub document: RequestDocument,
}

impl Draft {
    fn of(request: &HttpRequest) -> Self {
        Self {
            name: request.name.clone(),
            kind: request.kind,
            method: request.method,
            document: request.document.clone(),
        }
    }
}

/// A change the editor makes to a tab's draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftEdit {
    Name(String),
    Method(HttpMethod),
    /// The URL as typed: the parameters' table is rewritten from its query.
    Url(String),
    /// The table as edited: the URL's query is rewritten from it.
    Params(Vec<KeyValue>),
    Headers(Vec<KeyValue>),
    Body(RequestBody),
    Graphql(GraphqlDocument),
    Auth(RequestAuth),
    Transport(TransportSettings),
    Variables(Vec<Variable>),
    Extractions(Vec<Extraction>),
}

impl Draft {
    /// The edited side wins: the URL and its table are kept in step (`query::sync`).
    pub fn apply(&mut self, edit: DraftEdit) {
        let document = &mut self.document;
        match edit {
            DraftEdit::Name(name) => self.name = name,
            DraftEdit::Method(method) => self.method = method,
            DraftEdit::Url(url) => {
                let synced = api::http::sync_query(&url, &document.params, QuerySide::Url);
                document.url = synced.url;
                document.params = synced.params;
            }
            DraftEdit::Params(params) => {
                let synced = api::http::sync_query(&document.url, &params, QuerySide::Params);
                document.url = synced.url;
                document.params = synced.params;
            }
            DraftEdit::Headers(headers) => document.headers = headers,
            DraftEdit::Body(body) => document.body = body,
            DraftEdit::Graphql(graphql) => document.graphql = graphql,
            DraftEdit::Auth(auth) => document.auth = auth,
            DraftEdit::Transport(transport) => document.transport = transport,
            DraftEdit::Variables(variables) => document.variables = variables,
            DraftEdit::Extractions(rules) => document.extractions = rules,
        }
    }
}

/// Where a saved request sits: what it inherits from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub collection_id: String,
    pub folder_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Tab {
    /// The request's id once saved; `draft-n` for one that never was.
    pub key: String,
    pub request_id: Option<String>,
    pub place: Option<Place>,
    /// What Rust holds: `None` for a request never saved, which writes nothing until it is.
    pub saved: Option<Draft>,
    pub draft: Draft,
}

impl Tab {
    pub fn is_dirty(&self) -> bool {
        self.saved.as_ref() != Some(&self.draft)
    }

    fn of(request: &HttpRequest) -> Self {
        let draft = Draft::of(request);
        Self {
            key: request.id.clone(),
            request_id: Some(request.id.clone()),
            place: Some(Place {
                collection_id: request.collection_id.clone(),
                folder_id: request.folder_id.clone(),
            }),
            saved: Some(draft.clone()),
            draft,
        }
    }
}

/// Written by hand, it is read like any input: what is not a list of ids opens nothing.
fn read_stored(preferences: &Preferences) -> (Vec<String>, Option<String>) {
    let stored: serde_json::Value = preferences
        .get(TABS_KEY)
        .and_then(|stored| serde_json::from_str(stored).ok())
        .unwrap_or_default();
    let open = stored["open"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|id| id.as_str().map(str::to_owned))
        .collect();
    let active = stored["active"].as_str().map(str::to_owned);
    (open, active)
}

/// A draft on its way to Rust: what was sent, and whether the tab closes once it is written.
/// What a first save creates on its way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToCreate {
    Nothing,
    /// It takes the request at its root, whatever the ids say.
    Collection(String),
    /// Under the place the ids name.
    Folder(String),
}

/// Where a request never saved goes the first time it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveTarget {
    pub collection_id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub create: ToCreate,
}

/// A draft on its way to its first write.
struct Creating {
    key: String,
    pending: Pending<HttpRequest>,
    then_close: bool,
}

struct Saving {
    key: String,
    sent: Draft,
    pending: Pending<HttpRequest>,
    then_close: bool,
}

#[derive(Default)]
pub struct Tabs {
    open: Vec<Tab>,
    active: Option<String>,
    drafts: u32,
    /// The tabs left open last time, waiting to be read once the area is shown.
    to_restore: Option<(Vec<String>, Option<String>)>,
    restoring: Option<(Pending<Vec<HttpRequest>>, Option<String>)>,
    opening: Vec<Pending<HttpRequest>>,
    saving: Vec<Saving>,
    creating: Vec<Creating>,
    /// What a request takes as its changes once it is open: a send reopened onto it.
    replacing: std::collections::HashMap<String, (HttpMethod, RequestDocument)>,
    /// A draft asked to save, and whether the tab closes after: the question is open.
    asking_place: Option<(String, bool)>,
    /// A tab asked to close with changes in it: the question is open.
    closing: Option<String>,
    /// By tab, for as long as it is open.
    bodies_left: std::collections::HashMap<String, BodiesLeft>,
}

pub struct Answered {
    /// Drafts now saved, by their old key and the new one: what is kept by key follows.
    pub rekeyed: Vec<(String, Tab)>,
    /// The tree is read again: a save changes a name or a method on the rail.
    pub saved: bool,
    pub changed_preferences: bool,
}

impl Tabs {
    pub fn restore(&mut self, preferences: &Preferences) {
        self.to_restore = Some(read_stored(preferences));
    }

    /// Once a frame while the area is shown.
    pub fn update(&mut self, backend: &Backend, errors: &mut ErrorNotifier) -> Answered {
        let mut answered = Answered {
            rekeyed: Vec::new(),
            saved: false,
            changed_preferences: false,
        };
        if let Some((ids, active)) = self.to_restore.take() {
            let pending = api::http::requests(backend, ids);
            self.restoring = Some((pending, active));
        }
        if let Some((pending, active)) = &mut self.restoring
            && let Some(answer) = pending.poll()
        {
            let active = active.take();
            self.restoring = None;
            match answer {
                Ok(requests) => {
                    for request in &requests {
                        self.adopt(request);
                    }
                    self.active = active
                        .filter(|key| self.find(key).is_some())
                        .or_else(|| self.open.last().map(|tab| tab.key.clone()));
                }
                Err(error) => errors.report(i18n::keys::http::FAILED, &error),
            }
        }
        let mut opened = Vec::new();
        self.opening.retain_mut(|pending| {
            let Some(answer) = pending.poll() else {
                return true;
            };
            match answer {
                Ok(request) => opened.push(request),
                Err(error) => errors.report(i18n::keys::http::FAILED, &error),
            }
            false
        });
        for request in opened {
            if self.find(&request.id).is_none() {
                self.adopt(&request);
            }
            if let Some(replaced) = self.replacing.remove(&request.id) {
                self.replace_draft(&request.id, replaced);
            }
            self.active = Some(request.id.clone());
            answered.changed_preferences = true;
        }
        self.adopt_created(errors, &mut answered);
        let mut saved = Vec::new();
        self.saving.retain_mut(|saving| {
            let Some(answer) = saving.pending.poll() else {
                return true;
            };
            match answer {
                Ok(request) => saved.push((
                    saving.key.clone(),
                    saving.sent.clone(),
                    request,
                    saving.then_close,
                )),
                Err(error) => errors.report(i18n::keys::http::FAILED, &error),
            }
            false
        });
        for (key, sent, request, then_close) in saved {
            let held = Draft::of(&request);
            if let Some(tab) = self.find_mut(&key) {
                // Typing during the write keeps what was typed; otherwise the draft is what
                // Rust kept.
                if tab.draft == sent {
                    tab.draft = held.clone();
                }
                tab.saved = Some(held);
            }
            if then_close {
                self.close(&key);
                answered.changed_preferences = true;
            }
            answered.saved = true;
        }
        answered
    }

    fn adopt(&mut self, request: &HttpRequest) {
        self.open.push(Tab::of(request));
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.open
    }

    pub fn active(&self) -> Option<&Tab> {
        self.find(self.active.as_deref()?)
    }

    pub fn active_key(&self) -> Option<&str> {
        self.active.as_deref()
    }

    /// The request the rail lights up.
    pub fn active_request(&self) -> Option<&str> {
        self.active()?.request_id.as_deref()
    }

    pub fn closing(&self) -> Option<&Tab> {
        self.find(self.closing.as_deref()?)
    }

    pub fn tab(&self, key: &str) -> Option<&Tab> {
        self.find(key)
    }

    fn find(&self, key: &str) -> Option<&Tab> {
        self.open.iter().find(|tab| tab.key == key)
    }

    fn find_mut(&mut self, key: &str) -> Option<&mut Tab> {
        self.open.iter_mut().find(|tab| tab.key == key)
    }

    /// The names of the open tabs among `ids` whose changes are not saved: what a deletion would
    /// lose. Only the interface knows it — Rust counts what is deleted, not what is typed.
    pub fn unsaved_among(&self, ids: &[String]) -> Vec<String> {
        self.open
            .iter()
            .filter(|tab| {
                tab.request_id.as_ref().is_some_and(|id| ids.contains(id)) && tab.is_dirty()
            })
            .map(|tab| tab.draft.name.clone())
            .collect()
    }

    pub fn open(&mut self, backend: &Backend, id: String) {
        if self.find(&id).is_some() {
            self.active = Some(id);
            return;
        }
        self.opening.push(api::http::request(backend, id));
    }

    /// Of the kind asked, for good: a request is not turned into a socket afterwards.
    pub fn new_request(&mut self, kind: RequestKind, name: String) {
        self.drafts += 1;
        let key = format!("draft-{}", self.drafts);
        let draft = Draft {
            name,
            kind,
            method: HttpMethod::Get,
            document: RequestDocument::default(),
        };
        self.open.push(Tab {
            key: key.clone(),
            request_id: None,
            place: None,
            saved: None,
            draft,
        });
        self.active = Some(key);
    }

    /// A send put back onto its request: its values, as changes to save or not.
    pub fn open_with(
        &mut self,
        backend: &Backend,
        id: String,
        method: HttpMethod,
        document: RequestDocument,
    ) {
        if self.find(&id).is_some() {
            self.replace_draft(&id, (method, document));
            self.active = Some(id);
            return;
        }
        self.replacing.insert(id.clone(), (method, document));
        self.open(backend, id);
    }

    fn replace_draft(&mut self, key: &str, (method, document): (HttpMethod, RequestDocument)) {
        if let Some(tab) = self.find_mut(key) {
            tab.draft.method = method;
            tab.draft.document = document;
        }
    }

    /// A draft rebuilt from a send, written nowhere until saved.
    pub fn open_draft(
        &mut self,
        name: String,
        kind: RequestKind,
        method: HttpMethod,
        document: RequestDocument,
    ) {
        self.new_request(kind, name);
        if let Some(key) = self.active.clone() {
            self.replace_draft(&key, (method, document));
        }
    }

    pub fn activate(&mut self, key: &str) {
        if self.find(key).is_some() {
            self.active = Some(key.to_owned());
        }
    }

    /// What a tab edits, changed by the editor.
    pub fn edit(&mut self, key: &str, change: impl FnOnce(&mut Draft)) {
        if let Some(tab) = self.find_mut(key) {
            change(&mut tab.draft);
        }
    }

    /// Another kind of body; the one left is set aside, and comes back when chosen again.
    pub fn choose_body_kind(&mut self, key: &str, kind: BodyKind) {
        let Some(tab) = self.open.iter_mut().find(|tab| tab.key == key) else {
            return;
        };
        let current = &tab.draft.document.body;
        if BodyKind::of(current) == kind {
            return;
        }
        let left = self.bodies_left.entry(key.to_owned()).or_default();
        bodies::set_aside(left, current.clone());
        tab.draft.document.body = bodies::body_of_kind(kind, left);
    }

    /// A saved request's draft written whole; a request never saved needs a place first.
    pub fn save(&mut self, backend: &Backend, key: &str) {
        self.write(backend, key, false);
    }

    /// The close question's « Save »: the tab goes once what it holds is written.
    pub fn save_and_close(&mut self, backend: &Backend, key: &str) {
        self.closing = None;
        self.write(backend, key, true);
    }

    /// The draft the place is being asked for.
    pub fn asking_place(&self) -> Option<&Tab> {
        let (key, _) = self.asking_place.as_ref()?;
        self.find(key)
    }

    pub fn cancel_save_to(&mut self) {
        self.asking_place = None;
    }

    /// The answer to that question: one write for the request and what it lacks.
    pub fn save_to(&mut self, backend: &Backend, key: &str, target: SaveTarget) {
        let then_close = self
            .asking_place
            .take()
            .is_some_and(|(_, then_close)| then_close);
        let Some(tab) = self.find(key) else {
            return;
        };
        let draft = HttpRequestDraft {
            collection_id: target.collection_id,
            folder_id: target.folder_id,
            name: target.name,
            kind: tab.draft.kind,
            method: tab.draft.method,
            document: tab.draft.document.clone(),
        };
        let create = match target.create {
            ToCreate::Nothing => None,
            ToCreate::Collection(name) => Some(NewPlace {
                collection: Some(name),
                folder: None,
            }),
            ToCreate::Folder(name) => Some(NewPlace {
                collection: None,
                folder: Some(name),
            }),
        };
        self.creating.push(Creating {
            key: key.to_owned(),
            pending: api::http::create_request_in(backend, draft, create),
            then_close,
        });
    }

    /// A draft written for the first time takes the request's id for its key.
    fn adopt_created(&mut self, errors: &mut ErrorNotifier, answered: &mut Answered) {
        let mut created = Vec::new();
        self.creating.retain_mut(|creating| {
            let Some(answer) = creating.pending.poll() else {
                return true;
            };
            match answer {
                Ok(request) => created.push((creating.key.clone(), request, creating.then_close)),
                Err(error) => errors.report(i18n::keys::http::FAILED, &error),
            }
            false
        });
        for (key, request, then_close) in created {
            let new_key = request.id.clone();
            let Some(tab) = self.find_mut(&key) else {
                continue;
            };
            *tab = Tab::of(&request);
            answered.rekeyed.push((key.clone(), tab.clone()));
            if let Some(left) = self.bodies_left.remove(&key) {
                self.bodies_left.insert(new_key.clone(), left);
            }
            if self.active.as_deref() == Some(key.as_str()) {
                self.active = Some(new_key.clone());
            }
            if then_close {
                self.close(&new_key);
            }
            answered.saved = true;
            answered.changed_preferences = true;
        }
    }

    fn write(&mut self, backend: &Backend, key: &str, then_close: bool) {
        let Some(tab) = self.find(key) else {
            return;
        };
        let Some(id) = tab.request_id.clone() else {
            self.asking_place = Some((key.to_owned(), then_close));
            return;
        };
        let draft = tab.draft.clone();
        let patch = HttpRequestPatch {
            name: Some(draft.name.clone()),
            method: Some(draft.method),
            document: Some(draft.document.clone()),
        };
        let pending = api::http::save_request(backend, id, patch);
        self.saving.push(Saving {
            key: key.to_owned(),
            sent: draft,
            pending,
            then_close,
        });
    }

    /// Closed at once when closing costs nothing; asked first otherwise.
    pub fn ask_close(&mut self, key: &str) {
        match self.find(key) {
            Some(tab) if tab.is_dirty() => self.closing = Some(key.to_owned()),
            Some(_) => self.close(key),
            None => {}
        }
    }

    pub fn cancel_close(&mut self) {
        self.closing = None;
    }

    pub fn close(&mut self, key: &str) {
        if self.closing.as_deref() == Some(key) {
            self.closing = None;
        }
        let Some(at) = self.open.iter().position(|tab| tab.key == key) else {
            return;
        };
        self.open.remove(at);
        self.bodies_left.remove(key);
        if self.active.as_deref() == Some(key) {
            let next = at.min(self.open.len().saturating_sub(1));
            self.active = self.open.get(next).map(|tab| tab.key.clone());
        }
    }

    /// What the rail did to an open request reaches its tab: deleted, the tab closes; moved, it
    /// inherits from where it now sits; renamed, it takes the name — in what is saved and in
    /// what is being edited, so the next save does not write the old one back.
    pub fn follow(&mut self, tree: &HttpTree) {
        let mut gone = Vec::new();
        for tab in &mut self.open {
            let (Some(id), Some(saved)) = (&tab.request_id, &mut tab.saved) else {
                continue;
            };
            let Some(held) = tree::find_request(tree, id) else {
                gone.push(tab.key.clone());
                continue;
            };
            tab.place = Some(Place {
                collection_id: held.collection_id.clone(),
                folder_id: held.folder_id.clone(),
            });
            if saved.name != held.name {
                saved.name.clone_from(&held.name);
                tab.draft.name.clone_from(&held.name);
            }
        }
        for key in gone {
            self.close(&key);
        }
    }

    pub fn remember(&self, preferences: &mut Preferences) {
        let open: Vec<&str> = self
            .open
            .iter()
            .filter_map(|tab| tab.request_id.as_deref())
            .collect();
        let stored = serde_json::json!({ "open": open, "active": self.active_request() });
        preferences.set(TABS_KEY, &stored.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str, name: &str) -> HttpRequest {
        HttpRequest {
            id: id.to_owned(),
            collection_id: "API".to_owned(),
            folder_id: None,
            name: name.to_owned(),
            kind: RequestKind::Http,
            method: HttpMethod::Get,
            document: RequestDocument::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn opened(names: &[&str]) -> Tabs {
        let mut tabs = Tabs::default();
        for name in names {
            tabs.adopt(&request(name, name));
        }
        tabs.active = names.last().map(|name| (*name).to_owned());
        tabs
    }

    #[test]
    fn a_tab_is_modified_when_its_draft_differs_from_what_is_saved() {
        let mut tabs = opened(&["login"]);

        assert!(!tabs.tabs()[0].is_dirty());
        tabs.edit("login", |draft| draft.method = HttpMethod::Post);
        assert!(tabs.tabs()[0].is_dirty());
        assert_eq!(tabs.unsaved_among(&["login".to_owned()]), ["login"]);
    }

    #[test]
    fn a_new_request_is_modified_until_saved_and_never_remembered() {
        let mut tabs = opened(&["login"]);
        let mut preferences = Preferences::default();

        tabs.new_request(RequestKind::Websocket, "New request".to_owned());
        tabs.remember(&mut preferences);

        assert!(tabs.active().unwrap().is_dirty());
        assert_eq!(tabs.active().unwrap().draft.kind, RequestKind::Websocket);
        assert_eq!(
            preferences.get(TABS_KEY),
            Some(r#"{"open":["login"],"active":null}"#)
        );
    }

    #[test]
    fn closing_a_modified_tab_asks_and_the_next_one_comes_forward() {
        let mut tabs = opened(&["a", "b", "c"]);
        tabs.activate("b");

        tabs.edit("b", |draft| draft.name = "B".to_owned());
        tabs.ask_close("b");
        assert_eq!(tabs.closing().map(|tab| tab.key.as_str()), Some("b"));
        tabs.close("b");

        assert_eq!(tabs.active_key(), Some("c"));
        assert!(tabs.closing().is_none());
        tabs.ask_close("c");
        assert_eq!(tabs.active_key(), Some("a"));
    }

    #[test]
    fn a_tab_follows_a_rename_and_closes_with_its_deletion() {
        use super::super::tree::tests::{collection, request as node};
        let mut tabs = opened(&["login", "gone"]);
        tabs.edit("login", |draft| draft.method = HttpMethod::Put);
        let mut tree = HttpTree {
            collections: vec![collection(
                "API",
                vec![node("login", HttpMethod::Get, RequestKind::Http)],
            )],
        };
        if let engine::http::model::HttpNode::Request { request } =
            &mut tree.collections[0].children[0]
        {
            request.name = "Connexion".to_owned();
        }

        tabs.follow(&tree);

        assert_eq!(tabs.tabs().len(), 1);
        let tab = &tabs.tabs()[0];
        assert_eq!(tab.draft.name, "Connexion");
        assert_eq!(tab.saved.as_ref().unwrap().name, "Connexion");
        assert_eq!(tab.draft.method, HttpMethod::Put);
    }

    #[test]
    fn the_url_and_its_table_are_kept_in_step() {
        let mut tabs = opened(&["login"]);

        tabs.edit("login", |draft| {
            draft.apply(DraftEdit::Url(
                "https://api.test/users?page=2&q=a".to_owned(),
            ));
        });
        let params = &tabs.tabs()[0].draft.document.params;
        assert_eq!(
            params
                .iter()
                .map(|row| row.key.as_str())
                .collect::<Vec<_>>(),
            ["page", "q"]
        );

        let mut kept = params.clone();
        kept[0].enabled = false;
        tabs.edit("login", |draft| draft.apply(DraftEdit::Params(kept)));
        assert_eq!(
            tabs.tabs()[0].draft.document.url,
            "https://api.test/users?q=a"
        );
        assert_eq!(tabs.tabs()[0].draft.document.params.len(), 2);
    }

    #[test]
    fn what_is_stored_by_hand_opens_only_ids() {
        let mut preferences = Preferences::default();
        preferences.set(TABS_KEY, r#"{"open":["a",3,null,"b"],"active":7}"#);

        assert_eq!(
            read_stored(&preferences),
            (vec!["a".to_owned(), "b".to_owned()], None)
        );
        preferences.set(TABS_KEY, "not json");
        assert_eq!(read_stored(&preferences), (Vec::new(), None));
    }
}
