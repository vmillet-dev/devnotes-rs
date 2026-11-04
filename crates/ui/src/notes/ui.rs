//! What the notes' screens share: a kind's badge, a priority's pill, a folder's hue.

use egui::{
    Color32, CornerRadius, FontFamily, Pos2, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui,
    Vec2, WidgetInfo,
};
use engine::folders::model::FolderColour;
use engine::notes::kind::NoteKind;
use engine::notes::language::Language;
use engine::notes::priority::Priority;
use i18n::keys;

use super::language::{self as languages, Hue};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, address, hue_badge};

/// A snippet names its format; a Note, a list, a sheet and a drawing say what they are.
pub fn kind_badge(
    ui: &mut Ui,
    kind: NoteKind,
    language: Language,
    locale: i18n::Language,
) -> Response {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(locale, key, &[]);
    let mut icon = None;
    let (label, hue) = match kind {
        NoteKind::Snippet => (
            languages::label(language).to_string(),
            language_hue(ui, language),
        ),
        NoteKind::Note => (tr(keys::notes::kind_tag::NOTE), p.purple),
        NoteKind::Checklist => {
            icon = Some(Icon::Check);
            (tr(keys::notes::kind_tag::CHECKLIST), p.text_1)
        }
        NoteKind::Calc => (tr(keys::notes::kind_tag::CALC), p.pink),
        NoteKind::Drawing => (tr(keys::notes::kind_tag::DRAWING), p.teal),
    };
    let response = hue_badge(ui, &label, hue, icon);
    address(ui, &response, "kind-badge");
    response
}

/// The colour of a format's family: the label names it, the hue only groups it.
pub fn language_hue(ui: &Ui, language: Language) -> Color32 {
    let p = palette(ui.ctx());
    match languages::hue(language) {
        Hue::Blue => p.blue,
        Hue::Amber => p.amber_text,
        Hue::Green => p.green,
        Hue::Purple => p.purple,
        Hue::Red => p.red,
        Hue::Neutral => p.text_1,
    }
}

/// Three bars, the name and the hue: three clues. Nothing for no priority.
pub fn priority_pill(ui: &mut Ui, priority: Priority, locale: i18n::Language) -> Option<Response> {
    let p = palette(ui.ctx());
    let (ink, edge, bars) = match priority {
        Priority::None => return None,
        Priority::Low => (p.text_2, p.line, 1),
        Priority::Medium => (p.blue, p.blue.gamma_multiply(0.45), 2),
        Priority::High => (p.amber_text, p.amber_text.gamma_multiply(0.45), 3),
        Priority::Urgent => (p.red, p.red.gamma_multiply(0.45), 3),
    };
    let key = match priority {
        Priority::Low => keys::notes::priority::LOW,
        Priority::Medium => keys::notes::priority::MEDIUM,
        Priority::High => keys::notes::priority::HIGH,
        Priority::Urgent | Priority::None => keys::notes::priority::URGENT,
    };
    let name = i18n::translate(locale, key, &[]);
    let galley = widgets::text(ui, &name, 10.5, 600.0, FontFamily::Proportional, ink);
    let size = Vec2::new(7.0 + 10.5 + 5.0 + galley.size().x + 7.0, 18.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &name));
    address(ui, &response, "priority-pill");
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(10),
        Color32::TRANSPARENT,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    let base = rect.left() + 7.0;
    for (bar, height) in [4.0, 7.0, 10.0].into_iter().enumerate() {
        let colour = if bar < bars { ink } else { p.line_no };
        #[expect(clippy::cast_precision_loss, reason = "three bars")]
        let left = base + bar as f32 * 4.5;
        let bottom = rect.center().y + 5.0;
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(left, bottom - height),
                Pos2::new(left + 3.0, bottom),
            ),
            CornerRadius::same(1),
            colour,
        );
    }
    painter.galley(
        Pos2::new(base + 10.5 + 5.0, rect.center().y - galley.size().y / 2.0),
        galley,
        ink,
    );
    Some(response)
}

/// A task's box, drawn rather than typed: ☐ and ☑ are in none of the embedded faces.
pub fn tick_box(ui: &Ui, rect: Rect, done: bool, colour: Color32) {
    let side = rect.width().min(rect.height());
    let square = Rect::from_center_size(rect.center(), Vec2::splat(side)).shrink(1.0);
    ui.painter().rect(
        square,
        CornerRadius::same(2),
        Color32::TRANSPARENT,
        Stroke::new(1.0, colour),
        StrokeKind::Inside,
    );
    if done {
        icons::paint(ui, square.shrink(1.0), Icon::Check, colour);
    }
}

/// The five hues a folder is told apart by, on the board, the rail and its cards' chips.
pub fn folder_hue(ui: &Ui, colour: FolderColour) -> Color32 {
    let p = palette(ui.ctx());
    match colour {
        FolderColour::Blue => p.blue,
        FolderColour::Amber => p.amber_text,
        FolderColour::Purple => p.purple,
        FolderColour::Green => p.green,
        FolderColour::Red => p.red,
    }
}
