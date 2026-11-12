//! What the footer says of the lines chosen: their sum and their mean, exact, in the unit they
//! share. Only numbers add up; the words around the figures are the front's.

use std::str::FromStr;

use rust_decimal::Decimal;

use super::{CalcLine, CalcSheet, CalcUnit, CalcValue, text};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalcSummary {
    Figures {
        sum: CalcValue,
        mean: CalcValue,
    },
    /// None of the lines is a number.
    NothingToAdd,
    /// The numbers are not all in one unit; no unit at all is one.
    MixedUnits,
    /// The sum is past what a decimal holds, as a line that overflows.
    Overflow,
}

/// `lines` are zero-based; one that gave no number is left out.
pub fn summary(sheet: &CalcSheet, lines: &[u32]) -> CalcSummary {
    let numbers: Vec<(&str, Option<CalcUnit>)> = lines
        .iter()
        .filter_map(|line| sheet.rows.get(usize::try_from(*line).ok()?))
        .filter_map(|row| match &row.line {
            CalcLine::Value {
                value: CalcValue::Number { value, unit },
                ..
            } => Some((value.as_str(), *unit)),
            _ => None,
        })
        .collect();
    let Some(&(_, unit)) = numbers.first() else {
        return CalcSummary::NothingToAdd;
    };
    if numbers.iter().any(|(_, other)| *other != unit) {
        return CalcSummary::MixedUnits;
    }
    let values: Vec<&str> = numbers.iter().map(|(value, _)| *value).collect();
    let Some((sum, mean)) = figures(&values) else {
        return CalcSummary::Overflow;
    };
    CalcSummary::Figures {
        sum: CalcValue::Number { value: sum, unit },
        mean: CalcValue::Number { value: mean, unit },
    }
}

/// On the decimals themselves: `0.1 + 0.2` is `0.3`.
fn figures(values: &[&str]) -> Option<(String, String)> {
    let mut sum = Decimal::ZERO;
    for value in values {
        sum = sum.checked_add(Decimal::from_str(value).ok()?)?;
    }
    let mean = sum.checked_div(Decimal::from(values.len()))?;
    Some((text(sum), text(mean)))
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::super::evaluate;
    use super::*;

    fn sheet(text: &str) -> CalcSheet {
        evaluate(text, NaiveDate::from_ymd_opt(2026, 10, 16).unwrap())
    }

    fn sum_and_mean(values: &[&str]) -> (String, String) {
        figures(values).unwrap()
    }

    /// What the reference's footer gave, on the same strings (`golden/calc-format`).
    #[test]
    fn the_footer_gives_what_the_reference_gave() {
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../../../../../golden/calc-format/cases.json"))
                .unwrap();
        let mut checked = 0;
        for case in &cases {
            let Some(footer) = case["options"]["footer"].as_str() else {
                continue;
            };
            let input: Vec<&str> = case["input"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect();
            let (sum, mean) = sum_and_mean(&input);
            let got = if footer == "sum" { sum } else { mean };
            assert_eq!(
                got,
                case["output"].as_str().unwrap(),
                "{footer} of {input:?}"
            );
            checked += 1;
        }
        assert_eq!(checked, 6);
    }

    #[test]
    fn the_decimals_add_up_without_a_float_in_between() {
        assert_eq!(sum_and_mean(&["0.1", "0.2"]).0, "0.3");
        assert_eq!(sum_and_mean(&["100.521", "4.99", "26"]).0, "131.511");
        assert_eq!(sum_and_mean(&["-1.5", "0.5"]).0, "-1");
        assert_eq!(sum_and_mean(&["1", "2"]).1, "1.5");
        assert!(sum_and_mean(&["1", "1", "2"]).1.starts_with("1.333333"));
    }

    #[test]
    fn numbers_in_one_unit_are_summed_in_it() {
        let sheet = sheet("100,521 €\n4,99 €\n# Titre");

        let CalcSummary::Figures { sum, mean } = summary(&sheet, &[0, 1, 2]) else {
            panic!("figures");
        };
        let CalcValue::Number { value, unit } = sum else {
            panic!("a number");
        };
        assert_eq!(value, "105.511");
        assert!(unit.is_some());
        assert!(matches!(mean, CalcValue::Number { value, .. } if value == "52.7555"));
    }

    #[test]
    fn a_line_with_no_number_is_left_out_and_with_none_nothing_adds_up() {
        let sheet = sheet("# Titre\n12\n25 %\n01/02/2026 + 1 jour");

        assert_eq!(summary(&sheet, &[0]), CalcSummary::NothingToAdd);
        assert_eq!(summary(&sheet, &[0, 2]), CalcSummary::NothingToAdd);
        assert_eq!(summary(&sheet, &[]), CalcSummary::NothingToAdd);
        assert_eq!(summary(&sheet, &[99]), CalcSummary::NothingToAdd);
        let CalcSummary::Figures { sum, .. } = summary(&sheet, &[0, 1, 2]) else {
            panic!("figures");
        };
        assert!(matches!(sum, CalcValue::Number { value, .. } if value == "12"));
    }

    #[test]
    fn different_units_do_not_add_up_nor_a_unit_and_none() {
        let sheet = sheet("3 €\n4 Go\n5");

        assert_eq!(summary(&sheet, &[0, 1]), CalcSummary::MixedUnits);
        assert_eq!(summary(&sheet, &[0, 2]), CalcSummary::MixedUnits);
    }

    #[test]
    fn a_sum_past_what_a_decimal_holds_says_so() {
        let huge = "70000000000000000000000000000";
        let sheet = sheet(&format!("{huge}\n{huge}"));

        assert_eq!(summary(&sheet, &[0, 1]), CalcSummary::Overflow);
    }
}
