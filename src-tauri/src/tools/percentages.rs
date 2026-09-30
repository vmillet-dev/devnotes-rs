//! The usual questions about a percentage, each on its own two numbers, in exact decimal
//! arithmetic: `0.1 + 0.2` is `0.3` here.

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::numbers::{self, NumberProblem};
use crate::count::saturating_u32;

pub const MAX_DECIMALS: u32 = 12;

/// The two numbers of one question, as typed.
#[derive(Debug, Clone, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PercentPair {
    pub x: String,
    pub y: String,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PercentagesRequest {
    /// x % of y.
    pub of: PercentPair,
    /// x is what percentage of y.
    pub share: PercentPair,
    /// From x to y, by what percentage.
    pub change: PercentPair,
    /// y raised, or lowered, by x %.
    pub apply: PercentPair,
    pub lower: bool,
    /// What y was before it moved by x %.
    pub before: PercentPair,
    pub decimals: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Operand {
    X,
    Y,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PercentResult {
    /// A field is empty: nothing to answer yet.
    Empty,
    Answered {
        /// The numbers as read, for the formula written beside the result.
        x: String,
        y: String,
        exact: String,
        rounded: String,
    },
    /// One-based, in characters of the field.
    Unreadable {
        field: Operand,
        at: u32,
    },
    /// A division by zero: a share of 0, or a percentage that brings a value to nothing.
    DivisionByZero,
    /// An evolution from 0 is no percentage.
    FromZero,
    TooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PercentagesAnswer {
    pub of: PercentResult,
    pub share: PercentResult,
    pub change: PercentResult,
    pub apply: PercentResult,
    pub before: PercentResult,
}

enum Failure {
    DivisionByZero,
    FromZero,
    TooLarge,
}

fn read(text: &str, field: Operand) -> Result<Option<Decimal>, PercentResult> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    match numbers::number(text) {
        Ok(value) => Ok(Some(value)),
        Err(NumberProblem::Unreadable(at)) => Err(PercentResult::Unreadable {
            field,
            at: saturating_u32(at) + 1,
        }),
        Err(NumberProblem::TooLarge) => Err(PercentResult::TooLarge),
    }
}

fn answer(
    pair: &PercentPair,
    decimals: u32,
    compute: impl FnOnce(Decimal, Decimal) -> Result<Decimal, Failure>,
) -> PercentResult {
    let (x, y) = match (read(&pair.x, Operand::X), read(&pair.y, Operand::Y)) {
        (Err(problem), _) | (_, Err(problem)) => return problem,
        (Ok(Some(x)), Ok(Some(y))) => (x, y),
        _ => return PercentResult::Empty,
    };
    match compute(x, y) {
        Ok(value) => PercentResult::Answered {
            x: x.to_string(),
            y: y.to_string(),
            exact: value.normalize().to_string(),
            rounded: value
                .round_dp_with_strategy(decimals, RoundingStrategy::MidpointAwayFromZero)
                .normalize()
                .to_string(),
        },
        Err(Failure::DivisionByZero) => PercentResult::DivisionByZero,
        Err(Failure::FromZero) => PercentResult::FromZero,
        Err(Failure::TooLarge) => PercentResult::TooLarge,
    }
}

fn hundred() -> Decimal {
    Decimal::ONE_HUNDRED
}

fn multiply(a: Decimal, b: Decimal) -> Result<Decimal, Failure> {
    a.checked_mul(b).ok_or(Failure::TooLarge)
}

fn divide(a: Decimal, b: Decimal) -> Result<Decimal, Failure> {
    if b.is_zero() {
        return Err(Failure::DivisionByZero);
    }
    a.checked_div(b).ok_or(Failure::TooLarge)
}

fn factor(x: Decimal, lower: bool) -> Result<Decimal, Failure> {
    let rate = divide(x, hundred())?;
    if lower {
        Decimal::ONE.checked_sub(rate)
    } else {
        Decimal::ONE.checked_add(rate)
    }
    .ok_or(Failure::TooLarge)
}

pub fn answer_all(request: &PercentagesRequest) -> PercentagesAnswer {
    let decimals = request.decimals.min(MAX_DECIMALS);
    PercentagesAnswer {
        of: answer(&request.of, decimals, |x, y| {
            multiply(divide(x, hundred())?, y)
        }),
        share: answer(&request.share, decimals, |x, y| {
            multiply(divide(x, y)?, hundred())
        }),
        change: answer(&request.change, decimals, |x, y| {
            if x.is_zero() {
                return Err(Failure::FromZero);
            }
            let moved = y.checked_sub(x).ok_or(Failure::TooLarge)?;
            multiply(divide(moved, x.abs())?, hundred())
        }),
        apply: answer(&request.apply, decimals, |x, y| {
            multiply(y, factor(x, request.lower)?)
        }),
        before: answer(&request.before, decimals, |x, y| {
            divide(y, factor(x, false)?)
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(x: &str, y: &str) -> PercentPair {
        PercentPair {
            x: x.to_owned(),
            y: y.to_owned(),
        }
    }

    fn request() -> PercentagesRequest {
        PercentagesRequest {
            of: PercentPair::default(),
            share: PercentPair::default(),
            change: PercentPair::default(),
            apply: PercentPair::default(),
            lower: false,
            before: PercentPair::default(),
            decimals: 2,
        }
    }

    fn exact(result: &PercentResult) -> String {
        match result {
            PercentResult::Answered { exact, .. } => exact.clone(),
            other => panic!("not answered: {other:?}"),
        }
    }

    fn rounded(result: &PercentResult) -> String {
        match result {
            PercentResult::Answered { rounded, .. } => rounded.clone(),
            other => panic!("not answered: {other:?}"),
        }
    }

    #[test]
    fn each_question_of_the_ticket_answers_its_example() {
        let answer = answer_all(&PercentagesRequest {
            of: pair("15", "240"),
            share: pair("36", "240"),
            change: pair("80", "100"),
            apply: pair("15", "240"),
            before: pair("15", "276"),
            ..request()
        });

        assert_eq!(exact(&answer.of), "36");
        assert_eq!(exact(&answer.share), "15");
        assert_eq!(exact(&answer.change), "25");
        assert_eq!(exact(&answer.apply), "276");
        assert_eq!(exact(&answer.before), "240");
        assert_eq!(
            answer.of,
            PercentResult::Answered {
                x: "15".into(),
                y: "240".into(),
                exact: "36".into(),
                rounded: "36".into(),
            }
        );
    }

    #[test]
    fn lowered_rather_than_raised() {
        let answer = answer_all(&PercentagesRequest {
            apply: pair("15", "240"),
            lower: true,
            ..request()
        });

        assert_eq!(exact(&answer.apply), "204");
    }

    #[test]
    fn negatives_are_a_fall() {
        let answer = answer_all(&PercentagesRequest {
            change: pair("100", "80"),
            before: pair("-20", "80"),
            of: pair("-10", "50"),
            ..request()
        });

        assert_eq!(exact(&answer.change), "-20");
        assert_eq!(exact(&answer.before), "100");
        assert_eq!(exact(&answer.of), "-5");
        let from_below = answer_all(&PercentagesRequest {
            change: pair("-50", "-25"),
            ..request()
        });
        assert_eq!(
            exact(&from_below.change),
            "50",
            "a rise, measured on the size of the start"
        );
    }

    #[test]
    fn zero_is_said_rather_than_infinite() {
        let answer = answer_all(&PercentagesRequest {
            share: pair("36", "0"),
            change: pair("0", "100"),
            before: pair("-100", "50"),
            of: pair("0", "240"),
            ..request()
        });

        assert_eq!(answer.share, PercentResult::DivisionByZero);
        assert_eq!(answer.change, PercentResult::FromZero);
        assert_eq!(answer.before, PercentResult::DivisionByZero);
        assert_eq!(exact(&answer.of), "0");
    }

    #[test]
    fn a_comma_or_a_point_whatever_the_language() {
        let answer = answer_all(&PercentagesRequest {
            of: pair("12,5", "80"),
            share: pair("1 250,5", "2501"),
            ..request()
        });

        assert_eq!(exact(&answer.of), "10");
        assert_eq!(exact(&answer.share), "50");
    }

    #[test]
    fn results_floats_get_wrong_are_exact() {
        let answer = answer_all(&PercentagesRequest {
            of: pair("1.1", "100"),
            share: pair("0.3", "0.1"),
            apply: pair("10", "0.1"),
            ..request()
        });

        assert_eq!(
            exact(&answer.of),
            "1.1",
            "1.1 / 100 * 100 is 1.1000000000000001 in a float"
        );
        assert_eq!(exact(&answer.share), "300");
        assert_eq!(exact(&answer.apply), "0.11");
    }

    #[test]
    fn a_result_is_rounded_to_the_decimals_asked_and_kept_exact() {
        let answer = answer_all(&PercentagesRequest {
            share: pair("1", "3"),
            decimals: 3,
            ..request()
        });

        assert_eq!(rounded(&answer.share), "33.333");
        assert!(exact(&answer.share).starts_with("33.33333333333333333"));
        let half = answer_all(&PercentagesRequest {
            of: pair("12.5", "1"),
            decimals: 1,
            ..request()
        });
        assert_eq!(rounded(&half.of), "0.1", "0.125 rounds half away from zero");
    }

    #[test]
    fn large_and_very_small_values() {
        let answer = answer_all(&PercentagesRequest {
            of: pair("50", "1000000000000000000000000"),
            share: pair("0.000000001", "1000"),
            apply: pair("1e30", "1e30"),
            decimals: 12,
            ..request()
        });

        assert_eq!(exact(&answer.of), "500000000000000000000000");
        assert_eq!(exact(&answer.share), "0.0000000001");
        assert_eq!(answer.apply, PercentResult::TooLarge);
    }

    #[test]
    fn an_empty_field_asks_nothing_and_an_unreadable_one_says_where() {
        let answer = answer_all(&PercentagesRequest {
            of: pair("15", ""),
            share: pair("abc", "240"),
            change: pair("80", "10 0x"),
            ..request()
        });

        assert_eq!(answer.of, PercentResult::Empty);
        assert_eq!(
            answer.share,
            PercentResult::Unreadable {
                field: Operand::X,
                at: 1
            }
        );
        assert_eq!(
            answer.change,
            PercentResult::Unreadable {
                field: Operand::Y,
                at: 5
            }
        );
        assert_eq!(answer.apply, PercentResult::Empty);
    }
}
