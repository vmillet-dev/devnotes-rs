//! A note's card: a fixed 150 px, its title and kind on top, what the note holds by its kind,
//! and the footer at the bottom (`note-card` in the reference).

mod body;
mod footer;

use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{
    Color32, CornerRadius, FontFamily, FontId, Galley, Pos2, Rect, Role, Sense, Stroke, StrokeKind,
    Ui, UiBuilder, WidgetInfo, pos2, vec2,
};
use engine::notes::model::DisplayNote;
use i18n::keys;

use crate::action::Actions;
use crate::notes::ui::kind_badge;
use crate::services::time::Clock;
use crate::state::drawing_pictures::Thumbnails;
use crate::theme::{palette, spacing};
use crate::widgets::address;

/// What a card reads besides its note.
pub(super) struct Context<'a> {
    pub(super) language: i18n::Language,
    pub(super) clock: &'a dyn Clock,
    pub(super) thumbnails: &'a Thumbnails,
}

const FOOTER_HEIGHT: f32 = 18.0;

pub(super) fn show(
    ui: &mut Ui,
    rect: Rect,
    note: &DisplayNote,
    context: &Context<'_>,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let padding = spacing(ui.ctx()).card;
    let title = if note.title.is_empty() {
        i18n::translate(context.language, keys::notes::UNTITLED, &[])
    } else {
        note.title.clone()
    };
    let response = ui.allocate_rect(rect, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Article, true, &title));
    address(ui, &response, "note-card");
    // ⚠️ The click surface is a layer under the card's controls, not the card: what is drawn
    // after it, a task's box, takes the click first.
    let open = ui.allocate_rect(rect, Sense::click());
    open.widget_info(|| WidgetInfo::labeled(Role::Button, true, &title));
    address(ui, &open, "note-card-open");
    crate::theme::focus::ring(ui, &open, CornerRadius::same(10));
    if open.clicked() {
        actions.push(crate::action::Action::Notes(
            crate::notes::Action::OpenNote(note.id.clone()),
        ));
    }
    let (fill, edge) = if ui.rect_contains_pointer(rect) {
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

    let inner = rect.shrink(padding);
    let checklist = note.kind == engine::notes::kind::NoteKind::Checklist;
    let title_bottom = title_row(ui, inner, &title, note, context.language);
    let gap = if checklist { 6.0 } else { 8.0 };
    let footer_gap = if checklist { 4.0 } else { 8.0 };
    let footer = Rect::from_min_max(
        pos2(inner.left(), inner.bottom() - FOOTER_HEIGHT),
        inner.max,
    );
    let room = Rect::from_min_max(
        pos2(inner.left(), title_bottom + gap),
        pos2(inner.right(), footer.top() - footer_gap),
    );
    body::show(ui, room, note, context, actions);
    footer::show(ui, footer, note, context);
}

/// The title on two lines at most, the kind's badge on its first line. Answers where it ends.
fn title_row(
    ui: &mut Ui,
    inner: Rect,
    title: &str,
    note: &DisplayNote,
    language: i18n::Language,
) -> f32 {
    let mut badge_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::right_to_left(egui::Align::Min)),
    );
    badge_ui.add_space(0.0);
    let badge = kind_badge(&mut badge_ui, note.kind, note.language, language);
    let width = (badge.rect.left() - 6.0 - inner.left()).max(0.0);
    let p = palette(ui.ctx());
    let galley = galley(
        ui,
        title,
        &Font::proportional(13.0, 600.0, 1.3),
        p.text_0,
        width,
        2,
    );
    let at = inner.min + vec2(0.0, 1.0);
    let rect = Rect::from_min_size(at, galley.size());
    label(ui, rect, title, "note-card-title");
    ui.painter().galley(at, galley, Color32::TRANSPARENT);
    rect.bottom().max(badge.rect.bottom())
}

/// A text's face: its size, its CSS weight, its family, and its line height as a factor.
pub(super) struct Font {
    size: f32,
    weight: f32,
    family: FontFamily,
    line: f32,
}

impl Font {
    pub(super) fn proportional(size: f32, weight: f32, line: f32) -> Self {
        Self {
            size,
            weight,
            family: FontFamily::Proportional,
            line,
        }
    }

    pub(super) fn mono(size: f32, line: f32) -> Self {
        Self {
            size,
            weight: 400.0,
            family: FontFamily::Monospace,
            line,
        }
    }

    pub(super) fn height(&self) -> f32 {
        self.size * self.line
    }

    pub(super) fn format(&self, colour: Color32) -> TextFormat {
        TextFormat {
            font_id: FontId::new(self.size, self.family.clone()),
            color: colour,
            line_height: Some(self.height()),
            coords: egui::epaint::text::VariationCoords::new([(*b"wght", self.weight)]),
            ..TextFormat::default()
        }
    }
}

/// `words` in `width`, `rows` lines at most, an ellipsis where it is cut.
pub(super) fn galley(
    ui: &Ui,
    words: &str,
    font: &Font,
    colour: Color32,
    width: f32,
    rows: usize,
) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(words.to_string(), font.format(colour));
    job.wrap = TextWrapping {
        max_width: width.max(1.0),
        max_rows: rows,
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

/// A text that only reads: its node, found by `author_id`.
pub(super) fn label(ui: &mut Ui, rect: Rect, words: &str, author_id: &str) {
    let response = ui.allocate_rect(rect, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, words));
    address(ui, &response, author_id);
}

/// `galley` drawn at `at`, and its node.
pub(super) fn text_at(
    ui: &mut Ui,
    at: Pos2,
    galley: Arc<Galley>,
    words: &str,
    author_id: &str,
) -> Rect {
    let rect = Rect::from_min_size(at, galley.size());
    label(ui, rect, words, author_id);
    ui.painter().galley(at, galley, Color32::TRANSPARENT);
    rect
}

#[cfg(test)]
mod tests;
