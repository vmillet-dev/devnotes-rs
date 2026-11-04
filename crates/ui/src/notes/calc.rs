//! What a Calcul result reads on screen: the core sends exact values and codes, never words,
//! and they are written here in the language on screen. The sheet's own settings — decimals, a
//! locale of its own, the date's style — arrive with the preferences; until then a result is
//! written as their defaults write it: four significant figures, the language's numbers, dates
//! in figures.

use std::sync::LazyLock;

use engine::notes::calc::units::{Dimension, UnitEntry, UnitId};
use engine::notes::calc::words::{CalcWord, WordClass, WordEntry};
use engine::notes::calc::{CalcError, CalcUnit, CalcValue, SpanPart};
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

/// `places`: what the line asked for (`à 5 chiffres`), over the default.
pub fn value(value: &CalcValue, places: Option<u8>, language: Language) -> Written {
    match value {
        CalcValue::Number { value, unit } => {
            let money = unit.is_some_and(|unit| unit.per.is_none() && is_money(unit.unit));
            let shown = number(value, digits(value, money, places), false, language);
            match unit {
                None => shown,
                Some(unit) => Written {
                    text: format!("{}{NBSP}{}", shown.text, self::unit(*unit, language)),
                    approx: shown.approx,
                },
            }
        }
        CalcValue::Percent { value, signed } => {
            let shown = number(value, digits(value, false, places), *signed, language);
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
        CalcValue::Date { date } => exact_text(i18n::format::date_numeric(*date, language)),
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
            let shown = number(mantissa, digits(mantissa, false, None), false, language);
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
            let shown = number(value, digits(value, false, None), false, language);
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
        other => self::value(other, None, language).text,
    }
}

/// Said on its line in place of a result, a capital first.
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

fn number(value: &str, (min, max): (usize, usize), signed: bool, language: Language) -> Written {
    Written {
        text: i18n::format::exact(value, (min, max), signed, true, language),
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
mod tests {
    use super::*;

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
            let written = value(&input, None, language);
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
