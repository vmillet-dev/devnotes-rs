//! How long a quantity of data takes over a link: at the rate given, slowed by what protocols,
//! encryption and latency take, and at the rates of a few common connections. Exact decimals.

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::numbers::{self, NumberProblem};
use super::sizes::{self, QuantityProblem, SizeUnit};
use crate::count::saturating_u32;

pub const EFFICIENCY_PERCENT: (u32, u32) = (50, 100);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum RateUnit {
    BitPerSecond,
    KilobitPerSecond,
    MegabitPerSecond,
    GigabitPerSecond,
    BytePerSecond,
    KilobytePerSecond,
    MegabytePerSecond,
    GigabytePerSecond,
}

impl RateUnit {
    fn bits(self) -> Decimal {
        let thousand = |power: u32| Decimal::from(1000u64.pow(power));
        match self {
            Self::BitPerSecond => Decimal::ONE,
            Self::KilobitPerSecond => thousand(1),
            Self::MegabitPerSecond => thousand(2),
            Self::GigabitPerSecond => thousand(3),
            Self::BytePerSecond => Decimal::from(8),
            Self::KilobytePerSecond => thousand(1) * Decimal::from(8),
            Self::MegabytePerSecond => thousand(2) * Decimal::from(8),
            Self::GigabytePerSecond => thousand(3) * Decimal::from(8),
        }
    }
}

/// The connections every estimate is also given for, by their rate in bits per second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Connection {
    Fibre,
    Mobile4g,
    Adsl,
}

const CONNECTIONS: [(Connection, u64); 3] = [
    (Connection::Fibre, 1_000_000_000),
    (Connection::Mobile4g, 30_000_000),
    (Connection::Adsl, 10_000_000),
];

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransferRequest {
    /// A quantity, with its unit or without one.
    pub size: String,
    /// The unit of a quantity typed without one.
    pub size_unit: SizeUnit,
    pub rate: String,
    pub rate_unit: RateUnit,
    /// The share of the rate a link really carries, in percent; held between 50 and 100.
    pub efficiency: u32,
}

/// A length of time to the nearest second, for the front to spell. Past a year it stays in days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransferTime {
    pub days: u32,
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
    /// More than nothing, but less than half a second.
    pub under_a_second: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionEstimate {
    /// `None` for the rate typed.
    pub connection: Option<Connection>,
    /// In bits per second, exact.
    pub rate: String,
    pub span: TransferTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum TransferField {
    Size,
    Rate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TransferAnswer {
    Estimated {
        /// At the rate typed, slowed by the efficiency.
        estimate: TransferTime,
        /// At the full rate.
        theoretical: TransferTime,
        /// The efficiency used, once held within its bounds.
        efficiency: u32,
        /// Fastest first, the rate typed among the common ones.
        connections: Vec<ConnectionEstimate>,
    },
    /// One-based, in characters.
    Unreadable {
        field: TransferField,
        at: u32,
    },
    Negative {
        field: TransferField,
        at: u32,
    },
    /// A rate of nothing carries nothing, ever.
    ZeroRate,
    /// Past the 28 digits exact arithmetic holds.
    TooLarge,
}

fn span(seconds: Decimal) -> TransferTime {
    let rounded = seconds.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
    let whole = rounded.to_u64().unwrap_or(u64::MAX);
    TransferTime {
        days: saturating_u32(whole / 86_400),
        hours: saturating_u32(whole % 86_400 / 3600),
        minutes: saturating_u32(whole % 3600 / 60),
        seconds: saturating_u32(whole % 60),
        under_a_second: whole == 0 && !seconds.is_zero(),
    }
}

/// The rate as typed, its unit written after it or chosen beside it.
fn rate_bits(text: &str, unit: RateUnit) -> Result<Decimal, TransferAnswer> {
    let field = TransferField::Rate;
    let (value, end) = numbers::leading_number(text).map_err(|problem| match problem {
        NumberProblem::Unreadable(at) => TransferAnswer::Unreadable {
            field,
            at: saturating_u32(at) + 1,
        },
        NumberProblem::TooLarge => TransferAnswer::TooLarge,
    })?;
    if value.is_sign_negative() && !value.is_zero() {
        let at = text.chars().position(|c| c == '-').unwrap_or(0);
        return Err(TransferAnswer::Negative {
            field,
            at: saturating_u32(at) + 1,
        });
    }
    let rest: String = text.chars().skip(end).collect();
    if !rest.trim().is_empty() {
        let at = end + rest.chars().take_while(|c| c.is_whitespace()).count();
        return Err(TransferAnswer::Unreadable {
            field,
            at: saturating_u32(at) + 1,
        });
    }
    if value.is_zero() {
        return Err(TransferAnswer::ZeroRate);
    }
    value
        .checked_mul(unit.bits())
        .ok_or(TransferAnswer::TooLarge)
}

pub fn estimate(request: &TransferRequest) -> TransferAnswer {
    let bits = match sizes::read_quantity(&request.size, request.size_unit) {
        Ok((bits, ..)) => bits,
        Err(QuantityProblem::Unreadable(at)) => {
            return TransferAnswer::Unreadable {
                field: TransferField::Size,
                at,
            };
        }
        Err(QuantityProblem::Negative(at)) => {
            return TransferAnswer::Negative {
                field: TransferField::Size,
                at,
            };
        }
        Err(QuantityProblem::TooLarge) => return TransferAnswer::TooLarge,
    };
    let rate = match rate_bits(&request.rate, request.rate_unit) {
        Ok(rate) => rate,
        Err(refused) => return refused,
    };
    let efficiency = request
        .efficiency
        .clamp(EFFICIENCY_PERCENT.0, EFFICIENCY_PERCENT.1);
    let share = Decimal::from(efficiency) / Decimal::ONE_HUNDRED;

    let time_at = |rate: Decimal| bits.checked_div(rate * share);
    let Some(theoretical) = bits.checked_div(rate) else {
        return TransferAnswer::TooLarge;
    };
    let mut connections: Vec<(Decimal, Option<Connection>)> = CONNECTIONS
        .iter()
        .map(|(connection, rate)| (Decimal::from(*rate), Some(*connection)))
        .chain([(rate, None)])
        .collect();
    // Stable: the rate typed comes after a common one it equals.
    connections.sort_by_key(|(rate, _)| std::cmp::Reverse(*rate));
    let connections: Option<Vec<ConnectionEstimate>> = connections
        .into_iter()
        .map(|(rate, connection)| {
            time_at(rate).map(|seconds| ConnectionEstimate {
                connection,
                rate: rate.normalize().to_string(),
                span: span(seconds),
            })
        })
        .collect();
    match (time_at(rate), connections) {
        (Some(seconds), Some(connections)) => TransferAnswer::Estimated {
            estimate: span(seconds),
            theoretical: span(theoretical),
            efficiency,
            connections,
        },
        _ => TransferAnswer::TooLarge,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(size: &str, rate: &str, rate_unit: RateUnit, efficiency: u32) -> TransferAnswer {
        estimate(&TransferRequest {
            size: size.to_owned(),
            size_unit: SizeUnit::Gigabyte,
            rate: rate.to_owned(),
            rate_unit,
            efficiency,
        })
    }

    fn spans(answer: &TransferAnswer) -> (TransferTime, TransferTime, &[ConnectionEstimate]) {
        match answer {
            TransferAnswer::Estimated {
                estimate,
                theoretical,
                connections,
                ..
            } => (*estimate, *theoretical, connections),
            other => panic!("{other:?}"),
        }
    }

    fn of(days: u32, hours: u32, minutes: u32, seconds: u32) -> TransferTime {
        TransferTime {
            days,
            hours,
            minutes,
            seconds,
            under_a_second: false,
        }
    }

    #[test]
    fn a_dvd_over_a_hundred_megabits_at_ninety_percent() {
        let answer = ask("4,7", "100", RateUnit::MegabitPerSecond, 90);
        let (estimate, theoretical, connections) = spans(&answer);

        assert_eq!(estimate, of(0, 0, 6, 58));
        assert_eq!(theoretical, of(0, 0, 6, 16));
        assert_eq!(
            connections
                .iter()
                .map(|estimate| (estimate.connection, estimate.span))
                .collect::<Vec<_>>(),
            [
                (Some(Connection::Fibre), of(0, 0, 0, 42)),
                (None, of(0, 0, 6, 58)),
                (Some(Connection::Mobile4g), of(0, 0, 23, 13)),
                (Some(Connection::Adsl), of(0, 1, 9, 38)),
            ]
        );
    }

    #[test]
    fn a_rate_in_bytes_is_eight_times_one_in_bits() {
        let in_bytes = spans(&ask("4.7", "12.5", RateUnit::MegabytePerSecond, 100)).0;
        let in_bits = spans(&ask("4.7", "100", RateUnit::MegabitPerSecond, 100)).0;

        assert_eq!(in_bytes, in_bits);
        assert_eq!(in_bits, of(0, 0, 6, 16));
    }

    #[test]
    fn the_efficiency_is_held_between_half_and_all_of_the_rate() {
        let TransferAnswer::Estimated { efficiency, .. } =
            ask("1", "1", RateUnit::GigabitPerSecond, 10)
        else {
            panic!()
        };
        assert_eq!(efficiency, 50);
        let TransferAnswer::Estimated {
            efficiency,
            estimate,
            theoretical,
            ..
        } = ask("1", "1", RateUnit::GigabitPerSecond, 140)
        else {
            panic!()
        };
        assert_eq!(efficiency, 100);
        assert_eq!(estimate, theoretical);
    }

    #[test]
    fn a_rate_of_nothing_is_said_and_a_negative_one_located() {
        assert_eq!(
            ask("1", "0", RateUnit::MegabitPerSecond, 90),
            TransferAnswer::ZeroRate
        );
        assert_eq!(
            ask("1", " -5", RateUnit::MegabitPerSecond, 90),
            TransferAnswer::Negative {
                field: TransferField::Rate,
                at: 2
            }
        );
        assert_eq!(
            ask("-1", "5", RateUnit::MegabitPerSecond, 90),
            TransferAnswer::Negative {
                field: TransferField::Size,
                at: 1
            }
        );
        assert_eq!(
            ask("1", "5 Mbit/s", RateUnit::MegabitPerSecond, 90),
            TransferAnswer::Unreadable {
                field: TransferField::Rate,
                at: 3
            }
        );
        assert_eq!(
            ask("", "5", RateUnit::MegabitPerSecond, 90),
            TransferAnswer::Unreadable {
                field: TransferField::Size,
                at: 1
            }
        );
    }

    #[test]
    fn a_huge_size_stays_in_days_and_a_tiny_one_is_under_a_second() {
        let (estimate, ..) = spans(&ask("1 PB", "1", RateUnit::KilobitPerSecond, 100));
        assert_eq!(estimate.days, 92_592_592);
        assert_eq!(estimate.hours, 14);

        let (estimate, ..) = spans(&ask("1 o", "1", RateUnit::GigabitPerSecond, 100));
        assert!(estimate.under_a_second);
        assert_eq!(
            estimate,
            TransferTime {
                under_a_second: true,
                ..of(0, 0, 0, 0)
            }
        );
        assert_eq!(
            ask(&"9".repeat(40), "1", RateUnit::BitPerSecond, 100),
            TransferAnswer::TooLarge
        );
    }
}
