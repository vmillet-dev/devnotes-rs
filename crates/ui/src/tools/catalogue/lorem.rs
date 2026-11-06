//! Words, sentences or paragraphs of placeholder text, drawn again on demand.

use egui::{CornerRadius, Margin, Stroke, Ui};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::generate::{LoremAnswer, LoremRequest, LoremUnit};
use i18n::{Key, Language, Message, keys};

use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{CopyValue, check, hint, label, number_field, paragraph};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{Button, Segmented, address};

/// A readable measure.
const MAX_WIDTH: f32 = 860.0;

/// The request, and which draw of it: asking again draws another text.
type Asked = (LoremRequest, u32);

pub(super) struct LoremTool {
    unit: LoremUnit,
    count: String,
    opening: bool,
    draw: u32,
    answer: Live<Asked, LoremAnswer>,
}

impl Default for LoremTool {
    fn default() -> Self {
        Self {
            unit: LoremUnit::Paragraphs,
            count: "3".to_owned(),
            opening: true,
            draw: 0,
            answer: Live::default(),
        }
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::<LoremTool>::default()
}

fn counted(unit: LoremUnit) -> Key {
    match unit {
        LoremUnit::Words => keys::tools::lorem::counted::WORDS,
        LoremUnit::Sentences => keys::tools::lorem::counted::SENTENCES,
        LoremUnit::Paragraphs => keys::tools::lorem::counted::PARAGRAPHS,
    }
}

fn held_to(unit: LoremUnit) -> Key {
    match unit {
        LoremUnit::Words => keys::tools::lorem::held_to::WORDS,
        LoremUnit::Sentences => keys::tools::lorem::held_to::SENTENCES,
        LoremUnit::Paragraphs => keys::tools::lorem::held_to::PARAGRAPHS,
    }
}

impl LoremTool {
    fn request(&self) -> Option<Asked> {
        let count = self.count.trim().parse().ok()?;
        let request = LoremRequest {
            unit: self.unit,
            count,
            opening: self.opening,
            seed: None,
        };
        Some((request, self.draw))
    }

    fn options(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::lorem::units;
        let tr = |key| i18n::translate(language, key, &[]);
        let options = [
            (LoremUnit::Words, tr(units::WORDS)),
            (LoremUnit::Sentences, tr(units::SENTENCES)),
            (LoremUnit::Paragraphs, tr(units::PARAGRAPHS)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            Segmented::new(&options, "lorem-unit").show(ui, &mut self.unit);
            let words = tr(keys::tools::lorem::COUNT);
            label(ui, &words);
            number_field(ui, &mut self.count, &words, "lorem-count");
            let words = tr(keys::tools::lorem::OPENING);
            check(ui, &mut self.opening, &words, "lorem-opening");
        });
    }

    /// The count asked, when Rust held it to its bound: said rather than silently lowered.
    fn held(&self, ui: &mut Ui, language: Language) {
        let (Some(answer), Some((asked, _))) = (self.answer.value(), self.answer.answered()) else {
            return;
        };
        if asked.count > answer.at_most {
            let words = Message::new(held_to(asked.unit))
                .with("count", answer.at_most)
                .text(language);
            let said = hint(ui, &words);
            address(ui, &said, "lorem-held");
        }
    }

    fn card(&mut self, ui: &mut Ui, language: Language) {
        let Some(answer) = self.answer.value() else {
            return;
        };
        let p = palette(ui.ctx());
        let unit = self.answer.answered().map_or(self.unit, |(it, _)| it.unit);
        let words = Message::new(counted(unit))
            .with("count", answer.count)
            .text(language);
        let again = i18n::translate(language, keys::tools::lorem::AGAIN, &[]);
        let mut drawn_again = false;
        egui::Frame::new()
            .fill(p.bg_1)
            .stroke(Stroke::new(1.0, p.line_soft))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(20, 12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let said = paragraph(ui, &words, (12.5, 600.0), p.text_2);
                    address(ui, &said, "lorem-counted");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        CopyValue::new(&answer.text, "Lorem ipsum", language).show(ui);
                        drawn_again = Button::new(&again, "lorem-again")
                            .kind(crate::widgets::Kind::Accent)
                            .show(ui)
                            .clicked();
                    });
                });
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .id_salt("lorem-text")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        let text = egui::RichText::new(&answer.text).size(13.5).color(p.text_0);
                        let shown = ui.add(egui::Label::new(text).wrap());
                        address(ui, &shown, "lorem-output");
                    });
            });
        if drawn_again {
            self.draw += 1;
        }
    }
}

impl Tool for LoremTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = self.request();
        self.answer.update(turn, request, |backend, (asked, _)| {
            api::tools::lorem(backend, asked)
        });
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let width = ui.available_width().min(MAX_WIDTH);
        ui.set_max_width(width);
        ui.spacing_mut().item_spacing.y = 16.0;
        self.options(ui, drawn.language);
        self.held(ui, drawn.language);
        self.card(ui, drawn.language);
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let answer = self.answer.value().filter(|it| !it.text.is_empty())?;
        Some(ToolResult {
            title: i18n::translate(language, keys::tools::lorem::NOTE_TITLE, &[]),
            language: NoteLanguage::Txt,
            content: answer.text.clone(),
            warning: None,
        })
    }

    fn has_sample(&self) -> bool {
        true
    }

    /// Three paragraphs, newly drawn.
    fn sample(&mut self, language: Language) {
        self.clear();
        self.draw += 1;
        let _ = language;
    }

    /// Nothing typed to empty: Clear puts the options back as they were.
    fn clear(&mut self) {
        self.unit = LoremUnit::Paragraphs;
        "3".clone_into(&mut self.count);
        self.opening = true;
    }
}
