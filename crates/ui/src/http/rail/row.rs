//! One row of the HTTP rail: a level line per level, the twisty of a collection or a folder, then
//! its name — a request's method before it, in its colour. Nothing is kept on a request's row for
//! a twisty it does not have, nor at rest for its `⋯`, drawn over the end of the row once the
//! pointer or the keyboard is on it.

use egui::{
    Color32, CornerRadius, FontFamily, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use engine::http::model::{HttpItemKind, HttpMethod};
use i18n::keys;

use crate::state::http::moves::DropZone;
use crate::state::http::tree::{Badge, RailRow};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, MIN_TARGET, interact};

/// What a level of the tree is set in by: a request two folders deep still shows a dozen
/// characters of its name at the rail's floor.
pub(super) const INDENT: f32 = 8.0;
pub(super) const HEIGHT: f32 = 26.0;

/// How a row is drawn this frame, besides what it is.
#[derive(Default)]
pub(super) struct Look {
    pub(super) active: bool,
    /// A drop on it would land there.
    pub(super) drop: Option<DropZone>,
    /// Moved with the keys, or what now holds what was: how lit, 1 down to 0.
    pub(super) moved: Option<(Moved, f32)>,
    pub(super) dragged: bool,
    /// Its name is a field: no name button, no menu.
    pub(super) renaming: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Moved {
    Row,
    Parent,
}

pub(super) struct Drawn {
    pub(super) twisty: Option<Response>,
    pub(super) name: Option<Response>,
    pub(super) menu: Option<Response>,
    /// Where the name stands, a field in its place while it is renamed.
    pub(super) rest: Rect,
}

pub(super) fn show(ui: &mut Ui, row: &RailRow, look: &Look, language: i18n::Language) -> Drawn {
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
    if look.dragged {
        inside.multiply_opacity(0.45);
    }
    let twisty = (row.kind != HttpItemKind::Request).then(|| twisty(&mut inside, row, language));
    let rest = Rect::from_min_max(inside.cursor().min, rest.max);
    if look.renaming {
        return Drawn {
            twisty,
            name: None,
            menu: None,
            rest,
        };
    }
    let lit = inside.rect_contains_pointer(rest);
    let (name, menu) = name_and_menu(&mut inside, row, look, (lit, language));
    if let Some(zone) = look.drop {
        drop_mark(ui, rest, zone);
    }
    Drawn {
        twisty,
        name: Some(name),
        menu: Some(menu),
        rest,
    }
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

fn name_and_menu(
    ui: &mut Ui,
    row: &RailRow,
    look: &Look,
    (lit, language): (bool, i18n::Language),
) -> (Response, Response) {
    let p = palette(ui.ctx());
    let corner = CornerRadius::same(6);
    let label = match row.badge {
        Some(badge) => format!("{} {}", badge.short(), row.name),
        None => row.name.clone(),
    };
    let active = look.active;
    let name = interact(
        ui,
        Vec2::new(ui.available_width().max(MIN_TARGET), HEIGHT),
        // A row is dragged onto another; a press that never travels is its click.
        Sense::click_and_drag(),
        "http-node-name",
        || egui::WidgetInfo::selected(Role::Button, true, active, &label),
        corner,
    );
    if row.kind != HttpItemKind::Request {
        let expanded = row.expanded;
        ui.ctx()
            .accesskit_node_builder(name.id, |node| node.set_expanded(expanded));
    }
    let rect = name.rect;
    let collection = row.kind == HttpItemKind::Collection;
    let (fill, ink) = if active {
        (p.amber_text.gamma_multiply(0.08), p.amber_text)
    } else if name.hovered() {
        (p.bg_3, p.text_0)
    } else if collection {
        (Color32::TRANSPARENT, p.text_0)
    } else {
        (Color32::TRANSPARENT, p.text_1)
    };
    ui.painter().rect_filled(rect, corner, fill);
    moved_mark(ui, rect, look.moved);
    let menu_rect = Rect::from_min_size(
        rect.right_top() + Vec2::new(-MIN_TARGET, (HEIGHT - MIN_TARGET) / 2.0),
        Vec2::splat(MIN_TARGET),
    );
    let menu = menu_button(ui, menu_rect, row, language);
    let shown = lit || name.has_focus() || menu.has_focus() || menu_open(ui, &menu);
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
    let right = if shown {
        menu_rect.left()
    } else {
        rect.right() - 5.0
    };
    let weight = if collection { 600.0 } else { 400.0 };
    let galley = widgets::one_line(ui, &row.name, (12.5, weight), ink, right - left);
    ui.painter().galley(
        egui::pos2(left, middle - galley.size().y / 2.0),
        galley,
        ink,
    );
    if shown {
        let glyph = Rect::from_center_size(menu_rect.center(), Vec2::splat(16.0));
        if menu.hovered() {
            ui.painter()
                .rect_filled(menu_rect, CornerRadius::same(5), p.bg_3);
        }
        icons::paint(ui, glyph, Icon::Ellipsis, p.text_0);
    }
    (name, menu)
}

/// Over the end of the name, there at rest for the keyboard's order, inked only when shown.
fn menu_button(ui: &mut Ui, rect: Rect, row: &RailRow, language: i18n::Language) -> Response {
    let label = i18n::translate(
        language,
        keys::http::rail::OPTIONS,
        &[("name", row.name.as_str().into())],
    );
    let mut over = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    interact(
        &mut over,
        rect.size(),
        Sense::click(),
        "http-node-menu",
        || egui::WidgetInfo::labeled(Role::Button, true, &label),
        CornerRadius::same(5),
    )
}

fn menu_open(ui: &Ui, menu: &Response) -> bool {
    egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(menu))
}

/// Where a drop would land: a line between two rows, or the row lit when it would hold it.
fn drop_mark(ui: &Ui, rest: Rect, zone: DropZone) {
    let p = palette(ui.ctx());
    let line = |y: f32| {
        let bar = Rect::from_min_max(
            egui::pos2(rest.left() + 4.0, y - 1.0),
            egui::pos2(rest.right() - 4.0, y + 1.0),
        );
        ui.painter()
            .rect_filled(bar, CornerRadius::same(1), p.amber_text);
    };
    match zone {
        DropZone::Before => line(rest.top()),
        DropZone::After => line(rest.bottom()),
        DropZone::Inside => {
            let corner = CornerRadius::same(6);
            ui.painter()
                .rect_filled(rest, corner, p.amber_text.gamma_multiply(0.08));
            ui.painter().rect_stroke(
                rest,
                corner,
                Stroke::new(1.0, p.amber_dim),
                StrokeKind::Inside,
            );
        }
    }
}

/// A move made with the keys: the row in the accent, what now holds it ringed as a drop target
/// is, fading for as long as the mark lasts.
fn moved_mark(ui: &Ui, rect: Rect, moved: Option<(Moved, f32)>) {
    let Some((moved, strength)) = moved else {
        return;
    };
    let p = palette(ui.ctx());
    let corner = CornerRadius::same(6);
    let painter = ui.painter();
    match moved {
        Moved::Row => {
            painter.rect_filled(rect, corner, p.amber_text.gamma_multiply(0.16 * strength));
        }
        Moved::Parent => {
            painter.rect_stroke(
                rect,
                corner,
                Stroke::new(1.0, p.amber_dim.gamma_multiply(strength)),
                StrokeKind::Inside,
            );
        }
    }
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
