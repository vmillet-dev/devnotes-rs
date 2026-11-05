//! The HTTP area: its rail of collections beside the open requests (plan § 7 « Phase 6 »). The
//! rail is the library's, and so is its width.

mod close;
mod rail;
mod request;
mod response;
mod tabs;

use egui::{Color32, Rect, Role, Sense, Ui, UiBuilder, Vec2, WidgetInfo, pos2};
use engine::http::model::HttpMethod;
use i18n::keys;

use crate::action::{Action as Asked, Actions, AppAction};
use crate::state::AppState;
use crate::state::http::tree::Badge;
use crate::theme::palette;
use crate::widgets::{self, address};

/// What the area's controls ask, applied at the end of the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// A collection or a folder folded or unfolded.
    Toggle(String),
    Write(crate::state::http::Write),
    /// `Alt` and an arrow on a row: moved, then shown and said.
    KeyMove {
        item: engine::http::model::HttpItem,
        name: String,
        key: crate::state::http::moves::KeyMove,
    },
    /// The question, once Rust has counted what goes with it.
    AskDelete(engine::http::model::HttpItem),
    CancelDelete,
    /// A request of the rail, in its tab.
    OpenRequest(String),
    /// A draft of that kind, written nowhere until saved.
    NewRequest(engine::http::model::RequestKind),
    Activate(String),
    /// Closed at once, or asked first when it holds changes.
    AskClose(String),
    /// Closed, what was typed left.
    Close(String),
    CancelClose,
    Save(String),
    SaveAndClose(String),
    /// `Ctrl+S`: the tab in front, when it holds changes.
    SaveActive,
    Edit {
        key: String,
        edit: crate::state::http::tabs::DraftEdit,
    },
    /// Another kind of body; the one left is set aside.
    ChooseBody {
        key: String,
        kind: crate::state::http::bodies::BodyKind,
    },
    /// A part of the draft formatted, or its variables completed, by the core.
    Rewrite {
        key: String,
        part: crate::state::http::Rewritten,
    },
    /// The system's dialog, for the file a body sends.
    PickFile {
        key: String,
        target: crate::state::http::bodies::FileFor,
    },
    /// The tab's draft, sent; `None` for the tab in front (`Ctrl+Enter`).
    Send(Option<String>),
    CancelSend(String),
    ClearResponse(String),
    /// The request's share of the height, in tenths of a percent; `0` for automatic.
    SetShare(u16),
    /// An answer's JSON, as the explorer asks it.
    Explore {
        send_id: String,
        query: engine::json::model::JsonQuery,
    },
    /// The image an answer is, read once.
    ResponseImage(String),
    /// An answer's text, to the clipboard.
    Copy(String),
}

/// What only the area's drawing sees (D13).
#[derive(Default)]
pub struct View {
    rail_width: widgets::RailWidth,
    rail: rail::RailView,
    editor: request::EditorView,
    response: response::ResponseView,
    /// How tall the request's content was last drawn: what an automatic share follows.
    editor_height: f32,
    /// The share while the line is dragged, written once let go.
    dragged_share: Option<u16>,
}

pub fn show(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let (region, response) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Main, true, "area-http"));
    address(ui, &response, "area-http");
    let mut rest = region;
    if state.settings.rail_shown {
        let setting = (
            state.settings.rail_width,
            crate::services::settings::RAIL_WIDTH,
        );
        rest = view
            .rail_width
            .show(ui, region, setting, "http-rail-edge", |ui| {
                rail::show(ui, state, &mut view.rail, actions);
            });
    }
    let mut strip = ui.new_child(UiBuilder::new().max_rect(rest));
    tabs::show(&mut strip, state, actions);
    let below = Rect::from_min_max(rest.min + Vec2::new(0.0, tabs::HEIGHT), rest.max);
    let mut workspace = ui.new_child(UiBuilder::new().max_rect(below));
    match state.http.tabs().active() {
        Some(tab) => split(&mut workspace, below, state, (tab, view), actions),
        None => empty_workspace(&mut workspace, below, state.language, actions),
    }
    close::show(ui, state, actions);
    if let Some(width) = view.rail_width.rested(ui, state.settings.rail_width) {
        actions.push(Asked::App(AppAction::SetRailWidth(width)));
    }
}

/// What the line between a request and its answer leaves each, in points: the request line and
/// its tabs above, the status line and a few lines of body below.
const FLOORS: (f32, f32) = (148.0, 160.0);
const HANDLE: f32 = 8.0;

/// The request above, its answer below, the line between them dragged to share the height.
/// Left to itself the request takes what it holds, up to 60 %.
fn split(
    ui: &mut Ui,
    region: Rect,
    state: &AppState,
    (tab, view): (&crate::state::http::tabs::Tab, &mut View),
    actions: &mut Actions,
) {
    let total = region.height();
    let share = view
        .dragged_share
        .unwrap_or(state.settings.http_request_share);
    let wanted = if share > 0 {
        total * f32::from(share) / 1000.0
    } else {
        view.editor_height.min(total * 0.6)
    };
    let upper = wanted
        .max(FLOORS.0)
        .min((total - FLOORS.1 - HANDLE).max(FLOORS.0));
    let top = Rect::from_min_size(region.min, Vec2::new(region.width(), upper));
    let line = Rect::from_min_size(top.left_bottom(), Vec2::new(region.width(), HANDLE));
    let bottom = Rect::from_min_max(line.left_bottom(), region.max);
    let mut editor = ui.new_child(UiBuilder::new().max_rect(top));
    editor.set_clip_rect(top);
    view.editor_height = request::show(&mut editor, state, (tab, &mut view.editor), actions);
    let handle = response::split_handle(ui, line, state.language);
    if let Some(at) = handle.interact_pointer_pos().filter(|_| handle.dragged()) {
        view.dragged_share = Some(share_at(at.y - region.top(), total));
    }
    if handle.drag_stopped()
        && let Some(share) = view.dragged_share.take()
    {
        actions.push(Asked::Http(Action::SetShare(share)));
    }
    if handle.double_clicked()
        || (widgets::takes_keys(ui, &handle)
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)))
    {
        actions.push(Asked::Http(Action::SetShare(0)));
    }
    if widgets::takes_keys(ui, &handle) {
        for (key, step) in [(egui::Key::ArrowUp, -16.0), (egui::Key::ArrowDown, 16.0)] {
            if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)) {
                actions.push(Asked::Http(Action::SetShare(share_at(upper + step, total))));
            }
        }
    }
    let mut answer = ui.new_child(UiBuilder::new().max_rect(bottom));
    answer.set_clip_rect(bottom);
    response::show(&mut answer, state, (tab, &mut view.response), actions);
}

/// `upper` points of `total`, as tenths of a percent within the share's bounds.
fn share_at(upper: f32, total: f32) -> u16 {
    let (min, max) = crate::services::settings::HTTP_SHARE;
    let tenths = (upper / total.max(1.0) * 1000.0).round();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped"
    )]
    let tenths = tenths.clamp(f32::from(min), f32::from(max)) as u16;
    tenths
}

/// The first screen of the area says what it can do: the three kinds, as buttons.
fn empty_workspace(ui: &mut Ui, region: Rect, language: i18n::Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, keys::http::workspace::EMPTY, &[]);
    let names: Vec<String> = tabs::KINDS
        .iter()
        .map(|(_, id)| {
            i18n::Key::find(&format!("http.kinds.new.{id}"))
                .map(|key| i18n::translate(language, key, &[]))
                .unwrap_or_default()
        })
        .collect();
    let row: f32 = names
        .iter()
        .map(|name| widgets::Button::width(ui, name) + 8.0)
        .sum::<f32>()
        - 8.0;
    let height = 13.0 * 1.6 + 14.0 + 30.0;
    let top = region.center().y - height / 2.0;
    let mut column = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_max(
                pos2(region.left(), top),
                pos2(region.right(), region.bottom()),
            ))
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    column.spacing_mut().item_spacing.y = 14.0;
    let label = egui::Label::new(egui::RichText::new(&words).size(13.0).color(p.text_2));
    let said = column.add(label);
    address(&column, &said, "http-workspace-empty");
    let (rect, _) = column.allocate_exact_size(Vec2::new(row, 30.0), Sense::hover());
    let mut buttons = column.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    buttons.spacing_mut().item_spacing.x = 8.0;
    for ((kind, id), name) in tabs::KINDS.iter().zip(&names) {
        if widgets::Button::new(name, &format!("http-workspace-new-{id}"))
            .show(&mut buttons)
            .clicked()
        {
            actions.push(Asked::Http(Action::NewRequest(*kind)));
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;

/// The method in its colour, as the mockup draws it: a word, not a badge with a fill.
pub(crate) fn badge_hue(ui: &Ui, badge: Badge) -> Color32 {
    let p = palette(ui.ctx());
    match badge {
        Badge::Method(HttpMethod::Get) => p.green,
        Badge::Method(HttpMethod::Post) => p.amber_text,
        Badge::Method(HttpMethod::Put) => p.blue,
        Badge::Method(HttpMethod::Patch) => p.purple,
        Badge::Method(HttpMethod::Delete) => p.red,
        Badge::Method(HttpMethod::Head | HttpMethod::Options) => p.text_2,
        Badge::Graphql => p.pink,
        Badge::Websocket => p.teal,
    }
}
