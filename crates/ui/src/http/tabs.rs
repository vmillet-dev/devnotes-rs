//! One tab an open request: its method and name, a dot while modified, a × to close. The bar ends
//! with « New request », an HTTP request in one click and the other kinds in two.

use egui::{
    Color32, CornerRadius, FontFamily, Popup, PopupCloseBehavior, Rect, RectAlign, Response, Role,
    Sense, Stroke, Ui, Vec2, WidgetInfo, pos2,
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

pub(super) const HEIGHT: f32 = 40.0;

/// The three a request is created as, in the order they are offered: HTTP first.
pub(super) const KINDS: [(RequestKind, &str); 3] = [
    (RequestKind::Http, "http"),
    (RequestKind::Graphql, "graphql"),
    (RequestKind::Websocket, "websocket"),
];

fn push(actions: &mut Actions, action: Action) {
    actions.push(Asked::Http(action));
}

pub(super) fn show(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let region = ui.max_rect();
    let bar = Rect::from_min_size(region.min, Vec2::new(region.width(), HEIGHT));
    ui.painter().hline(
        bar.x_range(),
        bar.bottom() - 0.5,
        Stroke::new(1.0, p.line_soft),
    );
    let end_width = new_request_width(ui, language);
    let toggle_rect = Rect::from_center_size(
        pos2(bar.left() + 8.0 + 16.0, bar.center().y),
        Vec2::splat(32.0),
    );
    let mut toggle_ui = ui.new_child(egui::UiBuilder::new().max_rect(toggle_rect));
    let toggle = i18n::translate(language, keys::sidebar::TOGGLE, &[]);
    if widgets::SquareButton::new(Icon::Sidebar, &toggle, "http-rail-toggle")
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
                for tab in tabs.tabs() {
                    let active = tabs.active_key() == Some(tab.key.as_str());
                    tab_of(ui, tab, active, actions, language);
                }
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
}

fn tab_of(ui: &mut Ui, tab: &Tab, active: bool, actions: &mut Actions, language: i18n::Language) {
    let p = palette(ui.ctx());
    let badge = badge_of(tab);
    let dirty = tab.is_dirty();
    let name = &tab.draft.name;
    let method = widgets::text(
        ui,
        badge.short(),
        10.5,
        700.0,
        FontFamily::Monospace,
        Color32::WHITE,
    );
    let words = widgets::text(
        ui,
        name,
        13.0,
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
        Sense::click(),
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
    ui.add_space(6.0);
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
    let method = widgets::text(ui, badge.short(), 10.5, 700.0, FontFamily::Monospace, hue);
    let method_width = method.size().x;
    ui.painter()
        .galley(pos2(left, middle - method.size().y / 2.0), method, hue);
    left += method_width + 8.0;
    let ink = if active { p.text_0 } else { p.text_1 };
    let room = open.rect.right() - 4.0 - dot - left;
    let words = widgets::one_line(ui, name, (13.0, 400.0), ink, room);
    let words_width = words.size().x;
    ui.painter()
        .galley(pos2(left, middle - words.size().y / 2.0), words, ink);
    if dirty {
        let centre = pos2(left + words_width + 8.0 + 3.5, middle);
        ui.painter().circle_filled(centre, 3.5, p.amber_text);
    }
    close_glyph(ui, &close);
    if open.clicked() {
        push(actions, Action::Activate(tab.key.clone()));
    }
    // The middle button closes a tab, as in a browser.
    if open.clicked_by(egui::PointerButton::Middle) || close.clicked() {
        push(actions, Action::AskClose(tab.key.clone()));
    }
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
        CornerRadius::same(5),
    )
}

fn close_glyph(ui: &Ui, close: &Response) {
    let p = palette(ui.ctx());
    let square = Rect::from_center_size(close.rect.center(), Vec2::splat(widgets::MIN_TARGET));
    let ink = if close.hovered() {
        ui.painter()
            .rect_filled(square, CornerRadius::same(5), p.bg_3);
        p.text_0
    } else {
        p.text_2
    };
    let glyph = Rect::from_center_size(close.rect.center(), Vec2::splat(14.0));
    icons::paint(ui, glyph, Icon::X, ink);
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
            ui.spacing_mut().item_spacing.y = 2.0;
            let p = palette(ui.ctx());
            let title = tr(keys::http::kinds::MENU_TITLE).to_uppercase();
            let galley = widgets::text(ui, &title, 10.5, 600.0, FontFamily::Proportional, p.text_2);
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
    let corner = CornerRadius::same(6);
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
    let glyph = widgets::text(ui, badge.short(), 10.5, 700.0, FontFamily::Monospace, hue);
    ui.painter().galley(
        pos2(rect.left() + 10.0, rect.center().y - glyph.size().y / 2.0),
        glyph,
        hue,
    );
    let left = rect.left() + 54.0;
    let name = widgets::text(ui, &name, 12.5, 600.0, FontFamily::Proportional, p.text_0);
    ui.painter()
        .galley(pos2(left, rect.top() + 6.0), name, p.text_0);
    let line = widgets::one_line(
        ui,
        &line,
        (11.0, 400.0),
        p.text_2,
        rect.right() - 10.0 - left,
    );
    ui.painter()
        .galley(pos2(left, rect.top() + 24.0), line, p.text_2);
    response
}
