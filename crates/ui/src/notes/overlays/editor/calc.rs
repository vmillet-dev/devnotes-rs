//! A Calcul note (the reference's `calc-editor`): the sheet in `EditorView`, each line's result in
//! a column on the right as the core computes it, written in the language on screen. A result
//! clicked is picked, Shift for a range, Ctrl for one more; the footer sums what is picked, or the
//! lines a selection covers.

use std::collections::BTreeSet;

use egui::{Color32, FontFamily, Rect, Sense, Stroke, Ui, Vec2};
use engine::notes::calc::{CalcError, CalcLine, CalcSheet, CalcSummary, CalcValue, TokenKind};
use i18n::{Language, keys};

use crate::action::Actions;
use crate::api;
use crate::notes::Editing;
use crate::notes::calc;
use crate::state::AppState;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::theme::tokens::Palette;
use crate::widgets::editor::{Completion, Editor, EditorView, Row};
use crate::widgets::{self, IconButton};

/// What is computed once the hand pauses.
const EVALUATE_AFTER: f64 = 0.12;
/// How long « Copied » stays in the footer before the sum comes back.
const SAID_FOR: f64 = 2.0;
/// The column the results are laid in, on the right of each line.
const RESULTS_WIDTH: f32 = 260.0;
const FOOTER_HEIGHT: f32 = 28.0;
const FONT: f32 = 13.0;
const COPY_SIZE: f32 = 22.0;

pub(super) struct CalcView {
    pub(super) editor: Editor,
    /// When the text is next computed; at once on opening.
    evaluate_at: Option<f64>,
    picked: BTreeSet<u32>,
    anchor: u32,
    said: Option<(String, f64)>,
}

impl CalcView {
    pub(super) fn of(text: &str) -> Self {
        Self {
            editor: Editor::new(text),
            evaluate_at: Some(0.0),
            picked: BTreeSet::new(),
            anchor: 0,
            said: None,
        }
    }

    pub(super) fn text(&self) -> String {
        self.editor.text()
    }

    /// The text's selection when it spans lines, the results picked otherwise.
    fn chosen(&self) -> Vec<u32> {
        let selection = self.editor.buffer.selection().range();
        if selection.is_empty() {
            return self.picked.iter().copied().collect();
        }
        let first = self.editor.buffer.line_of(selection.start);
        let last = self.editor.buffer.line_of(selection.end);
        (first..=last)
            .filter_map(|line| u32::try_from(line).ok())
            .collect()
    }

    /// A click picks one, Shift a range from the last one picked, Ctrl one more or one less.
    fn pick(&mut self, sheet: &CalcSheet, line: u32, modifiers: egui::Modifiers) {
        if modifiers.shift {
            let (from, to) = (self.anchor.min(line), self.anchor.max(line));
            self.picked = (from..=to).filter(|at| gave_a_value(sheet, *at)).collect();
            return;
        }
        self.anchor = line;
        if modifiers.command {
            if !self.picked.remove(&line) {
                self.picked.insert(line);
            }
        } else if self.picked.len() == 1 && self.picked.contains(&line) {
            self.picked.clear();
        } else {
            self.picked = BTreeSet::from([line]);
        }
    }
}

fn row_of(sheet: &CalcSheet, line: u32) -> Option<&CalcLine> {
    let row = sheet.rows.get(usize::try_from(line).ok()?)?;
    Some(&row.line)
}

fn gave_a_value(sheet: &CalcSheet, line: u32) -> bool {
    matches!(row_of(sheet, line), Some(CalcLine::Value { .. }))
}

/// What a frame of the sheet did.
pub(super) struct Shown {
    pub(super) changed: bool,
    pub(super) left: bool,
}

pub(super) fn show(
    ui: &mut Ui,
    view: &mut CalcView,
    state: &AppState,
    session: u64,
    actions: &mut Actions,
) -> Shown {
    let language = state.language;
    let sheet = state.calc.sheet(session);
    let whole = ui.max_rect();
    let footer_top = whole.bottom() - FOOTER_HEIGHT;
    let body = Rect::from_min_max(whole.min, egui::pos2(whole.right(), footer_top));
    let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
    let shown = sheet_field(&mut body_ui, view, sheet, language);
    if let Some(sheet) = sheet {
        results(ui, view, sheet, &shown.rows, language);
    }
    let footer = Rect::from_min_max(egui::pos2(whole.left(), footer_top), whole.max);
    let mut footer_ui = ui.new_child(egui::UiBuilder::new().max_rect(footer));
    self::footer(&mut footer_ui, view, sheet, language);
    ui.advance_cursor_after_rect(whole);

    let now = ui.input(|input| input.time);
    if shown.changed {
        view.evaluate_at = Some(now + EVALUATE_AFTER);
    }
    if let Some(due) = view.evaluate_at {
        if now >= due {
            view.evaluate_at = None;
            actions.push(super::editor(Editing::Evaluate {
                session,
                text: view.text(),
            }));
        } else {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(due - now));
        }
    }
    Shown {
        changed: shown.changed,
        left: shown.left,
    }
}

struct Field {
    rows: Vec<Row>,
    changed: bool,
    left: bool,
}

fn sheet_field(
    ui: &mut Ui,
    view: &mut CalcView,
    sheet: Option<&CalcSheet>,
    language: Language,
) -> Field {
    let p = palette(ui.ctx());
    let colours = |line: usize, text: &str| colours_of(p, sheet, line, text);
    let label = i18n::translate(language, keys::calc::editor::LABEL, &[]);
    let hint = i18n::translate(language, keys::calc::editor::PLACEHOLDER, &[]);
    let text = view.text();
    let hover = |at: usize| hovered(&text, sheet, at, language);
    let caret = view.editor.buffer.selection().head;
    let (from, items) = completions(&text, caret, sheet, language);
    let completion = (!items.is_empty()).then_some(Completion {
        from,
        items: &items,
    });
    let output = EditorView::new(&mut view.editor, &label, "editor-body")
        .hint(&hint)
        .language(language)
        .margin(RESULTS_WIDTH + 16.0)
        .colours(&colours)
        .hover(&hover)
        .completion(completion)
        .show(ui);
    Field {
        rows: output.rows,
        changed: output.changed.is_some(),
        left: output.response.lost_focus(),
    }
}

/// The core's spans, in UTF-16 units of the line, as `char` ranges in their colours.
fn colours_of(
    p: &Palette,
    sheet: Option<&CalcSheet>,
    line: usize,
    text: &str,
) -> Vec<(std::ops::Range<usize>, Color32)> {
    let Some(row) = sheet.and_then(|sheet| sheet.rows.get(line)) else {
        return Vec::new();
    };
    row.spans
        .iter()
        .filter_map(|span| {
            let start = char_at(text, span.start)?;
            let end = char_at(text, span.end)?;
            Some((start..end, colour(p, span.kind)))
        })
        .collect()
}

/// The `char` a UTF-16 offset falls on; `None` past the line, which a sheet being typed can be.
fn char_at(text: &str, utf16: u32) -> Option<usize> {
    let mut units = 0;
    for (at, character) in text.chars().enumerate() {
        if units >= utf16 {
            return Some(at);
        }
        units += u32::try_from(character.len_utf16()).unwrap_or(1);
    }
    (units >= utf16).then(|| text.chars().count())
}

fn colour(p: &Palette, kind: TokenKind) -> Color32 {
    match kind {
        TokenKind::Number => p.amber_text,
        TokenKind::Unit => p.teal,
        TokenKind::Operator => p.text_1,
        TokenKind::Keyword => p.purple,
        TokenKind::Name => p.blue,
        TokenKind::Date => p.green,
        TokenKind::Comment => p.text_2,
        TokenKind::Heading => p.text_0,
        TokenKind::Error => p.red,
    }
}

/// After `en` or `à`, the units of the line's kind; elsewhere, the words of the table.
fn completions(
    text: &str,
    caret: usize,
    sheet: Option<&CalcSheet>,
    language: Language,
) -> (usize, Vec<String>) {
    let start_of_line = text
        .chars()
        .take(caret)
        .collect::<Vec<_>>()
        .iter()
        .rposition(|character| *character == '\n')
        .map_or(0, |at| at + 1);
    let before: String = text
        .chars()
        .skip(start_of_line)
        .take(caret - start_of_line)
        .collect();
    if before.trim_start().starts_with('#') || before.contains("//") {
        return (caret, Vec::new());
    }
    let line = text.chars().take(caret).filter(|it| *it == '\n').count();
    let dimension = sheet
        .and_then(|sheet| sheet.rows.get(line))
        .and_then(|row| row.converting);
    if let Some(typed) = calc::conversion_start(&before) {
        let from = caret - typed.chars().count();
        let units = dimension.map_or_else(Vec::new, |dimension| {
            calc::units_for(dimension, typed, language)
        });
        return (from, units);
    }
    let typed = calc::typed_word(&before);
    (
        caret - typed.chars().count(),
        calc::complete(typed, language),
    )
}

/// A name hovered says its exact value, where it was given and where it is read.
fn hovered(text: &str, sheet: Option<&CalcSheet>, at: usize, language: Language) -> Option<String> {
    let sheet = sheet?;
    let line = text.chars().take(at).filter(|it| *it == '\n').count();
    let line_start = text
        .chars()
        .take(at)
        .enumerate()
        .filter(|(_, character)| *character == '\n')
        .last()
        .map_or(0, |(found, _)| found + 1);
    let words = text.split('\n').nth(line)?;
    let span = sheet.rows.get(line)?.spans.iter().find(|span| {
        span.kind == TokenKind::Name
            && char_at(words, span.start).is_some_and(|start| line_start + start <= at)
            && char_at(words, span.end).is_some_and(|end| at <= line_start + end)
    })?;
    let name: String = words
        .chars()
        .skip(char_at(words, span.start)?)
        .take(char_at(words, span.end)? - char_at(words, span.start)?)
        .collect();
    let index = u32::try_from(line).ok()?;
    let variable = sheet
        .variables
        .iter()
        .rfind(|variable| variable.name == name && variable.line <= index)?;
    let value = calc::exact(&variable.value, language);
    let defined = i18n::translate(
        language,
        keys::calc::editor::DEFINED,
        &[("line", (variable.line + 1).into())],
    );
    let used = used_on(&variable.uses, language);
    Some(format!("{name} = {value}\n{defined} · {used}"))
}

fn used_on(uses: &[u32], language: Language) -> String {
    match uses {
        [] => i18n::translate(language, keys::calc::editor::UNUSED, &[]),
        [line] => i18n::translate(
            language,
            keys::calc::editor::USED_ONCE,
            &[("line", (line + 1).into())],
        ),
        lines => {
            let lines: Vec<String> = lines.iter().map(|line| (line + 1).to_string()).collect();
            i18n::translate(
                language,
                keys::calc::editor::USED_MANY,
                &[("lines", lines.join(", ").into())],
            )
        }
    }
}

/// The column on the right, on the rows on screen. The clicks here move no caret.
fn results(ui: &mut Ui, view: &mut CalcView, sheet: &CalcSheet, rows: &[Row], language: Language) {
    for row in rows {
        let Ok(line) = u32::try_from(row.line) else {
            continue;
        };
        match row_of(sheet, line) {
            Some(CalcLine::Value {
                value,
                name,
                aggregate,
                places,
                ..
            }) => {
                let shown = Result {
                    value,
                    places: *places,
                    named: name.is_some(),
                    aggregate: *aggregate,
                };
                value_cell(ui, view, sheet, (row, line), &shown, language);
            }
            Some(CalcLine::Error { error }) => failure_cell(ui, row, error, language),
            _ => {}
        }
    }
}

/// A line's result, as the column draws it.
struct Result<'a> {
    value: &'a CalcValue,
    places: Option<u8>,
    /// A line that names a variable: quieter than the lines it leads to.
    named: bool,
    aggregate: bool,
}

fn value_cell(
    ui: &mut Ui,
    view: &mut CalcView,
    sheet: &CalcSheet,
    (row, line): (&Row, u32),
    shown: &Result<'_>,
    language: Language,
) {
    let p = palette(ui.ctx());
    let right = row.rect.right() - 8.0;
    let written = calc::value(shown.value, shown.places, language);
    let approx = if written.approx { "≈ " } else { "" };
    let words = format!("{approx}{}", written.text);
    let selected = view.picked.contains(&line);
    let ink = if shown.named && !selected {
        p.text_1
    } else {
        p.text_0
    };
    let galley = widgets::text(ui, &words, FONT, 400.0, FontFamily::Monospace, ink);
    let width = galley.size().x.min(RESULTS_WIDTH - COPY_SIZE - 8.0);
    let cell = Rect::from_min_max(
        egui::pos2(right - width - 8.0, row.rect.top()),
        egui::pos2(right, row.rect.bottom()),
    );
    if selected {
        ui.painter()
            .rect_filled(cell, 5.0, p.pink.gamma_multiply(0.16));
    }
    ui.painter()
        .galley(egui::pos2(cell.left() + 4.0, cell.top()), galley, ink);
    if shown.aggregate {
        ui.painter()
            .hline(cell.x_range(), cell.top(), Stroke::new(1.0, p.text_1));
    }
    let response = ui.allocate_rect(cell, Sense::click());
    let label = i18n::translate(language, keys::calc::editor::PICK, &[]);
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_label(label);
        node.set_value(words.clone());
    });
    widgets::address(ui, &response, "calc-result");
    if response.clicked() {
        let modifiers = ui.input(|input| input.modifiers);
        view.pick(sheet, line, modifiers);
    }
    // Under the pointer only: on every line the icons would stand on one another.
    let hovered = ui
        .input(|input| input.pointer.latest_pos())
        .is_some_and(|at| row.rect.contains(at));
    if hovered || selected {
        copy_button(ui, view, shown.value, cell, language);
    }
}

fn failure_cell(ui: &mut Ui, row: &Row, error: &CalcError, language: Language) {
    let p = palette(ui.ctx());
    let words = calc::error(error, language);
    let galley = widgets::text(ui, &words, 12.0, 400.0, FontFamily::Proportional, p.red);
    let size = galley.size();
    let right = row.rect.right() - 8.0;
    let rect = Rect::from_min_size(
        egui::pos2(right - size.x.min(RESULTS_WIDTH), row.rect.top()),
        size,
    );
    ui.painter().galley(rect.min, galley, p.red);
    let response = ui.allocate_rect(rect, Sense::hover());
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_value(words.clone()));
    widgets::address(ui, &response, "calc-failure");
}

/// Every decimal kept, with its thousands never grouped: a sheet reads it back.
fn copy_button(
    ui: &mut Ui,
    view: &mut CalcView,
    value: &CalcValue,
    beside: Rect,
    language: Language,
) {
    let exact = calc::exact(value, language);
    let words = i18n::translate(
        language,
        keys::calc::editor::COPY_RESULT,
        &[("value", exact.as_str().into())],
    );
    let at = Rect::from_min_size(
        egui::pos2(beside.left() - COPY_SIZE - 2.0, beside.top()),
        Vec2::splat(COPY_SIZE),
    );
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(at));
    if IconButton::new(Icon::Copy, &words, "calc-copy")
        .show(&mut cell)
        .clicked()
    {
        ui.ctx().copy_text(exact.clone());
        let said = i18n::translate(
            language,
            keys::calc::editor::COPIED,
            &[("value", exact.into())],
        );
        view.said = Some((said, ui.input(|input| input.time) + SAID_FOR));
    }
}

fn footer(ui: &mut Ui, view: &mut CalcView, sheet: Option<&CalcSheet>, language: Language) {
    let p = palette(ui.ctx());
    let lines = u32::try_from(view.editor.buffer.line_count()).unwrap_or(u32::MAX);
    let names = sheet.map_or(0, distinct_names);
    let counts = format!(
        "{} · {}",
        i18n::translate(
            language,
            keys::calc::editor::LINES,
            &[("count", lines.into())]
        ),
        i18n::translate(
            language,
            keys::calc::editor::VARIABLES,
            &[("count", names.into())]
        ),
    );
    let summary = summary(ui, view, sheet, language);
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        let family = FontFamily::Monospace;
        let galley = widgets::text(ui, &counts, 11.0, 400.0, family.clone(), p.text_2);
        let response = ui.label(galley);
        widgets::address(ui, &response, "calc-counts");
        if let Some(summary) = summary {
            let galley = widgets::text(ui, &summary, 11.0, 400.0, family, p.text_1);
            let response = ui.label(galley);
            widgets::address(ui, &response, "calc-summary");
        }
    });
}

fn distinct_names(sheet: &CalcSheet) -> u32 {
    let names: BTreeSet<&str> = sheet
        .variables
        .iter()
        .map(|variable| variable.name.as_str())
        .collect();
    u32::try_from(names.len()).unwrap_or(u32::MAX)
}

/// What the footer says on its right: what was copied, or what the lines chosen add up to.
fn summary(
    ui: &Ui,
    view: &mut CalcView,
    sheet: Option<&CalcSheet>,
    language: Language,
) -> Option<String> {
    let now = ui.input(|input| input.time);
    match &view.said {
        Some((_, until)) if *until <= now => view.said = None,
        Some((said, until)) => {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(until - now));
            return Some(said.clone());
        }
        None => {}
    }
    let chosen = view.chosen();
    if chosen.is_empty() {
        return None;
    }
    let count = u32::try_from(chosen.len()).unwrap_or(u32::MAX);
    let selected = i18n::translate(
        language,
        keys::calc::editor::SELECTED,
        &[("count", count.into())],
    );
    let tr = |key| i18n::translate(language, key, &[]);
    let said = match api::notes::calc_summary(sheet?, &chosen) {
        CalcSummary::NothingToAdd => tr(keys::calc::editor::NOTHING_TO_ADD),
        CalcSummary::MixedUnits => tr(keys::calc::editor::MIXED_UNITS),
        CalcSummary::Overflow => tr(keys::calc::errors::OVERFLOW),
        CalcSummary::Figures { sum, mean } => {
            let shown = |value| {
                let written = calc::value(value, None, language);
                let approx = if written.approx { "≈ " } else { "" };
                format!("{approx}{}", written.text)
            };
            format!(
                "{} {} · {} {}",
                tr(keys::calc::editor::SUM),
                shown(&sum),
                tr(keys::calc::editor::MEAN),
                shown(&mean)
            )
        }
    };
    Some(format!("{selected} · {said}"))
}

#[cfg(test)]
mod tests;
