//! The fields panel over the body (the reference's `placeholder-panel`): each `{{field}}` with its
//! value, how many are filled, and the body shown filled. It folds on its own, and only a note
//! with fields has one. Values are a draft of the opening, written when a field is left.

use std::collections::BTreeMap;

use egui::{FontFamily, Margin, Rect, Sense, Stroke, Ui, Vec2};
use engine::notes::model::DisplayNote;
use i18n::{Language, keys};

use super::{EditorView, editor};
use crate::action::Actions;
use crate::notes::overlays::placeholder_form::field_row;
use crate::notes::{Editing, Fields};
use crate::state::AppState;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, Button};

/// Folded, what the head shows of the values.
const SUMMARY_LIMIT: usize = 2;
const HEAD_HEIGHT: f32 = 26.0;
/// Bounded: ten fields must not push the body off screen.
const BODY_SHARE: f32 = 0.3;

type Values = BTreeMap<String, String>;

/// What is typed, for the opening it began in.
#[derive(Debug, Default)]
pub(super) struct FieldsView {
    typed: Values,
    /// Against this and not the note: until the write answers, the note holds the old values.
    kept: Values,
    pub(super) previewing: bool,
    /// Expanded, the filled text under the code was asked for once in this opening.
    pub(super) asked: bool,
}

impl FieldsView {
    pub(super) fn of(note: &DisplayNote) -> Self {
        let stored: Values = note
            .placeholders
            .iter()
            .map(|field| (field.name.clone(), field.value.clone()))
            .collect();
        Self {
            typed: stored.clone(),
            kept: stored,
            previewing: false,
            asked: false,
        }
    }

    /// Every field by name, raw: `notes::placeholder::fill` decides what an empty one is worth.
    pub(super) fn values(&self, note: &DisplayNote) -> Values {
        let value = |name: &str| self.typed.get(name).cloned().unwrap_or_default();
        note.placeholders
            .iter()
            .map(|field| (field.name.clone(), value(&field.name)))
            .collect()
    }

    /// Written once it differs, on leaving a field and on every path that leaves none.
    pub(super) fn keep(&mut self, note: &DisplayNote, actions: &mut Actions) {
        let values = self.values(note);
        if filled(&values) == filled(&self.kept) {
            return;
        }
        self.kept.clone_from(&values);
        actions.push(editor(Editing::Fields(Fields::Keep(values))));
    }
}

fn filled(values: &Values) -> Vec<(&String, &String)> {
    let mut filled: Vec<_> = values.iter().collect();
    filled.retain(|(_, value)| !value.is_empty());
    filled
}

pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    actions: &mut Actions,
) {
    if note.placeholders.is_empty() {
        return;
    }
    // A field the text just gained: the values stored under its name, if it had some.
    for field in &note.placeholders {
        let typed = &mut editing.fields.typed;
        typed
            .entry(field.name.clone())
            .or_insert_with(|| field.value.clone());
    }
    let open = state.settings.editor_fields_open;
    let values = editing.fields.values(note);
    let changed = super::band(ui, Margin::symmetric(20, 7), !open, |ui| {
        let mut changed = false;
        // Sized: a layout of its own would take all the height left.
        let strip = Vec2::new(ui.available_width(), HEAD_HEIGHT);
        let layout = egui::Layout::right_to_left(egui::Align::Center);
        ui.allocate_ui_with_layout(strip, layout, |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if open {
                changed = head_actions(ui, state, (session, note), editing, actions);
            }
            if toggle(ui, state.language, open, &values, note) {
                if open {
                    editing.fields.previewing = false;
                    editing.fields.keep(note, actions);
                }
                actions.push(editor(Editing::Fields(Fields::Fold(!open))));
            }
        });
        changed
    });
    let changed = changed | (open && body(ui, state, note, editing, actions));
    // Expanded, the filled text stands under the code at all times.
    if changed && (editing.fields.previewing || state.settings.editor_fullscreen) {
        preview(session, note, editing, actions);
    }
}

pub(super) fn preview(
    session: u64,
    note: &DisplayNote,
    editing: &EditorView,
    actions: &mut Actions,
) {
    actions.push(editor(Editing::Fields(Fields::Preview {
        session,
        content: editing.body.clone(),
        values: editing.fields.values(note),
    })));
}

/// The strip is the button: a row that lights up reads as a control, a bare label as a heading.
fn toggle(
    ui: &mut Ui,
    language: Language,
    open: bool,
    values: &Values,
    note: &DisplayNote,
) -> bool {
    let p = palette(ui.ctx());
    let size = Vec2::new(ui.available_width(), HEAD_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 7.0, p.bg_2);
    }
    let ink = if response.hovered() {
        p.text_0
    } else {
        p.text_1
    };
    let caret = if open {
        Icon::ChevronDown
    } else {
        Icon::ChevronRight
    };
    let at = rect.left() + 8.0;
    let middle = rect.center().y;
    let square = Rect::from_center_size(egui::pos2(at + 6.0, middle), Vec2::splat(12.0));
    icons::paint(ui, square, caret, ink);
    let title = i18n::translate(language, keys::placeholders::PANEL_TITLE, &[]);
    let words = widgets::text(ui, &title, 12.0, 600.0, FontFamily::Proportional, ink);
    let title_at = egui::pos2(at + 20.0, middle - words.size().y / 2.0);
    let after_title = title_at.x + words.size().x + 8.0;
    ui.painter().galley(title_at, words, ink);
    let total = note.placeholders.len();
    let done = filled(values).len();
    let after_count = count(ui, (done, total), egui::pos2(after_title, middle));
    let verb = (!open).then(|| {
        let verb = i18n::translate(language, keys::placeholders::SHOW, &[]);
        let colour = if response.hovered() {
            p.amber_text
        } else {
            p.text_2
        };
        let words = widgets::text(ui, &verb, 11.0, 400.0, FontFamily::Proportional, colour);
        let at = egui::pos2(
            rect.right() - 8.0 - words.size().x,
            middle - words.size().y / 2.0,
        );
        let left = at.x;
        ui.painter().galley(at, words, colour);
        left
    });
    let line = summary(values, note, language);
    if let Some(right) = verb {
        let room = Rect::from_x_y_ranges(after_count + 8.0..=right - 8.0, rect.y_range());
        let words = widgets::text(ui, &line, 11.0, 400.0, FontFamily::Monospace, p.text_2);
        let at = egui::pos2(room.left(), middle - words.size().y / 2.0);
        ui.painter()
            .with_clip_rect(room)
            .galley(at, words, p.text_2);
    }
    let said = i18n::translate(
        language,
        keys::placeholders::FILLED_COUNT,
        &[
            ("filled", counted(done).into()),
            ("total", counted(total).into()),
        ],
    );
    ui.ctx().accesskit_node_builder(response.id, |node| {
        let held = if open { "" } else { line.as_str() };
        node.set_label(format!("{title}, {said} {held}").trim_end());
        node.set_value(format!("{done}/{total}"));
        node.set_expanded(open);
    });
    widgets::address(ui, &response, "placeholder-panel-toggle");
    response.clicked()
}

fn counted(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// The pill, amber once every field holds a value: that is when the copy is ready. Its right edge.
fn count(ui: &Ui, (done, total): (usize, usize), left_middle: egui::Pos2) -> f32 {
    let p = palette(ui.ctx());
    let complete = done == total;
    let ink = if complete { p.amber_text } else { p.text_2 };
    let line = if complete { p.amber_dim } else { p.line };
    let words = format!("{done}/{total}");
    let words = widgets::text(ui, &words, 10.5, 400.0, FontFamily::Monospace, ink);
    let size = words.size() + Vec2::new(16.0, 2.0);
    let pill = Rect::from_min_size(left_middle - Vec2::new(0.0, size.y / 2.0), size);
    let fill = if complete {
        p.amber_fill.gamma_multiply(0.1)
    } else {
        p.bg_2
    };
    ui.painter().rect_filled(pill, size.y / 2.0, fill);
    ui.painter().rect_stroke(
        pill,
        size.y / 2.0,
        Stroke::new(1.0, line),
        egui::StrokeKind::Inside,
    );
    let at = pill.center() - words.size() / 2.0;
    ui.painter().galley(at, words, ink);
    pill.right()
}

/// `host = db · port = 5432 · +2`: the first values filled.
fn summary(values: &Values, note: &DisplayNote, language: Language) -> String {
    let pairs: Vec<String> = note
        .placeholders
        .iter()
        .filter_map(|field| {
            let value = values.get(&field.name).filter(|value| !value.is_empty())?;
            Some(format!("{} = {value}", field.name))
        })
        .collect();
    let hidden = pairs.len().saturating_sub(SUMMARY_LIMIT);
    let mut shown: Vec<String> = pairs.into_iter().take(SUMMARY_LIMIT).collect();
    if hidden > 0 {
        shown.push(i18n::translate(
            language,
            keys::placeholders::SUMMARY_MORE,
            &[("count", counted(hidden).into())],
        ));
    }
    shown.join(" · ")
}

/// Right to left, as the head lays them out. `true` once the values changed.
fn head_actions(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    actions: &mut Actions,
) -> bool {
    let tr = |key| i18n::translate(state.language, key, &[]);
    let mut changed = false;
    let any = !filled(&editing.fields.values(note)).is_empty();
    if any
        && Button::new(&tr(keys::placeholders::RESET), "placeholder-panel-reset")
            .show(ui)
            .clicked()
    {
        editing.fields.typed.values_mut().for_each(String::clear);
        editing.fields.keep(note, actions);
        changed = true;
    }
    if Button::new(
        &tr(keys::placeholders::COPY_RAW),
        "placeholder-panel-copy-raw",
    )
    .icon(Icon::Copy)
    .show(ui)
    .clicked()
    {
        actions.push(editor(Editing::Fields(Fields::CopyRaw(
            editing.body.clone(),
        ))));
    }
    let previewing = editing.fields.previewing;
    if !state.settings.editor_fullscreen
        && Button::new(
            &tr(keys::placeholders::PREVIEW),
            "placeholder-panel-preview",
        )
        .icon(Icon::Eye)
        .armed(previewing)
        .show(ui)
        .clicked()
    {
        editing.fields.previewing = !previewing;
        if !previewing {
            preview(session, note, editing, actions);
        }
    }
    changed
}

/// One row a field, then the hint. `true` once a value changed.
fn body(
    ui: &mut Ui,
    state: &AppState,
    note: &DisplayNote,
    editing: &mut EditorView,
    actions: &mut Actions,
) -> bool {
    let p = palette(ui.ctx());
    let language = state.language;
    let tallest = ui.ctx().content_rect().height() * BODY_SHARE;
    let mut changed = false;
    let mut left = false;
    let frame = egui::Frame::new()
        .fill(p.bg_0)
        .inner_margin(Margin {
            left: 20,
            right: 20,
            top: 10,
            bottom: 12,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 6.0;
            egui::ScrollArea::vertical()
                .max_height(tallest)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for field in &note.placeholders {
                        let value = editing.fields.typed.entry(field.name.clone()).or_default();
                        let input = field_row(ui, field, value, language);
                        changed |= input.changed();
                        left |= input.lost_focus();
                    }
                    let hint = i18n::translate(language, keys::placeholders::PANEL_HINT, &[]);
                    ui.add(
                        egui::Label::new(egui::RichText::new(hint).size(10.5).color(p.text_2))
                            .wrap(),
                    );
                });
        });
    let rect = frame.response.rect;
    let line = Stroke::new(1.0, p.line_soft);
    ui.painter().hline(rect.x_range(), rect.top(), line);
    ui.painter().hline(rect.x_range(), rect.bottom(), line);
    if left {
        editing.fields.keep(note, actions);
    }
    changed
}

#[cfg(test)]
mod tests;
