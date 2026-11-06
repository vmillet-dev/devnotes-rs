//! The history under the body (the reference's `revision-panel`): folded to its count, it lists
//! the bodies kept, and a row opens the preview of what going back would change — instead of the
//! list, never beside it: it is the step between a row and the text it would erase.

use egui::{FontFamily, Margin, Sense, Stroke, Ui, Vec2};
use engine::notes::revision::DiffLine;
use i18n::{Language, keys};

use super::editor;
use crate::action::Actions;
use crate::notes::{Editing, Revisions};
use crate::state::AppState;
use crate::state::note_revisions::Preview;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, Button, Kind, address};

/// The tallest the panel stands unfolded; the body gives up the room.
pub(super) const OPEN_HEIGHT: f32 = 240.0;
/// A row of the list, and the note under it.
const ROW_HEIGHT: f32 = 28.0;
const NOTE_HEIGHT: f32 = 44.0;
/// Folded: the toggle alone.
pub(super) const FOLDED_HEIGHT: f32 = 34.0;

/// The room the panel takes at the editor's foot: none for a note with no body kept, what its
/// rows ask once open, within `room` so that the body keeps its own.
pub(super) fn height(state: &AppState, room: f32) -> f32 {
    let kept = state.revisions.kept().len();
    if kept == 0 {
        0.0
    } else if !state.revisions.is_open() {
        FOLDED_HEIGHT
    } else if state.revisions.preview().is_some() {
        OPEN_HEIGHT.min(room).max(FOLDED_HEIGHT)
    } else {
        #[expect(clippy::cast_precision_loss, reason = "twenty rows at most")]
        let rows = FOLDED_HEIGHT + kept as f32 * ROW_HEIGHT + NOTE_HEIGHT;
        rows.min(OPEN_HEIGHT).min(room).max(FOLDED_HEIGHT)
    }
}

pub(super) fn show(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let kept = state.revisions.kept();
    if kept.is_empty() {
        return;
    }
    let p = palette(ui.ctx());
    let rect = ui.max_rect();
    ui.painter()
        .hline(rect.x_range(), rect.top(), Stroke::new(1.0, p.line_soft));
    egui::Frame::new()
        .inner_margin(Margin::symmetric(20, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            toggle(ui, state, kept.len(), actions);
            if !state.revisions.is_open() {
                return;
            }
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| match state.revisions.preview() {
                    Some(preview) => preview_of(ui, state, preview, actions),
                    None => list(ui, state, actions),
                });
        });
}

fn toggle(ui: &mut Ui, state: &AppState, count: usize, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let count = u32::try_from(count).unwrap_or(u32::MAX);
    let words = i18n::translate(
        state.language,
        keys::revisions::TITLE,
        &[("count", count.into())],
    );
    let open = state.revisions.is_open();
    let galley = widgets::text(ui, &words, 12.0, 400.0, FontFamily::Proportional, p.text_1);
    let response = widgets::interact(
        ui,
        Vec2::new(galley.size().x + 22.0, 22.0),
        Sense::click(),
        "revisions-toggle",
        || egui::WidgetInfo::labeled(egui::Role::Button, true, &words),
        egui::CornerRadius::same(5),
    );
    let chevron = egui::Rect::from_center_size(
        egui::pos2(response.rect.left() + 7.0, response.rect.center().y),
        Vec2::splat(11.0),
    );
    let icon = if open {
        Icon::ArrowDown
    } else {
        Icon::ChevronRight
    };
    icons::paint(ui, chevron, icon, p.text_2);
    ui.painter().galley(
        egui::pos2(
            response.rect.left() + 18.0,
            response.rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        p.text_1,
    );
    if response.clicked() {
        actions.push(editor(Editing::Revisions(Revisions::Toggle)));
    }
}

/// A row a body kept: when, how long, and « View ».
fn list(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    for kept in state.revisions.kept() {
        let (when, _) = crate::services::time::relative(kept.taken_at, state.clock.as_ref());
        let size = i18n::translate(
            language,
            keys::revisions::CHARACTERS,
            &[("count", kept.characters.into())],
        );
        let look = i18n::translate(language, keys::revisions::LOOK, &[]);
        let words = format!("{}  ·  {size}", when.text(language));
        ui.horizontal(|ui| {
            let row = widgets::text(ui, &words, 12.0, 400.0, FontFamily::Proportional, p.text_1);
            ui.label(row);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let response = Button::new(&look, "revision-open")
                    .kind(Kind::Quiet)
                    .show(ui);
                if response.clicked() {
                    actions.push(editor(Editing::Revisions(Revisions::Look(kept.id.clone()))));
                }
            });
        });
    }
    let note = i18n::translate(language, keys::revisions::NOTE, &[]);
    let font = egui::FontId::new(11.0, FontFamily::Proportional);
    let width = ui.available_width();
    let note = ui.fonts_mut(|fonts| fonts.layout(note, font, p.text_2, width));
    ui.label(note);
}

/// What going back would change, what it would erase, and the two ways out.
fn preview_of(ui: &mut Ui, state: &AppState, preview: &Preview, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let (when, _) = crate::services::time::relative(preview.kept.taken_at, state.clock.as_ref());
    let head = format!(
        "{} {}",
        tr(keys::revisions::PREVIEW_OF),
        when.text(language)
    );
    ui.label(widgets::text(
        ui,
        &head,
        12.0,
        600.0,
        FontFamily::Proportional,
        p.text_0,
    ));
    if preview.lines.is_empty() {
        let same = ui.label(widgets::text(
            ui,
            &tr(keys::revisions::IDENTICAL),
            12.0,
            400.0,
            FontFamily::Proportional,
            p.text_1,
        ));
        address(ui, &same, "revision-identical");
    } else {
        diff(ui, &preview.lines, language);
    }
    let newer = u32::try_from(preview.newer).unwrap_or(u32::MAX);
    let warning = i18n::translate(
        language,
        keys::revisions::CONSEQUENCE,
        &[("count", newer.into())],
    );
    let note = ui.label(widgets::text(
        ui,
        &warning,
        11.5,
        400.0,
        FontFamily::Proportional,
        p.amber_text,
    ));
    address(ui, &note, "revision-consequence");
    ui.horizontal(|ui| {
        if Button::new(&tr(keys::revisions::BACK), "revision-back")
            .kind(Kind::Quiet)
            .show(ui)
            .clicked()
        {
            actions.push(editor(Editing::Revisions(Revisions::Back)));
        }
        let restore = Button::new(&tr(keys::revisions::RESTORE), "revision-restore")
            .kind(Kind::Filled)
            .enabled(!state.revisions.is_restoring())
            .show(ui);
        if restore.clicked() {
            actions.push(editor(Editing::Revisions(Revisions::Restore)));
        }
    });
}

/// The lines, `+` coming back in green, `−` going in red, the unchanged far from them skipped.
fn diff(ui: &mut Ui, lines: &[DiffLine], language: Language) {
    let p = palette(ui.ctx());
    let response = egui::Frame::new()
        .fill(p.bg_1)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for line in lines {
                let (sign, text, ink, author_id) = match line {
                    DiffLine::Kept { text } => (" ", text.clone(), p.text_1, "diff-kept"),
                    DiffLine::Restored { text } => ("+", text.clone(), p.green, "diff-restored"),
                    DiffLine::Dropped { text } => ("−", text.clone(), p.red, "diff-dropped"),
                    DiffLine::Skipped { count } => (
                        " ",
                        i18n::translate(
                            language,
                            keys::revisions::SKIPPED,
                            &[("count", (*count).into())],
                        ),
                        p.text_2,
                        "diff-skipped",
                    ),
                };
                let words = format!("{sign} {text}");
                let shown = ui.label(widgets::text(
                    ui,
                    &words,
                    12.0,
                    400.0,
                    FontFamily::Monospace,
                    ink,
                ));
                address(ui, &shown, author_id);
            }
        })
        .response;
    address(ui, &response, "revision-diff");
}

#[cfg(test)]
mod tests;
