//! The list view: the sections of the view the core answered, drawn a row at a time, only where
//! the viewport is (plan § 3.6), or the state that owns the region instead — loading, an error,
//! nothing found.

mod board;
mod card;
pub mod grid;
pub mod rows;
mod selection_bar;
pub mod shortcuts;

use egui::emath::GuiRounding as _;
use egui::text::{LayoutJob, TextFormat};
use egui::{
    Color32, CornerRadius, FontFamily, FontId, Rect, Role, Sense, Shape, Stroke, Ui, UiBuilder,
    Vec2, WidgetInfo, pos2, vec2,
};
use engine::notes::view::{NoteSection, NoteSectionKey, NotesView, SectionGroup};
use i18n::{Key, keys};

use super::language as languages;
use crate::action::{Action, Actions, GlobalAction};
use crate::state::AppState;
use crate::theme::{palette, spacing};
use crate::widgets::{self, Button, address, interact, priority_dot};
use rows::{CARD_HEIGHT, HEADING_HEIGHT, Row, Rows};

/// What only the canvas's drawing sees: the card with the focus, the one asked to take it, and
/// how many columns the cards were laid out in — what the arrows move through.
#[derive(Default)]
pub struct CanvasView {
    pub focused: Option<String>,
    pub request: Option<String>,
    pub columns: usize,
    /// The card that had the focus the frame before: a `Shift`-click focuses what it clicks.
    pub anchor: Option<String>,
    /// The selection bar's: a tag being typed, and a deletion waiting for its confirmation.
    pub tag_draft: String,
    pub confirm_delete: bool,
    board: board::BoardScreen,
}

pub fn show(ui: &mut Ui, state: &AppState, canvas_view: &mut CanvasView, actions: &mut Actions) {
    let query = &state.notes_query;
    let language = state.language;
    let canvas = spacing(ui.ctx()).canvas;
    let (region, response) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Region, true, "canvas"));
    address(ui, &response, "canvas");
    let mut ui = ui.new_child(UiBuilder::new().max_rect(region));
    let ui = &mut ui;
    let tr = |key: Key| i18n::translate(language, key, &[]);
    if state.board_place().is_some() {
        board::show(ui, state, canvas_view, actions);
        return;
    }
    if query.error().is_some() {
        // Loading is the one failure that empties the whole screen: it has its own retry.
        padded(ui, canvas, |ui| {
            state_text(ui, &tr(keys::errors::NOTES_LOAD_FAILED), "canvas-error");
            if Button::new(&tr(keys::errors::RETRY), "canvas-retry")
                .show(ui)
                .clicked()
            {
                actions.push(Action::Notes(super::Action::Reload));
            }
        });
        return;
    }
    let Some(view) = query.view() else {
        padded(ui, canvas, |ui| {
            state_text(ui, &tr(keys::notes::LOADING), "canvas-loading");
        });
        return;
    };
    if view.matched == 0 {
        padded(ui, canvas, |ui| {
            state_text(ui, &tr(keys::notes::NO_RESULTS), "canvas-no-results");
        });
        return;
    }
    let context = card::Context::of(state, &on_screen(view));
    sections(ui, view, &context, canvas_view, actions);
    hint(ui, language);
    let shown: Vec<_> = view
        .cards()
        .map(|card| selection_bar::Shown {
            id: &card.id,
            pinned: card.pinned,
            tags: &card.tags,
        })
        .collect();
    selection_bar::show(ui, state, &shown, canvas_view, actions);
}

/// The keys that matter, on the bottom edge whatever is above it.
fn hint(ui: &mut Ui, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, keys::notes::KEYBOARD_HINT, &[]);
    let width = widgets::arrowed(ui, None, &words, 11.0, p.text_2);
    let region = ui.max_rect();
    let band = Rect::from_min_max(pos2(region.left(), region.bottom() - 24.0), region.max);
    let mut band_ui = ui.new_child(UiBuilder::new().max_rect(band));
    band_ui
        .painter()
        .rect_filled(band, CornerRadius::ZERO, p.bg_0.gamma_multiply(0.92));
    let rect = Rect::from_center_size(band.center(), vec2(width, 14.0));
    let response = band_ui.allocate_rect(rect, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    address(&band_ui, &response, "canvas-keyboard-hint");
    widgets::arrowed(&band_ui, Some(rect.left_center()), &words, 11.0, p.text_2);
}

fn on_screen(view: &NotesView) -> Vec<&str> {
    view.cards().map(|card| card.id.as_str()).collect()
}

pub(super) fn padded(ui: &mut Ui, padding: f32, draw: impl FnOnce(&mut Ui)) {
    egui::Frame::new().inner_margin(padding).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        draw(ui);
    });
}

pub(super) fn state_text(ui: &mut Ui, words: &str, author_id: &str) {
    let p = palette(ui.ctx());
    let galley = widgets::text(ui, words, 13.0, 400.0, FontFamily::Proportional, p.text_2);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, words));
    address(ui, &response, author_id);
    ui.painter().galley(rect.min, galley, Color32::TRANSPARENT);
}

fn sections(
    ui: &mut Ui,
    view: &NotesView,
    context: &card::Context<'_>,
    canvas_view: &mut CanvasView,
    actions: &mut Actions,
) {
    let s = spacing(ui.ctx());
    egui::ScrollArea::vertical()
        .id_salt("canvas")
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            let width = ui.available_width() - 2.0 * s.canvas;
            let rows = Rows::of(view, width, s.grid, s.section);
            let origin = ui.min_rect().min + Vec2::splat(s.canvas);
            ui.set_height(rows.height() + 2.0 * s.canvas);
            canvas_view.columns = rows.columns();
            // A card asked to take the focus may be scrolled out of view, so not drawn.
            if let Some(top) = canvas_view
                .request
                .as_deref()
                .and_then(|id| rows.top_of(view, id))
            {
                let rect = Rect::from_min_size(origin + vec2(0.0, top), vec2(width, CARD_HEIGHT));
                ui.scroll_to_rect(rect, None);
            }
            canvas_view.anchor = canvas_view.focused.take();
            let top = viewport.top() - s.canvas;
            for (at, row) in rows.visible(top, viewport.bottom() - s.canvas) {
                let min = origin + vec2(0.0, at);
                match row {
                    Row::Heading { section } => {
                        let rect = Rect::from_min_size(min, vec2(width, HEADING_HEIGHT));
                        heading(ui, rect, &view.sections[section], context.language);
                    }
                    Row::Cards { section, first } => {
                        let at = (&rows, view, section, first);
                        cards(ui, min, at, context, canvas_view, actions);
                    }
                }
            }
        });
}

fn cards(
    ui: &mut Ui,
    min: egui::Pos2,
    (rows, view, section, first): (&Rows, &NotesView, usize, usize),
    context: &card::Context<'_>,
    canvas_view: &mut CanvasView,
    actions: &mut Actions,
) {
    let grid = spacing(ui.ctx()).grid;
    let shown = &view.sections[section];
    let cells = shown.cards.len() + usize::from(shown.show_create_ghost);
    for (column, cell) in (first..cells.min(first + rows.columns())).enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "a handful of columns")]
        let left = column as f32 * (rows.card_width() + grid);
        // On whole points: a badge laid out right to left from a fraction is drawn blurred.
        let rect = Rect::from_min_size(min + vec2(left, 0.0), vec2(rows.card_width(), CARD_HEIGHT))
            .round_ui();
        let mut cell_ui = ui.new_child(UiBuilder::new().max_rect(rect).id_salt((section, cell)));
        match shown.cards.get(cell) {
            Some(shown_card) => {
                card::show(
                    &mut cell_ui,
                    rect,
                    &view.display(shown_card),
                    context,
                    (canvas_view, actions),
                );
            }
            None => ghost(&mut cell_ui, rect, context.language, actions),
        }
    }
}

/// « Today · 3 », in small capitals, a priority section's dot before it.
fn heading(ui: &mut Ui, rect: Rect, section: &NoteSection, language: i18n::Language) {
    let p = palette(ui.ctx());
    let mut left = rect.left();
    if let Some(SectionGroup::Priority { priority }) = section.group {
        let mut dot_ui = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(
            pos2(left, rect.center().y - 4.0),
            Vec2::splat(8.0),
        )));
        priority_dot(&mut dot_ui, priority);
        left += 8.0 + 8.0;
    }
    let title = title(section, language).to_uppercase();
    let mut count = format!("· {}", section.cards.len());
    if section.has_expiring_notes {
        count.push_str(" · ");
        count.push_str(&i18n::translate(
            language,
            keys::sections::EXPIRING_HINT,
            &[],
        ));
    }
    let mut job = LayoutJob::default();
    job.append(
        &title,
        0.0,
        TextFormat {
            font_id: FontId::new(11.0, FontFamily::Proportional),
            color: p.text_2,
            extra_letter_spacing: 1.0,
            coords: egui::epaint::text::VariationCoords::new([(*b"wght", 700.0)]),
            ..TextFormat::default()
        },
    );
    job.append(
        &count,
        8.0,
        TextFormat {
            font_id: FontId::new(11.0, FontFamily::Monospace),
            color: p.text_2,
            ..TextFormat::default()
        },
    );
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let at = pos2(left, rect.center().y - galley.size().y / 2.0);
    let response = ui.allocate_rect(Rect::from_min_size(at, galley.size()), Sense::hover());
    let label = format!("{title} {count}");
    response.widget_info(|| WidgetInfo::labeled(Role::Heading, true, &label));
    address(ui, &response, "note-section");
    ui.painter().galley(at, galley, Color32::TRANSPARENT);
}

/// A dated section is named by its key, a gathered one by what it gathers.
fn title(section: &NoteSection, language: i18n::Language) -> String {
    let tr = |key: Key| i18n::translate(language, key, &[]);
    match section.group {
        Some(SectionGroup::Language { language: format }) => languages::label(format).to_string(),
        Some(SectionGroup::Kind { kind }) => tr(super::kind_plural(kind)),
        Some(SectionGroup::Priority { priority }) => tr(super::priority_section(priority)),
        None => tr(match section.key {
            NoteSectionKey::Pinned => keys::sections::PINNED,
            NoteSectionKey::Today => keys::sections::TODAY,
            NoteSectionKey::Week => keys::sections::WEEK,
            NoteSectionKey::Older => keys::sections::OLDER,
            NoteSectionKey::Results | NoteSectionKey::Priority | NoteSectionKey::Format => {
                keys::sections::RESULTS
            }
            NoteSectionKey::All => keys::sections::ALL,
        }),
    }
}

/// « Paste or create », dashed, where the next card would go.
fn ghost(ui: &mut Ui, rect: Rect, language: i18n::Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, keys::notes::CREATE_OR_PASTE, &[]);
    let corner = CornerRadius::same(10);
    let response = interact(
        ui,
        rect.size(),
        Sense::click(),
        "create-ghost",
        || WidgetInfo::labeled(Role::Button, true, &words),
        corner,
    );
    let lit = response.hovered();
    let (edge, ink) = if lit {
        (p.amber_dim, p.amber_text)
    } else {
        (p.line, p.text_2)
    };
    ui.painter().extend(dashed(rect, Stroke::new(1.0, edge)));
    let plus = widgets::text(ui, "+", 20.0, 400.0, FontFamily::Proportional, ink);
    let label = widgets::text(ui, &words, 12.0, 400.0, FontFamily::Proportional, ink);
    let height = plus.size().y + 6.0 + label.size().y;
    let top = rect.center().y - height / 2.0;
    ui.painter().galley(
        pos2(rect.center().x - plus.size().x / 2.0, top),
        plus.clone(),
        Color32::TRANSPARENT,
    );
    ui.painter().galley(
        pos2(
            rect.center().x - label.size().x / 2.0,
            top + plus.size().y + 6.0,
        ),
        label,
        Color32::TRANSPARENT,
    );
    if response.clicked() {
        actions.push(Action::Global(GlobalAction::NewNote));
    }
}

/// The four sides of `rect`, dashed: egui strokes no dashed rectangle of its own.
fn dashed(rect: Rect, stroke: Stroke) -> Vec<Shape> {
    let rect = rect.shrink(0.5);
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    Shape::dashed_line(&corners, stroke, 4.0, 3.0)
}

#[cfg(test)]
pub(super) mod tests;
