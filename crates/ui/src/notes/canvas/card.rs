//! A note's card: a fixed 150 px, its title on top.

use egui::{
    Color32, CornerRadius, FontFamily, Rect, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo,
};
use engine::notes::model::DisplayNote;
use i18n::keys;

use crate::theme::{palette, spacing};
use crate::widgets::{self, address};

pub(super) fn show(ui: &mut Ui, rect: Rect, note: &DisplayNote, language: i18n::Language) {
    let p = palette(ui.ctx());
    let padding = spacing(ui.ctx()).card;
    let title = if note.title.is_empty() {
        i18n::translate(language, keys::notes::UNTITLED, &[])
    } else {
        note.title.clone()
    };
    let response = ui.allocate_rect(rect, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Article, true, &title));
    address(ui, &response, "note-card");
    let (fill, edge) = if response.hovered() {
        (p.bg_2, p.line)
    } else {
        (p.bg_1, p.line_soft)
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        fill,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    let galley = widgets::text(ui, &title, 13.0, 600.0, FontFamily::Proportional, p.text_0);
    let inner = rect.shrink(padding);
    ui.painter()
        .with_clip_rect(inner)
        .galley(inner.min, galley, Color32::TRANSPARENT);
}
