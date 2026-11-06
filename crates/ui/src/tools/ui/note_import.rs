//! « Import from a note » beside an input the open tool lists in the catalogue's `from_note`, and
//! nothing beside one it does not. The note is copied in once: nothing ties the tool to it
//! afterwards.

use egui::{CornerRadius, FontFamily, Pos2, Rect, Role, Sense, Stroke, Ui, Vec2, WidgetInfo};
use i18n::keys;

use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::tools::Drawn;
use crate::widgets::{self, hovered, interact, towards};

pub fn note_import(ui: &mut Ui, drawn: &mut Drawn<'_>, field: &'static str) {
    let id = format!("note-import-{field}");
    note_import_into(ui, drawn, (field, field), &id);
}

/// For a tool with two inputs alike: `(field, slot)`, the name in `from_note` and which of the two.
pub fn note_import_into(
    ui: &mut Ui,
    drawn: &mut Drawn<'_>,
    (field, slot): (&'static str, &'static str),
    author_id: &str,
) {
    if !drawn.entry.from_note.iter().any(|(name, _)| *name == field) {
        return;
    }
    let p = palette(ui.ctx());
    let words = i18n::translate(drawn.language, keys::tools::from_note::OPEN, &[]);
    let galley = widgets::text(ui, &words, 11.5, 400.0, FontFamily::Proportional, p.text_1);
    let corner = CornerRadius::same(7);
    let response = interact(
        ui,
        Vec2::new(galley.size().x + 10.0 + 13.0 + 6.0 + 10.0, 26.0),
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::Button, true, &words),
        corner,
    );
    let rect = response.rect;
    let hover = hovered(ui, &response);
    ui.painter().rect_stroke(
        rect,
        corner,
        Stroke::new(1.0, towards(p.line, p.amber_dim, hover)),
        egui::StrokeKind::Inside,
    );
    let ink = towards(p.text_1, p.text_0, hover);
    let glyph = Rect::from_center_size(
        Pos2::new(rect.left() + 16.5, rect.center().y),
        Vec2::splat(13.0),
    );
    icons::paint(ui, glyph, Icon::FileText, ink);
    let at = Pos2::new(rect.left() + 29.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, ink);
    if response.clicked() {
        drawn.importing = Some((field, slot));
    }
}
