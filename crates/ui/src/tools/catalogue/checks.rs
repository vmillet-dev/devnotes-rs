//! Luhn and IBAN are Rust's; the page names the country in the language on screen. ⚠️ A card
//! number is a secret: gone with the tool, and nothing is ever kept as a note.

use crate::theme::tokens::{font, gap};
use egui::Ui;
use engine::tools::check_numbers::{Affix, DrawnNumber, NumbersAnswer, NumbersRequest};
use engine::tools::checks::{CardNetwork, CheckAnswer, CheckKind, CheckRequest, IbanVerdict};
use engine::tools::identity::Country;
use i18n::{Language, Message, keys};
use serde_json::json;
use zeroize::Zeroize;

use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{
    CopyValue, ResultRow, card, columns, group_title, hint, label, line, named, number_field,
    paragraph, problem, result_rows, section_head, section_title,
};
use crate::tools::{Drawn, Tool};
use crate::widgets::{Button, ChoiceMenu, Kind, Segmented, TextField, address};

/// The published French example IBAN: valid, and nobody's account.
const SAMPLE: &str = "FR76 3000 6000 0112 3456 7890 189";

/// What a card number or an IBAN takes, and no more.
const INPUT: f32 = 420.0;
/// A prefix or a suffix.
const AFFIX: f32 = 150.0;

/// A new draw with the same options is part of what is asked.
type Asked = (NumbersRequest, u32);

pub(super) struct ChecksTool {
    text: String,
    kind: Option<CheckKind>,
    answer: Live<CheckRequest, CheckAnswer>,
    drawn_kind: CheckKind,
    prefix: String,
    suffix: String,
    length: String,
    count: String,
    country: Country,
    draw: u32,
    numbers: Live<Asked, NumbersAnswer>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(ChecksTool {
        text: String::new(),
        kind: None,
        answer: Live::default(),
        drawn_kind: CheckKind::Luhn,
        prefix: String::new(),
        suffix: String::new(),
        length: "16".to_owned(),
        count: "5".to_owned(),
        country: Country::Fr,
        draw: 0,
        numbers: Live::default(),
    })
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
    ui.add_space(gap::GROUPS);
    ui.horizontal(|ui| {
        ui.add_space(gap::BLOCKS);
        ui.spacing_mut().item_spacing.x = gap::INSET;
        let said = paragraph(ui, title, (font::SUBTITLE, 600.0), p.text_0);
        address(ui, &said, "checks-reading");
        let said = crate::tools::ui::pill(ui, &verdict, ink);
        address(ui, &said, "checks-verdict");
    });
    if let Some(reason) = reason {
        ui.add_space(gap::TIGHT);
        ui.horizontal(|ui| {
            ui.add_space(gap::BLOCKS);
            paragraph(ui, &reason, (font::LABEL, 400.0), p.text_1);
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
        let (number, completed) = (
            api::tools::raw_number(luhn.grouped),
            api::tools::raw_number(luhn.completed),
        );
        let rows = [
            ResultRow::new(&names.3, luhn.grouped).raw(&number),
            ResultRow::new(&names.4, luhn.completed).raw(&completed),
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
    let compact = api::tools::raw_number(iban.printed);
    let parts = iban_parts(iban.verdict, language);
    let names = (
        tr(words::LENGTH),
        tr(words::KEY),
        tr(words::COUNTRY),
        tr(words::kinds::IBAN),
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
        rows.push(ResultRow::new(&names.3, iban.printed).raw(&compact));
        let corrected = match iban.verdict {
            IbanVerdict::WrongChecksum { corrected, .. } => corrected.as_str(),
            _ => "",
        };
        let corrected_raw = api::tools::raw_number(corrected);
        if !corrected.is_empty() {
            rows.push(ResultRow::new(&names.4, corrected).raw(&corrected_raw));
        }
        result_rows(ui, &rows, language);
    });
}

fn number_list(ui: &mut Ui, numbers: &[DrawnNumber], language: Language) {
    use keys::tools::checks as words;
    let count = u32::try_from(numbers.len()).unwrap_or(u32::MAX);
    let counted = Message::new(words::GENERATED)
        .with("count", count)
        .text(language);
    let all: Vec<&str> = numbers.iter().map(|it| it.raw.as_str()).collect();
    let all = all.join("\n");
    section_head(ui, &counted, |ui| {
        let name = i18n::translate(language, words::GENERATED_ALL, &[]);
        CopyValue::new(&all, &name, language)
            .label(keys::tools::COPY_ALL)
            .show(ui);
    });
    let names: Vec<String> = (1..=numbers.len()).map(|at| at.to_string()).collect();
    card(ui, "checks-numbers", |ui| {
        let rows: Vec<_> = numbers
            .iter()
            .zip(&names)
            .map(|(number, name)| ResultRow::new(name, &number.printed).raw(&number.raw))
            .collect();
        result_rows(ui, &rows, language);
    });
}

fn refusal(answer: &NumbersAnswer, language: Language) -> String {
    use keys::tools::checks::generate_problems as words;
    match answer {
        NumbersAnswer::NotDigits { affix, at } => {
            let key = match affix {
                Affix::Prefix => words::PREFIX,
                Affix::Suffix => words::SUFFIX,
            };
            Message::new(key).with("at", *at).text(language)
        }
        NumbersAnswer::TooLong { digits } => Message::new(words::TOO_LONG)
            .with("count", *digits)
            .text(language),
        NumbersAnswer::Impossible => i18n::translate(language, words::IMPOSSIBLE, &[]),
        NumbersAnswer::Drawn(_) => String::new(),
    }
}

impl ChecksTool {
    fn intro(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::checks as words;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(words::VALIDATE_TITLE));
        hint(ui, &tr(words::INTRO));
        let options = [
            (None, tr(words::kinds::AUTO)),
            (Some(CheckKind::Luhn), tr(words::kinds::LUHN)),
            (Some(CheckKind::Iban), tr(words::kinds::IBAN)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        Segmented::new(&options, "checks-kind").show(ui, &mut self.kind);
        let name = tr(words::INPUT_LABEL);
        ui.scope(|ui| {
            ui.set_max_width(INPUT.min(ui.available_width()));
            TextField::new(&mut self.text, &name, "checks-input")
                .hint(&tr(words::PLACEHOLDER))
                .show(ui);
        });
        crate::tools::ui::notice(ui, &tr(words::PRIVATE), "checks-private");
    }

    fn validate(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        self.intro(ui, language);
        self.answer_of(ui, language);
        hint(
            ui,
            &i18n::translate(language, keys::tools::checks::FOOTNOTE, &[]),
        );
    }

    fn numbers_request(&self) -> Option<Asked> {
        let count = self.count.trim().parse().ok()?;
        let request = match self.drawn_kind {
            CheckKind::Luhn => NumbersRequest::Luhn {
                prefix: self.prefix.clone(),
                suffix: self.suffix.clone(),
                length: self.length.trim().parse().ok()?,
                count,
            },
            CheckKind::Iban => NumbersRequest::Iban {
                country: self.country,
                count,
            },
        };
        Some((request, self.draw))
    }

    fn generate(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::checks as words;
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(words::GENERATE_TITLE));
        hint(ui, &tr(words::GENERATE_INTRO));
        let options = [
            (CheckKind::Luhn, tr(words::kinds::LUHN)),
            (CheckKind::Iban, tr(words::kinds::IBAN)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        Segmented::new(&options, "checks-generate-kind").show(ui, &mut self.drawn_kind);
        match self.drawn_kind {
            CheckKind::Luhn => self.affixes(ui, language),
            CheckKind::Iban => self.countries(ui, language),
        }
        line(ui, |ui| {
            if self.drawn_kind == CheckKind::Luhn {
                let name = tr(words::NUMBER_LENGTH);
                label(ui, &name);
                number_field(ui, &mut self.length, &name, "checks-length");
            }
            let name = tr(words::COUNT);
            label(ui, &name);
            number_field(ui, &mut self.count, &name, "checks-count");
            if Button::new(&tr(words::AGAIN), "checks-generate")
                .kind(Kind::Accent)
                .show(ui)
                .clicked()
            {
                self.draw += 1;
            }
        });
        self.drawn_numbers(ui, language);
        hint(ui, &tr(words::GENERATED_NOTE));
    }

    fn affixes(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::checks as words;
        let tr = |key| i18n::translate(language, key, &[]);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap::INSET;
            for (value, name, id) in [
                (&mut self.prefix, tr(words::PREFIX), "checks-prefix"),
                (&mut self.suffix, tr(words::SUFFIX), "checks-suffix"),
            ] {
                ui.vertical(|ui| {
                    ui.set_width(AFFIX);
                    label(ui, &name);
                    TextField::new(value, &name, id).compact().show(ui);
                });
            }
        });
    }

    fn countries(&mut self, ui: &mut Ui, language: Language) {
        let countries = api::tools::iban_countries();
        let names: Vec<(Country, String)> = countries
            .into_iter()
            .map(|it| {
                let code = it.as_str();
                let name = i18n::format::country(code, language).unwrap_or(code);
                (it, format!("{name} ({code})"))
            })
            .collect();
        let options: Vec<(Country, &str)> =
            names.iter().map(|(id, it)| (*id, it.as_str())).collect();
        let name = i18n::translate(language, keys::tools::checks::COUNTRY, &[]);
        line(ui, |ui| {
            label(ui, &name);
            ChoiceMenu::new(&name, "checks-country", &options)
                .language(language)
                .show(ui, &mut self.country);
        });
    }

    fn drawn_numbers(&self, ui: &mut Ui, language: Language) {
        match self.numbers.value() {
            Some(NumbersAnswer::Drawn(numbers)) => number_list(ui, numbers, language),
            Some(refused) => {
                let said = refusal(refused, language);
                problem(ui, &said, "checks-generate-problem");
            }
            None => {}
        }
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
        let request = self.numbers_request();
        self.numbers.update(turn, request, |backend, (asked, _)| {
            api::tools::draw_numbers(backend, asked)
        });
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::validate, Self::generate));
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
        self.prefix.zeroize();
        self.suffix.zeroize();
        self.numbers = Live::default();
    }
}
