//! A quantity of data in every unit: bits, the decimal multiples of the byte (1000) and the
//! binary ones (1024), in exact decimal arithmetic.

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::numbers::{self, NumberProblem};
use crate::count::saturating_u32;

pub const MAX_DECIMALS: u32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SizeUnit {
    Bit,
    Kilobit,
    Megabit,
    Gigabit,
    Terabit,
    Byte,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    Petabyte,
    Kibibyte,
    Mebibyte,
    Gibibyte,
    Tebibyte,
    Pebibyte,
}

const ALL: [SizeUnit; 16] = [
    SizeUnit::Byte,
    SizeUnit::Bit,
    SizeUnit::Kilobit,
    SizeUnit::Megabit,
    SizeUnit::Gigabit,
    SizeUnit::Terabit,
    SizeUnit::Kilobyte,
    SizeUnit::Megabyte,
    SizeUnit::Gigabyte,
    SizeUnit::Terabyte,
    SizeUnit::Petabyte,
    SizeUnit::Kibibyte,
    SizeUnit::Mebibyte,
    SizeUnit::Gibibyte,
    SizeUnit::Tebibyte,
    SizeUnit::Pebibyte,
];

const DECIMAL: [SizeUnit; 5] = [
    SizeUnit::Kilobyte,
    SizeUnit::Megabyte,
    SizeUnit::Gigabyte,
    SizeUnit::Terabyte,
    SizeUnit::Petabyte,
];

const BINARY: [SizeUnit; 5] = [
    SizeUnit::Kibibyte,
    SizeUnit::Mebibyte,
    SizeUnit::Gibibyte,
    SizeUnit::Tebibyte,
    SizeUnit::Pebibyte,
];

impl SizeUnit {
    pub(crate) fn bits(self) -> Decimal {
        let thousand = |power: u32| Decimal::from(1000u64.pow(power));
        let kibi = |power: u32| Decimal::from(1u64 << (10 * power));
        match self {
            Self::Bit => Decimal::ONE,
            Self::Kilobit => thousand(1),
            Self::Megabit => thousand(2),
            Self::Gigabit => thousand(3),
            Self::Terabit => thousand(4),
            Self::Byte => Decimal::from(8),
            Self::Kilobyte => thousand(1) * Decimal::from(8),
            Self::Megabyte => thousand(2) * Decimal::from(8),
            Self::Gigabyte => thousand(3) * Decimal::from(8),
            Self::Terabyte => thousand(4) * Decimal::from(8),
            Self::Petabyte => thousand(5) * Decimal::from(8),
            Self::Kibibyte => kibi(1) * Decimal::from(8),
            Self::Mebibyte => kibi(2) * Decimal::from(8),
            Self::Gibibyte => kibi(3) * Decimal::from(8),
            Self::Tebibyte => kibi(4) * Decimal::from(8),
            Self::Pebibyte => kibi(5) * Decimal::from(8),
        }
    }

    /// The symbols a person types, in English and in French. An all-lowercase `gb` is taken as
    /// bytes, as it is meant nine times in ten; `Gb`, `Gbit` are bits.
    fn read(symbol: &str) -> Option<Self> {
        let unit = match symbol {
            "b" | "bit" | "bits" => Self::Bit,
            "Kb" | "kbit" | "Kbit" | "kbits" | "Kbits" => Self::Kilobit,
            "Mb" | "Mbit" | "mbit" | "Mbits" => Self::Megabit,
            "Gb" | "Gbit" | "gbit" | "Gbits" => Self::Gigabit,
            "Tb" | "Tbit" | "tbit" | "Tbits" => Self::Terabit,
            "B" | "o" | "byte" | "bytes" | "octet" | "octets" => Self::Byte,
            "kB" | "KB" | "kb" | "ko" | "Ko" => Self::Kilobyte,
            "MB" | "mb" | "Mo" | "mo" => Self::Megabyte,
            "GB" | "gb" | "Go" | "go" => Self::Gigabyte,
            "TB" | "tb" | "To" | "to" => Self::Terabyte,
            "PB" | "pb" | "Po" | "po" => Self::Petabyte,
            "KiB" | "kiB" | "Kio" | "kio" => Self::Kibibyte,
            "MiB" | "Mio" | "mio" => Self::Mebibyte,
            "GiB" | "Gio" | "gio" => Self::Gibibyte,
            "TiB" | "Tio" | "tio" => Self::Tebibyte,
            "PiB" | "Pio" | "pio" => Self::Pebibyte,
            _ => return None,
        };
        Some(unit)
    }
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SizesRequest {
    /// A number, with its unit or without one.
    pub text: String,
    /// The unit of a number typed without one.
    pub unit: SizeUnit,
    /// How many decimals the rounded values keep; the exact ones keep all.
    pub decimals: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SizeRow {
    pub unit: SizeUnit,
    pub exact: String,
    pub rounded: String,
}

/// What one decimal unit is in the binary unit of the same rank: `1 GB = 0.931 GiB`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SizeGap {
    pub decimal: SizeUnit,
    pub binary: SizeUnit,
    pub exact: String,
    pub rounded: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SizesAnswer {
    Converted {
        unit: SizeUnit,
        /// The unit was written in the text, rather than chosen beside it.
        unit_in_text: bool,
        rows: Vec<SizeRow>,
        gap: SizeGap,
    },
    /// One-based, in characters.
    Unreadable {
        at: u32,
    },
    Negative {
        at: u32,
    },
    /// Past the 28 digits exact arithmetic holds.
    TooLarge,
}

fn written(value: Decimal) -> String {
    value.normalize().to_string()
}

fn rounded(value: Decimal, decimals: u32) -> String {
    written(value.round_dp_with_strategy(decimals, RoundingStrategy::MidpointAwayFromZero))
}

/// The rank whose decimal unit the quantity reaches, from kilo to peta.
fn rank(unit: SizeUnit, bytes: Decimal) -> usize {
    if let Some(rank) = DECIMAL.iter().position(|decimal| *decimal == unit) {
        return rank;
    }
    if let Some(rank) = BINARY.iter().position(|binary| *binary == unit) {
        return rank;
    }
    DECIMAL
        .iter()
        .rposition(|decimal| bytes >= decimal.bits() / Decimal::from(8))
        .unwrap_or(0)
}

/// Why a quantity cannot be read; the places are one-based, in characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuantityProblem {
    Unreadable(u32),
    Negative(u32),
    TooLarge,
}

/// A quantity of data as typed: its bits, the unit it was read in, and whether the text named it.
pub(crate) fn read_quantity(
    text: &str,
    unit: SizeUnit,
) -> Result<(Decimal, SizeUnit, bool), QuantityProblem> {
    let (value, end) = numbers::leading_number(text).map_err(|problem| match problem {
        NumberProblem::Unreadable(at) => QuantityProblem::Unreadable(saturating_u32(at) + 1),
        NumberProblem::TooLarge => QuantityProblem::TooLarge,
    })?;
    if value.is_sign_negative() && !value.is_zero() {
        let at = text.chars().position(|c| c == '-').unwrap_or(0);
        return Err(QuantityProblem::Negative(saturating_u32(at) + 1));
    }

    let rest: String = text.chars().skip(end).collect();
    let symbol = rest.trim();
    let (unit, unit_in_text) = if symbol.is_empty() {
        (unit, false)
    } else {
        let at = end + rest.chars().take_while(|c| c.is_whitespace()).count();
        let unit =
            SizeUnit::read(symbol).ok_or(QuantityProblem::Unreadable(saturating_u32(at) + 1))?;
        (unit, true)
    };
    let bits = value
        .checked_mul(unit.bits())
        .ok_or(QuantityProblem::TooLarge)?;
    Ok((bits, unit, unit_in_text))
}

pub fn convert(request: &SizesRequest) -> SizesAnswer {
    let decimals = request.decimals.min(MAX_DECIMALS);
    let (bits, unit, unit_in_text) = match read_quantity(&request.text, request.unit) {
        Ok(read) => read,
        Err(QuantityProblem::Unreadable(at)) => return SizesAnswer::Unreadable { at },
        Err(QuantityProblem::Negative(at)) => return SizesAnswer::Negative { at },
        Err(QuantityProblem::TooLarge) => return SizesAnswer::TooLarge,
    };
    let rows: Option<Vec<SizeRow>> = ALL
        .iter()
        .map(|row| {
            bits.checked_div(row.bits()).map(|value| SizeRow {
                unit: *row,
                exact: written(value),
                rounded: rounded(value, decimals),
            })
        })
        .collect();
    let Some(rows) = rows else {
        return SizesAnswer::TooLarge;
    };

    let rank = rank(unit, bits / Decimal::from(8));
    let ratio = DECIMAL[rank].bits() / BINARY[rank].bits();
    SizesAnswer::Converted {
        unit,
        unit_in_text,
        rows,
        gap: SizeGap {
            decimal: DECIMAL[rank],
            binary: BINARY[rank],
            exact: written(ratio),
            rounded: rounded(ratio, decimals),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(text: &str, unit: SizeUnit) -> SizesAnswer {
        convert(&SizesRequest {
            text: text.to_owned(),
            unit,
            decimals: 3,
        })
    }

    fn value(answer: &SizesAnswer, unit: SizeUnit) -> (String, String) {
        let SizesAnswer::Converted { rows, .. } = answer else {
            panic!("not converted: {answer:?}");
        };
        let row = rows.iter().find(|row| row.unit == unit).unwrap();
        (row.exact.clone(), row.rounded.clone())
    }

    fn exact(text: &str, unit: SizeUnit) -> String {
        value(&ask(text, SizeUnit::Byte), unit).0
    }

    #[test]
    fn a_gigabyte_is_a_billion_bytes_not_nearly() {
        let answer = ask("1 GB", SizeUnit::Byte);

        assert_eq!(value(&answer, SizeUnit::Byte).0, "1000000000");
        assert_eq!(value(&answer, SizeUnit::Bit).0, "8000000000");
        assert_eq!(value(&answer, SizeUnit::Megabyte).0, "1000");
        assert_eq!(
            value(&answer, SizeUnit::Gibibyte).0,
            "0.931322574615478515625"
        );
        assert_eq!(value(&answer, SizeUnit::Gibibyte).1, "0.931");
        assert_eq!(value(&answer, SizeUnit::Gigabit).0, "8");
        let SizesAnswer::Converted {
            unit,
            unit_in_text,
            gap,
            ..
        } = answer
        else {
            panic!("converted");
        };
        assert_eq!((unit, unit_in_text), (SizeUnit::Gigabyte, true));
        assert_eq!(
            gap,
            SizeGap {
                decimal: SizeUnit::Gigabyte,
                binary: SizeUnit::Gibibyte,
                exact: "0.931322574615478515625".into(),
                rounded: "0.931".into(),
            }
        );
    }

    #[test]
    fn each_unit_reaches_every_other_and_comes_back() {
        for from in ALL {
            let answer = ask("3", from);
            for to in ALL {
                let there = value(&answer, to).0;
                let back = value(
                    &convert(&SizesRequest {
                        text: there.clone(),
                        unit: to,
                        decimals: 3,
                    }),
                    from,
                )
                .0;
                let back = back.parse::<Decimal>().unwrap();
                let error = (back - Decimal::from(3)).abs();
                assert!(
                    error < Decimal::new(1, 12),
                    "3 {from:?} → {there} {to:?} → {back}"
                );
            }
        }
    }

    #[test]
    fn english_and_french_symbols_are_read() {
        for (text, unit) in [
            ("1.5 GB", SizeUnit::Gigabyte),
            ("1,5 Go", SizeUnit::Gigabyte),
            ("750 KiB", SizeUnit::Kibibyte),
            ("750 Kio", SizeUnit::Kibibyte),
            ("100 Mbit", SizeUnit::Megabit),
            ("100 Mb", SizeUnit::Megabit),
            ("100 mb", SizeUnit::Megabyte),
            ("2 To", SizeUnit::Terabyte),
            ("12 octets", SizeUnit::Byte),
            ("64 bits", SizeUnit::Bit),
            ("3 PiB", SizeUnit::Pebibyte),
        ] {
            let SizesAnswer::Converted { unit: read, .. } = ask(text, SizeUnit::Byte) else {
                panic!("{text} was not read");
            };
            assert_eq!(read, unit, "{text}");
        }
        assert_eq!(exact("1,5 Go", SizeUnit::Byte), "1500000000");
        assert_eq!(exact("100 Mbit", SizeUnit::Megabyte), "12.5");
    }

    #[test]
    fn a_number_alone_is_in_the_unit_chosen() {
        let answer = ask("2048", SizeUnit::Kibibyte);

        assert_eq!(value(&answer, SizeUnit::Mebibyte).0, "2");
        assert!(matches!(
            answer,
            SizesAnswer::Converted {
                unit_in_text: false,
                ..
            }
        ));
    }

    #[test]
    fn decimals_floats_get_wrong_stay_exact() {
        assert_eq!(exact("0.1 kB", SizeUnit::Byte), "100");
        assert_eq!(exact("0.3 MB", SizeUnit::Kilobyte), "300");
        assert_eq!(exact("1 bit", SizeUnit::Byte), "0.125");
        assert_eq!(exact("1 KiB", SizeUnit::Kilobyte), "1.024");
    }

    #[test]
    fn the_largest_values_go_past_a_u64_of_bytes() {
        let answer = ask("20000 PB", SizeUnit::Byte);

        assert_eq!(value(&answer, SizeUnit::Byte).0, "20000000000000000000");
        assert_eq!(value(&answer, SizeUnit::Bit).0, "160000000000000000000");
        assert_eq!(ask("1e30 PiB", SizeUnit::Byte), SizesAnswer::TooLarge);
        assert_eq!(ask(&"9".repeat(40), SizeUnit::Byte), SizesAnswer::TooLarge);
    }

    #[test]
    fn the_rounded_value_keeps_the_decimals_asked() {
        let rows = |decimals| {
            let answer = convert(&SizesRequest {
                text: "1 GB".into(),
                unit: SizeUnit::Byte,
                decimals,
            });
            value(&answer, SizeUnit::Gibibyte).1
        };

        assert_eq!(rows(0), "1");
        assert_eq!(rows(6), "0.931323");
        assert_eq!(rows(40), "0.931322574615");
    }

    #[test]
    fn the_gap_is_said_at_the_rank_the_quantity_reaches() {
        let gap = |text: &str| match ask(text, SizeUnit::Byte) {
            SizesAnswer::Converted { gap, .. } => (gap.decimal, gap.binary),
            other => panic!("{other:?}"),
        };

        assert_eq!(gap("750 KiB"), (SizeUnit::Kilobyte, SizeUnit::Kibibyte));
        assert_eq!(gap("3 TB"), (SizeUnit::Terabyte, SizeUnit::Tebibyte));
        assert_eq!(gap("5000000 B"), (SizeUnit::Megabyte, SizeUnit::Mebibyte));
        assert_eq!(gap("100 Mbit"), (SizeUnit::Megabyte, SizeUnit::Mebibyte));
        assert_eq!(gap("12 B"), (SizeUnit::Kilobyte, SizeUnit::Kibibyte));
    }

    #[test]
    fn a_negative_or_unreadable_value_says_where() {
        assert_eq!(
            ask("-5 MB", SizeUnit::Byte),
            SizesAnswer::Negative { at: 1 }
        );
        assert_eq!(
            ask("5 MX", SizeUnit::Byte),
            SizesAnswer::Unreadable { at: 3 }
        );
        assert_eq!(
            ask("  lots", SizeUnit::Byte),
            SizesAnswer::Unreadable { at: 3 }
        );
        assert!(matches!(
            ask("-0", SizeUnit::Byte),
            SizesAnswer::Converted { .. }
        ));
    }
}
