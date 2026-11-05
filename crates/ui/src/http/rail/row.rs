//! One row of the HTTP rail: a level line per level, the twisty of a collection or a folder, then
//! its name — a request's method before it, in its colour. Nothing is kept on a request's row for
//! a twisty it does not have.

use egui::{Color32, CornerRadius, FontFamily, Rect, Response, Role, Sense, Stroke, Ui, Vec2};
use engine::http::model::{HttpItemKind, HttpMethod};
use i18n::keys;

use crate::state::http::tree::{Badge, RailRow};
use crate::theme::palette;
use crate::widgets::{self, MIN_TARGET, interact};

/// What a level of the tree is set in by: a request two folders deep still shows a dozen
/// characters of its name at the rail's floor.
const INDENT: f32 = 8.0;
pub(super) const HEIGHT: f32 = 26.0;

pub(super) struct Drawn {
    pub(super) twisty: Option<Response>,
    pub(super) name: Response,
}

pub(super) fn show(ui: &mut Ui, row: &RailRow, active: bool, language: i18n::Language) -> Drawn {
    let (whole, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), HEIGHT), Sense::hover());
    level_lines(ui, whole, row.depth);
    #[expect(clippy::cast_precision_loss, reason = "a few levels")]
    let indent = row.depth as f32 * INDENT;
    let rest = Rect::from_min_max(whole.min + Vec2::new(indent, 0.0), whole.max);
    let mut inside = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rest)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    inside.spacing_mut().item_spacing.x = 0.0;
    let twisty = (row.kind != HttpItemKind::Request).then(|| twisty(&mut inside, row, language));
    let name = name_button(&mut inside, row, active);
    Drawn { twisty, name }
}

/// A line a level, so that depth is read at rest: at 8 px a level, a row above a folder and a
/// row inside it are one character apart. Drawn in the indent itself, mid-level.
fn level_lines(ui: &Ui, row: Rect, depth: usize) {
    let p = palette(ui.ctx());
    for level in 0..depth {
        #[expect(clippy::cast_precision_loss, reason = "a few levels")]
        let x = row.left() + level as f32 * INDENT + INDENT / 2.0;
        ui.painter()
            .vline(x.round() + 0.5, row.y_range(), Stroke::new(1.0, p.line));
    }
}

fn twisty(ui: &mut Ui, row: &RailRow, language: i18n::Language) -> Response {
    let p = palette(ui.ctx());
    let key = if row.expanded {
        keys::http::rail::COLLAPSE
    } else {
        keys::http::rail::EXPAND
    };
    let label = i18n::translate(language, key, &[("name", row.name.as_str().into())]);
    let response = interact(
        ui,
        Vec2::new(MIN_TARGET, HEIGHT),
        Sense::click(),
        "http-node-twisty",
        || egui::WidgetInfo::labeled(Role::Button, true, &label),
        CornerRadius::same(5),
    );
    let expanded = row.expanded;
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_expanded(expanded));
    let ink = if response.hovered() {
        p.text_0
    } else {
        p.text_2
    };
    widgets::triangle(ui, response.rect.center(), row.expanded, ink);
    response
}

fn name_button(ui: &mut Ui, row: &RailRow, active: bool) -> Response {
    let p = palette(ui.ctx());
    let corner = CornerRadius::same(6);
    let label = match row.badge {
        Some(badge) => format!("{} {}", badge.short(), row.name),
        None => row.name.clone(),
    };
    let response = interact(
        ui,
        Vec2::new(ui.available_width().max(MIN_TARGET), HEIGHT),
        Sense::click(),
        "http-node-name",
        || egui::WidgetInfo::selected(Role::Button, true, active, &label),
        corner,
    );
    if row.kind != HttpItemKind::Request {
        let expanded = row.expanded;
        ui.ctx()
            .accesskit_node_builder(response.id, |node| node.set_expanded(expanded));
    }
    let rect = response.rect;
    let collection = row.kind == HttpItemKind::Collection;
    let (fill, ink) = if active {
        (p.amber_text.gamma_multiply(0.08), p.amber_text)
    } else if response.hovered() {
        (p.bg_3, p.text_0)
    } else if collection {
        (Color32::TRANSPARENT, p.text_0)
    } else {
        (Color32::TRANSPARENT, p.text_1)
    };
    ui.painter().rect_filled(rect, corner, fill);
    let mut left = rect.left() + 5.0;
    let middle = rect.center().y;
    if let Some(badge) = row.badge {
        let hue = badge_hue(ui, badge);
        let galley = widgets::text(ui, badge.short(), 10.0, 700.0, FontFamily::Monospace, hue);
        let width = galley.size().x;
        ui.painter().galley(
            egui::pos2(left, middle - galley.size().y / 2.0),
            galley,
            hue,
        );
        left += width + 6.0;
    }
    let weight = if collection { 600.0 } else { 400.0 };
    let galley = widgets::one_line(
        ui,
        &row.name,
        (12.5, weight),
        ink,
        rect.right() - 5.0 - left,
    );
    ui.painter().galley(
        egui::pos2(left, middle - galley.size().y / 2.0),
        galley,
        ink,
    );
    response
}

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
