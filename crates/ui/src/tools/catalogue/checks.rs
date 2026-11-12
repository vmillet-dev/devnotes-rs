//! Luhn and IBAN are Rust's; the page names the country in the language on screen. ⚠️ A card
//! number is a secret: gone with the tool, and nothing is ever kept as a note.

use egui::Ui;
use engine::tools::checks::{CardNetwork, CheckAnswer, CheckKind, CheckRequest, IbanVerdict};
use i18n::{Language, Message, keys};
use serde_json::json;
use zeroize::Zeroize;

use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{
    ResultRow, card, group_title, hint, named, paragraph, problem, result_rows,
};
use crate::tools::{Drawn, Tool};
use crate::widgets::{Segmented, TextField, address};

/// The published French example IBAN: valid, and nobody's account.
const SAMPLE: &str = "FR76 3000 6000 0112 3456 7890 189";

#[derive(Default)]
pub(super) struct ChecksTool {
    text: String,
    kind: Option<CheckKind>,
    answer: Live<CheckRequest, CheckAnswer>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::<ChecksTool>::default()
}

fn head(ui: &mut Ui, title: &str, valid: bool, reason: Option<String>, language: Language) {
    let p = palette(ui.ctx());
    let verdict = if valid {
        keys::tools::checks::VALID
    } else {
        keys::tools::checks::INVALID
    };
    let verdict = i18n::translate(language, verdict, &[]);
    let ink = if valid { p.green } else { p.red };
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.spacing_mut().item_spacing.x = 10.0;
        let said = paragraph(ui, title, (14.0, 600.0), p.text_0);
        address(ui, &said, "checks-reading");
        let said = crate::tools::ui::pill(ui, &verdict, ink);
        address(ui, &said, "checks-verdict");
    });
    if let Some(reason) = reason {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            paragraph(ui, &reason, (12.0, 400.0), p.text_1);
        });
    }
}

struct Luhn<'a> {
    grouped: &'a str,
    length: u32,
    valid: bool,
    expected_last: u32,
    completed: &'a str,
    network: Option<CardNetwork>,
}

fn luhn_card(ui: &mut Ui, luhn: &Luhn<'_>, language: Language) {
    use keys::tools::checks as words;
    let tr = |key| i18n::translate(language, key, &[]);
    let title = if luhn.network.is_some() {
        words::titles::CARD
    } else {
        words::titles::LUHN
    };
    let reason = (!luhn.valid).then(|| {
        Message::new(words::LUHN_INVALID)
            .with("digit", luhn.expected_last)
            .text(language)
    });
    let length = Message::new(words::DIGITS)
        .with("count", luhn.length)
        .text(language);
    let key = if luhn.valid {
        words::LUHN_KEY_RIGHT
    } else {
        words::LUHN_KEY_WRONG
    };
    let given = luhn
        .grouped
        .chars()
        .last()
        .map(String::from)
        .unwrap_or_default();
    let key = Message::new(key)
        .with("given", given)
        .with("expected", luhn.expected_last)
        .text(language);
    let network = luhn
        .network
        .map(|it| named(language, "tools.checks.networks", &json!(it)));
    let names = (
        tr(words::LENGTH),
        tr(words::KEY),
        tr(words::NETWORK),
        tr(words::NUMBER),
        tr(words::COMPLETED),
    );
    card(ui, "checks-result", |ui| {
        head(ui, &tr(title), luhn.valid, reason, language);
        group_title(ui, &tr(words::CHECKS_TITLE));
        let mut rows = vec![
            ResultRow::new(&names.0, &length).reading(),
            ResultRow::new(&names.1, &key).reading(),
        ];
        if let Some(network) = &network {
            rows.push(ResultRow::new(&names.2, network).reading());
        }
        result_rows(ui, &rows, language);
        group_title(ui, &tr(words::CONTENT));
        let rows = [
            ResultRow::new(&names.3, luhn.grouped),
            ResultRow::new(&names.4, luhn.completed),
        ];
        result_rows(ui, &rows, language);
    });
}

struct Iban<'a> {
    country: &'a str,
    printed: &'a str,
    check_digits: &'a str,
    length: u32,
    verdict: &'a IbanVerdict,
}

fn iban_reason(iban: &Iban<'_>, language: Language) -> Option<String> {
    use keys::tools::checks as words;
    Some(match iban.verdict {
        IbanVerdict::Valid { .. } => return None,
        IbanVerdict::WrongChecksum { expected, .. } => Message::new(words::WRONG_CHECKSUM)
            .with("expected", expected.as_str())
            .text(language),
        IbanVerdict::WrongLength { expected, found } => Message::new(words::WRONG_LENGTH)
            .with("expected", *expected)
            .with("found", *found)
            .text(language),
        IbanVerdict::WrongFormat => i18n::translate(language, words::WRONG_FORMAT, &[]),
        IbanVerdict::UnknownCountry => Message::new(words::UNKNOWN_COUNTRY)
            .with("country", iban.country)
            .text(language),
    })
}

/// « 27 characters, right for FR ».
fn iban_length(iban: &Iban<'_>, language: Language) -> String {
    use keys::tools::checks as words;
    match iban.verdict {
        IbanVerdict::WrongLength { expected, found } => Message::new(words::LENGTH_WRONG)
            .with("found", *found)
            .with("expected", *expected)
            .with("country", iban.country),
        IbanVerdict::UnknownCountry => Message::new(words::CHARACTERS).with("count", iban.length),
        _ => Message::new(words::LENGTH_RIGHT)
            .with("count", iban.length)
            .with("country", iban.country),
    }
    .text(language)
}

/// « 14 · modulo 97 right », or the digits the rest asks for.
fn iban_key(iban: &Iban<'_>, language: Language) -> String {
    use keys::tools::checks as words;
    match iban.verdict {
        IbanVerdict::Valid { .. } | IbanVerdict::WrongFormat => Message::new(words::IBAN_KEY_RIGHT)
            .with("given", iban.check_digits)
            .text(language),
        IbanVerdict::WrongChecksum { expected, .. } => Message::new(words::IBAN_KEY_WRONG)
            .with("given", iban.check_digits)
            .with("expected", expected.as_str())
            .text(language),
        _ => iban.check_digits.to_owned(),
    }
}

/// What a valid IBAN holds, each by its name.
fn iban_parts(verdict: &IbanVerdict, language: Language) -> Vec<(String, String)> {
    use keys::tools::checks as words;
    let IbanVerdict::Valid {
        bban,
        bank,
        branch,
        account,
        rib_key,
    } = verdict
    else {
        return Vec::new();
    };
    let tr = |key| i18n::translate(language, key, &[]);
    let mut parts = Vec::new();
    if let Some(bank) = bank {
        parts.push((tr(words::BANK), bank.clone()));
    }
    if let Some(branch) = branch {
        parts.push((tr(words::BRANCH), branch.clone()));
    }
    match account {
        Some(account) => parts.push((tr(words::ACCOUNT), account.clone())),
        None => parts.push((tr(words::BBAN), bban.clone())),
    }
    if let Some(rib) = rib_key {
        let shown = if rib.given == rib.expected {
            rib.given.clone()
        } else {
            Message::new(words::RIB_WRONG)
                .with("given", rib.given.as_str())
                .with("expected", rib.expected.as_str())
                .text(language)
        };
        parts.push((tr(words::RIB), shown));
    }
    parts
}

fn iban_card(ui: &mut Ui, iban: &Iban<'_>, language: Language) {
    use keys::tools::checks as words;
    let tr = |key| i18n::translate(language, key, &[]);
    let valid = matches!(iban.verdict, IbanVerdict::Valid { .. });
    let reason = iban_reason(iban, language);
    let length = iban_length(iban, language);
    let key = iban_key(iban, language);
    let country = i18n::format::country(iban.country, language).unwrap_or(iban.country);
    let country = format!("{country} ({})", iban.country);
    let compact = iban.printed.replace(' ', "");
    let parts = iban_parts(iban.verdict, language);
    let names = (
        tr(words::LENGTH),
        tr(words::KEY),
        tr(words::COUNTRY),
        tr(words::COMPACT),
        tr(words::CORRECTED),
    );
    card(ui, "checks-result", |ui| {
        head(ui, &tr(words::titles::IBAN), valid, reason, language);
        group_title(ui, &tr(words::CHECKS_TITLE));
        let rows = [
            ResultRow::new(&names.0, &length).reading(),
            ResultRow::new(&names.1, &key).reading(),
        ];
        result_rows(ui, &rows, language);
        group_title(ui, &tr(words::CONTENT));
        let mut rows = vec![ResultRow::new(&names.2, &country).reading()];
        rows.extend(parts.iter().map(|(n, v)| ResultRow::new(n, v).reading()));
        rows.push(ResultRow::new(&names.3, &compact));
        if let IbanVerdict::WrongChecksum { corrected, .. } = iban.verdict {
            rows.push(ResultRow::new(&names.4, corrected));
        }
        result_rows(ui, &rows, language);
    });
}

impl ChecksTool {
    fn intro(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::checks as words;
        let tr = |key| i18n::translate(language, key, &[]);
        hint(ui, &tr(words::INTRO));
        let options = [
            (None, tr(words::kinds::AUTO)),
            (Some(CheckKind::Luhn), tr(words::kinds::LUHN)),
            (Some(CheckKind::Iban), tr(words::kinds::IBAN)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        Segmented::new(&options, "checks-kind").show(ui, &mut self.kind);
        let name = tr(words::INPUT_LABEL);
        TextField::new(&mut self.text, &name, "checks-input")
            .hint(&tr(words::PLACEHOLDER))
            .show(ui);
        crate::tools::ui::notice(ui, &tr(words::PRIVATE), "checks-private");
    }

    fn answer_of(&self, ui: &mut Ui, language: Language) {
        use keys::tools::checks as words;
        match self.answer.value() {
            Some(CheckAnswer::Unreadable { at }) => {
                let said = Message::new(words::UNREADABLE)
                    .with("at", *at)
                    .text(language);
                problem(ui, &said, "checks-problem");
            }
            Some(CheckAnswer::TooShort) => {
                let said = i18n::translate(language, words::TOO_SHORT, &[]);
                problem(ui, &said, "checks-problem");
            }
            Some(CheckAnswer::Luhn {
                grouped,
                length,
                valid,
                expected_last,
                completed,
                network,
                ..
            }) => {
                let luhn = Luhn {
                    grouped,
                    length: *length,
                    valid: *valid,
                    expected_last: *expected_last,
                    completed,
                    network: *network,
                };
                luhn_card(ui, &luhn, language);
            }
            Some(CheckAnswer::Iban {
                country,
                printed,
                check_digits,
                length,
                verdict,
                ..
            }) => {
                let iban = Iban {
                    country,
                    printed,
                    check_digits,
                    length: *length,
                    verdict,
                };
                iban_card(ui, &iban, language);
            }
            None => {
                hint(ui, &i18n::translate(language, words::EMPTY, &[]));
            }
        }
    }
}

impl Tool for ChecksTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = (!self.text.trim().is_empty()).then(|| CheckRequest {
            text: self.text.clone(),
            kind: self.kind,
        });
        self.answer.update(turn, request, api::tools::check_digits);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        egui::ScrollArea::vertical()
            .id_salt("checks")
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                self.intro(ui, language);
                self.answer_of(ui, language);
                hint(
                    ui,
                    &i18n::translate(language, keys::tools::checks::FOOTNOTE, &[]),
                );
            });
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        self.kind = None;
        SAMPLE.clone_into(&mut self.text);
    }

    fn clear(&mut self) {
        self.left();
    }

    fn left(&mut self) {
        self.text.zeroize();
        self.answer = Live::default();
    }
}
