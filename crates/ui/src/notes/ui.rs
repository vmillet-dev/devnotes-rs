//! What the notes' screens share: a kind's icon, a snippet's format, a priority's pill and
//! bars, a folder's hue.

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
use crate::widgets::{self, address};

/// A kind's hue: a tile is told apart by its colour before its shape. Olive is the one hue no
/// language uses, so a snippet's tile never reads as its format.
pub fn kind_hue(ui: &Ui, kind: NoteKind) -> Color32 {
    let p = palette(ui.ctx());
    match kind {
        NoteKind::Snippet => p.olive,
        NoteKind::Note => p.purple,
        NoteKind::Checklist => p.text_1,
        NoteKind::Calc => p.pink,
        NoteKind::Drawing => p.teal,
    }
}

/// What a kind is called: a card's icon says it to the screen reader and in its tooltip.
pub fn kind_name(kind: NoteKind, locale: i18n::Language) -> String {
    let key = match kind {
        NoteKind::Snippet => keys::notes::kind_tag::SNIPPET,
        NoteKind::Note => keys::notes::kind_tag::NOTE,
        NoteKind::Checklist => keys::notes::kind_tag::CHECKLIST,
        NoteKind::Calc => keys::notes::kind_tag::CALC,
        NoteKind::Drawing => keys::notes::kind_tag::DRAWING,
    };
    i18n::translate(locale, key, &[])
}

fn kind_glyph(kind: NoteKind) -> Icon {
    match kind {
        NoteKind::Snippet => Icon::CodeXml,
        NoteKind::Note => Icon::FileText,
        NoteKind::Checklist => Icon::SquareCheck,
        NoteKind::Calc => Icon::Sigma,
        NoteKind::Drawing => Icon::Brush,
    }
}

/// The kind's icon on a tile tinted with its hue, filling `rect`: painted, never named.
pub fn kind_tile(ui: &Ui, rect: Rect, kind: NoteKind) {
    let hue = kind_hue(ui, kind);
    let corner = CornerRadius::same(rect_corner(rect));
    ui.painter()
        .rect_filled(rect, corner, hue.gamma_multiply(0.18));
    let glyph = Rect::from_center_size(rect.center(), Vec2::splat(rect.width() * 2.0 / 3.0));
    icons::paint(ui, glyph, kind_glyph(kind), hue);
}

fn rect_corner(rect: Rect) -> u8 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a tile of a few dozen points"
    )]
    let corner = (rect.width() * 0.28).round() as u8;
    corner
}

/// A kind as its icon alone, 18 px: its name is its tooltip and what a screen reader says. Learnt
/// where a kind is chosen, beside its word (the « New note » menu, the palette).
pub fn kind_icon(ui: &mut Ui, kind: NoteKind, locale: i18n::Language) -> Response {
    let name = kind_name(kind, locale);
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Image, true, &name));
    address(ui, &response, "kind-icon");
    kind_tile(ui, rect, kind);
    widgets::tooltip(response, &name)
}

/// A snippet's format, as its abbreviation in its language's hue; plain text says « Texte » in
/// grey, so the place is never empty. Nothing for the other kinds, which have no format.
pub fn format_label(
    ui: &mut Ui,
    kind: NoteKind,
    language: Language,
    locale: i18n::Language,
) -> Option<Response> {
    if kind != NoteKind::Snippet {
        return None;
    }
    let p = palette(ui.ctx());
    let (words, hue) = if language == Language::Txt {
        (
            i18n::translate(locale, keys::notes::kind_tag::PLAIN, &[]),
            p.text_1,
        )
    } else {
        (
            languages::label(language).to_string(),
            language_hue(ui, language),
        )
    };
    let galley = widgets::text(ui, &words, 10.0, 600.0, FontFamily::Monospace, hue);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    let name = languages::name(language);
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    address(ui, &response, "note-format");
    ui.painter().galley(rect.min, galley, hue);
    Some(widgets::tooltip(response, name))
}

/// How a row names a note's kind: the icon, and a snippet's format after it.
pub fn kind_mark(
    ui: &mut Ui,
    kind: NoteKind,
    language: Language,
    locale: i18n::Language,
) -> Response {
    let icon = kind_icon(ui, kind, locale);
    match format_label(ui, kind, language, locale) {
        Some(format) => icon.union(format),
        None => icon,
    }
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

/// The priority as a card's corner has room for it: three bars, its name their tooltip and what
/// a screen reader says; « Urgent » written out beside them, in red. Nothing for no priority.
pub fn priority_bars(ui: &mut Ui, priority: Priority, locale: i18n::Language) -> Option<Response> {
    let p = palette(ui.ctx());
    let (ink, lit, key) = match priority {
        Priority::None => return None,
        Priority::Low => (p.text_2, 1, keys::notes::priority::LOW),
        Priority::Medium => (p.blue, 2, keys::notes::priority::MEDIUM),
        Priority::High => (p.amber_text, 3, keys::notes::priority::HIGH),
        Priority::Urgent => (p.red, 3, keys::notes::priority::URGENT),
    };
    let name = i18n::translate(locale, key, &[]);
    let word = (priority == Priority::Urgent)
        .then(|| widgets::text(ui, &name, 10.0, 700.0, FontFamily::Proportional, ink));
    let bars = 3.0 * 2.0 + 2.0;
    let width = bars + word.as_ref().map_or(0.0, |word| 4.0 + word.size().x);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 12.0), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &name));
    address(ui, &response, "priority-bars");
    let bottom = rect.center().y + 5.0;
    for (bar, height) in [4.0, 7.0, 10.0].into_iter().enumerate() {
        let colour = if bar < lit {
            ink
        } else {
            ink.gamma_multiply(0.28)
        };
        #[expect(clippy::cast_precision_loss, reason = "three bars")]
        let left = rect.left() + bar as f32 * 3.0;
        let shape = Rect::from_min_max(
            Pos2::new(left, bottom - height),
            Pos2::new(left + 2.0, bottom),
        );
        ui.painter()
            .rect_filled(shape, CornerRadius::same(1), colour);
    }
    if let Some(word) = word {
        let at = Pos2::new(
            rect.left() + bars + 4.0,
            rect.center().y - word.size().y / 2.0,
        );
        ui.painter().galley(at, word, ink);
    }
    Some(widgets::tooltip(response, &name))
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
