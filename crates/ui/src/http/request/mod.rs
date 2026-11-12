//! The open request: its name, its method and URL on one line, the variables it uses, and its
//! parts a tab each.

pub(crate) mod auth;
mod body;
mod composer;
mod extractions;
mod graphql;
mod parts;
pub(crate) mod table;
pub(crate) mod transport;
mod url;

#[cfg(test)]
mod settings_tests;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use egui::{
    CornerRadius, FontFamily, FontId, Galley, Margin, Rect, Role, Sense, Stroke, StrokeKind,
    TextEdit, Ui, Vec2, WidgetInfo, pos2,
};
use engine::http::model::{HttpMethod, RequestAuth, RequestKind};
use engine::http::scripts::model::ScriptKind;
use engine::http::send::{ImpliedHeader, ImpliedSource};
use engine::http::variables::UsedVariable;
use i18n::keys;

use super::{Action, scripts, variables_table};
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::state::http::tabs::{DraftEdit, Tab};
use crate::theme::palette;
use crate::widgets::editor::Editor;
use crate::widgets::{self, Button, ChoiceMenu, interact};

/// What sends, said in the button.
const SEND_KEYS: &str = "Ctrl ↵";

/// A part of the request, a tab of the editor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Section {
    #[default]
    Params,
    Headers,
    Auth,
    Body,
    Query,
    Variables,
    Scripts,
    Tests,
    Settings,
    Message,
    Protocols,
}

impl Section {
    fn id(self) -> &'static str {
        match self {
            Self::Params => "params",
            Self::Headers => "headers",
            Self::Auth => "auth",
            Self::Body => "body",
            Self::Query => "query",
            Self::Variables => "variables",
            Self::Scripts => "scripts",
            Self::Tests => "tests",
            Self::Settings => "settings",
            Self::Message => "message",
            Self::Protocols => "protocols",
        }
    }

    fn name(self) -> i18n::Key {
        match self {
            Self::Params => keys::http::editor::sections::PARAMS,
            Self::Headers => keys::http::editor::sections::HEADERS,
            Self::Auth => keys::http::editor::sections::AUTH,
            Self::Body => keys::http::editor::sections::BODY,
            Self::Query => keys::http::editor::sections::QUERY,
            Self::Variables => keys::http::editor::sections::VARIABLES,
            Self::Scripts => keys::http::editor::sections::SCRIPTS,
            Self::Tests => keys::http::editor::sections::TESTS,
            Self::Settings => keys::http::editor::sections::SETTINGS,
            Self::Message => keys::http::editor::sections::MESSAGE,
            Self::Protocols => keys::http::editor::sections::PROTOCOLS,
        }
    }
}

/// The parts each kind is edited by, in their order.
fn sections(kind: RequestKind) -> &'static [Section] {
    match kind {
        RequestKind::Http => &[
            Section::Params,
            Section::Headers,
            Section::Auth,
            Section::Body,
            Section::Variables,
            Section::Scripts,
            Section::Tests,
            Section::Settings,
        ],
        RequestKind::Graphql => &[
            Section::Query,
            Section::Headers,
            Section::Auth,
            Section::Variables,
            Section::Scripts,
            Section::Tests,
            Section::Settings,
        ],
        RequestKind::Websocket => &[
            Section::Message,
            Section::Headers,
            Section::Auth,
            Section::Variables,
            Section::Protocols,
        ],
    }
}

/// The boxes of the tab in front, which take its text and its history with it when it is left.
#[derive(Default)]
pub(crate) struct TextBoxes {
    key: String,
    body: Editor,
    query: Editor,
    variables: Editor,
    /// The message being written: the socket's, sent and not saved.
    message: Editor,
    /// `Ctrl+Enter` on a socket that is not open: said, never connected unasked.
    refused: bool,
    protocols: Option<String>,
    /// The auth's secret is shown.
    revealed: bool,
    numbers: transport::Typed,
    secrets_shown: variables_table::Revealed,
    scripts: scripts::ScriptsView,
    tests: scripts::ScriptsView,
}

impl TextBoxes {
    fn of(&mut self, key: &str) -> &mut Self {
        if self.key != key {
            *self = Self {
                key: key.to_owned(),
                ..Self::default()
            };
        }
        self
    }
}

/// What only the editor's drawing sees: the part chosen, kept from one request to the next.
#[derive(Default)]
pub(crate) struct EditorView {
    chosen: Section,
    boxes: TextBoxes,
}

impl EditorView {
    /// The part chosen, or the first when the kind leaves it behind.
    fn section(&self, kind: RequestKind) -> Section {
        let offered = sections(kind);
        if offered.contains(&self.chosen) {
            self.chosen
        } else {
            offered[0]
        }
    }
}

fn edit(actions: &mut Actions, tab: &Tab, edit: DraftEdit) {
    actions.push(Asked::Http(Action::Edit {
        key: tab.key.clone(),
        edit,
    }));
}

/// Answers how tall what it holds is: the line under it follows that while left automatic.
pub(crate) fn show(
    ui: &mut Ui,
    state: &AppState,
    (tab, view): (&Tab, &mut EditorView),
    actions: &mut Actions,
) -> f32 {
    let language = state.language;
    let shown = egui::ScrollArea::vertical()
        .id_salt(("http-editor", &tab.key))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(Margin::symmetric(20, 14))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 12.0;
                    name(ui, tab, actions, language);
                    line(ui, state, tab, actions);
                    let used = state.http.used();
                    if !used.is_empty() {
                        chips(ui, state, used, actions);
                    }
                    if state.http.environments().asking() == Some(tab.key.as_str()) {
                        protected(ui, state, tab, actions);
                    }
                    take_focus(state, tab, view, actions);
                    section_tabs(ui, state, tab, view);
                    let section = view.section(tab.draft.kind);
                    let boxes = view.boxes.of(&tab.key);
                    match section {
                        Section::Params => params(ui, tab, actions, language),
                        Section::Headers => headers(ui, state, tab, actions),
                        Section::Body => body::show(ui, state, tab, &mut boxes.body, actions),
                        Section::Query => graphql::show(ui, state, tab, boxes, actions),
                        Section::Auth => {
                            let field = auth::AuthField {
                                auth: &tab.draft.document.auth,
                                above: tab.place.as_ref().and(state.http.inherited()),
                                allow_inherit: true,
                            };
                            if let Some(auth) =
                                auth::show(ui, &field, &mut boxes.revealed, language)
                            {
                                edit(actions, tab, DraftEdit::Auth(auth));
                            }
                        }
                        Section::Variables => variables(ui, tab, boxes, actions, language),
                        Section::Scripts => {
                            let phases = &[ScriptKind::Before, ScriptKind::After];
                            scripts_of(ui, state, tab, (boxes, phases), actions);
                        }
                        Section::Tests => {
                            let phases = &[ScriptKind::Tests];
                            scripts_of(ui, state, tab, (boxes, phases), actions);
                        }
                        Section::Settings => settings(ui, state, tab, boxes, actions),
                        Section::Message => composer::show(ui, state, tab, boxes, actions),
                        Section::Protocols => {
                            let typed = &mut boxes.protocols;
                            composer::protocols(ui, tab, typed, language, actions);
                        }
                    }
                });
        });
    shown.content_size.y
}

fn name(ui: &mut Ui, tab: &Tab, actions: &mut Actions, language: i18n::Language) {
    let p = palette(ui.ctx());
    let mut name = tab.draft.name.clone();
    let response = ui.add(
        TextEdit::singleline(&mut name)
            .id(ui.scope_id().with(("http-request-name", &tab.key)))
            .frame(egui::Frame::new())
            .margin(Margin::symmetric(4, 2))
            .font(FontId::new(16.0, FontFamily::Proportional))
            .text_color(p.text_0)
            .desired_width(f32::INFINITY),
    );
    let label = i18n::translate(language, keys::http::editor::NAME, &[]);
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_label(label));
    widgets::address(ui, &response, "http-request-name");
    if response.changed() {
        edit(actions, tab, DraftEdit::Name(name));
    }
}

/// The method — or the kind, which is no control — the URL and « Save ».
fn line(ui: &mut Ui, state: &AppState, tab: &Tab, actions: &mut Actions) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        if tab.draft.kind == RequestKind::Http {
            let options: Vec<(HttpMethod, &str)> = HttpMethod::ALL
                .iter()
                .map(|method| (*method, method.as_str()))
                .collect();
            let mut method = tab.draft.method;
            let label = tr(keys::http::editor::METHOD);
            if ChoiceMenu::new(&label, "http-method", &options)
                .show(ui, &mut method)
                .changed()
            {
                edit(actions, tab, DraftEdit::Method(method));
            }
        } else {
            kind_badge(ui, tab.draft.kind, language);
        }
        let save = tr(keys::http::editor::SAVE);
        let sending = state.http.sends().is_sending(&tab.key);
        let send = if sending {
            tr(keys::http::send::CANCEL)
        } else {
            tr(keys::http::send::SEND)
        };
        let sends = tab.draft.kind != RequestKind::Websocket;
        let connect = tr(keys::http::socket::DISCONNECT);
        let send_width = if sends {
            let keys = (!sending).then_some(SEND_KEYS);
            Button::tall_width(ui, &send, keys) + 10.0
        } else {
            Button::tall_width(ui, &connect, None) + 10.0
        };
        let code = tr(keys::http::code::OPEN);
        let code_width = if sends {
            Button::tall_width(ui, &code, None) + 10.0
        } else {
            0.0
        };
        let save_width = Button::tall_width(ui, &save, None);
        let room = ui.available_width() - save_width - send_width - code_width - 10.0;
        let scopes = url::scopes(state.http.used());
        let (label, hint) = (
            tr(keys::http::editor::URL),
            tr(if tab.draft.kind == RequestKind::Websocket {
                keys::http::editor::SOCKET_PLACEHOLDER
            } else {
                keys::http::editor::URL_PLACEHOLDER
            }),
        );
        let field = url::UrlField {
            label: &label,
            hint: &hint,
            scopes: &scopes,
        };
        let mut typed = tab.draft.document.url.clone();
        if url::show(ui, &mut typed, &field, room).changed() {
            edit(actions, tab, DraftEdit::Url(typed));
        }
        if sends {
            send_buttons(ui, state, tab, actions);
        } else {
            connect_button(ui, state, tab, actions);
        }
        if Button::new(&save, "http-save")
            .tall()
            .enabled(tab.is_dirty())
            .show(ui)
            .clicked()
        {
            actions.push(Asked::Http(Action::Save(tab.key.clone())));
        }
    });
}

fn connect_button(ui: &mut Ui, state: &AppState, tab: &Tab, actions: &mut Actions) {
    let live = state.http.sockets().is_live(&tab.key);
    let (key, kind) = if live {
        (keys::http::socket::DISCONNECT, widgets::Kind::Outline)
    } else {
        (keys::http::socket::CONNECT, widgets::Kind::Filled)
    };
    let words = i18n::translate(state.language, key, &[]);
    if Button::new(&words, "http-socket-connect")
        .kind(kind)
        .tall()
        .show(ui)
        .clicked()
    {
        actions.push(Asked::Http(Action::Send(Some(tab.key.clone()))));
    }
}

/// « Send », or « Cancel » while it runs, and « Code ».
fn send_buttons(ui: &mut Ui, state: &AppState, tab: &Tab, actions: &mut Actions) {
    let tr = |key| i18n::translate(state.language, key, &[]);
    let sending = state.http.sends().is_sending(&tab.key);
    let (send, kind) = if sending {
        (tr(keys::http::send::CANCEL), widgets::Kind::Outline)
    } else {
        (tr(keys::http::send::SEND), widgets::Kind::Filled)
    };
    let mut button = Button::new(&send, "http-send").kind(kind).tall();
    if !sending {
        button = button.keys(SEND_KEYS);
    }
    let pressed = button.show(ui);
    let pressed = if sending {
        pressed
    } else {
        widgets::tooltip(pressed, &tr(keys::http::send::SEND_HINT))
    };
    if pressed.clicked() && sending {
        actions.push(Asked::Http(Action::CancelSend(tab.key.clone())));
    } else if pressed.clicked() {
        actions.push(Asked::Http(Action::Send(Some(tab.key.clone()))));
    }
    let code = tr(keys::http::code::OPEN);
    let pressed = Button::new(&code, "http-code-open").tall().show(ui);
    let pressed = widgets::tooltip(pressed, &tr(keys::http::code::OPEN_LABEL));
    if pressed.clicked() {
        let asked = crate::state::http::code::Asked::Open(tab.key.clone());
        actions.push(Asked::Http(Action::Code(asked)));
    }
}

/// Where a method would be chosen on an HTTP request, at its height: a label, not a control.
fn kind_badge(ui: &mut Ui, kind: RequestKind, language: i18n::Language) {
    let p = palette(ui.ctx());
    let key = match kind {
        RequestKind::Graphql => keys::http::kinds::badge::GRAPHQL,
        RequestKind::Http | RequestKind::Websocket => keys::http::kinds::badge::WEBSOCKET,
    };
    let words = i18n::translate(language, key, &[]);
    let galley = widgets::text(ui, &words, 12.0, 700.0, FontFamily::Monospace, p.text_1);
    let size = Vec2::new(galley.size().x + 24.0, url::HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        p.bg_2,
        Stroke::new(1.0, p.line),
        StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, p.text_1);
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    widgets::address(ui, &response, "http-kind-badge");
}

/// Each `{{name}}` the request holds, in the colour of where it resolves from; the tooltip says
/// from where, or that nothing sets it.
fn chips(ui: &mut Ui, state: &AppState, used: &[UsedVariable], actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(6.0);
        for known in used {
            let scope = known.resolved.as_ref().map(|found| found.scope);
            let hue = url::hue(p, scope);
            let written = format!("{{{{{}}}}}", known.name);
            let galley = widgets::text(ui, &written, 12.0, 400.0, FontFamily::Monospace, hue);
            let size = galley.size() + Vec2::new(16.0, 8.0);
            let response = interact(
                ui,
                size,
                Sense::click(),
                "http-used-variable",
                || WidgetInfo::labeled(Role::Label, true, &written),
                CornerRadius::same(6),
            );
            let rect = response.rect;
            ui.painter()
                .rect_filled(rect, CornerRadius::same(6), hue.gamma_multiply(0.16));
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, hue);
            let tip = match &known.resolved {
                Some(found) => {
                    let key = i18n::Key::find(&format!(
                        "http.variables.used.from.{}",
                        found.scope.as_str()
                    ));
                    let from = key.map_or_else(String::new, |key| {
                        i18n::translate(
                            language,
                            key,
                            &[("name", found.from.clone().unwrap_or_default().into())],
                        )
                    });
                    format!("{} · {from}", known.name)
                }
                None => format!(
                    "{} · {} · {}",
                    known.name,
                    i18n::translate(language, keys::http::variables::used::NONE, &[]),
                    define_words(state),
                ),
            };
            let response = widgets::tooltip(response, &tip);
            if response.clicked() && known.resolved.is_none() {
                actions.push(Asked::Http(Action::Define(known.name.clone())));
            }
        }
    });
}

/// « Set in Staging », or in the globals with no environment.
fn define_words(state: &AppState) -> String {
    match state.http.environments().active() {
        Some(active) => i18n::translate(
            state.language,
            keys::http::variables::used::DEFINE,
            &[("name", active.name.as_str().into())],
        ),
        None => i18n::translate(
            state.language,
            keys::http::variables::used::DEFINE_GLOBALS,
            &[],
        ),
    }
}

/// Under a protected environment, what could change something waits for this yes.
fn protected(ui: &mut Ui, state: &AppState, tab: &Tab, actions: &mut Actions) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let name = state
        .http
        .environments()
        .active()
        .map_or("", |active| active.name.as_str());
    let (question, go) = if tab.draft.kind == RequestKind::Websocket {
        (
            i18n::translate(
                language,
                keys::http::environments::ASK_CONNECT,
                &[("name", name.into())],
            ),
            tr(keys::http::environments::CONNECT_ANYWAY),
        )
    } else {
        (
            i18n::translate(
                language,
                keys::http::environments::ASK_SEND,
                &[
                    ("name", name.into()),
                    ("method", tab.draft.method.as_str().into()),
                ],
            ),
            tr(keys::http::environments::SEND_ANYWAY),
        )
    };
    let cancel = tr(keys::http::rail::CANCEL);
    let ids = (
        "http-protected-confirm",
        "http-protected-send",
        "http-protected-cancel",
    );
    match protected_question(ui, (&question, &go, &cancel), ids) {
        Some(true) => actions.push(Asked::Http(Action::SendAnyway(tab.key.clone()))),
        Some(false) => actions.push(Asked::Http(Action::StopAsking)),
        None => {}
    }
}

/// `true` on the go, `false` on the way back. A send's question and a run's read alike.
pub(super) fn protected_question(
    ui: &mut Ui,
    (question, go, cancel): (&str, &str, &str),
    (said_id, go_id, cancel_id): (&str, &str, &str),
) -> Option<bool> {
    let p = palette(ui.ctx());
    let mut chosen = None;
    egui::Frame::new()
        .fill(p.red.gamma_multiply(0.12))
        .stroke(Stroke::new(1.0, p.red.gamma_multiply(0.5)))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let words = egui::RichText::new(question).size(12.5).color(p.text_0);
                let said = ui.add(egui::Label::new(words));
                widgets::address(ui, &said, said_id);
                if Button::new(go, go_id)
                    .kind(widgets::Kind::Destructive)
                    .show(ui)
                    .clicked()
                {
                    chosen = Some(true);
                }
                if Button::new(cancel, cancel_id).show(ui).clicked() {
                    chosen = Some(false);
                }
            });
        });
    chosen
}

fn section_tabs(ui: &mut Ui, state: &AppState, tab: &Tab, view: &mut EditorView) {
    let p = palette(ui.ctx());
    let language = state.language;
    let current = view.section(tab.draft.kind);
    let label = i18n::translate(language, keys::http::editor::PARTS, &[]);
    let row = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for section in sections(tab.draft.kind) {
            let on = *section == current;
            let words = i18n::translate(language, section.name(), &[]);
            let ink = if on { p.text_0 } else { p.text_1 };
            let galley = widgets::text(ui, &words, 13.0, 500.0, FontFamily::Proportional, ink);
            let mark = mark(ui, state, tab, *section);
            let extra = mark
                .as_ref()
                .map_or(0.0, |(galley, _)| galley.size().x + 12.0);
            let size = Vec2::new(galley.size().x + 24.0 + extra, 34.0);
            let response = interact(
                ui,
                size,
                Sense::click(),
                &format!("http-section-{}", section.id()),
                || WidgetInfo::selected(Role::Tab, true, on, &words),
                CornerRadius::ZERO,
            );
            let rect = response.rect;
            ui.painter().galley(
                pos2(rect.left() + 12.0, rect.center().y - galley.size().y / 2.0),
                galley,
                ink,
            );
            if let Some((galley, author_id)) = mark {
                let size = galley.size();
                let at = pos2(rect.right() - 12.0 - size.x, rect.center().y - size.y / 2.0);
                let at = Rect::from_min_size(at, size);
                // A child: `put` would move the cursor, and the next tab with it.
                let mut over = ui.new_child(egui::UiBuilder::new().max_rect(at));
                let said = over.add(egui::Label::new(galley).selectable(false));
                widgets::address(ui, &said, author_id);
            }
            if on {
                let edge = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.max);
                ui.painter()
                    .rect_filled(edge, CornerRadius::ZERO, p.amber_fill);
            }
            if response.clicked() {
                view.chosen = *section;
            }
        }
    });
    let rect = row.response.rect;
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, p.line_soft),
    );
    row.response
        .widget_info(|| WidgetInfo::labeled(Role::TabList, true, &label));
}

/// What a part's tab says beside its name, seen from the others: how many rows it sends, that
/// its auth is inherited, that its body is unusual.
fn mark(
    ui: &Ui,
    state: &AppState,
    tab: &Tab,
    section: Section,
) -> Option<(Arc<Galley>, &'static str)> {
    let p = palette(ui.ctx());
    let language = state.language;
    let count = counted(tab, section);
    let (words, ink, author_id) = if count > 0 {
        (count.to_string(), p.text_2, "http-section-count")
    } else if section == Section::Auth && tab.draft.document.auth == RequestAuth::Inherit {
        let words = i18n::translate(language, keys::http::auth::INHERITS_SHORT, &[]);
        (words, p.text_2, "http-auth-inherits")
    } else if section == Section::Body && state.http.body_answer().is_some_and(|said| said.unusual)
    {
        let method = tab.draft.method.as_str();
        let words = i18n::translate(
            language,
            keys::http::body::UNUSUAL_SHORT,
            &[("method", method.into())],
        );
        (words, p.amber_text, "http-body-unusual-mark")
    } else {
        return None;
    };
    let galley = widgets::text(ui, &words, 11.0, 600.0, FontFamily::Proportional, ink);
    Some((galley, author_id))
}

/// How many rows of a part are sent: the number its tab wears.
fn counted(tab: &Tab, section: Section) -> usize {
    let document = &tab.draft.document;
    let sent = |rows: &[engine::http::model::KeyValue]| {
        rows.iter()
            .filter(|row| row.enabled && !row.key.is_empty())
            .count()
    };
    match section {
        Section::Params => sent(&document.params),
        Section::Headers => sent(&document.headers),
        Section::Variables => document
            .variables
            .iter()
            .filter(|row| row.enabled && !row.name.is_empty())
            .count(),
        Section::Scripts => [&document.scripts.before, &document.scripts.after]
            .iter()
            .filter(|source| !source.trim().is_empty())
            .count(),
        Section::Tests => crate::api::http::tests_declared(&document.scripts.tests),
        Section::Message => document.websocket.messages.len(),
        Section::Protocols => document.websocket.protocols.len(),
        Section::Settings => transport::set_fields(&document.transport),
        Section::Auth | Section::Body | Section::Query => 0,
    }
}

/// « Open the script »: the tab's own, on the phase and the line that stopped.
fn take_focus(state: &AppState, tab: &Tab, view: &mut EditorView, actions: &mut Actions) {
    let Some(focus) = state.http.script_focus().filter(|it| it.key == tab.key) else {
        return;
    };
    let boxes = view.boxes.of(&tab.key);
    if focus.phase == ScriptKind::Tests {
        boxes.tests.focus(focus.phase, focus.line);
        view.chosen = Section::Tests;
    } else {
        boxes.scripts.focus(focus.phase, focus.line);
        view.chosen = Section::Scripts;
    }
    actions.push(Asked::Http(Action::ScriptFocusTaken));
}

fn scripts_of(
    ui: &mut Ui,
    state: &AppState,
    tab: &Tab,
    (boxes, phases): (&mut TextBoxes, &[ScriptKind]),
    actions: &mut Actions,
) {
    let document = &tab.draft.document;
    let above = tab
        .place
        .as_ref()
        .and(state.http.inherited())
        .map_or(&[][..], |above| above.scripts.as_slice());
    let field = scripts::ScriptsField {
        scripts: &document.scripts,
        owner: &tab.key,
        phases,
        above: Some(above),
        request_name: &tab.draft.name,
    };
    let tests = phases == [ScriptKind::Tests];
    let view = if tests {
        &mut boxes.tests
    } else {
        &mut boxes.scripts
    };
    let language = state.language;
    let mut removal = None;
    let changed = scripts::show(ui, &field, view, language, |ui, phase| {
        if phase == ScriptKind::After && !document.extractions.is_empty() {
            removal = extractions::show(ui, tab, language);
        }
    });
    if let Some(rules) = removal {
        edit(actions, tab, DraftEdit::Extractions(rules));
    }
    match changed {
        Some(scripts::Changed::Scripts(scripts)) => {
            edit(actions, tab, DraftEdit::Scripts(scripts));
        }
        Some(scripts::Changed::Level(origin)) => {
            let phase = if tests {
                ScriptKind::Tests
            } else {
                boxes.scripts.phase()
            };
            actions.push(Asked::Http(Action::OpenLevel { origin, phase }));
        }
        None => {}
    }
}

fn settings(
    ui: &mut Ui,
    state: &AppState,
    tab: &Tab,
    boxes: &mut TextBoxes,
    actions: &mut Actions,
) {
    let field = transport::TransportField {
        settings: tab.draft.document.transport,
        base: tab
            .place
            .as_ref()
            .and(state.http.inherited())
            .map_or_else(Default::default, |above| above.transport),
        above_is_default: tab.place.is_none(),
    };
    if let Some(settings) = transport::show(ui, &field, &mut boxes.numbers, state.language) {
        edit(actions, tab, DraftEdit::Transport(settings));
    }
}

fn variables(
    ui: &mut Ui,
    tab: &Tab,
    boxes: &mut TextBoxes,
    actions: &mut Actions,
    language: i18n::Language,
) {
    let label = i18n::translate(language, keys::http::editor::sections::VARIABLES, &[]);
    let table = variables_table::Table {
        kind: "request",
        label: &label,
    };
    let rows = &tab.draft.document.variables;
    match variables_table::show(ui, &table, rows, &mut boxes.secrets_shown, language) {
        Some(variables_table::Changed::Rows(rows)) => {
            edit(actions, tab, DraftEdit::Variables(rows));
        }
        Some(variables_table::Changed::Filler(filler)) => {
            if let Some(id) = filler.request_id {
                actions.push(Asked::Http(Action::OpenRequest(id)));
            }
        }
        None => {}
    }
}

fn params(ui: &mut Ui, tab: &Tab, actions: &mut Actions, language: i18n::Language) {
    let label = i18n::translate(language, keys::http::editor::sections::PARAMS, &[]);
    let table = table::Table {
        kind: "params",
        label: &label,
        headers: false,
    };
    if let Some(rows) = table::show(ui, &table, &tab.draft.document.params, language) {
        edit(actions, tab, DraftEdit::Params(rows));
    }
}

fn headers(ui: &mut Ui, state: &AppState, tab: &Tab, actions: &mut Actions) {
    let language = state.language;
    let implied = state.http.implied();
    if !implied.is_empty() {
        implied_list(ui, implied, language);
    }
    let label = i18n::translate(language, keys::http::editor::sections::HEADERS, &[]);
    let table = table::Table {
        kind: "headers",
        label: &label,
        headers: true,
    };
    if let Some(rows) = table::show(ui, &table, &tab.draft.document.headers, language) {
        edit(actions, tab, DraftEdit::Headers(rows));
    }
}

/// What leaves with the request without a row of its own, and why: Rust composes the request,
/// so Rust says.
fn implied_list(ui: &mut Ui, implied: &[ImpliedHeader], language: i18n::Language) {
    let p = palette(ui.ctx());
    let title = i18n::translate(language, keys::http::headers::IMPLIED, &[]);
    let words = egui::RichText::new(&title).size(11.5).color(p.text_2);
    ui.add(egui::Label::new(words));
    let list = ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        for header in implied {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let name = egui::RichText::new(&header.name)
                    .monospace()
                    .size(12.0)
                    .color(p.text_1);
                ui.add(egui::Label::new(name));
                let value = egui::RichText::new(&header.value)
                    .monospace()
                    .size(12.0)
                    .color(p.text_0);
                let said = ui.add(egui::Label::new(value).truncate());
                widgets::address(ui, &said, "http-implied-value");
                let from = egui::RichText::new(origin(header, language))
                    .size(11.5)
                    .color(p.text_2);
                ui.add(egui::Label::new(from));
            });
        }
    });
    list.response
        .widget_info(|| WidgetInfo::labeled(Role::List, true, &title));
    widgets::address(ui, &list.response, "http-implied-headers");
}

/// « by default », « from the body », or the folder or collection that hands it down.
fn origin(header: &ImpliedHeader, language: i18n::Language) -> String {
    if let Some(from) = &header.from {
        let key = match from.kind {
            engine::http::model::HttpItemKind::Folder => keys::http::auth::from::FOLDER,
            _ => keys::http::auth::from::COLLECTION,
        };
        return i18n::translate(language, key, &[("name", from.name.as_str().into())]);
    }
    let key = match header.source {
        ImpliedSource::Body => keys::http::headers::from::BODY,
        ImpliedSource::Url => keys::http::headers::from::URL,
        ImpliedSource::Default | ImpliedSource::Above => keys::http::headers::from::DEFAULT,
    };
    i18n::translate(language, key, &[])
}
