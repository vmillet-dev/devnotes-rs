//! Which environment a request is sent with, at the end of the tabs' strip; none means the
//! globals alone. The choice is the library's.

use egui::{
    Color32, CornerRadius, FontFamily, Popup, PopupCloseBehavior, RectAlign, Response, Role, Sense,
    Stroke, Ui, Vec2, WidgetInfo, pos2,
};
use engine::http::environments::{Environment, Variable};
use i18n::keys;

use super::Action;
use crate::action::{Action as Asked, Actions};
use crate::notes::ui::folder_hue;
use crate::state::AppState;
use crate::theme::palette;
use crate::widgets::{self, interact};

const HEIGHT: f32 = 30.0;
const PANEL: f32 = 280.0;
const ROW: f32 = 30.0;
const DOT: f32 = 8.0;

fn dot(ui: &Ui, at: egui::Pos2, hue: Option<Color32>) {
    let p = palette(ui.ctx());
    match hue {
        Some(hue) => {
            ui.painter().circle_filled(at, DOT / 2.0, hue);
        }
        None => {
            ui.painter()
                .circle_stroke(at, DOT / 2.0 - 0.5, Stroke::new(1.0, p.text_2));
        }
    }
}

/// How wide the trigger is drawn: what the strip leaves it.
pub(super) fn width(ui: &Ui, state: &AppState) -> f32 {
    let p = palette(ui.ctx());
    let name = active_name(state);
    let galley = widgets::text(ui, &name, 12.5, 400.0, FontFamily::Proportional, p.text_1);
    galley.size().x + 44.0
}

fn active_name(state: &AppState) -> String {
    state.http.environments().active().map_or_else(
        || i18n::translate(state.language, keys::http::environments::NONE, &[]),
        |active| active.name.clone(),
    )
}

pub(super) fn show(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let trigger = trigger(ui, state);
    Popup::menu(&trigger)
        .frame(widgets::menu_frame(ui.ctx()))
        .align(RectAlign::BOTTOM_END)
        .close_behavior(PopupCloseBehavior::CloseOnClick)
        .show(|ui| panel(ui, state, actions));
}

fn trigger(ui: &mut Ui, state: &AppState) -> Response {
    let p = palette(ui.ctx());
    let name = active_name(state);
    let label = i18n::translate(
        state.language,
        keys::http::environments::PICKER_LABEL,
        &[("name", name.as_str().into())],
    );
    let size = Vec2::new(width(ui, state), HEIGHT);
    let corner = CornerRadius::same(7);
    let response = interact(
        ui,
        size,
        Sense::click(),
        "http-environment-picker",
        || WidgetInfo::labeled(Role::Button, true, &label),
        corner,
    );
    let rect = response.rect;
    ui.painter().rect(
        rect,
        corner,
        p.bg_2,
        Stroke::new(1.0, p.line_soft),
        egui::StrokeKind::Inside,
    );
    let hue = state
        .http
        .environments()
        .active()
        .map(|active| folder_hue(ui, active.colour));
    dot(ui, pos2(rect.left() + 14.0, rect.center().y), hue);
    let galley = widgets::text(ui, &name, 12.5, 400.0, FontFamily::Proportional, p.text_1);
    let at = pos2(rect.left() + 24.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, p.text_1);
    let caret = egui::Rect::from_center_size(
        pos2(rect.right() - 12.0, rect.center().y),
        Vec2::splat(12.0),
    );
    crate::theme::icons::paint(ui, caret, crate::theme::icons::Icon::ChevronDown, p.text_2);
    response
}

fn panel(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.set_width(PANEL);
    ui.spacing_mut().item_spacing.y = 2.0;
    let title = tr(keys::http::environments::ACTIVE_TITLE).to_uppercase();
    let galley = widgets::text(ui, &title, 10.5, 600.0, FontFamily::Proportional, p.text_2);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(PANEL, 22.0), Sense::hover());
    ui.painter()
        .galley(pos2(rect.left() + 10.0, rect.top() + 6.0), galley, p.text_2);
    let environments = state.http.environments();
    let none = Entry {
        name: &tr(keys::http::environments::NONE),
        meta: &tr(keys::http::environments::NONE_META),
        hue: None,
        on: environments.active().is_none(),
    };
    if entry(ui, &none).clicked() {
        choose(actions, None);
    }
    for environment in environments.all() {
        let meta = meta(environment, language);
        let shown = Entry {
            name: &environment.name,
            meta: &meta,
            hue: Some(folder_hue(ui, environment.colour)),
            on: environments.active_id() == Some(environment.id.as_str()),
        };
        if entry(ui, &shown).clicked() {
            choose(actions, Some(environment.id.clone()));
        }
    }
}

fn choose(actions: &mut Actions, id: Option<String>) {
    actions.push(Asked::Http(Action::ChooseEnvironment(id)));
}

/// « protected », or how many variables it holds.
fn meta(environment: &Environment, language: i18n::Language) -> String {
    if environment.protected {
        return i18n::translate(language, keys::http::environments::PROTECTED_BADGE, &[]);
    }
    let named = |variable: &&Variable| !variable.name.is_empty();
    let count = environment.variables.iter().filter(named).count();
    i18n::translate(
        language,
        keys::http::environments::COUNT,
        &[("count", u32::try_from(count).unwrap_or(u32::MAX).into())],
    )
}

/// An entry of the panel.
struct Entry<'a> {
    name: &'a str,
    meta: &'a str,
    hue: Option<Color32>,
    on: bool,
}

fn entry(ui: &mut Ui, shown: &Entry<'_>) -> Response {
    let p = palette(ui.ctx());
    let response = interact(
        ui,
        Vec2::new(PANEL, ROW),
        Sense::click(),
        "http-environment-option",
        || WidgetInfo::selected(Role::MenuItem, true, shown.on, shown.name),
        CornerRadius::same(6),
    );
    let rect = response.rect;
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), p.bg_3);
    }
    let middle = rect.center().y;
    if shown.on {
        let tick =
            egui::Rect::from_center_size(pos2(rect.left() + 14.0, middle), Vec2::splat(12.0));
        crate::theme::icons::paint(ui, tick, crate::theme::icons::Icon::Check, p.amber_text);
    }
    dot(ui, pos2(rect.left() + 32.0, middle), shown.hue);
    let name = widgets::text(
        ui,
        shown.name,
        12.5,
        400.0,
        FontFamily::Proportional,
        p.text_0,
    );
    let at = pos2(rect.left() + 44.0, middle - name.size().y / 2.0);
    ui.painter().galley(at, name, p.text_0);
    let meta = widgets::text(
        ui,
        shown.meta,
        11.0,
        400.0,
        FontFamily::Proportional,
        p.text_2,
    );
    let at = pos2(
        rect.right() - 10.0 - meta.size().x,
        middle - meta.size().y / 2.0,
    );
    ui.painter().galley(at, meta, p.text_2);
    response
}
