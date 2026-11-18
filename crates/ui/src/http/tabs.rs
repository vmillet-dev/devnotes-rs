//! One tab an open request: its method and name, a dot while modified, a × to close. A tab is
//! dragged along the strip, or moved by `Alt` and an arrow. The bar ends with « New request », an
//! HTTP request in one click and the other kinds in two.

use crate::theme::tokens::{BAR_HEIGHT, BAR_INSET, BAR_TOGGLE, font, gap, radius};
use egui::{
    Color32, CornerRadius, FontFamily, Key, Modifiers, PointerButton, Popup, PopupCloseBehavior,
    Rect, RectAlign, Response, Role, Sense, Stroke, Ui, Vec2, WidgetInfo, pos2, vec2,
};
use engine::http::model::{HttpMethod, RequestKind};
use i18n::keys;

use super::Action;
use super::badge_hue;
use crate::action::{Action as Asked, Actions, AppAction};
use crate::state::AppState;
use crate::state::http::tabs::Tab;
use crate::state::http::tree::Badge;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, interact};

pub(super) const HEIGHT: f32 = BAR_HEIGHT;

/// The three a request is created as, in the order they are offered: HTTP first.
pub(super) const KINDS: [(RequestKind, &str); 3] = [
    (RequestKind::Http, "http"),
    (RequestKind::Graphql, "graphql"),
    (RequestKind::Websocket, "websocket"),
];

/// Held this close to an end of the strip, a dragged tab scrolls it, by this much a frame.
const EDGE: f32 = 32.0;
const EDGE_STEP: f32 = 8.0;

/// What only the strip's drawing sees: the tab being dragged.
#[derive(Default)]
pub(super) struct TabsView {
    dragging: Option<String>,
}

fn push(actions: &mut Actions, action: Action) {
    actions.push(Asked::Http(action));
}

pub(super) fn show(ui: &mut Ui, state: &AppState, view: &mut TabsView, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let region = ui.max_rect();
    let bar = Rect::from_min_size(region.min, Vec2::new(region.width(), HEIGHT));
    ui.painter().hline(
        bar.x_range(),
        bar.bottom() - 0.5,
        Stroke::new(1.0, p.line_soft),
    );
    let end_width = end_width(ui, state);
    let toggle_rect = Rect::from_center_size(
        pos2(bar.left() + BAR_INSET + BAR_TOGGLE / 2.0, bar.center().y),
        Vec2::splat(BAR_TOGGLE),
    );
    let mut toggle_ui = ui.new_child(egui::UiBuilder::new().max_rect(toggle_rect));
    let toggle = i18n::translate(language, keys::sidebar::TOGGLE, &[]);
    if widgets::SquareButton::new(Icon::Sidebar, &toggle, "http-rail-toggle")
        .keys(
            state
                .settings
                .bindings
                .of(crate::services::shortcuts::RAIL_ACTION),
        )
        .pressed(state.settings.rail_shown)
        .show(&mut toggle_ui)
        .clicked()
    {
        actions.push(Asked::App(AppAction::ToggleRail));
    }
    let mut strip = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(
                pos2(toggle_rect.right() + 8.0, bar.top()),
                pos2(bar.right() - end_width - 20.0, bar.bottom()),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Min)),
    );
    strip.spacing_mut().item_spacing = Vec2::ZERO;
    let tabs = state.http.tabs();
    let shown = egui::ScrollArea::horizontal()
        .id_salt("http-tabs")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(&mut strip, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                let mut drawn = Vec::new();
                for tab in tabs.tabs() {
                    let active = tabs.active_key() == Some(tab.key.as_str());
                    // Under the tab's key: the focus stays on a tab the keys move.
                    let id = ui.scope_id().with(("http-tab", &tab.key));
                    let (open, whole) = ui
                        .scope_builder(egui::UiBuilder::new().scope_id(id), |ui| {
                            tab_of(ui, tab, active, actions, language)
                        })
                        .inner;
                    if open.drag_started_by(PointerButton::Primary) {
                        view.dragging = Some(tab.key.clone());
                    }
                    drawn.push(whole);
                }
                let at = tabs
                    .tabs()
                    .iter()
                    .position(|tab| view.dragging.as_ref() == Some(&tab.key));
                drop_dragged(ui, view, (&drawn, at), actions);
            });
        });
    let label = i18n::translate(language, keys::http::tabs::LABEL, &[]);
    let list = widgets::region(&mut strip, shown.inner_rect, "http-tabs", Sense::hover());
    list.widget_info(|| WidgetInfo::labeled(Role::TabList, true, &label));
    widgets::address(&strip, &list, "http-tabs");
    let end = Rect::from_min_max(
        pos2(bar.right() - end_width - 10.0, bar.top()),
        pos2(bar.right() - 10.0, bar.bottom()),
    );
    let mut end_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(end)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    new_request(&mut end_ui, actions, language);
    end_ui.add_space(gap::CONTROLS);
    super::environment_picker::show(&mut end_ui, state, actions);
}

/// The tab and its ×: what answers for the tab itself, and the room both take.
fn tab_of(
    ui: &mut Ui,
    tab: &Tab,
    active: bool,
    actions: &mut Actions,
    language: i18n::Language,
) -> (Response, Rect) {
    let p = palette(ui.ctx());
    let badge = badge_of(tab);
    let dirty = tab.is_dirty();
    let name = &tab.draft.name;
    let method = widgets::text(
        ui,
        badge.short(),
        font::CAPTION,
        700.0,
        FontFamily::Monospace,
        Color32::WHITE,
    );
    let words = widgets::text(
        ui,
        name,
        font::READING,
        400.0,
        FontFamily::Proportional,
        Color32::WHITE,
    );
    let dot = if dirty { 8.0 + 7.0 } else { 0.0 };
    let width = (14.0 + method.size().x + 8.0 + words.size().x + dot + 4.0).min(240.0 - 34.0);
    let modified = i18n::translate(language, keys::http::tabs::MODIFIED, &[]);
    // Reserved under what is drawn next: the active tab's fill goes under its words.
    let fill = ui.painter().add(egui::Shape::Noop);
    let label = if dirty {
        format!("{} {name}, {modified}", badge.short())
    } else {
        format!("{} {name}", badge.short())
    };
    let open = interact(
        ui,
        Vec2::new(width, HEIGHT),
        // Dragged along the strip; a press that never travels is its click.
        Sense::click_and_drag(),
        "http-tab-open",
        || WidgetInfo::selected(Role::Tab, true, active, &label),
        CornerRadius::ZERO,
    );
    let close_label = i18n::translate(
        language,
        keys::http::tabs::CLOSE,
        &[("name", name.as_str().into())],
    );
    let close = close_button(ui, &close_label);
    ui.add_space(gap::SNUG);
    let whole = open.rect.union(close.rect);
    let whole = Rect::from_min_max(whole.min, pos2(whole.max.x + 6.0, whole.max.y));
    if active {
        ui.painter().set(
            fill,
            egui::epaint::RectShape::filled(whole, CornerRadius::ZERO, p.bg_1),
        );
        let edge = Rect::from_min_max(pos2(whole.left(), whole.bottom() - 2.0), whole.max);
        ui.painter()
            .rect_filled(edge, CornerRadius::ZERO, p.amber_fill);
    }
    ui.painter().vline(
        whole.right() - 0.5,
        whole.y_range(),
        Stroke::new(1.0, p.line_soft),
    );
    let middle = open.rect.center().y;
    let mut left = open.rect.left() + 14.0;
    let hue = badge_hue(ui, badge);
    let method = widgets::text(
        ui,
        badge.short(),
        font::CAPTION,
        700.0,
        FontFamily::Monospace,
        hue,
    );
    let method_width = method.size().x;
    ui.painter()
        .galley(pos2(left, middle - method.size().y / 2.0), method, hue);
    left += method_width + 8.0;
    let ink = if active { p.text_0 } else { p.text_1 };
    let room = open.rect.right() - 4.0 - dot - left;
    let words = widgets::one_line(ui, name, (font::READING, 400.0), ink, room);
    let words_width = words.size().x;
    ui.painter()
        .galley(pos2(left, middle - words.size().y / 2.0), words, ink);
    if dirty {
        let centre = pos2(left + words_width + 8.0 + 3.5, middle);
        ui.painter().circle_filled(centre, 3.5, p.amber_text);
    }
    close_glyph(ui, &close);
    asked(ui, tab, (&open, &close), actions);
    (open, whole)
}

/// What the pointer and the keys ask of a tab: in front, closed, moved one place.
fn asked(ui: &Ui, tab: &Tab, (open, close): (&Response, &Response), actions: &mut Actions) {
    // What is clicked takes the keys: `Alt` and an arrow move it.
    if open.clicked() {
        open.request_focus();
        push(actions, Action::Activate(tab.key.clone()));
    }
    // The middle button closes a tab, as in a browser: a press that moved too, let go on it.
    let middle = open.clicked_by(PointerButton::Middle)
        || (open.drag_stopped_by(PointerButton::Middle) && open.contains_pointer());
    if middle || close.clicked() {
        push(actions, Action::AskClose(tab.key.clone()));
    }
    if widgets::takes_keys(ui, open) {
        for (key, right) in [(Key::ArrowLeft, false), (Key::ArrowRight, true)] {
            if ui.input_mut(|input| input.consume_key(Modifiers::ALT, key)) {
                let key = tab.key.clone();
                push(actions, Action::NudgeTab { key, right });
            }
        }
    }
}

/// Where the dragged tab would land, marked by a bar between two tabs; let go, it is moved
/// there. Held at an end of the strip, the strip scrolls toward the tabs it hides.
fn drop_dragged(
    ui: &mut Ui,
    view: &mut TabsView,
    (drawn, at): (&[Rect], Option<usize>),
    actions: &mut Actions,
) {
    let (Some(key), Some(from)) = (view.dragging.clone(), at) else {
        view.dragging = None;
        return;
    };
    let (down, at) = ui.input(|input| (input.pointer.primary_down(), input.pointer.latest_pos()));
    let Some(at) = at else {
        view.dragging = None;
        return;
    };
    let gap = drawn.iter().filter(|rect| rect.center().x < at.x).count();
    // Either side of the tab itself is where it already is.
    let moves = gap != from && gap != from + 1;
    if down {
        if moves {
            mark_gap(ui, drawn, gap);
        }
        let seen = ui.clip_rect();
        let step = if at.x < seen.left() + EDGE {
            EDGE_STEP
        } else if at.x > seen.right() - EDGE {
            -EDGE_STEP
        } else {
            return;
        };
        ui.scroll_with_delta(vec2(step, 0.0));
        // The pointer may rest there: the strip goes on scrolling without it moving.
        ui.ctx().request_repaint();
        return;
    }
    view.dragging = None;
    if moves {
        push(actions, Action::MoveTab { key, gap });
    }
}

/// The rail's drop bar, upright: in the accent, before the tab at `gap` or after the last one.
fn mark_gap(ui: &Ui, drawn: &[Rect], gap: usize) {
    let (Some(first), Some(last)) = (drawn.first(), drawn.last()) else {
        return;
    };
    let x = drawn.get(gap).map_or(last.right(), Rect::left);
    let p = palette(ui.ctx());
    let bar = Rect::from_min_max(
        pos2(x - 1.0, first.top() + gap::SNUG),
        pos2(x + 1.0, first.bottom() - gap::SNUG),
    );
    ui.painter()
        .rect_filled(bar, CornerRadius::same(radius::MARK), p.amber_text);
}

fn badge_of(tab: &Tab) -> Badge {
    match tab.draft.kind {
        RequestKind::Http => Badge::Method(tab.draft.method),
        RequestKind::Graphql => Badge::Graphql,
        RequestKind::Websocket => Badge::Websocket,
    }
}

fn close_button(ui: &mut Ui, label: &str) -> Response {
    interact(
        ui,
        Vec2::new(widgets::MIN_TARGET, HEIGHT),
        Sense::click(),
        "http-tab-close",
        || WidgetInfo::labeled(Role::Button, true, label),
        CornerRadius::same(radius::CONTROL),
    )
}

fn close_glyph(ui: &Ui, close: &Response) {
    let p = palette(ui.ctx());
    let square = Rect::from_center_size(close.rect.center(), Vec2::splat(widgets::MIN_TARGET));
    let ink = if close.hovered() {
        ui.painter()
            .rect_filled(square, CornerRadius::same(radius::CONTROL), p.bg_3);
        p.text_0
    } else {
        p.text_2
    };
    let glyph = Rect::from_center_size(close.rect.center(), Vec2::splat(14.0));
    icons::paint(ui, glyph, Icon::X, ink);
}

/// What the end of the strip takes: « New request » and the environment beside it.
fn end_width(ui: &Ui, state: &AppState) -> f32 {
    new_request_width(ui, state.language) + super::environment_picker::width(ui, state) + 8.0
}

fn new_request_width(ui: &Ui, language: i18n::Language) -> f32 {
    let label = i18n::translate(language, keys::http::tabs::NEW, &[]);
    widgets::Button::width(ui, &label) + 26.0
}

/// « + New request », split: an HTTP request in one click, the other kinds in two.
fn new_request(ui: &mut Ui, actions: &mut Actions, language: i18n::Language) {
    let tr = |key| i18n::translate(language, key, &[]);
    ui.spacing_mut().item_spacing.x = 0.0;
    let main = widgets::Button::new(&tr(keys::http::tabs::NEW), "http-tab-new").show(ui);
    if main.clicked() {
        push(actions, Action::NewRequest(RequestKind::Http));
    }
    let caret = caret(ui, &tr(keys::http::tabs::NEW_KIND), main.rect.height());
    Popup::menu(&caret)
        .frame(widgets::menu_frame(ui.ctx()))
        .align(RectAlign::BOTTOM_END)
        .close_behavior(PopupCloseBehavior::CloseOnClick)
        .show(|ui| {
            ui.set_width(300.0);
            ui.spacing_mut().item_spacing.y = gap::LINE;
            let p = palette(ui.ctx());
            let title = tr(keys::http::kinds::MENU_TITLE).to_uppercase();
            let galley = widgets::text(
                ui,
                &title,
                font::CAPTION,
                600.0,
                FontFamily::Proportional,
                p.text_2,
            );
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(galley.size().x, 22.0), Sense::hover());
            ui.painter()
                .galley(pos2(rect.left() + 10.0, rect.top() + 6.0), galley, p.text_2);
            for (kind, id) in KINDS {
                if kind_item(ui, kind, id, language).clicked() {
                    push(actions, Action::NewRequest(kind));
                }
            }
        });
}

/// The chevron joined to « New request », framed as it is.
fn caret(ui: &mut Ui, label: &str, height: f32) -> Response {
    let p = palette(ui.ctx());
    let corner = CornerRadius {
        nw: 0,
        sw: 0,
        ne: 6,
        se: 6,
    };
    let response = interact(
        ui,
        Vec2::new(26.0, height),
        Sense::click(),
        "http-tab-new-kind",
        || WidgetInfo::labeled(Role::Button, true, label),
        corner,
    );
    let edge = if response.hovered() {
        p.amber_dim
    } else {
        p.line_soft
    };
    ui.painter().rect(
        response.rect,
        corner,
        p.bg_2,
        Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    let glyph = Rect::from_center_size(response.rect.center(), Vec2::splat(12.0));
    icons::paint(ui, glyph, Icon::ChevronDown, p.text_1);
    response
}

/// A kind of request: the badge its tab and its row will wear, its name, what it is for.
fn kind_item(ui: &mut Ui, kind: RequestKind, id: &str, language: i18n::Language) -> Response {
    let p = palette(ui.ctx());
    let name = i18n::Key::find(&format!("http.kinds.new.{id}"))
        .map(|key| i18n::translate(language, key, &[]))
        .unwrap_or_default();
    let line = i18n::Key::find(&format!("http.kinds.lines.{id}"))
        .map(|key| i18n::translate(language, key, &[]))
        .unwrap_or_default();
    let corner = CornerRadius::same(radius::CONTROL);
    let response = interact(
        ui,
        Vec2::new(ui.available_width(), 44.0),
        Sense::click(),
        &format!("http-new-kind-{id}"),
        || WidgetInfo::labeled(Role::MenuItem, true, &name),
        corner,
    );
    let rect = response.rect;
    if response.hovered() {
        ui.painter().rect_filled(rect, corner, p.bg_3);
    }
    let badge = match kind {
        RequestKind::Http => Badge::Method(HttpMethod::Get),
        RequestKind::Graphql => Badge::Graphql,
        RequestKind::Websocket => Badge::Websocket,
    };
    let hue = badge_hue(ui, badge);
    let glyph = widgets::text(
        ui,
        badge.short(),
        font::CAPTION,
        700.0,
        FontFamily::Monospace,
        hue,
    );
    ui.painter().galley(
        pos2(rect.left() + 10.0, rect.center().y - glyph.size().y / 2.0),
        glyph,
        hue,
    );
    let left = rect.left() + 54.0;
    let name = widgets::text(
        ui,
        &name,
        font::BODY,
        600.0,
        FontFamily::Proportional,
        p.text_0,
    );
    ui.painter()
        .galley(pos2(left, rect.top() + 6.0), name, p.text_0);
    let line = widgets::one_line(
        ui,
        &line,
        (font::META, 400.0),
        p.text_2,
        rect.right() - 10.0 - left,
    );
    ui.painter()
        .galley(pos2(left, rect.top() + 24.0), line, p.text_2);
    response
}
