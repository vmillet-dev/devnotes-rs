//! The switch between the areas: at the head of the rail, with their names, and in the bar only
//! while the rail is put away, since nothing else then leads to the other areas.

use egui::{
    CornerRadius, FontFamily, Margin, Rect, Response, Role, Sense, Stroke, Ui, Vec2, WidgetInfo,
    pos2,
};
use i18n::{Key, Language, Message, keys};

use crate::action::{Action as Asked, Actions, AppAction};
use crate::state::Area;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, tooltip};

/// In the switch's order, which is also the order of `Ctrl+1/2/3`.
const AREAS: [(Area, Icon, Key, &str); 3] = [
    (Area::Notes, Icon::FileText, keys::areas::NOTES, "notes"),
    (Area::Tools, Icon::Wrench, keys::areas::TOOLS, "tools"),
    (Area::Http, Icon::ArrowLeftRight, keys::areas::HTTP, "http"),
];

/// What the rail's switch takes in height, its frame included.
const RAIL_OPTION: f32 = 46.0;
/// Below the rail's switch, on top of what the rail keeps between its parts.
const RAIL_GAP: f32 = 4.0;

/// One area's control, its tooltip naming the key; the click is taken here.
fn option(
    ui: &mut Ui,
    size: Vec2,
    (rank, area, name, id): (usize, Area, &str, &str),
    (current, language): (Area, Language),
    actions: &mut Actions,
) -> Response {
    let on = area == current;
    let response = widgets::interact(
        ui,
        size,
        Sense::click(),
        &format!("area-option-{id}"),
        || WidgetInfo::selected(Role::Tab, true, on, name),
        CornerRadius::same(6),
    );
    let hint = Message::new(keys::areas::HINT)
        .with("area", name)
        .with("key", format!("Ctrl+{}", rank + 1))
        .text(language);
    let response = tooltip(response, &hint);
    if on {
        let fill = palette(ui.ctx()).bg_3;
        ui.painter()
            .rect_filled(response.rect, CornerRadius::same(6), fill);
    }
    if response.clicked() && !on {
        actions.push(Asked::App(AppAction::ShowArea(area)));
    }
    response
}

fn ink(ui: &Ui, response: &Response, on: bool) -> egui::Color32 {
    let p = palette(ui.ctx());
    if on {
        p.amber_text
    } else if response.hovered() {
        p.text_0
    } else {
        p.text_2
    }
}

/// In the bar: icons alone, their names for the screen reader and the tooltip.
pub(super) fn bar(ui: &mut Ui, current: Area, language: Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let group = egui::Frame::new()
        .fill(p.bg_2)
        .stroke(Stroke::new(1.0, p.line_soft))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.horizontal(|ui| {
                for (rank, (area, icon, name, id)) in AREAS.into_iter().enumerate() {
                    let name = i18n::translate(language, name, &[]);
                    let size = Vec2::new(26.0, 24.0);
                    let about = (rank, area, name.as_str(), id);
                    let response = option(ui, size, about, (current, language), actions);
                    let glyph = Rect::from_center_size(response.rect.center(), Vec2::splat(14.0));
                    icons::paint(ui, glyph, icon, ink(ui, &response, area == current));
                }
            });
        });
    widgets::address(ui, &group.response, "area-switch");
}

/// At the head of a rail: three equal columns, each its icon over its name.
pub(crate) fn rail(ui: &mut Ui, current: Area, language: Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let group = egui::Frame::new()
        .fill(p.bg_2)
        .stroke(Stroke::new(1.0, p.line_soft))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let width = (ui.available_width() - 8.0) / 3.0;
            ui.horizontal(|ui| {
                for (rank, (area, icon, name, id)) in AREAS.into_iter().enumerate() {
                    let name = i18n::translate(language, name, &[]);
                    let size = Vec2::new(width, RAIL_OPTION);
                    let about = (rank, area, name.as_str(), id);
                    let response = option(ui, size, about, (current, language), actions);
                    let on = area == current;
                    let colour = ink(ui, &response, on);
                    let rect = response.rect;
                    let glyph = Rect::from_center_size(
                        pos2(rect.center().x, rect.top() + 14.0),
                        Vec2::splat(16.0),
                    );
                    icons::paint(ui, glyph, icon, colour);
                    let weight = if on { 600.0 } else { 400.0 };
                    let words =
                        widgets::text(ui, &name, 11.0, weight, FontFamily::Proportional, colour);
                    let at = pos2(rect.center().x - words.size().x / 2.0, rect.top() + 26.0);
                    ui.painter().galley(at, words, colour);
                }
            });
        });
    widgets::address(ui, &group.response, "area-switch");
    ui.add_space(RAIL_GAP);
}
