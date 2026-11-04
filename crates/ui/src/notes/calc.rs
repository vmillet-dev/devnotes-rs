//! What a Calcul result reads on screen: the core sends exact values and codes, never words,
//! and they are written here in the language on screen. The sheet's own settings — decimals, a
//! locale of its own, the date's style — arrive with the preferences; until then a result is
//! written as their defaults write it: four significant figures, the language's numbers, dates
//! in figures.

use engine::notes::calc::units::UnitId;
use engine::notes::calc::{CalcUnit, CalcValue, SpanPart};
use i18n::Language;

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
        CalcValue::Date { date } => exact(i18n::format::date_numeric(*date, language)),
        CalcValue::Span { parts } => exact(span(parts, language)),
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
            exact(format!("{sign}{prefix}{digits}"))
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
            exact(format!("{sign}{numerator}{below}"))
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

fn exact(text: String) -> Written {
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
fn span(parts: &[SpanPart], language: Language) -> String {
    parts
        .iter()
        .map(|part| {
            let count = number(&part.value, (0, 0), false, language).text;
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
