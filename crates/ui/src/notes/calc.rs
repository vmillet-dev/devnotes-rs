//! What a Calcul result reads on screen: the core sends exact values and codes, never words,
//! and they are written here in the language on screen. The sheet's own settings — decimals, a
//! locale of its own, the date's style — arrive with the preferences; until then a result is
//! written as their defaults write it: four significant figures, the language's numbers, dates
//! in figures.

use std::sync::LazyLock;

use crate::services::settings::{CalcDate, Settings};
use engine::notes::calc::units::{Dimension, UnitEntry, UnitId};
use engine::notes::calc::words::{CalcWord, WordClass, WordEntry};
use engine::notes::calc::{CalcError, CalcLine, CalcSheet, CalcUnit, CalcValue, SpanPart};
use i18n::Language;

use crate::api;

/// Typed while writing, a shorter start proposes too much: `de` is no `demain`.
const PROPOSED_FROM: usize = 3;
/// What a plain number can be written as after `en`, beside `%`.
const NOTATIONS: [CalcWord; 7] = [
    CalcWord::Fraction,
    CalcWord::Multiple,
    CalcWord::AsDecimal,
    CalcWord::Hex,
    CalcWord::Bin,
    CalcWord::Oct,
    CalcWord::Sci,
];

static WORDS: LazyLock<Vec<WordEntry>> = LazyLock::new(api::notes::calc_words);
static UNITS: LazyLock<Vec<UnitEntry>> = LazyLock::new(api::notes::calc_units);

/// A result, and whether writing it rounded it: `≈ 100,52 €`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub text: String,
    pub approx: bool,
}

const NBSP: char = '\u{a0}';
const NARROW_NBSP: char = '\u{202f}';

/// A locale a sheet can write its numbers in, beside the language on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberFormat {
    pub id: &'static str,
    decimal: char,
    group: char,
}

const fn locale(id: &'static str, decimal: char, group: char) -> NumberFormat {
    NumberFormat { id, decimal, group }
}

/// The reference's `CALC_NUMBER_FORMATS`, in its order.
pub const NUMBER_FORMATS: [NumberFormat; 14] = [
    locale("fr-FR", ',', NARROW_NBSP),
    locale("fr-CH", ',', NARROW_NBSP),
    locale("en-US", '.', ','),
    locale("en-GB", '.', ','),
    locale("de-DE", ',', '.'),
    locale("es-ES", ',', '.'),
    locale("it-IT", ',', '.'),
    locale("pt-BR", ',', '.'),
    locale("nl-NL", ',', '.'),
    locale("pl-PL", ',', NBSP),
    locale("sv-SE", ',', NBSP),
    locale("ru-RU", ',', NBSP),
    locale("ja-JP", '.', ','),
    locale("zh-CN", '.', ','),
];

/// How a sheet is written, read from the settings: never how it is computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    pub language: Language,
    /// Over the default, under what a line asks for itself.
    pub decimals: Option<u8>,
    pub numbers: Option<NumberFormat>,
    pub dates: CalcDate,
}

impl Format {
    pub fn of(settings: &Settings, language: Language) -> Self {
        Self {
            language,
            decimals: settings.calc_decimals,
            numbers: NUMBER_FORMATS
                .into_iter()
                .find(|format| format.id == settings.calc_number_format),
            dates: settings.calc_date_format,
        }
    }

    /// The language on screen and nothing of its own.
    pub fn plain(language: Language) -> Self {
        Self {
            language,
            decimals: None,
            numbers: None,
            dates: CalcDate::Numeric,
        }
    }
}

/// What `id` makes of `value`, shown beside its name in the preferences.
pub fn sample(value: &str, id: &str, language: Language) -> String {
    let format = Format {
        numbers: NUMBER_FORMATS.into_iter().find(|format| format.id == id),
        ..Format::plain(language)
    };
    number(value, (2, 2), false, format).text
}

/// `places`: what the line asked for (`à 5 chiffres`), over the default.
pub fn value(value: &CalcValue, places: Option<u8>, format: Format) -> Written {
    let language = format.language;
    let places = places.or(format.decimals);
    match value {
        CalcValue::Number { value, unit } => {
            let money = unit.is_some_and(|unit| unit.per.is_none() && is_money(unit.unit));
            let shown = number(value, digits(value, money, places), false, format);
            match unit {
                None => shown,
                Some(unit) => Written {
                    text: format!("{}{NBSP}{}", shown.text, self::unit(*unit, language)),
                    approx: shown.approx,
                },
            }
        }
        CalcValue::Percent { value, signed } => {
            let shown = number(value, digits(value, false, places), *signed, format);
            let text = i18n::translate(
                language,
                i18n::keys::calc::PERCENT,
                &[("value", shown.text.into())],
            );
            Written {
                text,
                approx: shown.approx,
            }
        }
        CalcValue::Date { date } => exact_text(match format.dates {
            CalcDate::Numeric => i18n::format::date_numeric(*date, language),
            CalcDate::Short => i18n::format::date_short(*date, language),
            CalcDate::Long => i18n::format::date_long(*date, language),
        }),
        CalcValue::Span { parts } => exact_text(span(parts, true, language)),
        CalcValue::Base {
            digits,
            radix,
            negative,
        } => {
            let prefix = match radix {
                2 => "0b",
                8 => "0o",
                16 => "0x",
                _ => "",
            };
            let sign = if *negative { "-" } else { "" };
            exact_text(format!("{sign}{prefix}{digits}"))
        }
        CalcValue::Scientific { mantissa, exponent } => {
            let shown = number(mantissa, digits(mantissa, false, None), false, format);
            let sign = if *exponent < 0 { '-' } else { '+' };
            Written {
                text: format!("{}e{sign}{}", shown.text, exponent.unsigned_abs()),
                approx: shown.approx,
            }
        }
        CalcValue::Fraction {
            numerator,
            denominator,
            negative,
        } => {
            let sign = if *negative { "-" } else { "" };
            let below = if denominator == "1" {
                String::new()
            } else {
                format!("/{denominator}")
            };
            exact_text(format!("{sign}{numerator}{below}"))
        }
        CalcValue::Multiplier { value } => {
            let shown = number(value, digits(value, false, None), false, format);
            Written {
                text: format!("×{}", shown.text),
                approx: shown.approx,
            }
        }
    }
}

/// Every decimal kept, as the copy of one result and a name's tooltip give it: its thousands never
/// grouped, since a sheet reads `1.578` and `1,578` as decimals.
pub fn exact(value: &CalcValue, language: Language) -> String {
    let every = |figures: &str, signed: bool| {
        let digits = figures
            .split_once('.')
            .map_or(0, |(_, fraction)| fraction.len());
        i18n::format::exact(figures, (0, digits.min(20)), signed, false, language)
    };
    match value {
        CalcValue::Scientific { mantissa, exponent } => {
            let sign = if *exponent < 0 { '-' } else { '+' };
            format!(
                "{}e{sign}{}",
                every(mantissa, false),
                exponent.unsigned_abs()
            )
        }
        CalcValue::Multiplier { value } => format!("×{}", every(value, false)),
        CalcValue::Span { parts } => span(parts, false, language),
        CalcValue::Percent { value, signed } => i18n::translate(
            language,
            i18n::keys::calc::PERCENT,
            &[("value", every(value, *signed).into())],
        ),
        CalcValue::Number { value, unit } => match unit {
            None => every(value, false),
            Some(unit) => format!(
                "{}{NBSP}{}",
                every(value, false),
                self::unit(*unit, language)
            ),
        },
        other => self::value(other, None, Format::plain(language)).text,
    }
}

/// Said on its line in place of a result, a capital first.
/// The text with each result after it as a comment: pasted into another sheet, the comments are
/// read past and the same lines compute again.
pub fn copy_text(text: &str, sheet: &CalcSheet, format: Format) -> String {
    let lines = text.split('\n').enumerate().map(|(index, line)| {
        let Some(CalcLine::Value { value, places, .. }) =
            sheet.rows.get(index).map(|row| &row.line)
        else {
            return line.to_owned();
        };
        let shown = self::value(value, *places, format);
        let approx = if shown.approx { "≈ " } else { "" };
        let written = line.split("//").next().unwrap_or(line).trim_end();
        format!("{written} // {approx}{}", shown.text)
    });
    lines.collect::<Vec<_>>().join("\n")
}

pub fn error(error: &CalcError, language: Language) -> String {
    let dimension = |dimension: Dimension| {
        let name = name_of(dimension);
        i18n::Key::find(&format!("calc.dimensions.{name}"))
            .map_or(name, |key| i18n::translate(language, key, &[]))
    };
    let tr = |name: &str, params: &[(&str, i18n::Value)]| {
        i18n::Key::find(&format!("calc.errors.{name}"))
            .map_or_else(String::new, |key| i18n::translate(language, key, params))
    };
    let text = match error {
        CalcError::Unreadable { .. } => tr("unreadable", &[]),
        CalcError::UnknownName { name } => tr("unknownName", &[("name", name.as_str().into())]),
        CalcError::Reserved { word } => tr("reserved", &[("word", word.as_str().into())]),
        CalcError::Mismatch {
            expected,
            found,
            unit: None,
        } => tr(
            "mismatch",
            &[
                ("found", dimension(*found).into()),
                ("expected", dimension(*expected).into()),
            ],
        ),
        CalcError::Mismatch {
            expected,
            unit: Some(found),
            ..
        } => tr(
            "mismatchUnit",
            &[
                ("unit", unit(*found, language).into()),
                ("expected", dimension(*expected).into()),
            ],
        ),
        CalcError::NotConvertible { from, to } => {
            let to = to.map_or_else(|| "%".to_owned(), |to| unit(to, language));
            tr(
                "notConvertible",
                &[("from", dimension(*from).into()), ("to", to.into())],
            )
        }
        other => {
            let kind = serde_json::to_value(other).ok();
            let kind = kind.as_ref().and_then(|it| it["kind"].as_str());
            tr(kind.unwrap_or("unsupported"), &[])
        }
    };
    capitalised(&text)
}

fn name_of(dimension: Dimension) -> String {
    let name = serde_json::to_value(dimension).ok();
    let name = name.as_ref().and_then(serde_json::Value::as_str);
    name.unwrap_or_default().to_owned()
}

fn capitalised(text: &str) -> String {
    let mut characters = text.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect()
    })
}

fn spellings(entry: &WordEntry, language: Language) -> &[&'static str] {
    match language {
        Language::Fr => &entry.fr,
        Language::En => &entry.en,
    }
}

/// The word being typed, which the table's words complete.
pub fn typed_word(before: &str) -> &str {
    let start = before
        .char_indices()
        .rev()
        .take_while(|(_, it)| it.is_alphabetic() || matches!(it, '\'' | '’'))
        .last()
        .map_or(before.len(), |(at, _)| at);
    &before[start..]
}

/// The words the start typed may become, in the spelling of the language on screen: the values
/// of the sheet and the phrases of several words, one spelling a concept.
pub fn complete(typed: &str, language: Language) -> Vec<String> {
    let start = typed.to_lowercase().replace('’', "'");
    if start.chars().count() < PROPOSED_FROM {
        return Vec::new();
    }
    let mut found: Vec<String> = Vec::new();
    for entry in WORDS.iter() {
        let fits: Vec<&&str> = spellings(entry, language)
            .iter()
            .filter(|spelling| {
                (entry.class == WordClass::Value || spelling.contains(' '))
                    && spelling.starts_with(&start)
            })
            .collect();
        let whole = fits.iter().any(|spelling| ***spelling == *start);
        if let (Some(first), false) = (fits.first(), whole)
            && !found.iter().any(|word| word == **first)
        {
            found.push((**first).to_owned());
        }
    }
    found
}

/// The start of a unit after a conversion word, in either language: a sheet reads both.
pub fn conversion_start(before: &str) -> Option<&str> {
    let typed = before
        .rsplit(char::is_whitespace)
        .next()
        .unwrap_or_default();
    let head = before[..before.len() - typed.len()].to_lowercase();
    if head.trim_end().len() == head.len() {
        return None;
    }
    let head = head.trim_end();
    let spelled = WORDS
        .iter()
        .filter(|entry| matches!(entry.word, CalcWord::Convert | CalcWord::At))
        .flat_map(|entry| entry.fr.iter().chain(&entry.en))
        .any(|spelling| {
            head.strip_suffix(spelling)
                .is_some_and(|rest| rest.is_empty() || rest.ends_with(char::is_whitespace))
        });
    spelled.then_some(typed)
}

/// The units of a kind that start as typed; after a plain number, `%` and the notations too.
pub fn units_for(dimension: Dimension, typed: &str, language: Language) -> Vec<String> {
    let mut units: Vec<String> = UNITS
        .iter()
        .filter(|entry| entry.dimension == dimension)
        .map(|entry| symbol(entry.id, language))
        .collect();
    if dimension == Dimension::Number {
        units.push("%".to_owned());
        units.extend(NOTATIONS.iter().filter_map(|word| {
            let entry = WORDS.iter().find(|entry| entry.word == *word)?;
            spellings(entry, language)
                .first()
                .map(|it| (*it).to_owned())
        }));
    }
    let start = typed.to_lowercase();
    units.retain(|unit| unit.to_lowercase().starts_with(&start));
    units
}

fn exact_text(text: String) -> Written {
    Written {
        text,
        approx: false,
    }
}

fn unit(unit: CalcUnit, language: Language) -> String {
    match unit.per {
        None => symbol(unit.unit, language),
        Some(per) => format!("{}/{}", symbol(unit.unit, language), symbol(per, language)),
    }
}

/// `Go` or `GB`: the translation of the unit's name.
fn symbol(unit: UnitId, language: Language) -> String {
    let name = serde_json::to_value(unit)
        .ok()
        .and_then(|name| name.as_str().map(str::to_string))
        .unwrap_or_default();
    i18n::Key::find(&format!("calc.units.{name}"))
        .map_or(name, |key| i18n::translate(language, key, &[]))
}

/// `6 sem 3 j`: whole parts, each with its unit.
fn span(parts: &[SpanPart], grouped: bool, language: Language) -> String {
    parts
        .iter()
        .map(|part| {
            let count = i18n::format::exact(&part.value, (0, 0), false, grouped, language);
            format!("{count}{NBSP}{}", symbol(part.unit, language))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn number(value: &str, (min, max): (usize, usize), signed: bool, format: Format) -> Written {
    let text = match format.numbers {
        None => i18n::format::exact(value, (min, max), signed, true, format.language),
        Some(locale) => i18n::format::exact(value, (min, max), signed, true, Language::En)
            .chars()
            .map(|written| match written {
                '.' => locale.decimal,
                ',' => locale.group,
                other => other,
            })
            .collect(),
    };
    Written {
        text,
        approx: fraction(value).len() > max,
    }
}

/// The decimals of `-12.50`, its trailing zeros dropped: `5`.
fn fraction(value: &str) -> &str {
    value
        .split_once('.')
        .map_or("", |(_, fraction)| fraction.trim_end_matches('0'))
}

/// At least and at most how many decimals: what the line asked for, else two for money and
/// four significant figures, two decimals at least — but a small value keeps its first figures,
/// 0,045 €/h is not 0,05.
fn digits(value: &str, money: bool, places: Option<u8>) -> (usize, usize) {
    if let Some(places) = places {
        let places = usize::from(places.min(20));
        return (places, places);
    }
    if money {
        return (2, 2);
    }
    let whole = value
        .trim_start_matches('-')
        .split('.')
        .next()
        .unwrap_or("");
    let fraction = fraction(value);
    if whole.bytes().all(|digit| digit == b'0') && !fraction.is_empty() {
        let first = fraction.find(|digit| digit != '0').unwrap_or(0);
        return (0, (first + 4).min(8));
    }
    let figures = whole.trim_start_matches('0').len();
    (0, 4usize.saturating_sub(figures).max(2))
}

fn is_money(unit: UnitId) -> bool {
    matches!(unit, UnitId::Euro | UnitId::Dollar)
}

#[cfg(test)]
mod format_tests {
    use super::*;

    fn written(format: Format) -> String {
        let value = CalcValue::Number {
            value: "1578.13".to_owned(),
            unit: None,
        };
        value_of(&value, format)
    }

    fn value_of(value: &CalcValue, format: Format) -> String {
        self::value(value, None, format).text
    }

    #[test]
    fn a_locale_writes_its_own_separators() {
        let german = Format {
            numbers: Some(NUMBER_FORMATS[4]),
            ..Format::plain(Language::En)
        };

        assert_eq!(written(german), "1.578,13");
    }

    #[test]
    fn the_decimals_chosen_stand_under_what_a_line_asks() {
        let three = Format {
            decimals: Some(3),
            ..Format::plain(Language::En)
        };
        let value = CalcValue::Number {
            value: "2.5".to_owned(),
            unit: None,
        };

        assert_eq!(self::value(&value, None, three).text, "2.500");
        assert_eq!(self::value(&value, Some(1), three).text, "2.5");
    }

    #[test]
    fn a_date_follows_the_style_chosen() {
        let date = CalcValue::Date {
            date: chrono::NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
        };
        let long = Format {
            dates: CalcDate::Long,
            ..Format::plain(Language::En)
        };

        assert_ne!(
            value_of(&date, long),
            value_of(&date, Format::plain(Language::En))
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn computed(text: &str) -> CalcSheet {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
        engine::notes::calc::evaluate(text, today)
    }

    #[test]
    fn a_copy_writes_each_result_as_a_comment_in_place_of_the_old_one() {
        let text = "# Budget\nttc = 100,521 € // ancien\nune phrase\n2 + 2";

        let copied = copy_text(text, &computed(text), Format::plain(Language::Fr));

        assert_eq!(
            copied.replace(NBSP, " "),
            "# Budget\nttc = 100,521 € // ≈ 100,52 €\nune phrase\n2 + 2 // 4"
        );
    }

    #[test]
    fn a_copy_reads_back_as_the_same_sheet() {
        let text = "a = 100,521 €\nb = 4,99 €\ntotal\na × 12";
        let sheet = computed(text);

        let copied = copy_text(text, &sheet, Format::plain(Language::Fr));

        let lines = |sheet: CalcSheet| {
            let rows = sheet.rows.into_iter();
            rows.map(|row| row.line).collect::<Vec<_>>()
        };
        assert_eq!(lines(computed(&copied)), lines(sheet));
    }

    #[test]
    fn a_value_is_given_whole_with_no_thousands_grouped() {
        let money = CalcValue::Number {
            value: "1234.56789".to_owned(),
            unit: Some(CalcUnit {
                unit: UnitId::Euro,
                per: None,
            }),
        };

        assert_eq!(exact(&money, Language::En), format!("1234.56789{NBSP}€"));
        assert_eq!(exact(&money, Language::Fr), format!("1234,56789{NBSP}€"));
        let times = CalcValue::Multiplier {
            value: "1.5".to_owned(),
        };
        assert_eq!(exact(&times, Language::En), "×1.5");
    }

    #[test]
    fn an_error_is_said_with_a_capital_and_its_words() {
        let unknown = CalcError::UnknownName {
            name: "prix".to_owned(),
        };
        assert!(error(&unknown, Language::En).contains("prix"));
        let text = error(&CalcError::DivisionByZero, Language::En);
        assert!(
            text.chars().next().is_some_and(char::is_uppercase),
            "{text}"
        );
    }

    #[test]
    fn the_word_typed_is_the_letters_before_the_caret() {
        assert_eq!(typed_word("3 + aujourd’h"), "aujourd’h");
        assert_eq!(typed_word("3 + "), "");
    }

    #[test]
    fn three_letters_propose_a_word_and_a_word_whole_proposes_nothing() {
        assert!(complete("to", Language::En).is_empty());
        assert!(complete("tod", Language::En).contains(&"today".to_owned()));
        assert!(complete("today", Language::En).is_empty());
    }

    #[test]
    fn after_a_conversion_word_the_units_of_the_kind_are_proposed() {
        assert_eq!(conversion_start("3 km en m"), Some("m"));
        assert_eq!(conversion_start("3 km to "), Some(""));
        assert_eq!(conversion_start("3 km"), None);

        let lengths = units_for(Dimension::Length, "k", Language::En);
        assert_eq!(lengths, ["km"]);
        let plain = units_for(Dimension::Number, "", Language::En);
        assert!(plain.contains(&"%".to_owned()));
        assert!(plain.contains(&"hex".to_owned()));
    }

    /// What the reference's formatter wrote, at the settings' defaults (`golden/calc-format`).
    #[test]
    fn values_are_written_as_the_reference_wrote_them() {
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../../../../golden/calc-format/cases.json"))
                .unwrap();
        let mut checked = 0;
        for case in cases {
            let options = &case["options"];
            if options["locale"] != serde_json::Value::Null
                || options["decimals"] != "auto"
                || options["date"] != "numeric"
            {
                continue;
            }
            let language = if options["lang"] == "fr" {
                Language::Fr
            } else {
                Language::En
            };
            let input: CalcValue = serde_json::from_value(case["input"].clone()).unwrap();
            let written = value(&input, None, Format::plain(language));
            assert_eq!(
                written.text, case["output"]["text"],
                "{input:?} in {language:?}"
            );
            assert_eq!(written.approx, case["output"]["approx"], "{input:?}");
            checked += 1;
        }
        assert_eq!(checked, 20);
    }
}
