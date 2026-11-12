//! A size in every unit, and how long it takes over a link. Every value is Rust's, exact; the page
//! rounds nothing, it only writes the digits in its language.

use egui::Ui;
use engine::notes::language::Language as NoteLanguage;
use engine::tools::sizes::{SizeRow, SizeUnit, SizesAnswer, SizesRequest};
use engine::tools::transfer::{
    ConnectionEstimate, RateUnit, TransferAnswer, TransferField, TransferRequest, TransferTime,
};
use i18n::{Language, Message, keys};
use serde_json::json;

use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{
    ResultRow, card, columns, group_title, hint, label, named, number_field, paragraph, problem,
    result_rows, section_title,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{ChoiceMenu, Segmented, TextField, address};

const GROUPS: [(&str, &[SizeUnit]); 3] = [
    (
        "bytes",
        &[
            SizeUnit::Byte,
            SizeUnit::Bit,
            SizeUnit::Kilobit,
            SizeUnit::Megabit,
            SizeUnit::Gigabit,
            SizeUnit::Terabit,
        ],
    ),
    (
        "decimal",
        &[
            SizeUnit::Kilobyte,
            SizeUnit::Megabyte,
            SizeUnit::Gigabyte,
            SizeUnit::Terabyte,
            SizeUnit::Petabyte,
        ],
    ),
    (
        "binary",
        &[
            SizeUnit::Kibibyte,
            SizeUnit::Mebibyte,
            SizeUnit::Gibibyte,
            SizeUnit::Tebibyte,
            SizeUnit::Pebibyte,
        ],
    ),
];

const RATES: [RateUnit; 8] = [
    RateUnit::BitPerSecond,
    RateUnit::KilobitPerSecond,
    RateUnit::MegabitPerSecond,
    RateUnit::GigabitPerSecond,
    RateUnit::BytePerSecond,
    RateUnit::KilobytePerSecond,
    RateUnit::MegabytePerSecond,
    RateUnit::GigabytePerSecond,
];

const EFFICIENCY: (u32, u32) = (50, 100);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Conversion,
    Transfer,
}

pub(super) struct SizesTool {
    tab: Tab,
    text: String,
    unit: SizeUnit,
    decimals: String,
    size: String,
    size_unit: SizeUnit,
    rate: String,
    rate_unit: RateUnit,
    efficiency: u32,
    answer: Live<SizesRequest, SizesAnswer>,
    transfer: Live<TransferRequest, TransferAnswer>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(SizesTool {
        tab: Tab::Conversion,
        text: String::new(),
        unit: SizeUnit::Megabyte,
        decimals: "3".to_owned(),
        size: String::new(),
        size_unit: SizeUnit::Gigabyte,
        rate: String::new(),
        rate_unit: RateUnit::MegabitPerSecond,
        efficiency: 90,
        answer: Live::default(),
        transfer: Live::default(),
    })
}

fn symbol(unit: SizeUnit, language: Language) -> String {
    named(language, "tools.sizes.units", &json!(unit))
}

fn rate_name(unit: RateUnit, language: Language) -> String {
    named(language, "tools.sizes.rateUnits", &json!(unit))
}

/// Digits in the language on screen, as Rust wrote them.
fn digits(value: &str, language: Language) -> String {
    let fraction = value.split_once('.').map_or(0, |(_, it)| it.len());
    i18n::format::exact(value, (fraction, fraction), false, true, language)
}

fn is_zero(value: &str) -> bool {
    value
        .trim_start_matches('-')
        .bytes()
        .all(|it| it == b'0' || it == b'.')
}

/// What rounding would write as a bare 0, as a power of ten: `0.000008` is `8 × 10^-6`.
fn scientific(value: &str, decimals: usize, language: Language) -> Option<(String, bool)> {
    let (sign, rest) = value.strip_prefix('-').map_or(("", value), |it| ("-", it));
    let fraction = rest.strip_prefix("0.")?;
    let zeros = fraction.bytes().take_while(|it| *it == b'0').count();
    let figures = &fraction[zeros..];
    if figures.is_empty() {
        return None;
    }
    let exponent = i32::try_from(zeros).ok()? + 1;
    let kept = figures.len().min(decimals + 1);
    let mantissa = format!("{}.{}", &figures[..1], &figures[1..kept]);
    let shown = i18n::format::exact(
        mantissa.trim_end_matches('.'),
        (0, decimals),
        false,
        false,
        language,
    );
    Some((
        format!("{sign}{shown} × 10^-{exponent}"),
        figures.len() <= kept,
    ))
}

/// « ≈ » before what rounding changed; a power of ten where it would have left a bare 0.
fn shown_value(row: &SizeRow, decimals: usize, language: Language) -> String {
    let power = is_zero(&row.rounded)
        .then(|| scientific(&row.exact, decimals, language))
        .flatten();
    let (text, exact) =
        power.unwrap_or_else(|| (digits(&row.rounded, language), row.rounded == row.exact));
    if exact { text } else { format!("≈ {text}") }
}

/// « 6 min 58 s », « 3 d 4 h », « < 1 s ».
fn time(span: &TransferTime, language: Language) -> String {
    use keys::tools::sizes::time;
    if span.under_a_second {
        return i18n::translate(language, time::UNDER_ASECOND, &[]);
    }
    let value = |key, count: u32| {
        let shown = i18n::format::number(f64::from(count), language);
        Message::new(key).with("value", shown).text(language)
    };
    let parts: Vec<String> = [
        (time::DAYS, span.days),
        (time::HOURS, span.hours),
        (time::MINUTES, span.minutes),
        (time::SECONDS, span.seconds),
    ]
    .into_iter()
    .filter(|(_, count)| *count > 0)
    .map(|(key, count)| value(key, count))
    .collect();
    if parts.is_empty() {
        value(time::SECONDS, 0)
    } else {
        parts.join(" ")
    }
}

fn unit_options(language: Language) -> Vec<(SizeUnit, String)> {
    GROUPS
        .iter()
        .flat_map(|(_, units)| units.iter())
        .map(|unit| {
            let name = named(language, "tools.sizes.unitNames", &json!(unit));
            (*unit, name)
        })
        .collect()
}

fn unit_menu(ui: &mut Ui, chosen: &mut SizeUnit, author_id: &str, language: Language) {
    let options = unit_options(language);
    let options: Vec<(SizeUnit, &str)> =
        options.iter().map(|(id, it)| (*id, it.as_str())).collect();
    let name = i18n::translate(language, keys::tools::sizes::UNIT, &[]);
    ChoiceMenu::new(&name, author_id, &options).show(ui, chosen);
}

fn field_name(field: TransferField, language: Language) -> String {
    named(language, "tools.sizes.fields", &json!(field))
}

fn transfer_problem(answer: &TransferAnswer, language: Language) -> Option<String> {
    use keys::tools::sizes::transfer_problems as words;
    let at_field = |key, field: &TransferField, at: &u32| {
        Message::new(key)
            .with("field", field_name(*field, language))
            .with("at", *at)
            .text(language)
    };
    Some(match answer {
        TransferAnswer::Unreadable { field, at } => at_field(words::UNREADABLE, field, at),
        TransferAnswer::Negative { field, at } => at_field(words::NEGATIVE, field, at),
        TransferAnswer::ZeroRate => i18n::translate(language, words::ZERO_RATE, &[]),
        TransferAnswer::TooLarge => i18n::translate(language, words::TOO_LARGE, &[]),
        TransferAnswer::Estimated { .. } => return None,
    })
}

impl SizesTool {
    fn decimals(&self) -> u32 {
        self.decimals.trim().parse().unwrap_or(3).min(12)
    }

    fn value_input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(keys::tools::sizes::VALUE));
        let name = tr(keys::tools::sizes::INPUT_LABEL);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let room = ui.available_width() - 170.0;
            ui.scope(|ui| {
                ui.set_max_width(room);
                TextField::new(&mut self.text, &name, "sizes-input")
                    .hint(&tr(keys::tools::sizes::PLACEHOLDER))
                    .show(ui);
            });
            unit_menu(ui, &mut self.unit, "sizes-unit", language);
        });
        let name = tr(keys::tools::sizes::DECIMALS);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            label(ui, &name);
            number_field(ui, &mut self.decimals, &name, "sizes-decimals");
        });
        match self.answer.value() {
            Some(SizesAnswer::Converted {
                unit_in_text, gap, ..
            }) => {
                if *unit_in_text {
                    let said = hint(ui, &tr(keys::tools::sizes::UNIT_IN_TEXT));
                    address(ui, &said, "sizes-unit-in-text");
                }
                let words = Message::new(keys::tools::sizes::GAP)
                    .with("decimal", symbol(gap.decimal, language))
                    .with("binary", symbol(gap.binary, language))
                    .with("ratio", digits(&gap.rounded, language))
                    .text(language);
                let p = palette(ui.ctx());
                egui::Frame::new()
                    .fill(p.bg_1)
                    .stroke(egui::Stroke::new(1.0, p.line_soft))
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(16, 14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 8.0;
                        let family = egui::FontFamily::Monospace;
                        let shown = crate::widgets::text(ui, &words, 18.0, 400.0, family, p.text_0);
                        let said = ui.label(shown);
                        address(ui, &said, "sizes-gap");
                        hint(ui, &tr(keys::tools::sizes::CONVENTIONS));
                    });
            }
            Some(SizesAnswer::Unreadable { at }) => {
                let words = Message::new(keys::tools::sizes::UNREADABLE)
                    .with("at", *at)
                    .text(language);
                problem(ui, &words, "sizes-problem");
            }
            Some(SizesAnswer::Negative { at }) => {
                let words = Message::new(keys::tools::sizes::NEGATIVE)
                    .with("at", *at)
                    .text(language);
                problem(ui, &words, "sizes-problem");
            }
            Some(SizesAnswer::TooLarge) => {
                problem(ui, &tr(keys::tools::sizes::TOO_LARGE), "sizes-problem");
            }
            None => {}
        }
    }

    fn converted(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let Some(SizesAnswer::Converted { rows, .. }) = self.answer.value() else {
            hint(ui, &tr(keys::tools::sizes::EMPTY));
            return;
        };
        let decimals = usize::try_from(self.decimals()).unwrap_or(3);
        card(ui, "sizes-results", |ui| {
            for (group, units) in GROUPS {
                let path = format!("tools.sizes.groups.{group}");
                let title = i18n::Key::find(&path)
                    .map_or_else(String::new, |key| i18n::translate(language, key, &[]));
                group_title(ui, &title);
                let shown: Vec<(String, String, &str)> = units
                    .iter()
                    .filter_map(|unit| rows.iter().find(|row| row.unit == *unit))
                    .map(|row| {
                        let value = shown_value(row, decimals, language);
                        (symbol(row.unit, language), value, row.exact.as_str())
                    })
                    .collect();
                let lines: Vec<_> = shown
                    .iter()
                    .map(|(name, value, exact)| ResultRow::new(name, value).copied(exact))
                    .collect();
                result_rows(ui, &lines, language);
            }
        });
        hint(ui, &tr(keys::tools::sizes::ROUNDED));
    }

    fn transfer_input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(keys::tools::sizes::TRANSFER_SIZE);
        section_title(ui, &name);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let room = ui.available_width() - 170.0;
            ui.scope(|ui| {
                ui.set_max_width(room);
                TextField::new(&mut self.size, &name, "transfer-size")
                    .hint("4,7")
                    .show(ui);
            });
            unit_menu(ui, &mut self.size_unit, "transfer-size-unit", language);
        });
        let name = tr(keys::tools::sizes::RATE);
        section_title(ui, &name);
        let rates: Vec<(RateUnit, String)> = RATES
            .iter()
            .map(|it| (*it, rate_name(*it, language)))
            .collect();
        let rates: Vec<(RateUnit, &str)> =
            rates.iter().map(|(id, it)| (*id, it.as_str())).collect();
        let unit_name = tr(keys::tools::sizes::RATE_UNIT);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let room = ui.available_width() - 170.0;
            ui.scope(|ui| {
                ui.set_max_width(room);
                TextField::new(&mut self.rate, &name, "transfer-rate")
                    .hint("100")
                    .show(ui);
            });
            ChoiceMenu::new(&unit_name, "transfer-rate-unit", &rates).show(ui, &mut self.rate_unit);
        });
        let name = tr(keys::tools::sizes::EFFICIENCY);
        label(ui, &name);
        let range = EFFICIENCY.0..=EFFICIENCY.1;
        let slider = ui.add(egui::Slider::new(&mut self.efficiency, range).suffix(" %"));
        ui.ctx()
            .accesskit_node_builder(slider.id, |node| node.set_label(name.clone()));
        address(ui, &slider, "transfer-efficiency");
        hint(ui, &tr(keys::tools::sizes::EFFICIENCY_HINT));
        hint(ui, &tr(keys::tools::sizes::BITS_AND_BYTES));
    }

    fn rate_label(&self, language: Language) -> String {
        self.transfer.answered().map_or_else(String::new, |asked| {
            format!(
                "{} {}",
                asked.rate.trim(),
                rate_name(asked.rate_unit, language)
            )
        })
    }

    fn connection_name(&self, estimate: &ConnectionEstimate, language: Language) -> String {
        match estimate.connection {
            None => self.rate_label(language),
            Some(connection) => named(language, "tools.sizes.connections", &json!(connection)),
        }
    }

    fn estimate(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let Some(answer) = self.transfer.value() else {
            hint(ui, &tr(keys::tools::sizes::TRANSFER_EMPTY));
            return;
        };
        let TransferAnswer::Estimated {
            estimate,
            theoretical,
            efficiency,
            connections,
        } = answer
        else {
            if let Some(words) = transfer_problem(answer, language) {
                problem(ui, &words, "transfer-problem");
            }
            return;
        };
        let p = palette(ui.ctx());
        let at = Message::new(keys::tools::sizes::ESTIMATE_AT)
            .with("rate", self.rate_label(language))
            .with("efficiency", *efficiency)
            .text(language);
        hint(ui, &at);
        let said = paragraph(ui, &time(estimate, language), (22.0, 700.0), p.text_0);
        address(ui, &said, "transfer-time");
        let words = Message::new(keys::tools::sizes::THEORETICAL)
            .with("time", time(theoretical, language))
            .text(language);
        let said = hint(ui, &words);
        address(ui, &said, "transfer-theoretical");
        section_title(ui, &tr(keys::tools::sizes::BY_CONNECTION));
        let rows: Vec<(String, String)> = connections
            .iter()
            .map(|it| (self.connection_name(it, language), time(&it.span, language)))
            .collect();
        card(ui, "transfer-connections", |ui| {
            let shown: Vec<_> = rows.iter().map(|(n, v)| ResultRow::new(n, v)).collect();
            result_rows(ui, &shown, language);
        });
    }

    fn conversion_result(&self, language: Language) -> Option<ToolResult> {
        let Some(SizesAnswer::Converted { rows, unit, .. }) = self.answer.value() else {
            return None;
        };
        let read = rows.iter().find(|row| row.unit == *unit)?;
        let lines: Vec<String> = rows
            .iter()
            .map(|row| format!("{:5} {}", symbol(row.unit, language), row.exact))
            .collect();
        Some(ToolResult {
            title: Message::new(keys::tools::sizes::NOTE_TITLE)
                .with("value", digits(&read.exact, language))
                .with("unit", symbol(*unit, language))
                .text(language),
            language: NoteLanguage::Txt,
            content: lines.join("\n"),
            warning: None,
        })
    }

    fn transfer_result(&self, language: Language) -> Option<ToolResult> {
        let Some(TransferAnswer::Estimated {
            efficiency,
            connections,
            ..
        }) = self.transfer.value()
        else {
            return None;
        };
        let asked = self.transfer.answered()?;
        let rows: Vec<(String, String)> = connections
            .iter()
            .map(|it| (self.connection_name(it, language), time(&it.span, language)))
            .collect();
        let width = rows.iter().map(|(name, _)| name.chars().count()).max()?;
        let lines: Vec<String> = rows
            .iter()
            .map(|(name, value)| format!("{name:width$}  {value}"))
            .collect();
        Some(ToolResult {
            title: Message::new(keys::tools::sizes::NOTE_TRANSFER)
                .with("size", asked.size.trim())
                .with("rate", self.rate_label(language))
                .with("efficiency", *efficiency)
                .text(language),
            language: NoteLanguage::Txt,
            content: lines.join("\n"),
            warning: None,
        })
    }
}

impl Tool for SizesTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = (!self.text.trim().is_empty()).then(|| SizesRequest {
            text: self.text.clone(),
            unit: self.unit,
            decimals: self.decimals(),
        });
        self.answer.update(turn, request, api::tools::convert_size);
        let typed = !self.size.trim().is_empty() && !self.rate.trim().is_empty();
        let request = typed.then(|| TransferRequest {
            size: self.size.clone(),
            size_unit: self.size_unit,
            rate: self.rate.clone(),
            rate_unit: self.rate_unit,
            efficiency: self.efficiency,
        });
        self.transfer
            .update(turn, request, api::tools::estimate_transfer);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::sizes::tabs;
        let tr = |key| i18n::translate(drawn.language, key, &[]);
        let options = [
            (Tab::Conversion, tr(tabs::CONVERSION)),
            (Tab::Transfer, tr(tabs::TRANSFER)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        Segmented::new(&options, "sizes-tab").show(ui, &mut self.tab);
        ui.add_space(12.0);
        match self.tab {
            Tab::Conversion => columns(ui, self, drawn, (Self::value_input, Self::converted)),
            Tab::Transfer => columns(ui, self, drawn, (Self::transfer_input, Self::estimate)),
        }
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        match self.tab {
            Tab::Conversion => self.conversion_result(language),
            Tab::Transfer => self.transfer_result(language),
        }
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        self.unit = SizeUnit::Gigabyte;
        "4,7".clone_into(&mut self.text);
        "4,7".clone_into(&mut self.size);
        self.size_unit = SizeUnit::Gigabyte;
        "100".clone_into(&mut self.rate);
        self.rate_unit = RateUnit::MegabitPerSecond;
        self.efficiency = 90;
    }

    fn clear(&mut self) {
        self.text.clear();
        self.size.clear();
        self.rate.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_rounding_would_lose_is_a_power_of_ten() {
        let shown = scientific("0.000008", 3, Language::En);

        assert_eq!(shown, Some(("8 × 10^-6".to_owned(), true)));
    }

    #[test]
    fn a_rounded_value_is_said_to_be() {
        let row = SizeRow {
            unit: SizeUnit::Gibibyte,
            exact: "4.3772".to_owned(),
            rounded: "4.377".to_owned(),
        };

        assert_eq!(shown_value(&row, 3, Language::En), "≈ 4.377");
    }
}
