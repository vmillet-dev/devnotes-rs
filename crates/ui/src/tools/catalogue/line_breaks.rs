//! A text's line endings, seen and rewritten. ⚠️ No text field for the input: the window turns
//! every CRLF of a paste into LF, the very thing this tool is asked about. A paste is read raw
//! from the clipboard, and the text drawn with its endings.

use std::sync::Arc;

use egui::text::LayoutJob;
use egui::{
    CornerRadius, Event, FontId, Margin, Role, Sense, Stroke, TextFormat, Ui, Vec2, WidgetInfo,
};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::text::{
    FinalNewline, FinalNewlineChange, LineBreaksAnswer, LineBreaksRequest, LineEnding,
};
use i18n::{Language, Message, keys};

use crate::api;
use crate::backend::Pending;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::note_picker::PickedNote;
use crate::tools::ui::{CopyValue, check, columns, hint, note_import, section_head};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{self, Button, Segmented, address, interact};

/// Windows and Unix endings mixed, and blanks before them: what the tool is for.
const SAMPLE: &str = "first line\r\nsecond line\nthird line   \r\nlast line  \n";

/// Enough to see what a text is made of; the rest is counted, not drawn.
const MAX_VISIBLE_LINES: usize = 400;

const ZONE_MIN: f32 = 140.0;
const ZONE_MAX: f32 = 360.0;

pub(super) struct LineBreaksTool {
    text: String,
    ending: Option<LineEnding>,
    trim: bool,
    final_newline: FinalNewline,
    answer: Live<LineBreaksRequest, LineBreaksAnswer>,
    wants_paste: bool,
    /// What the window made of a paste from the keyboard, for a clipboard that gives nothing.
    typed: Option<String>,
    reading: Option<Pending<Option<String>>>,
}

impl Default for LineBreaksTool {
    fn default() -> Self {
        Self {
            text: String::new(),
            ending: None,
            trim: true,
            final_newline: FinalNewline::Keep,
            answer: Live::default(),
            wants_paste: false,
            typed: None,
            reading: None,
        }
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::<LineBreaksTool>::default()
}

/// A line of a text, what an editor hides set apart.
#[derive(Debug, PartialEq, Eq)]
struct VisibleLine<'a> {
    content: &'a str,
    /// Spaces and tabs at the end, drawn apart so a trim can be seen.
    trailing: &'a str,
    ending: Option<LineEnding>,
}

/// Drawing only: which endings a text has, and what they become, is Rust's to say.
fn visible_lines(text: &str) -> (Vec<VisibleLine<'_>>, usize) {
    let mut lines = Vec::new();
    let mut rest = text;
    loop {
        let found = rest.find(['\n', '\r']);
        let (line, ending, next) = match found {
            None => (rest, None, ""),
            Some(at) if rest[at..].starts_with("\r\n") => {
                (&rest[..at], Some(LineEnding::Crlf), &rest[at + 2..])
            }
            Some(at) if rest[at..].starts_with('\r') => {
                (&rest[..at], Some(LineEnding::Cr), &rest[at + 1..])
            }
            Some(at) => (&rest[..at], Some(LineEnding::Lf), &rest[at + 1..]),
        };
        let content = line.trim_end_matches([' ', '\t']);
        lines.push(VisibleLine {
            content,
            trailing: &line[content.len()..],
            ending,
        });
        if ending.is_none() {
            break;
        }
        rest = next;
    }
    // A text ending with a newline ends on an empty line with nothing after it: not a line.
    if lines.len() > 1
        && lines
            .last()
            .is_some_and(|it| it.content.is_empty() && it.trailing.is_empty())
    {
        lines.pop();
    }
    let hidden = lines.len().saturating_sub(MAX_VISIBLE_LINES);
    lines.truncate(MAX_VISIBLE_LINES);
    (lines, hidden)
}

fn ending_mark(ending: LineEnding) -> &'static str {
    match ending {
        LineEnding::Lf => "LF",
        LineEnding::Crlf => "CRLF",
        LineEnding::Cr => "CR",
    }
}

/// A text with what an editor hides: each line's ending, and the spaces left at its end.
fn line_view(ui: &mut Ui, text: &str, author_id: &str, language: Language) {
    let p = palette(ui.ctx());
    let (lines, hidden) = visible_lines(text);
    let mut job = LayoutJob::default();
    let plain = TextFormat::simple(FontId::monospace(12.5), p.text_0);
    for line in &lines {
        job.append(line.content, 0.0, plain.clone());
        let blank = TextFormat {
            background: p.red.gamma_multiply(0.2),
            ..plain.clone()
        };
        job.append(line.trailing, 0.0, blank);
        if let Some(ending) = line.ending {
            let ink = match ending {
                LineEnding::Lf => p.text_2,
                LineEnding::Crlf => p.blue,
                LineEnding::Cr => p.purple,
            };
            let mark = TextFormat::simple(FontId::monospace(10.0), ink);
            job.append(ending_mark(ending), 3.0, mark);
        }
        job.append("\n", 0.0, plain.clone());
    }
    job.wrap.max_width = ui.available_width();
    let shown = ui.add(egui::Label::new(job));
    address(ui, &shown, author_id);
    if hidden > 0 {
        let hidden = u32::try_from(hidden).unwrap_or(u32::MAX);
        let words = Message::new(keys::tools::line_breaks::MORE_LINES)
            .with("count", hidden)
            .text(language);
        hint(ui, &words);
    }
}

/// The text in its box, held between two heights.
fn text_box(ui: &mut Ui, id: &str, content: impl FnOnce(&mut Ui)) {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(ZONE_MIN);
            egui::ScrollArea::vertical()
                .id_salt(id)
                .max_height(ZONE_MAX)
                .show(ui, content);
        });
}

impl LineBreaksTool {
    fn request(&self) -> Option<LineBreaksRequest> {
        (!self.text.is_empty()).then(|| LineBreaksRequest {
            text: self.text.clone(),
            ending: self.ending,
            trim_trailing: self.trim,
            final_newline: self.final_newline,
        })
    }

    fn input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let mut pasted = false;
        section_head(ui, &tr(keys::tools::INPUT), |ui| {
            pasted = Button::new(&tr(keys::tools::line_breaks::PASTE), "line-breaks-paste")
                .show(ui)
                .clicked();
            note_import(ui, drawn, "text");
        });
        self.wants_paste |= pasted;
        self.paste_zone(ui, language);
        self.found(ui, language);
        self.options(ui, language);
    }

    /// Focusable, for a paste to land: the arrows and Tab go on past it.
    fn paste_zone(&mut self, ui: &mut Ui, language: Language) {
        let name = i18n::translate(language, keys::tools::line_breaks::INPUT_LABEL, &[]);
        let at = ui.cursor().min;
        let width = ui.available_width();
        let text = self.text.as_str();
        text_box(ui, "line-breaks-input-box", |ui| {
            if text.is_empty() {
                let words = i18n::translate(language, keys::tools::line_breaks::PLACEHOLDER, &[]);
                hint(ui, &words);
            } else {
                line_view(ui, text, "line-breaks-view", language);
            }
        });
        let height = ui.cursor().min.y - at.y - ui.spacing().item_spacing.y;
        let rect = egui::Rect::from_min_size(at, Vec2::new(width, height.max(1.0)));
        let mut layer = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        let zone = interact(
            &mut layer,
            rect.size(),
            Sense::click(),
            "line-breaks-input",
            || WidgetInfo::labeled(Role::TextInput, true, &name),
            CornerRadius::same(10),
        );
        if zone.clicked() {
            zone.request_focus();
        }
        if widgets::takes_keys(ui, &zone) {
            let typed = ui.input(|input| {
                input.events.iter().find_map(|event| match event {
                    Event::Paste(text) => Some(text.clone()),
                    _ => None,
                })
            });
            if typed.is_some() {
                self.wants_paste = true;
                self.typed = typed;
            }
        }
    }

    fn found(&self, ui: &mut Ui, language: Language) {
        let Some(answer) = self.answer.value() else {
            return;
        };
        let found = answer.found;
        let words = format!("LF {}   CRLF {}   CR {}", found.lf, found.crlf, found.cr);
        let kinds = [found.lf, found.crlf, found.cr];
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let said = hint(ui, &words);
            address(ui, &said, "line-breaks-found");
            if kinds.iter().filter(|count| **count > 0).count() > 1 {
                let mixed = i18n::translate(language, keys::tools::line_breaks::MIXED, &[]);
                crate::tools::ui::pill(ui, &mixed, palette(ui.ctx()).amber_fill);
            }
        });
    }

    fn options(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::line_breaks::{endings, finals};
        let tr = |key| i18n::translate(language, key, &[]);
        crate::tools::ui::label(ui, &tr(keys::tools::line_breaks::ENDING));
        let options = [
            (None, tr(endings::KEEP)),
            (Some(LineEnding::Lf), tr(endings::LF)),
            (Some(LineEnding::Crlf), tr(endings::CRLF)),
            (Some(LineEnding::Cr), tr(endings::CR)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        Segmented::new(&options, "line-breaks-ending").show(ui, &mut self.ending);
        crate::tools::ui::label(ui, &tr(keys::tools::line_breaks::FINAL));
        let options = [
            (FinalNewline::Keep, tr(finals::KEEP)),
            (FinalNewline::Add, tr(finals::ADD)),
            (FinalNewline::Remove, tr(finals::REMOVE)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        Segmented::new(&options, "line-breaks-final").show(ui, &mut self.final_newline);
        let words = tr(keys::tools::line_breaks::TRIM);
        check(ui, &mut self.trim, &words, "line-breaks-trim");
    }

    fn output(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let result = tr(keys::tools::RESULT);
        let answer = self.answer.value();
        crate::tools::ui::section_head(ui, &result, |ui| {
            if let Some(answer) = answer {
                CopyValue::new(&answer.text, &result, language).show(ui);
            }
        });
        let Some(answer) = answer else {
            hint(ui, &tr(keys::tools::line_breaks::EMPTY));
            return;
        };
        text_box(ui, "line-breaks-output-box", |ui| {
            line_view(ui, &answer.text, "line-breaks-output", language);
        });
        let said = hint(ui, &changes(answer, language));
        address(ui, &said, "line-breaks-changes");
    }
}

/// « 2 endings converted · 1 line trimmed · final newline added ».
fn changes(answer: &LineBreaksAnswer, language: Language) -> String {
    use keys::tools::line_breaks::final_changes;
    let converted = Message::new(keys::tools::line_breaks::CONVERTED)
        .with("count", answer.converted)
        .text(language);
    let trimmed = Message::new(keys::tools::line_breaks::TRIMMED)
        .with("count", answer.trimmed)
        .text(language);
    let last = match answer.final_newline {
        FinalNewlineChange::Unchanged => final_changes::UNCHANGED,
        FinalNewlineChange::Added => final_changes::ADDED,
        FinalNewlineChange::Removed => final_changes::REMOVED,
    };
    let last = i18n::translate(language, last, &[]);
    format!("{converted} · {trimmed} · {last}")
}

impl Tool for LineBreaksTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        if std::mem::take(&mut self.wants_paste) {
            let clipboard = Arc::clone(turn.clipboard);
            let typed = self.typed.take();
            self.reading =
                Some(turn.backend.compute(move |_| {
                    Ok(clipboard.text().filter(|text| !text.is_empty()).or(typed))
                }));
        }
        if let Some(reading) = &mut self.reading
            && let Some(answer) = reading.poll()
        {
            self.reading = None;
            if let Ok(Some(text)) = answer {
                self.text = text;
            }
        }
        let request = self.request();
        self.answer
            .update(turn, request, api::tools::fix_line_breaks);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::input, Self::output));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let answer = self.answer.value().filter(|it| !it.text.is_empty())?;
        Some(ToolResult {
            title: i18n::translate(language, keys::tools::line_breaks::NOTE_TITLE, &[]),
            language: NoteLanguage::Txt,
            content: answer.text.clone(),
            warning: None,
        })
    }

    fn imported(&mut self, _: &str, note: &PickedNote) {
        self.text.clone_from(&note.content);
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        SAMPLE.clone_into(&mut self.text);
    }

    fn clear(&mut self) {
        self.text.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_says_its_ending_and_the_spaces_left_at_its_end() {
        let (lines, hidden) = visible_lines("one  \r\ntwo\nthree\r");

        assert_eq!(hidden, 0);
        assert_eq!(
            lines[0],
            VisibleLine {
                content: "one",
                trailing: "  ",
                ending: Some(LineEnding::Crlf)
            }
        );
        assert_eq!(lines[1].ending, Some(LineEnding::Lf));
        assert_eq!(lines[2].ending, Some(LineEnding::Cr));
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn past_the_lines_drawn_the_rest_is_counted() {
        let text = "x\n".repeat(MAX_VISIBLE_LINES + 3);

        let (lines, hidden) = visible_lines(&text);

        assert_eq!(lines.len(), MAX_VISIBLE_LINES);
        assert_eq!(hidden, 3);
    }

    #[test]
    fn a_line_without_ending_is_kept() {
        let (lines, _) = visible_lines("alone");

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].ending, None);
    }
}
