//! One row of the rail: what opens it, its name, and the ⋯ that opens its panel. A name longer
//! than the rail is cut, never wrapped: a wrapped row breaks the levels.

use egui::{
    Color32, CornerRadius, FontFamily, Pos2, Rect, Response, Role, Sense, Shape, Stroke, Ui, Vec2,
    WidgetInfo, pos2, vec2,
};
use engine::folders::model::FolderColour;

use crate::notes::ui::folder_hue;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, MIN_TARGET, interact};

/// One level of folders: the level is what says where you are.
pub(super) const INDENT: f32 = 16.0;
const HEIGHT: f32 = 26.0;

/// The column before the name, sized once so that every level lines up.
pub(super) enum Lead {
    /// No column at all: « + New folder ».
    None,
    Blank,
    /// « All spaces », standing in for a twisty.
    Sparkle,
    Twisty(Twisty),
}

pub(super) struct Twisty {
    pub(super) open: bool,
    pub(super) label: String,
    pub(super) author_id: &'static str,
}

/// What is drawn before the label, inside the name.
pub(super) enum Mark {
    Nothing,
    Pin,
    Swatch(FolderColour),
    Plus,
}

pub(super) struct Name {
    pub(super) label: String,
    pub(super) author_id: &'static str,
    pub(super) mark: Mark,
    pub(super) active: bool,
}

pub(super) struct Drawn {
    pub(super) twisty: Option<Response>,
    pub(super) name: Response,
    pub(super) menu: Option<Response>,
}

pub(super) fn show(
    ui: &mut Ui,
    depth: usize,
    lead: &Lead,
    name: &Name,
    menu: Option<(&str, &'static str)>,
) -> Drawn {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        #[expect(clippy::cast_precision_loss, reason = "a few levels")]
        ui.add_space(depth as f32 * INDENT);
        let row = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), HEIGHT));
        let lit = ui.rect_contains_pointer(row);
        let twisty = lead_column(ui, lead);
        let room = ui.available_width()
            - if menu.is_some() {
                MIN_TARGET + 2.0
            } else {
                0.0
            };
        let name = name_button(ui, room, name);
        let menu = menu.map(|(label, author_id)| menu_button(ui, label, author_id, lit));
        Drawn { twisty, name, menu }
    })
    .inner
}

fn lead_column(ui: &mut Ui, lead: &Lead) -> Option<Response> {
    let p = palette(ui.ctx());
    match lead {
        Lead::None => None,
        Lead::Blank => {
            ui.allocate_exact_size(Vec2::new(MIN_TARGET, HEIGHT), Sense::hover());
            None
        }
        Lead::Sparkle => {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(MIN_TARGET, HEIGHT), Sense::hover());
            let glyph = Rect::from_center_size(rect.center(), Vec2::splat(12.0));
            icons::paint(ui, glyph, Icon::Sparkle, p.text_2);
            None
        }
        Lead::Twisty(twisty) => {
            let (label, open) = (twisty.label.as_str(), twisty.open);
            let response = interact(
                ui,
                Vec2::new(MIN_TARGET, HEIGHT),
                Sense::click(),
                twisty.author_id,
                || {
                    let mut info = WidgetInfo::labeled(Role::Button, true, label);
                    info.selected = Some(open);
                    info
                },
                CornerRadius::same(5),
            );
            let ink = if response.hovered() {
                p.text_0
            } else {
                p.text_2
            };
            triangle(ui, response.rect.center(), twisty.open, ink);
            Some(response)
        }
    }
}

/// ▾ or ▸, drawn: the glyphs are in none of the embedded faces.
fn triangle(ui: &Ui, centre: Pos2, open: bool, ink: Color32) {
    let points = if open {
        vec![
            centre + vec2(-4.0, -2.0),
            centre + vec2(4.0, -2.0),
            centre + vec2(0.0, 3.0),
        ]
    } else {
        vec![
            centre + vec2(-2.0, -4.0),
            centre + vec2(3.0, 0.0),
            centre + vec2(-2.0, 4.0),
        ]
    };
    ui.painter()
        .add(Shape::convex_polygon(points, ink, Stroke::NONE));
}

fn name_button(ui: &mut Ui, width: f32, name: &Name) -> Response {
    let p = palette(ui.ctx());
    let corner = CornerRadius::same(6);
    let (label, active) = (name.label.as_str(), name.active);
    let response = interact(
        ui,
        Vec2::new(width.max(MIN_TARGET), HEIGHT),
        Sense::click(),
        name.author_id,
        || WidgetInfo::selected(Role::Button, true, active, label),
        corner,
    );
    let rect = response.rect;
    let (fill, ink) = if name.active {
        (p.amber_text.gamma_multiply(0.08), p.amber_text)
    } else if response.hovered() {
        (p.bg_3, p.text_0)
    } else {
        (Color32::TRANSPARENT, p.text_1)
    };
    ui.painter().rect_filled(rect, corner, fill);
    let mut left = rect.left() + 8.0;
    let middle = rect.center().y;
    match name.mark {
        Mark::Nothing => {}
        Mark::Pin => {
            let glyph = Rect::from_center_size(pos2(left + 6.0, middle), Vec2::splat(12.0));
            icons::paint(ui, glyph, Icon::Pin, p.text_2);
            left += 12.0 + 6.0;
        }
        Mark::Swatch(colour) => {
            let swatch = Rect::from_center_size(pos2(left + 4.0, middle), Vec2::splat(8.0));
            ui.painter()
                .rect_filled(swatch, CornerRadius::same(2), folder_hue(ui, colour));
            left += 8.0 + 6.0;
        }
        Mark::Plus => {
            let glyph = Rect::from_center_size(pos2(left + 6.0, middle), Vec2::splat(12.0));
            icons::paint(ui, glyph, Icon::Plus, p.text_2);
            left += 12.0 + 6.0;
        }
    }
    let (size, ink) = if matches!(name.mark, Mark::Plus) {
        (11.5, p.text_2)
    } else {
        (12.5, ink)
    };
    let galley = cut(ui, &name.label, size, ink, rect.right() - 8.0 - left);
    ui.painter().galley(
        pos2(left, middle - galley.size().y / 2.0),
        galley,
        Color32::TRANSPARENT,
    );
    response
}

/// Quiet at rest but never hidden: a control taken away would leave the keyboard's order too.
fn menu_button(ui: &mut Ui, label: &str, author_id: &str, lit: bool) -> Response {
    let p = palette(ui.ctx());
    let corner = CornerRadius::same(5);
    let response = interact(
        ui,
        Vec2::splat(MIN_TARGET),
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::Button, true, label),
        corner,
    );
    let shown = lit || response.hovered() || response.has_focus();
    let ink = if shown {
        ui.painter().rect_filled(response.rect, corner, p.bg_3);
        p.text_0
    } else {
        p.text_2.gamma_multiply(0.35)
    };
    let glyph = Rect::from_center_size(response.rect.center(), Vec2::splat(16.0));
    icons::paint(ui, glyph, Icon::Ellipsis, ink);
    response
}

/// One line, an ellipsis where it is cut.
fn cut(ui: &Ui, words: &str, size: f32, ink: Color32, width: f32) -> std::sync::Arc<egui::Galley> {
    let full = widgets::text(ui, words, size, 400.0, FontFamily::Proportional, ink);
    if full.size().x <= width {
        return full;
    }
    let mut job = egui::text::LayoutJob::single_section(
        words.to_owned(),
        egui::TextFormat::simple(egui::FontId::new(size, FontFamily::Proportional), ink),
    );
    job.wrap = egui::text::TextWrapping {
        max_width: width.max(1.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}
