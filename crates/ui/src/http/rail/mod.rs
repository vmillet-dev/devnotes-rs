//! The collections, their folders and requests, in the library rail's frame (`http-rail` in the
//! reference). Only the rows in view are drawn: a library holds thousands of requests. Moving is
//! a pointer drag, doubled by `Alt` and the arrows.

mod forms;
mod row;

use egui::{FontFamily, Key, Margin, Modifiers, Pos2, Rect, Ui, UiBuilder, Vec2};
use engine::http::model::{HttpItem, HttpItemKind, RequestKind};
use i18n::keys;

use super::Action;
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::state::http::Write;
use crate::state::http::moves::{self, DropZone, KeyMove};
use crate::state::http::tree::RailRow;
use crate::theme::icons::Icon;
use crate::theme::{palette, spacing};
use crate::widgets::{self, Button, IconButton, MenuItem};
use forms::{Draft, Typed};
use row::{Look, Moved};

/// From this width the head has room for « ＋ Collection » beside its title; under it, an icon.
const WORDED_FROM: f32 = 216.0;

/// What only the rail's drawing sees: the name being typed, what is dragged and where it would
/// land, and the rows as last drawn.
#[derive(Default)]
pub(super) struct RailView {
    form: Option<Form>,
    dragging: Option<RailRow>,
    drop: Option<(String, DropZone)>,
    drawn: Vec<(Rect, usize)>,
    /// The height of what is opened under a row — a field, a question — as last drawn.
    attached: f32,
    /// The beat of the last move scrolled into view.
    scrolled: Option<u32>,
}

/// A name typed under a row, or in its place: one at a time.
enum Form {
    Create(Creating, Draft),
    Rename(String, Draft),
}

/// A collection at the head, a folder or a request under the row it is made in.
#[derive(Clone)]
struct Creating {
    made: Made,
    collection_id: String,
    folder_id: Option<String>,
    /// The row it is typed under, `None` at the head of the rail.
    under: Option<String>,
    depth: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Made {
    Collection,
    Folder,
    Request(RequestKind),
}

impl RailView {
    fn creating_under(&self, id: Option<&str>) -> Option<&Creating> {
        match &self.form {
            Some(Form::Create(creating, _)) if creating.under.as_deref() == id => Some(creating),
            _ => None,
        }
    }

    fn renaming(&self, id: &str) -> bool {
        matches!(&self.form, Some(Form::Rename(renamed, _)) if renamed == id)
    }
}

pub(super) fn show(ui: &mut Ui, state: &AppState, view: &mut RailView, actions: &mut Actions) {
    let s = spacing(ui.ctx());
    // The tree is wider than the head by what the rail's padding can spare: the rows are what
    // run out of room.
    let margin = Margin {
        left: 4,
        right: 4,
        top: to_margin(s.card),
        bottom: to_margin(s.canvas),
    };
    let http = &state.http;
    let language = state.language;
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        ui.set_min_height(ui.available_height());
        ui.spacing_mut().item_spacing.y = 6.0;
        head(ui, view, actions, language);
        if view.creating_under(None).is_some() {
            create_form(ui, view, actions, language);
        } else if http.loaded() && http.rows().is_empty() {
            empty(ui, view, actions, language);
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        tree(ui, state, view, actions);
        said(ui, state);
    });
    drop_dragged(ui, state, view, actions);
}

fn to_margin(points: f32) -> i8 {
    #[expect(clippy::cast_possible_truncation, reason = "a few points")]
    let margin = points.round() as i8;
    margin
}

fn push(actions: &mut Actions, action: Action) {
    actions.push(Asked::Http(action));
}

fn item(row: &RailRow) -> HttpItem {
    HttpItem {
        kind: row.kind,
        id: row.id.clone(),
    }
}

/// « COLLECTIONS », and what makes one: its name when there is room, an icon otherwise. Not a
/// bare « + », a few pixels from the tabs' own.
fn head(ui: &mut Ui, view: &mut RailView, actions: &mut Actions, language: i18n::Language) {
    let p = palette(ui.ctx());
    let new = i18n::translate(language, keys::http::rail::NEW_COLLECTION, &[]);
    let worded = (ui.available_width() + 8.0 >= WORDED_FROM).then(|| {
        format!(
            "＋ {}",
            i18n::translate(language, keys::http::rail::COLLECTION, &[])
        )
    });
    ui.horizontal(|ui| {
        let title = i18n::translate(language, keys::http::rail::TITLE, &[]).to_uppercase();
        let galley = widgets::text(ui, &title, 10.5, 600.0, FontFamily::Proportional, p.text_2);
        let button = worded
            .as_deref()
            .map_or(widgets::MIN_TARGET, |words| Button::width(ui, words));
        let room = ui.available_width() - button - ui.spacing().item_spacing.x;
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(room, widgets::MIN_TARGET), egui::Sense::hover());
        ui.painter().galley(
            Pos2::new(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0),
            galley,
            p.text_2,
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, &title));
        widgets::address(ui, &response, "http-rail-title");
        let clicked = if let Some(words) = &worded {
            let response = Button::new(words, "http-collection-create-open").show(ui);
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_label(new.as_str()));
            response.clicked()
        } else {
            IconButton::new(Icon::FolderPlus, &new, "http-collection-create-open")
                .show(ui)
                .clicked()
        };
        if clicked {
            start_collection(view, actions);
        }
    });
}

fn start_collection(view: &mut RailView, actions: &mut Actions) {
    push(actions, Action::CancelDelete);
    let creating = Creating {
        made: Made::Collection,
        collection_id: String::new(),
        folder_id: None,
        under: None,
        depth: 0,
    };
    view.form = Some(Form::Create(creating, Draft::default()));
}

/// An empty rail offers what to do as a button, rather than pointing at a glyph.
fn empty(ui: &mut Ui, view: &mut RailView, actions: &mut Actions, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, keys::http::rail::EMPTY, &[]);
    let new = i18n::translate(language, keys::http::rail::NEW_COLLECTION, &[]);
    egui::Frame::new()
        .inner_margin(Margin::symmetric(10, 4))
        .show(ui, |ui| {
            let label =
                egui::Label::new(egui::RichText::new(&words).size(11.5).color(p.text_2)).wrap();
            let response = ui.add(label);
            widgets::address(ui, &response, "http-rail-empty");
            if Button::new(&new, "http-rail-empty-create")
                .show(ui)
                .clicked()
            {
                start_collection(view, actions);
            }
        });
}

/// The rows in view, and what is opened under one of them, its height taken from the frame
/// before.
fn tree(ui: &mut Ui, state: &AppState, view: &mut RailView, actions: &mut Actions) {
    let rows = state.http.rows();
    let attached = attached_row(state, view);
    let extra = if attached.is_some() {
        view.attached
    } else {
        0.0
    };
    let scroll_to = state
        .http
        .said()
        .map(|(beat, _)| *beat)
        .filter(|beat| view.scrolled != Some(*beat))
        .and(state.http.mark().map(|mark| mark.id.clone()));
    view.drawn.clear();
    let mut measured = None;
    egui::ScrollArea::vertical()
        .id_salt("http-rail-tree")
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            #[expect(clippy::cast_precision_loss, reason = "rows of a rail")]
            let total = rows.len() as f32 * row::HEIGHT + extra;
            let width = ui.available_width();
            let (whole, _) = ui.allocate_exact_size(Vec2::new(width, total), egui::Sense::hover());
            for (at, row) in rows.iter().enumerate() {
                #[expect(clippy::cast_precision_loss, reason = "rows of a rail")]
                let mut top = at as f32 * row::HEIGHT;
                if attached.is_some_and(|under| under < at) {
                    top += extra;
                }
                let rect = Rect::from_min_size(
                    whole.min + Vec2::new(0.0, top),
                    Vec2::new(width, row::HEIGHT),
                );
                let seen = rect.translate(-whole.min.to_vec2());
                if scroll_to.as_deref() == Some(row.id.as_str()) {
                    ui.scroll_to_rect(rect, None);
                }
                let under = attached == Some(at);
                if !seen.intersects(viewport) && !under {
                    continue;
                }
                // ⚠️ An explicit id, not a salt: a salted child's widgets take their ids from its
                // rank, and a row moved with the keys would lose its focus.
                let id = ui.scope_id().with(("http-row", &row.id));
                let mut row_ui = ui.new_child(UiBuilder::new().scope_id(id).max_rect(rect));
                draw_row(&mut row_ui, state, view, actions, row);
                view.drawn.push((rect, at));
                if under {
                    let below =
                        Rect::from_min_size(rect.left_bottom(), Vec2::new(width, f32::INFINITY));
                    let mut below_ui = ui.new_child(
                        UiBuilder::new()
                            .scope_id(id.with("attached"))
                            .max_rect(below),
                    );
                    attached_under(&mut below_ui, state, view, actions, row);
                    measured = Some(below_ui.min_rect().height());
                }
            }
        });
    view.attached = measured.unwrap_or(0.0);
    if scroll_to.is_some() {
        view.scrolled = state.http.said().map(|(beat, _)| *beat);
    }
}

/// The row a field or a question is opened under: the deletion asked, or a name being typed.
fn attached_row(state: &AppState, view: &RailView) -> Option<usize> {
    let id = state
        .http
        .deleting()
        .filter(|deleting| deleting.contents.is_some())
        .map(|deleting| deleting.item.id.as_str())
        .or(match &view.form {
            Some(Form::Create(creating, _)) => creating.under.as_deref(),
            _ => None,
        })?;
    state.http.rows().iter().position(|row| row.id == id)
}

fn draw_row(
    ui: &mut Ui,
    state: &AppState,
    view: &mut RailView,
    actions: &mut Actions,
    row: &RailRow,
) {
    let language = state.language;
    let look = Look {
        active: state.http.tabs().active_request() == Some(row.id.as_str()),
        drop: view
            .drop
            .as_ref()
            .filter(|(id, _)| *id == row.id)
            .map(|(_, zone)| *zone),
        moved: moved(ui.ctx(), state, row),
        dragged: view
            .dragging
            .as_ref()
            .is_some_and(|dragged| dragged.id == row.id),
        renaming: view.renaming(&row.id),
    };
    if look.moved.is_some() {
        ui.ctx().request_repaint();
    }
    let drawn = row::show(ui, row, &look, language);
    if drawn.twisty.is_some_and(|twisty| twisty.clicked()) {
        push(actions, Action::Toggle(row.id.clone()));
    }
    if look.renaming {
        rename_form(ui, drawn.rest, view, actions, row, language);
        return;
    }
    let (Some(name), Some(menu)) = (drawn.name, drawn.menu) else {
        return;
    };
    // As a button in the reference: what is clicked takes the keys, F2 and Alt with an arrow.
    if name.clicked() {
        name.request_focus();
    }
    if name.clicked() {
        let opened = if row.kind == HttpItemKind::Request {
            Action::OpenRequest(row.id.clone())
        } else {
            Action::Toggle(row.id.clone())
        };
        push(actions, opened);
    }
    if name.drag_started() {
        view.dragging = Some(row.clone());
    }
    row_keys(ui, &name, view, actions, row);
    if let Some(chosen) = node_menu(&menu, row, language) {
        on_menu(view, actions, row, chosen, language);
    }
}

/// How lit the mark of a move made with the keys still is on this row, if it is on it.
fn moved(ctx: &egui::Context, state: &AppState, row: &RailRow) -> Option<(Moved, f32)> {
    let mark = state.http.mark()?;
    let which = if mark.id == row.id {
        Moved::Row
    } else if mark.parent_id.as_deref() == Some(row.id.as_str()) {
        Moved::Parent
    } else {
        return None;
    };
    // The mark is the style the fade comes down to: where motion is cut short it stays whole.
    let strength = if state.reduced_motion {
        1.0
    } else {
        crate::theme::motion::fading(ctx, mark.since)
    };
    Some((which, strength))
}

/// `Alt` and an arrow move the row, `F2` renames it, `Delete` asks before deleting it.
fn row_keys(
    ui: &Ui,
    name: &egui::Response,
    view: &mut RailView,
    actions: &mut Actions,
    row: &RailRow,
) {
    if !widgets::takes_keys(ui, name) {
        return;
    }
    let keys = [
        (Key::ArrowUp, KeyMove::Up),
        (Key::ArrowDown, KeyMove::Down),
        (Key::ArrowRight, KeyMove::In),
        (Key::ArrowLeft, KeyMove::Out),
    ];
    for (key, moved) in keys {
        if ui.input_mut(|input| input.consume_key(Modifiers::ALT, key)) {
            push(
                actions,
                Action::KeyMove {
                    item: item(row),
                    name: row.name.clone(),
                    key: moved,
                },
            );
        }
    }
    if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::F2)) {
        start_rename(view, actions, row);
    }
    if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Delete)) {
        view.form = None;
        push(actions, Action::AskDelete(item(row)));
    }
}

fn start_rename(view: &mut RailView, actions: &mut Actions, row: &RailRow) {
    push(actions, Action::CancelDelete);
    view.form = Some(Form::Rename(row.id.clone(), Draft::named(&row.name)));
}

/// The author ids of a row's menu, the address of what it does.
const NEW_HTTP: &str = "http-node-new-http";
const NEW_GRAPHQL: &str = "http-node-new-graphql";
const NEW_WEBSOCKET: &str = "http-node-new-websocket";
const NEW_FOLDER: &str = "http-node-new-folder";
const RENAME: &str = "http-node-rename";
const DUPLICATE: &str = "http-node-duplicate";
const DELETE: &str = "http-node-delete";

/// A request holds nothing: it is renamed, copied or deleted. The three kinds of request sit
/// behind one entry, as a card's priorities do.
fn node_menu(
    menu: &egui::Response,
    row: &RailRow,
    language: i18n::Language,
) -> Option<&'static str> {
    use keys::http::{kinds::new, rail::actions};
    let tr = |key| i18n::translate(language, key, &[]);
    let words = [
        tr(new::HTTP),
        tr(new::GRAPHQL),
        tr(new::WEBSOCKET),
        tr(actions::NEW_REQUEST),
        tr(actions::NEW_FOLDER),
        tr(actions::RENAME),
        tr(actions::DUPLICATE),
        tr(actions::DELETE),
    ];
    let kinds = [
        MenuItem::new(&words[0], NEW_HTTP),
        MenuItem::new(&words[1], NEW_GRAPHQL),
        MenuItem::new(&words[2], NEW_WEBSOCKET),
    ];
    let rename = MenuItem::new(&words[5], RENAME);
    let duplicate = MenuItem::new(&words[6], DUPLICATE);
    let delete = MenuItem::new(&words[7], DELETE).destructive();
    let chosen = if row.kind == HttpItemKind::Request {
        widgets::menu(menu, &[rename, duplicate, delete]).map(str::to_owned)
    } else {
        let items = [
            MenuItem::new(&words[3], "http-node-new").children(&kinds),
            MenuItem::new(&words[4], NEW_FOLDER),
            rename,
            duplicate,
            delete,
        ];
        widgets::menu(menu, &items).map(str::to_owned)
    }?;
    [
        NEW_HTTP,
        NEW_GRAPHQL,
        NEW_WEBSOCKET,
        NEW_FOLDER,
        RENAME,
        DUPLICATE,
        DELETE,
    ]
    .into_iter()
    .find(|known| *known == chosen)
}

fn on_menu(
    view: &mut RailView,
    actions: &mut Actions,
    row: &RailRow,
    chosen: &str,
    language: i18n::Language,
) {
    let made = match chosen {
        NEW_HTTP => Made::Request(RequestKind::Http),
        NEW_GRAPHQL => Made::Request(RequestKind::Graphql),
        NEW_WEBSOCKET => Made::Request(RequestKind::Websocket),
        NEW_FOLDER => Made::Folder,
        RENAME => return start_rename(view, actions, row),
        DUPLICATE => {
            let name = i18n::translate(
                language,
                keys::http::rail::COPY_NAME,
                &[("name", row.name.as_str().into())],
            );
            let write = Write::Duplicate {
                item: item(row),
                name,
            };
            return push(actions, Action::Write(write));
        }
        _ => {
            view.form = None;
            return push(actions, Action::AskDelete(item(row)));
        }
    };
    push(actions, Action::CancelDelete);
    let creating = Creating {
        made,
        collection_id: row.collection_id.clone(),
        folder_id: (row.kind == HttpItemKind::Folder).then(|| row.id.clone()),
        under: Some(row.id.clone()),
        depth: row.depth + 1,
    };
    view.form = Some(Form::Create(creating, Draft::default()));
}

/// What opens under a row: the name of what is made in it, or the question before deleting it.
fn attached_under(
    ui: &mut Ui,
    state: &AppState,
    view: &mut RailView,
    actions: &mut Actions,
    row: &RailRow,
) {
    let language = state.language;
    if let Some(deleting) = state.http.deleting()
        && deleting.item.id == row.id
        && let Some(contents) = deleting.contents
    {
        let under = crate::state::http::tree::requests_under(state.http.tree(), &item(row));
        let unsaved = state.http.tabs().unsaved_among(&under);
        forms::confirm_delete(ui, (row, contents, &unsaved), actions, language);
        return;
    }
    if view.creating_under(Some(&row.id)).is_some() {
        create_form(ui, view, actions, language);
    }
}

fn create_form(ui: &mut Ui, view: &mut RailView, actions: &mut Actions, language: i18n::Language) {
    let Some(Form::Create(creating, draft)) = &mut view.form else {
        return;
    };
    let creating = creating.clone();
    let placeholder = match creating.made {
        Made::Collection => keys::http::rail::placeholders::COLLECTION,
        Made::Folder => keys::http::rail::placeholders::FOLDER,
        Made::Request(_) => keys::http::rail::placeholders::REQUEST,
    };
    let words = forms::Words {
        hint: i18n::translate(language, placeholder, &[]),
        submit: i18n::translate(language, keys::http::rail::CREATE, &[]),
    };
    #[expect(clippy::cast_precision_loss, reason = "a few levels")]
    let indent = 4.0 + creating.depth as f32 * row::INDENT;
    let typed = ui
        .horizontal(|ui| {
            ui.add_space(indent);
            ui.vertical(|ui| {
                forms::name_form(
                    ui,
                    draft,
                    &words,
                    ("http-create-input", "http-create-submit"),
                )
            })
            .inner
        })
        .inner;
    match typed {
        Typed::Typing => {}
        Typed::Dropped => view.form = None,
        Typed::Name(name) => {
            view.form = None;
            let write = match creating.made {
                Made::Collection => Write::CreateCollection(name),
                Made::Folder => Write::CreateFolder {
                    collection_id: creating.collection_id,
                    folder_id: creating.folder_id,
                    name,
                },
                Made::Request(kind) => Write::CreateRequest {
                    collection_id: creating.collection_id,
                    folder_id: creating.folder_id,
                    name,
                    kind,
                },
            };
            push(actions, Action::Write(write));
        }
    }
}

fn rename_form(
    ui: &mut Ui,
    rest: Rect,
    view: &mut RailView,
    actions: &mut Actions,
    row: &RailRow,
    language: i18n::Language,
) {
    let Some(Form::Rename(_, draft)) = &mut view.form else {
        return;
    };
    let words = forms::Words {
        hint: i18n::translate(
            language,
            keys::http::rail::RENAME_LABEL,
            &[("name", row.name.as_str().into())],
        ),
        submit: i18n::translate(language, keys::http::rail::actions::RENAME, &[]),
    };
    let mut inside = ui.new_child(
        UiBuilder::new()
            .max_rect(rest)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let typed = forms::inline_form(
        &mut inside,
        draft,
        &words,
        ("http-rename-input", "http-rename-submit"),
    );
    match typed {
        Typed::Typing => {}
        Typed::Dropped => view.form = None,
        Typed::Name(name) => {
            view.form = None;
            if name != row.name {
                let write = Write::Rename {
                    item: item(row),
                    name,
                };
                push(actions, Action::Write(write));
            }
        }
    }
}

/// The row under the pointer and which part of it: a container takes what lands mid-row, a
/// collection whatever is not a collection.
fn drop_dragged(ui: &Ui, state: &AppState, view: &mut RailView, actions: &mut Actions) {
    let Some(dragged) = view.dragging.clone() else {
        return;
    };
    let (down, at) = ui.input(|input| (input.pointer.any_down(), input.pointer.latest_pos()));
    let rows = state.http.rows();
    let target = at.and_then(|at| {
        let (rect, index) = view.drawn.iter().find(|(rect, _)| rect.contains(at))?;
        let target = &rows[*index];
        let share = (at.y - rect.top()) / rect.height();
        let zone = match target.kind {
            HttpItemKind::Request if share < 0.5 => DropZone::Before,
            HttpItemKind::Request => DropZone::After,
            HttpItemKind::Collection if dragged.kind != HttpItemKind::Collection => {
                DropZone::Inside
            }
            _ if share < 0.25 => DropZone::Before,
            _ if share > 0.75 => DropZone::After,
            _ => DropZone::Inside,
        };
        let railmove = moves::drop_move(state.http.tree(), &item(&dragged), &item(target), zone)?;
        Some((target.id.clone(), zone, railmove))
    });
    view.drop = target.as_ref().map(|(id, zone, _)| (id.clone(), *zone));
    if down {
        ui.ctx().request_repaint();
        return;
    }
    view.dragging = None;
    view.drop = None;
    if let Some((_, _, railmove)) = target {
        push(actions, Action::Write(Write::Move(railmove)));
    }
}

/// Where the last move went, said in a live region kept on the page so that its change is
/// announced.
fn said(ui: &mut Ui, state: &AppState) {
    let sentence = state
        .http
        .said()
        .map(|(_, message)| message.text(state.language))
        .unwrap_or_default();
    let (_, response) = ui.allocate_exact_size(Vec2::new(1.0, 1.0), egui::Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &sentence));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_live(egui::accesskit::Live::Polite);
    });
    widgets::address(ui, &response, "http-rail-moved");
}
