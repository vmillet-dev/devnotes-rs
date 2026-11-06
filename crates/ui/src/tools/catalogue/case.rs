//! A text in every case, and its slug.

use egui::{CornerRadius, Margin, Stroke, Ui};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::text::{
    CaseAnswer, CaseGroup, CaseRequest, SlugOptions, SlugSeparator, TextCase, TitleLanguage,
};
use i18n::{Language, Message, keys};

use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{
    ResultRow, card, check, columns, group_title, hint, label, note_import, paragraph, result_rows,
    section_head, section_title, text_area,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::Segmented;

pub(super) struct CaseTool {
    text: String,
    strip_accents: bool,
    per_line: bool,
    title_language: TitleLanguage,
    separator: SlugSeparator,
    lowercase: bool,
    answer: Live<CaseRequest, CaseAnswer>,
}

impl Default for CaseTool {
    fn default() -> Self {
        Self {
            text: String::new(),
            strip_accents: true,
            per_line: true,
            title_language: TitleLanguage::English,
            separator: SlugSeparator::Dash,
            lowercase: true,
            answer: Live::default(),
        }
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::<CaseTool>::default()
}

/// Each case spelled in itself: a note keeps them in no one language.
fn case_name(case: TextCase) -> &'static str {
    match case {
        TextCase::Camel => "camelCase",
        TextCase::Pascal => "PascalCase",
        TextCase::Snake => "snake_case",
        TextCase::Kebab => "kebab-case",
        TextCase::Constant => "CONSTANT_CASE",
        TextCase::Title => "Title Case",
        TextCase::Sentence => "Sentence case",
        TextCase::Dot => "dot.case",
        TextCase::Path => "path/case",
        TextCase::Train => "Train-Case",
        TextCase::Lower => "lowercase",
        TextCase::Upper => "UPPERCASE",
        TextCase::Flat => "flatcase",
    }
}

/// On screen, the two whose name is a word of the language rather than an example of itself.
fn shown_name(case: TextCase, language: Language) -> String {
    match case {
        TextCase::Upper => i18n::translate(language, keys::tools::case::UPPER, &[]),
        TextCase::Lower => i18n::translate(language, keys::tools::case::LOWER, &[]),
        other => case_name(other).to_owned(),
    }
}

impl CaseTool {
    fn request(&self) -> Option<CaseRequest> {
        (!self.text.trim().is_empty()).then(|| CaseRequest {
            text: self.text.clone(),
            strip_accents: self.strip_accents,
            per_line: self.per_line,
            title_case_language: self.title_language,
            slug: SlugOptions {
                separator: self.separator,
                lowercase: self.lowercase,
            },
        })
    }

    fn input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let tr = |key| i18n::translate(drawn.language, key, &[]);
        section_head(ui, &tr(keys::tools::case::TEXT), |ui| {
            note_import(ui, drawn, "text");
        });
        let tr = |key| i18n::translate(drawn.language, key, &[]);
        text_area(
            ui,
            &mut self.text,
            (&tr(keys::tools::case::INPUT_LABEL), "case-input"),
            &tr(keys::tools::case::PLACEHOLDER),
            3,
        );
        self.words(ui, drawn.language);
        ui.add_space(8.0);
        section_title(ui, &tr(keys::tools::case::OPTIONS));
        let words = tr(keys::tools::case::STRIP_ACCENTS);
        check(ui, &mut self.strip_accents, &words, "case-strip-accents");
        let words = tr(keys::tools::case::PER_LINE);
        check(ui, &mut self.per_line, &words, "case-per-line");
        label(ui, &tr(keys::tools::case::TITLE_CASE));
        let languages = [
            (TitleLanguage::English, tr(keys::tools::case::ENGLISH)),
            (TitleLanguage::French, tr(keys::tools::case::FRENCH)),
        ];
        let languages = languages.each_ref().map(|(id, name)| (*id, name.as_str()));
        Segmented::new(&languages, "case-title-language").show(ui, &mut self.title_language);
        hint(ui, &tr(keys::tools::case::TITLE_CASE_HINT));
        self.slug_options(ui, drawn.language);
        acronyms(ui, drawn.language);
    }

    fn words(&self, ui: &mut Ui, language: Language) {
        let Some(answer) = self.answer.value().filter(|it| !it.words.is_empty()) else {
            return;
        };
        let p = palette(ui.ctx());
        label(
            ui,
            &i18n::translate(language, keys::tools::case::WORDS, &[]),
        );
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::splat(6.0);
            for word in &answer.words {
                egui::Frame::new()
                    .fill(p.bg_2)
                    .stroke(Stroke::new(1.0, p.line_soft))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(8, 2))
                    .show(ui, |ui| {
                        let text = egui::RichText::new(word)
                            .font(egui::FontId::monospace(11.5))
                            .color(p.text_0);
                        ui.label(text);
                    });
            }
        });
    }

    fn slug_options(&mut self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        label(ui, &tr(keys::tools::case::SLUG));
        let separators = [
            (SlugSeparator::Dash, tr(keys::tools::case::separators::DASH)),
            (
                SlugSeparator::Underscore,
                tr(keys::tools::case::separators::UNDERSCORE),
            ),
            (SlugSeparator::Dot, tr(keys::tools::case::separators::DOT)),
        ];
        let separators = separators.each_ref().map(|(id, name)| (*id, name.as_str()));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            Segmented::new(&separators, "case-slug-separator").show(ui, &mut self.separator);
            let words = tr(keys::tools::case::LOWERCASE);
            check(ui, &mut self.lowercase, &words, "case-slug-lowercase");
        });
        hint(ui, &tr(keys::tools::case::SLUG_HINT));
    }

    fn output(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let Some(answer) = self.answer.value().filter(|it| !it.conversions.is_empty()) else {
            hint(ui, &tr(keys::tools::case::EMPTY));
            return;
        };
        card(ui, "case-results", |ui| {
            for (group, title) in [
                (CaseGroup::Code, keys::tools::case::FOR_CODE),
                (CaseGroup::Text, keys::tools::case::FOR_TEXT),
            ] {
                group_title(ui, &tr(title));
                let shown: Vec<_> = answer
                    .conversions
                    .iter()
                    .filter(|conversion| conversion.group == group)
                    .map(|conversion| (shown_name(conversion.case, language), &conversion.value))
                    .collect();
                let rows: Vec<_> = shown
                    .iter()
                    .map(|(name, value)| ResultRow::new(name, value))
                    .collect();
                result_rows(ui, &rows, language);
            }
            group_title(ui, &tr(keys::tools::case::FOR_URL));
            result_rows(ui, &[ResultRow::new("Slug", &answer.slug)], language);
        });
    }
}

/// Identifiers, so in no language: only the sentences around them are translated.
fn acronyms(ui: &mut Ui, language: Language) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    egui::Frame::new()
        .fill(p.amber_fill.gamma_multiply(0.06))
        .stroke(Stroke::new(1.0, p.amber_dim))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let words = format!(
                "{} parseURL → parse_url, HTTPServer → http_server.",
                tr(keys::tools::case::ACRONYMS)
            );
            paragraph(ui, &words, (12.0, 400.0), p.text_1);
            let words = format!("{} v2Api.", tr(keys::tools::case::ACRONYMS_DIGITS));
            paragraph(ui, &words, (12.0, 400.0), p.text_1);
        });
}

impl Tool for CaseTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = self.request();
        self.answer.update(turn, request, api::tools::convert_case);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::input, Self::output));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let answer = self
            .answer
            .value()
            .filter(|it| !it.conversions.is_empty())?;
        let mut rows: Vec<(&str, &str)> = answer
            .conversions
            .iter()
            .map(|it| (case_name(it.case), it.value.as_str()))
            .collect();
        rows.push(("slug", answer.slug.as_str()));
        let width = rows.iter().map(|(name, _)| name.len()).max()?;
        // A list converted line by line keeps its lines under the first, past the names.
        let under = format!("\n{}", " ".repeat(width + 2));
        let content: Vec<String> = rows
            .iter()
            .map(|(name, value)| format!("{name:width$}  {}", value.replace('\n', &under)))
            .collect();
        let asked = self.answer.answered()?;
        let first = asked.text.trim().lines().next().unwrap_or_default().trim();
        let text: String = first.chars().take(40).collect();
        Some(ToolResult {
            title: Message::new(keys::tools::case::NOTE_TITLE)
                .with("text", text)
                .text(language),
            language: NoteLanguage::Txt,
            content: content.join("\n"),
            warning: None,
        })
    }

    fn imported(&mut self, _: &str, note: &crate::tools::ui::note_picker::PickedNote) {
        self.text.clone_from(&note.content);
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, language: Language) {
        self.text = i18n::translate(language, keys::tools::case::SAMPLE, &[]);
    }

    fn clear(&mut self) {
        self.text.clear();
    }
}
