//! What every tool is drawn in: the breadcrumb, its star, Example and Clear.

use egui::{CornerRadius, Rect, Role, Sense, Ui, UiBuilder, Vec2, WidgetInfo};
use i18n::keys;

use super::catalogue::ToolEntry;
use super::{Action, Asked, Drawn, View, bar, star, star_hint};
use crate::action::Actions;
use crate::state::AppState;
use crate::theme::palette;
use crate::widgets::{self, Button, address, hovered, interact, towards};

/// The body's margin, on every side.
const PADDING: f32 = 20.0;

pub(super) fn show(
    ui: &mut Ui,
    rest: Rect,
    state: &AppState,
    (entry, view): (&'static ToolEntry, &mut View),
    actions: &mut Actions,
) {
    let (mut row, body) = bar(ui, rest, PADDING, state, actions);
    head(&mut row, state, (entry, view), actions);
    let (_, response) = ui.allocate_exact_size(Vec2::ZERO, Sense::hover());
    let label = entry.id;
    response.widget_info(|| WidgetInfo::labeled(Role::Group, true, label));
    address(ui, &response, "tool-body");
    let inner = body.shrink(PADDING);
    let mut content = ui.new_child(
        UiBuilder::new()
            .id_salt(entry.id)
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    content.set_clip_rect(body);
    let mut drawn = Drawn {
        language: state.language,
        actions,
    };
    view.session(entry).ui(&mut content, &mut drawn);
}

fn head(
    ui: &mut Ui,
    state: &AppState,
    (entry, view): (&'static ToolEntry, &mut View),
    actions: &mut Actions,
) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    if crumb(ui, &tr(keys::tools::TITLE)).clicked() {
        actions.push(Asked::Tools(Action::Home(None)));
    }
    let p = palette(ui.ctx());
    let name = entry.name(language);
    let galley = widgets::wrapped(ui, &name, (16.0, 700.0), p.text_0, f32::INFINITY);
    let (rect, title) = ui.allocate_exact_size(galley.size(), Sense::hover());
    title.widget_info(|| WidgetInfo::labeled(Role::Heading, true, &name));
    address(ui, &title, "tool-title");
    galley.paint(ui, rect.min, p.text_0);
    let on = state.tools.is_favourite(entry.id);
    let label = tr(keys::tools::favourites::TOGGLE);
    if star(ui, on, &label, "tool-favourite", &star_hint(language, on)).clicked() {
        actions.push(Asked::Tools(Action::ToggleFavourite(entry.id)));
    }
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let tool = view.session(entry);
        if Button::new(&tr(keys::tools::CLEAR), "tool-clear")
            .show(ui)
            .clicked()
        {
            tool.clear();
        }
        if tool.has_sample()
            && Button::new(&tr(keys::tools::SAMPLE), "tool-sample")
                .show(ui)
                .clicked()
        {
            tool.sample();
            actions.push(Asked::Tools(Action::SampleLoaded));
        }
    });
}

/// « Tools › », back to the home.
fn crumb(ui: &mut Ui, words: &str) -> egui::Response {
    let p = palette(ui.ctx());
    let shown = format!("{words} ›");
    let galley = widgets::text(
        ui,
        &shown,
        12.5,
        400.0,
        egui::FontFamily::Proportional,
        p.text_2,
    );
    let response = interact(
        ui,
        galley.size(),
        Sense::click(),
        "tool-back",
        || WidgetInfo::labeled(Role::Button, true, words),
        CornerRadius::same(4),
    );
    let ink = towards(p.text_2, p.text_0, hovered(ui, &response));
    let at = response.rect.left_center() - Vec2::new(0.0, galley.size().y / 2.0);
    ui.painter().galley(at, galley, ink);
    response
}
