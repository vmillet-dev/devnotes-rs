//! The HTTP area's state: the collections as the rail shows them. Every write is persisted, then
//! the tree read again: the core renumbers what a move or a copy touched, and the interface never
//! guesses the result.

pub mod bodies;
pub mod code;
pub mod containers;
pub mod cookies;
pub mod environments;
pub mod history;
pub mod moves;
pub mod send;
pub mod sockets;
pub mod tabs;
pub mod tree;

use std::collections::BTreeSet;

use engine::http::BodyAnswer;
use engine::http::VariablesRequest;
use engine::http::graphql::{GraphqlAnswer, GraphqlDocument};
use engine::http::model::{
    Extraction, HttpContents, HttpItem, HttpMethod, HttpTree, RequestBody, RequestDocument,
    RequestKind,
};
use engine::http::send::ImpliedHeader;
use engine::http::settings::Inherited;
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
pub const MARK_SECONDS: f64 = crate::theme::motion::MARK as f64;

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
    environment: Option<String>,
    revision: Revision,
}

/// An image answer: its `data:` URL on its way, then its texture; `None` when it is none.
enum Image {
    Asked(Pending<Option<String>>),
    Shown(Option<egui::TextureHandle>),
}

/// The session an answer is explored under: its send's name, as a number.
fn session_of(send_id: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    send_id.hash(&mut hasher);
    hasher.finish()
}

/// A `data:` URL's picture, or `None` when it holds none that decodes.
fn decoded(url: &str) -> Option<egui::ColorImage> {
    use base64::Engine as _;
    let (_, data) = url.split_once(";base64,")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()?;
    let picture = image::load_from_memory(&bytes).ok()?.into_rgba8();
    let size = [picture.width() as usize, picture.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        picture.as_raw(),
    ))
}

/// A part of a draft the core rewrites: what it held when asked, so that typing meanwhile wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rewritten {
    /// A JSON body, by Prettier.
    Body,
    Query,
    Variables,
    /// The variables with a `null` for each one the operation lacks.
    Missing,
}

/// The system's dialog open for a file of that tab's body.
struct Picking {
    key: String,
    target: bodies::FileFor,
    pending: Pending<Option<std::path::PathBuf>>,
}

struct Rewriting {
    key: String,
    part: Rewritten,
    before: String,
    pending: Pending<Option<String>>,
}

/// The text of `part` in `draft`.
fn part_of(draft: &tabs::Draft, part: Rewritten) -> &str {
    let document = &draft.document;
    match part {
        Rewritten::Body => match &document.body {
            RequestBody::Json { text } => text,
            _ => "",
        },
        Rewritten::Query => &document.graphql.query,
        Rewritten::Variables | Rewritten::Missing => &document.graphql.variables,
    }
}

/// What the workspace shows: the open requests, or a page from the rail's foot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Requests,
    Environments,
    History,
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
    sends: send::Sends,
    /// An answer's JSON explored, by its send.
    explored: super::explored::ExploredState,
    /// An image answer's texture, by its send: decoded once asked.
    images: std::collections::HashMap<String, Image>,
    /// What the request in front sends unasked, and where its `{{variables}}` resolve from:
    /// Rust composes the request, so Rust says; the last answer stays while the next is read.
    implied: QueryCell<Question, Vec<ImpliedHeader>>,
    used: QueryCell<Question, Vec<UsedVariable>>,
    /// What the body in front implies, whether it reads, whether its method seldom carries one.
    body: QueryCell<(RequestBody, HttpMethod), BodyAnswer>,
    /// The operations of the GraphQL document in front, and what its variables lack.
    graphql: QueryCell<GraphqlDocument, GraphqlAnswer>,
    /// A body, a query or variables formatted, or variables completed: written once back,
    /// unless typed on meanwhile.
    rewriting: Vec<Rewriting>,
    picking: Vec<Picking>,
    /// What the request in front inherits: its auth and transport, said beside its own.
    inherited: QueryCell<(tabs::Place, Revision), Inherited>,
    environments: environments::Environments,
    page: Page,
    cookies: cookies::CookieJar,
    sockets: sockets::Sockets,
    history: history::History,
    /// How many sends « Clear » just removed: for the status line.
    history_cleared: Option<u32>,
    code: code::CodeDialog,
    container: Option<containers::Form>,
    /// How many cookies the jar just threw away: for the status line.
    cookies_removed: Option<u32>,
    /// The folds or the tabs changed: the library's preferences are written.
    unremembered: bool,
}

impl HttpState {
    /// The folds and the tabs, read again when a library opens.
    pub fn restore(&mut self, preferences: &Preferences) {
        self.tabs.restore(preferences);
        self.environments.restore(preferences);
        let stored = preferences.get(COLLAPSED_KEY).unwrap_or("[]");
        self.collapsed = serde_json::from_str::<Vec<serde_json::Value>>(stored)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|id| id.as_str().map(str::to_owned))
            .collect();
        self.lay_out();
    }

    /// Once a frame, with the library open: the tree is read the first time the area asks.
    pub fn update(
        &mut self,
        (ctx, backend): (&egui::Context, &Backend),
        now: f64,
        errors: &mut ErrorNotifier,
    ) {
        self.explored.update(backend);
        self.adopt_images(ctx);
        self.adopt_rewrites(errors);
        self.cookies_removed = self.cookies.update(backend, errors);
        self.adopt_container(errors);
        self.adopt_files(errors);
        for id in self.adopt_writes(errors) {
            self.tabs.open(backend, id);
        }
        let answered = self.tabs.update(backend, errors);
        // An extraction or a script may have written a variable.
        if self.sends.update() {
            self.revision.bump();
        }
        self.forget_closed(backend);
        self.sockets.update();
        for (old, tab) in &answered.rekeyed {
            self.sends.rekey(old, &tab.key);
            self.sockets.rekey(old, &tab.key);
            if let Some(place) = &tab.place {
                let id = place.folder_id.clone();
                self.unfold(id.as_deref().unwrap_or(&place.collection_id));
            }
        }
        if let Some(tab) = self.code.key().and_then(|key| self.tabs.tab(key)) {
            let environment = self.environments.active_id().map(str::to_owned);
            let asked = (environment, self.revision);
            self.code.update(backend, tab, asked, errors);
        }
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
        self.environments
            .update((ctx, backend), (self.revision, now), errors);
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
        if let Some(place) = &tab.place {
            self.inherited.want((place.clone(), self.revision));
            self.inherited.update(|(place, _)| {
                api::http::inherited(backend, place.collection_id, place.folder_id)
            });
        }
        self.body
            .want((tab.draft.document.body.clone(), tab.draft.method));
        self.body
            .update(|(body, method)| api::http::describe_body(backend, body, method));
        self.graphql.want(tab.draft.document.graphql.clone());
        self.graphql
            .update(|document| api::http::describe_graphql(backend, document));
        let question = Question {
            kind: tab.draft.kind,
            document: tab.draft.document.clone(),
            place: tab.place.clone(),
            environment: self.environments.active_id().map(str::to_owned),
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
                environment_id: question.environment,
            };
            api::http::used_variables(backend, request)
        });
    }

    pub fn inherited(&self) -> Option<&Inherited> {
        self.inherited.value()
    }

    pub fn environments(&self) -> &environments::Environments {
        &self.environments
    }

    /// Remembered with the library's preferences.
    pub fn choose_environment(&mut self, id: Option<String>) {
        self.environments.choose(id);
        self.unremembered = true;
    }

    pub fn ask_environments(&mut self, backend: &Backend, asked: environments::Asked, now: f64) {
        let remembered = matches!(
            asked,
            environments::Asked::Use(_) | environments::Asked::Delete(_)
        );
        self.environments.ask(backend, asked, now);
        self.unremembered |= remembered;
    }

    pub fn define(&mut self, backend: &Backend, name: String, now: f64) {
        self.environments.define(backend, name, now);
        self.page = Page::Environments;
    }

    fn adopt_container(&mut self, errors: &mut ErrorNotifier) {
        let Some(done) = self.container.as_mut().and_then(|form| form.update(errors)) else {
            return;
        };
        self.container = None;
        if done {
            self.revision.bump();
        }
    }

    pub fn code(&self) -> &code::CodeDialog {
        &self.code
    }

    pub fn ask_code(&mut self, asked: code::Asked) {
        self.code.ask(asked);
    }

    /// Where a draft can go: the places of the tree that was read.
    pub fn save_places(&self) -> Vec<tree::SavePlace> {
        tree::save_places(self.tree())
    }

    pub fn history(&self) -> &history::History {
        &self.history
    }

    /// Once a frame, beside `update`: the history's days are cut in the local time of the clock.
    pub fn update_history(
        &mut self,
        (ctx, backend): (&egui::Context, &Backend),
        (now, tz_offset): (f64, i32),
        errors: &mut ErrorNotifier,
    ) {
        let open = self.page == Page::History;
        let asked = (open, self.revision, now, tz_offset);
        let (cleared, reopened) = self.history.update((ctx, backend), asked, errors);
        self.history_cleared = cleared;
        match reopened {
            Some(history::Reopened::Onto { request_id, draft }) => {
                self.tabs
                    .open_with(backend, request_id, draft.method, draft.document);
                self.page = Page::Requests;
            }
            Some(history::Reopened::Draft { name, draft }) => {
                self.tabs
                    .open_draft(name, draft.kind, draft.method, draft.document);
                self.page = Page::Requests;
            }
            None => {}
        }
    }

    pub fn take_history_cleared(&mut self) -> Option<u32> {
        self.history_cleared.take()
    }

    pub fn ask_history(&mut self, backend: &Backend, asked: history::Asked) {
        match asked {
            history::Asked::Reopen(replace) => self.reopen(backend, replace),
            other => self.history.ask(backend, other),
        }
    }

    /// The request a send goes back to, while the tree still holds it.
    pub fn reopen_into(&self, item: &engine::http::history::HistoryItem) -> Option<&str> {
        let id = item.request_id.as_deref()?;
        tree::find_request(self.tree(), id).map(|held| held.name.as_str())
    }

    /// A send goes back onto the request it came from, as changes to save; a tab with changes of
    /// its own is not written over without asking. That request gone, it is a draft.
    fn reopen(&mut self, backend: &Backend, replace: bool) {
        let Some(entry) = self.history.entry().cloned() else {
            return;
        };
        let held = entry
            .item
            .request_id
            .as_deref()
            .and_then(|id| tree::find_request(self.tree(), id))
            .map(|held| (held.id.clone(), held.name.clone()));
        let Some((request_id, name)) = held else {
            self.history
                .reopen_draft(backend, entry.item.id.clone(), entry.item.sent_at);
            return;
        };
        let dirty = self.tabs.tab(&request_id).is_some_and(tabs::Tab::is_dirty);
        if dirty && !replace {
            self.history.ask_replace(name);
            return;
        }
        self.history.reopen_onto(backend, request_id, &entry);
    }

    pub fn sockets(&self) -> &sockets::Sockets {
        &self.sockets
    }

    pub fn receive_socket(&mut self, event: engine::http::websocket::WebsocketEvent) {
        self.sockets.receive(event);
    }

    pub fn send_message(&mut self, backend: &Backend, key: &str, text: String) {
        self.sockets.send(backend, key, text);
    }

    pub fn clear_socket_log(&mut self, key: &str) {
        self.sockets.clear_log(key);
    }

    pub fn ask_close(&mut self, key: &str) {
        let live = self.sockets.is_live(key);
        self.tabs.ask_close(key, live);
    }

    pub fn cookies(&self) -> &cookies::CookieJar {
        &self.cookies
    }

    pub fn ask_cookies(&mut self, backend: &Backend, asked: cookies::Asked) {
        self.cookies.ask(backend, asked);
    }

    pub fn take_cookies_removed(&mut self) -> Option<u32> {
        self.cookies_removed.take()
    }

    pub fn container(&self) -> Option<&containers::Form> {
        self.container.as_ref()
    }

    pub fn open_settings(&mut self, backend: &Backend, container: containers::Container) {
        self.container = Some(containers::Form::open(backend, container));
    }

    pub fn edit_settings(&mut self, settings: engine::http::model::ContainerSettings) {
        if let Some(form) = &mut self.container {
            form.settings = Some(settings);
        }
    }

    pub fn save_settings(&mut self, backend: &Backend) {
        if let Some(form) = &mut self.container {
            form.save(backend);
        }
    }

    pub fn close_settings(&mut self) {
        self.container = None;
    }

    pub fn page(&self) -> Page {
        self.page
    }

    /// Leaving the environments writes what waits to be.
    pub fn show_page(&mut self, backend: &Backend, page: Page) {
        if self.page == Page::Environments && page != Page::Environments {
            self.environments.flush(backend);
        }
        self.page = page;
    }

    pub fn toggle_page(&mut self, backend: &Backend, page: Page) {
        let next = if self.page == page {
            Page::Requests
        } else {
            page
        };
        self.show_page(backend, next);
    }

    pub fn stop_asking(&mut self) {
        self.environments.stop_asking();
    }

    /// The yes a protected environment asked for.
    pub fn send_anyway(&mut self, backend: &Backend, key: &str, now: f64) {
        self.environments.stop_asking();
        let Some(tab) = self.tabs.tab(key) else {
            return;
        };
        let environment = self.environments.active_id().map(str::to_owned);
        if tab.draft.kind == RequestKind::Websocket {
            self.sockets.connect(backend, tab, environment);
        } else {
            self.sends.send(backend, tab, (now, environment), true);
        }
    }

    /// A rule of the tab's draft: saved with the request, as any other part of it.
    pub fn add_extraction(&mut self, key: &str, rule: Extraction) {
        let Some(tab) = self.tabs.tab(key) else {
            return;
        };
        let mut rules = tab.draft.document.extractions.clone();
        rules.retain(|known| known.variable != rule.variable);
        rules.push(rule);
        self.tabs.edit(key, |draft| {
            draft.apply(tabs::DraftEdit::Extractions(rules));
        });
    }

    pub fn body_answer(&self) -> Option<&BodyAnswer> {
        self.body.value()
    }

    pub fn graphql_answer(&self) -> Option<&GraphqlAnswer> {
        self.graphql.value()
    }

    /// A part of the tab's draft rewritten by the core: formatted, or its variables completed.
    pub fn rewrite(&mut self, backend: &Backend, key: &str, part: Rewritten) {
        let Some(tab) = self.tabs.tab(key) else {
            return;
        };
        let before = part_of(&tab.draft, part).to_owned();
        let pending = match part {
            Rewritten::Body | Rewritten::Variables => {
                api::http::prettier(backend, before.clone(), true)
            }
            Rewritten::Query => api::http::prettier(backend, before.clone(), false),
            Rewritten::Missing => {
                api::http::add_graphql_variables(backend, tab.draft.document.graphql.clone())
            }
        };
        self.rewriting.push(Rewriting {
            key: key.to_owned(),
            part,
            before,
            pending,
        });
    }

    /// Off the frame: the dialog blocks until answered.
    pub fn pick_file(
        &mut self,
        backend: &Backend,
        dialog: &std::sync::Arc<dyn platform::dialog::FileDialog + Send + Sync>,
        key: String,
        target: bodies::FileFor,
    ) {
        let dialog = std::sync::Arc::clone(dialog);
        self.picking.push(Picking {
            key,
            target,
            pending: backend.compute(move |_| Ok(dialog.open())),
        });
    }

    fn adopt_files(&mut self, errors: &mut ErrorNotifier) {
        let mut chosen = Vec::new();
        self.picking.retain_mut(|picking| {
            let Some(answer) = picking.pending.poll() else {
                return true;
            };
            match answer {
                Ok(Some(path)) => chosen.push((picking.key.clone(), picking.target, path)),
                Ok(None) => {}
                Err(error) => errors.report(keys::http::FAILED, &error),
            }
            false
        });
        for (key, target, path) in chosen {
            let Some(tab) = self.tabs.tab(&key) else {
                continue;
            };
            let path = path.to_string_lossy().into_owned();
            if let Some(body) = bodies::with_file(&tab.draft.document.body, target, path) {
                self.tabs
                    .edit(&key, |draft| draft.apply(tabs::DraftEdit::Body(body)));
            }
        }
    }

    fn adopt_rewrites(&mut self, errors: &mut ErrorNotifier) {
        let mut done = Vec::new();
        self.rewriting.retain_mut(|rewriting| {
            let Some(answer) = rewriting.pending.poll() else {
                return true;
            };
            match answer {
                Ok(Some(text)) => done.push((
                    rewriting.key.clone(),
                    rewriting.part,
                    rewriting.before.clone(),
                    text,
                )),
                Ok(None) => {}
                Err(error) => errors.report(keys::http::FAILED, &error),
            }
            false
        });
        for (key, part, before, text) in done {
            let Some(tab) = self.tabs.tab(&key) else {
                continue;
            };
            if part_of(&tab.draft, part) != before {
                continue;
            }
            let mut graphql = tab.draft.document.graphql.clone();
            let edit = match part {
                Rewritten::Body => tabs::DraftEdit::Body(RequestBody::Json { text }),
                Rewritten::Query => {
                    graphql.query = text;
                    tabs::DraftEdit::Graphql(graphql)
                }
                Rewritten::Variables | Rewritten::Missing => {
                    graphql.variables = text;
                    tabs::DraftEdit::Graphql(graphql)
                }
            };
            self.tabs.edit(&key, |draft| draft.apply(edit));
        }
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

    pub fn sends(&self) -> &send::Sends {
        &self.sends
    }

    /// An answer's graph or tree, asked again whenever the explorer asks something else.
    pub fn explore(&mut self, send_id: &str, query: engine::json::model::JsonQuery) {
        self.explored.want(session_of(send_id), query);
    }

    pub fn explored(&self, send_id: &str) -> Option<(&engine::json::model::JsonView, &str)> {
        self.explored.of(session_of(send_id))
    }

    /// The image an answer is, read once.
    pub fn ask_image(&mut self, backend: &Backend, send_id: &str) {
        if !self.images.contains_key(send_id) {
            let pending = api::http::response_image(backend, send_id.to_owned());
            self.images
                .insert(send_id.to_owned(), Image::Asked(pending));
        }
    }

    pub fn response_image(&self, send_id: &str) -> Option<&egui::TextureHandle> {
        match self.images.get(send_id)? {
            Image::Shown(texture) => texture.as_ref(),
            Image::Asked(_) => None,
        }
    }

    fn adopt_images(&mut self, ctx: &egui::Context) {
        for (id, image) in &mut self.images {
            let Image::Asked(pending) = image else {
                continue;
            };
            let Some(answer) = pending.poll() else {
                continue;
            };
            let texture = answer
                .ok()
                .flatten()
                .and_then(|url| decoded(&url))
                .map(|picture| {
                    ctx.load_texture(
                        format!("http-image-{id}"),
                        picture,
                        egui::TextureOptions::LINEAR,
                    )
                });
            *image = Image::Shown(texture);
        }
        let held: Vec<String> = self.sends.ids().map(str::to_owned).collect();
        self.images.retain(|id, _| held.contains(id));
    }

    /// The tab `key`'s draft, sent as it is now; `None` for the tab in front.
    pub fn send(&mut self, backend: &Backend, key: Option<&str>, now: f64) {
        let from_the_button = key.is_some();
        let key = key.or(self.tabs.active_key()).map(str::to_owned);
        let Some(tab) = key.as_deref().and_then(|key| self.tabs.tab(key)) else {
            return;
        };
        // The shortcut connects and never cuts: only the button disconnects.
        if tab.draft.kind == RequestKind::Websocket && self.sockets.is_live(&tab.key) {
            if from_the_button {
                let key = tab.key.clone();
                self.sockets.close(backend, &key);
            }
            return;
        }
        let protected = self
            .environments
            .active()
            .is_some_and(|active| active.protected);
        if protected && api::http::asks_before_sending(tab.draft.kind, tab.draft.method) {
            let key = tab.key.clone();
            self.environments.ask_before(&key);
            return;
        }
        self.environments.stop_asking();
        let environment = self.environments.active_id().map(str::to_owned);
        if tab.draft.kind == RequestKind::Websocket {
            self.sockets.connect(backend, tab, environment);
        } else {
            self.sends.send(backend, tab, (now, environment), false);
        }
    }

    pub fn cancel_send(&self, backend: &Backend, key: &str) {
        self.sends.cancel(backend, key);
    }

    /// The answer shown goes, and what Rust kept of it.
    pub fn clear_response(&mut self, backend: &Backend, key: &str) {
        self.sends.forget(backend, key);
    }

    /// A tab closed takes its send with it.
    fn forget_closed(&mut self, backend: &Backend) {
        let closed: Vec<String> = self
            .sends
            .keys()
            .filter(|key| self.tabs.tab(key).is_none())
            .map(str::to_owned)
            .collect();
        for key in closed {
            self.sends.forget(backend, &key);
        }
        let closed: Vec<String> = self
            .sockets
            .keys()
            .filter(|key| self.tabs.tab(key).is_none())
            .map(str::to_owned)
            .collect();
        for key in closed {
            self.sockets.forget(backend, &key);
        }
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
        self.environments.remember(preferences);
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
