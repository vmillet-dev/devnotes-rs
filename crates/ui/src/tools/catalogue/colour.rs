//! A colour in four notations and its contrast against another: everything is computed in Rust,
//! the swatches are all the page paints.

use egui::{Color32, CornerRadius, Margin, Response, Role, Sense, Ui, Vec2, WidgetInfo};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::colour::{
    ColourAnswer, ColourReading, ColourRequest, Contrast, ContrastFix, Notations, TextSizes,
    WcagLevel,
};
use i18n::{Language, Message, keys};

use crate::api;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{
    ResultRow, card, columns, hint, label, paragraph, problem, result_rows, section_title,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{self, Button, IconButton, Kind, TextField, address, tooltip};

/// The swatch that opens the picker.
const PICKER: Vec2 = Vec2::new(64.0, 44.0);

pub(super) struct ColourTool {
    colour: String,
    against: String,
    answer: Live<ColourRequest, ColourAnswer>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(ColourTool {
        colour: "#1e90ff".to_owned(),
        against: "#ffffff".to_owned(),
        answer: Live::default(),
    })
}

/// `#rrggbb` or `#rrggbbaa`, as Rust writes a swatch.
fn swatch_of(text: &str) -> Option<Color32> {
    let digits = text.strip_prefix('#')?;
    let byte = |at: usize| u8::from_str_radix(digits.get(at..at + 2)?, 16).ok();
    let alpha = if digits.len() >= 8 { byte(6)? } else { 255 };
    Some(Color32::from_rgba_unmultiplied(
        byte(0)?,
        byte(2)?,
        byte(4)?,
        alpha,
    ))
}

fn read(reading: &ColourReading) -> Option<(&Notations, &str, bool)> {
    match reading {
        ColourReading::Read {
            notations,
            swatch,
            out_of_gamut,
            ..
        } => Some((notations, swatch, *out_of_gamut)),
        _ => None,
    }
}

/// The one line a reader wants: whether body text can sit on this background.
fn verdict(contrast: &Contrast, language: Language) -> String {
    use keys::tools::colour::verdicts;
    let key = if !contrast.aa.large {
        verdicts::FAILS_ALL
    } else if !contrast.aa.normal {
        verdicts::FAILS_NORMAL
    } else if contrast.aaa.normal {
        verdicts::AAA
    } else {
        verdicts::AA
    };
    i18n::translate(language, key, &[])
}

/// A verdict in its pill: green when the text passes, red when it fails.
fn pill(ui: &mut Ui, words: &str, passes: bool) -> Response {
    let p = palette(ui.ctx());
    let ink = if passes { p.green } else { p.red };
    let shown = widgets::wrapped(ui, words, (12.0, 400.0), p.text_1, f32::INFINITY);
    let padding = Vec2::new(10.0, 3.0);
    let (rect, response) = ui.allocate_exact_size(shown.size() + padding * 2.0, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, words));
    let corner = CornerRadius::same(u8::MAX);
    ui.painter()
        .rect_filled(rect, corner, ink.gamma_multiply(0.15));
    shown.paint(ui, rect.min + padding, p.text_1);
    response
}

/// One level, a pill a size of text.
fn level_row(ui: &mut Ui, name: &str, sizes: TextSizes, language: Language) {
    use keys::tools::colour as words;
    let tr = |key| i18n::translate(language, key, &[]);
    let said = |passes: bool| tr(if passes { words::PASS } else { words::FAIL });
    let ink = palette(ui.ctx()).text_2;
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.spacing_mut().item_spacing.x = 12.0;
        paragraph(ui, name, (11.5, 700.0), ink);
        let normal = format!("{} · {}", tr(words::NORMAL_TEXT), said(sizes.normal));
        pill(ui, &normal, sizes.normal);
        let large = format!("{} · {}", tr(words::LARGE_TEXT), said(sizes.large));
        pill(ui, &large, sizes.large);
    });
}

/// The two colours typed, never the theme's: the preview reads the same in both.
fn sample_box(ui: &mut Ui, (text, background): (Color32, Color32), language: Language) {
    let words = i18n::translate(language, keys::tools::colour::SAMPLE_TEXT, &[]);
    let shown = egui::Frame::new()
        .fill(background)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                paragraph(ui, "Aa", (24.0, 700.0), text);
                paragraph(ui, &words, (14.0, 400.0), text);
            });
        });
    address(ui, &shown.response, "colour-sample");
}

impl ColourTool {
    fn colour_side(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::colour as words;
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(words::COLOUR);
        section_title(ui, &name);
        let shown = self
            .answer
            .value()
            .and_then(|it| read(&it.colour))
            .and_then(|(_, swatch, _)| swatch_of(swatch));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            self.picker(ui, shown, &tr(words::PICK));
            TextField::new(&mut self.colour, &name, "colour-input")
                .hint(&tr(words::PLACEHOLDER))
                .show(ui);
        });
        let Some(answer) = self.answer.value() else {
            return;
        };
        if matches!(answer.colour, ColourReading::Invalid) {
            problem(ui, &tr(words::INVALID), "colour-invalid");
        }
        let Some((notations, _, out_of_gamut)) = read(&answer.colour) else {
            return;
        };
        card(ui, "colour-notations", |ui| {
            let rows = [
                ResultRow::new("HEX", &notations.hex),
                ResultRow::new("RGB", &notations.rgb),
                ResultRow::new("HSL", &notations.hsl),
                ResultRow::new("OKLCH", &notations.oklch),
            ];
            result_rows(ui, &rows, language);
        });
        if out_of_gamut {
            let said = hint(ui, &tr(words::OUT_OF_GAMUT));
            address(ui, &said, "colour-gamut");
        }
    }

    /// egui's picker, for the colour to be seen before it is written.
    fn picker(&mut self, ui: &mut Ui, shown: Option<Color32>, name: &str) {
        let mut rgb = shown.map_or([0, 0, 0], |it| [it.r(), it.g(), it.b()]);
        let picked = ui
            .scope(|ui| {
                ui.spacing_mut().interact_size = PICKER;
                egui::color_picker::color_edit_button_srgb(ui, &mut rgb)
            })
            .inner;
        ui.ctx()
            .accesskit_node_builder(picked.id, |node| node.set_label(name.to_owned()));
        address(ui, &picked, "colour-picker");
        if picked.changed() {
            self.colour = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
        }
    }

    fn contrast_side(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::colour as words;
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(words::CONTRAST));
        self.pair(ui, language);
        let Some(answer) = self.answer.value() else {
            return;
        };
        if matches!(answer.against, ColourReading::Invalid) {
            problem(ui, &tr(words::INVALID), "colour-against-invalid");
        }
        let Some(contrast) = answer.contrast.clone() else {
            return;
        };
        let paint =
            |reading: &ColourReading| read(reading).and_then(|(_, swatch, _)| swatch_of(swatch));
        if let (Some(text), Some(background)) = (paint(&answer.colour), paint(&answer.against)) {
            sample_box(ui, (text, background), language);
        }
        self.reading(ui, &contrast, language);
    }

    /// The text and the background side by side, trading places on the button between them.
    fn pair(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::colour as words;
        let tr = |key| i18n::translate(language, key, &[]);
        let each = (ui.available_width() - 40.0) / 2.0;
        let field = |ui: &mut Ui, value: &mut String, name: &str, id: &str, hint_words: &str| {
            ui.allocate_ui(Vec2::new(each, 0.0), |ui| {
                ui.vertical(|ui| {
                    label(ui, name);
                    TextField::new(value, name, id).hint(hint_words).show(ui);
                });
            });
        };
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let placeholder = tr(words::PLACEHOLDER);
            field(
                ui,
                &mut self.colour,
                &tr(words::TEXT),
                "colour-text",
                &placeholder,
            );
            ui.vertical(|ui| {
                ui.add_space(26.0);
                let name = tr(words::SWAP);
                let swap = IconButton::new(Icon::ArrowLeftRight, &name, "colour-swap").show(ui);
                if tooltip(swap, &name).clicked() {
                    std::mem::swap(&mut self.colour, &mut self.against);
                }
            });
            let placeholder = tr(words::BACKGROUND_PLACEHOLDER);
            let name = tr(words::BACKGROUND);
            field(ui, &mut self.against, &name, "colour-against", &placeholder);
        });
    }

    fn reading(&mut self, ui: &mut Ui, contrast: &Contrast, language: Language) {
        use keys::tools::colour as words;
        let p = palette(ui.ctx());
        let tr = |key| i18n::translate(language, key, &[]);
        let ratio = i18n::format::fixed(contrast.ratio, 2, language);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            let shown = egui::RichText::new(format!("{ratio}:1"))
                .font(egui::FontId::monospace(30.0))
                .color(p.text_0);
            let said = ui.add(egui::Label::new(shown));
            address(ui, &said, "colour-ratio");
            let words = verdict(contrast, language);
            let said = pill(ui, &words, contrast.aa.normal);
            address(ui, &said, "colour-verdict");
        });
        card(ui, "colour-verdicts", |ui| {
            ui.add_space(8.0);
            level_row(ui, "AA", contrast.aa, language);
            ui.add_space(4.0);
            level_row(ui, "AAA", contrast.aaa, language);
            ui.add_space(6.0);
        });
        hint(ui, &tr(words::WCAG));
        match &contrast.fix {
            Some(ContrastFix::Found { level, hex }) => {
                let key = match level {
                    WcagLevel::Aa => words::fix::AA,
                    WcagLevel::Aaa => words::fix::AAA,
                };
                if Button::new(&tr(key), "colour-fix")
                    .kind(Kind::Outline)
                    .show(ui)
                    .clicked()
                {
                    self.colour.clone_from(hex);
                }
            }
            Some(ContrastFix::Unreachable { level }) => {
                let key = match level {
                    WcagLevel::Aa => words::unreachable::AA,
                    WcagLevel::Aaa => words::unreachable::AAA,
                };
                let said = hint(ui, &tr(key));
                address(ui, &said, "colour-unreachable");
            }
            None => {}
        }
    }
}

impl Tool for ColourTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = ColourRequest {
            colour: self.colour.clone(),
            against: self.against.clone(),
        };
        self.answer
            .update(turn, Some(request), api::tools::describe_colour);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::colour_side, Self::contrast_side));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let (notations, _, _) = read(&self.answer.value()?.colour)?;
        let lines = [
            notations.hex.as_str(),
            notations.rgb.as_str(),
            notations.hsl.as_str(),
            notations.oklch.as_str(),
        ];
        Some(ToolResult {
            title: Message::new(keys::tools::colour::NOTE_TITLE)
                .with("hex", notations.hex.as_str())
                .text(language),
            language: NoteLanguage::Css,
            content: lines.join("\n"),
            warning: None,
        })
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        "#3b82f6".clone_into(&mut self.colour);
        "#ffffff".clone_into(&mut self.against);
    }

    fn clear(&mut self) {
        self.colour.clear();
        self.against.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_swatch_is_read_with_or_without_its_alpha() {
        assert_eq!(swatch_of("#102030"), Some(Color32::from_rgb(16, 32, 48)));
        assert!(swatch_of("#10203080").is_some());
        assert_eq!(swatch_of("red"), None);
    }
}
