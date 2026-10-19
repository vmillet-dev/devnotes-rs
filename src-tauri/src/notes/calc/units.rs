//! What a quantity is measured in: lengths (screen ones included), masses, durations,
//! frequencies, data sizes, temperatures, and euros and dollars, which are units like the others
//! and are never converted into each other.

use rust_decimal::Decimal;
use serde::Serialize;
use specta::Type;

use crate::tools::sizes::SizeUnit;

/// How many base units of each kind a quantity carries: `€/h` is one euro over one second.
/// Temperature stands apart: a degree is not a factor of a kelvin, so it never compounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Dim(pub [i8; BASES]);

pub const BASES: usize = 7;
const LENGTH: usize = 0;
const MASS: usize = 1;
const TIME: usize = 2;
const DATA: usize = 3;
const TEMPERATURE: usize = 4;
const EURO: usize = 5;
const DOLLAR: usize = 6;

impl Dim {
    pub const NONE: Self = Self([0; BASES]);

    const fn base(index: usize) -> Self {
        let mut exponents = [0; BASES];
        exponents[index] = 1;
        Self(exponents)
    }

    pub const LENGTH: Self = Self::base(LENGTH);
    pub const MASS: Self = Self::base(MASS);
    pub const TIME: Self = Self::base(TIME);
    pub const DATA: Self = Self::base(DATA);
    pub const TEMPERATURE: Self = Self::base(TEMPERATURE);
    pub const EURO: Self = Self::base(EURO);
    pub const DOLLAR: Self = Self::base(DOLLAR);
    /// Once a second: a hertz is a second to the power of minus one.
    pub const FREQUENCY: Self = Self::NONE.over(Self::TIME);

    #[must_use]
    pub fn times(self, other: Self) -> Self {
        let mut exponents = self.0;
        for (exponent, added) in exponents.iter_mut().zip(other.0) {
            *exponent = exponent.saturating_add(added);
        }
        Self(exponents)
    }

    #[must_use]
    pub const fn over(self, other: Self) -> Self {
        let mut exponents = self.0;
        let mut index = 0;
        while index < BASES {
            exponents[index] = exponents[index].saturating_sub(other.0[index]);
            index += 1;
        }
        Self(exponents)
    }

    pub fn is_none(self) -> bool {
        self == Self::NONE
    }

    pub fn has_temperature(self) -> bool {
        self.0[TEMPERATURE] != 0
    }

    pub fn is_currency(self) -> bool {
        self == Self::EURO || self == Self::DOLLAR
    }

    /// What it is called in a message: a compound is a rate, whatever it divides.
    pub fn dimension(self) -> Dimension {
        match self {
            d if d.is_none() => Dimension::Number,
            d if d == Self::LENGTH => Dimension::Length,
            d if d == Self::MASS => Dimension::Mass,
            d if d == Self::TIME => Dimension::Time,
            d if d == Self::DATA => Dimension::Data,
            d if d == Self::TEMPERATURE => Dimension::Temperature,
            d if d == Self::EURO => Dimension::Euro,
            d if d == Self::DOLLAR => Dimension::Dollar,
            d if d == Self::FREQUENCY => Dimension::Frequency,
            _ => Dimension::Rate,
        }
    }

    /// One base over another, the only compounds a sheet names: `€/h`, `Mo/s`, `km/h`.
    pub fn as_rate(self) -> Option<(Self, Self)> {
        let mut above = None;
        let mut below = None;
        for (index, exponent) in self.0.iter().enumerate() {
            match exponent {
                0 => {}
                1 if above.is_none() => above = Some(Self::base(index)),
                -1 if below.is_none() => below = Some(Self::base(index)),
                _ => return None,
            }
        }
        above.zip(below)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Dimension {
    Number,
    Length,
    Mass,
    Time,
    Data,
    Temperature,
    Euro,
    Dollar,
    Frequency,
    Rate,
    Date,
}

/// Named, never spelled: the front writes each in the language on screen, `Go` or `GB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum UnitId {
    Nanometre,
    Micrometre,
    Millimetre,
    Centimetre,
    Metre,
    Kilometre,
    Inch,
    Foot,
    Yard,
    Mile,
    /// A CSS pixel: an inch over the sheet's `ppi`.
    Pixel,
    /// A typographic point, the 72nd of an inch.
    Point,
    /// The sheet's `em` in pixels, and its `rem`.
    Em,
    Rem,
    Milligram,
    Gram,
    Kilogram,
    Tonne,
    Pound,
    Ounce,
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
    Bit,
    Kilobit,
    Megabit,
    Gigabit,
    Terabit,
    Petabit,
    Byte,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    Petabyte,
    Exabyte,
    Zettabyte,
    Yottabyte,
    Kibibyte,
    Mebibyte,
    Gibibyte,
    Tebibyte,
    Pebibyte,
    Exbibyte,
    Zebibyte,
    Yobibyte,
    Celsius,
    Fahrenheit,
    Kelvin,
    Hertz,
    Kilohertz,
    Megahertz,
    Gigahertz,
    Terahertz,
    Euro,
    Dollar,
}

pub struct UnitDef {
    pub id: UnitId,
    pub dim: Dim,
    /// Base units in one: a kilometre is 1000 metres. A temperature's amount is kept in
    /// degrees Celsius, converted by formula rather than by factor.
    pub factor: Decimal,
    /// What a person types, in French and in English, in the case that tells `Mo` from `mo`.
    pub symbols: &'static [&'static str],
}

/// A unit's size in base units: exact decimals, one over another, or the sizes tool's own for
/// data. A pixel's and an `em`'s follow the sheet's settings: theirs here are the defaults.
#[derive(Clone, Copy)]
enum Size {
    Exact(&'static str),
    Over(&'static str, &'static str),
    Data(SizeUnit),
}

use Size::{Data, Exact, Over};
use UnitId as U;

/// ⚠️ Lowercase `to` and `mo` are left out of the data sizes the sizes tool reads: in a sheet
/// they are the English `to` and a month.
const UNITS: &[(UnitId, Dim, Size, &[&str])] = &[
    (U::Nanometre, Dim::LENGTH, Exact("0.000000001"), &["nm"]),
    (
        U::Micrometre,
        Dim::LENGTH,
        Exact("0.000001"),
        &["µm", "μm", "um"],
    ),
    (U::Millimetre, Dim::LENGTH, Exact("0.001"), &["mm"]),
    (U::Centimetre, Dim::LENGTH, Exact("0.01"), &["cm"]),
    (
        U::Metre,
        Dim::LENGTH,
        Exact("1"),
        &["m", "mètre", "mètres", "metre", "metres", "meter", "meters"],
    ),
    (U::Kilometre, Dim::LENGTH, Exact("1000"), &["km"]),
    (
        U::Inch,
        Dim::LENGTH,
        Exact("0.0254"),
        &["in", "po", "pouce", "pouces", "inch", "inches"],
    ),
    (
        U::Foot,
        Dim::LENGTH,
        Exact("0.3048"),
        &["ft", "pied", "pieds", "foot", "feet"],
    ),
    (
        U::Yard,
        Dim::LENGTH,
        Exact("0.9144"),
        &["yd", "yard", "yards"],
    ),
    (
        U::Mile,
        Dim::LENGTH,
        Exact("1609.344"),
        &["mi", "mile", "miles"],
    ),
    (U::Pixel, Dim::LENGTH, Over("0.0254", "96"), &["px"]),
    (U::Point, Dim::LENGTH, Over("0.0254", "72"), &["pt"]),
    (U::Em, Dim::LENGTH, Over("0.4064", "96"), &["em"]),
    (U::Rem, Dim::LENGTH, Over("0.4064", "96"), &["rem"]),
    (U::Milligram, Dim::MASS, Exact("0.000001"), &["mg"]),
    (
        U::Gram,
        Dim::MASS,
        Exact("0.001"),
        &["g", "gramme", "grammes", "gram", "grams"],
    ),
    (U::Kilogram, Dim::MASS, Exact("1"), &["kg"]),
    (
        U::Tonne,
        Dim::MASS,
        Exact("1000"),
        &["t", "tonne", "tonnes"],
    ),
    (
        U::Pound,
        Dim::MASS,
        Exact("0.45359237"),
        &["lb", "lbs", "livre", "livres", "pound", "pounds"],
    ),
    (
        U::Ounce,
        Dim::MASS,
        Exact("0.028349523125"),
        &["oz", "once", "onces", "ounce", "ounces"],
    ),
    (U::Nanosecond, Dim::TIME, Exact("0.000000001"), &["ns"]),
    (
        U::Microsecond,
        Dim::TIME,
        Exact("0.000001"),
        &["µs", "μs", "us"],
    ),
    (U::Millisecond, Dim::TIME, Exact("0.001"), &["ms"]),
    (
        U::Second,
        Dim::TIME,
        Exact("1"),
        &["s", "sec", "seconde", "secondes", "second", "seconds"],
    ),
    (
        U::Minute,
        Dim::TIME,
        Exact("60"),
        &["min", "mn", "minute", "minutes"],
    ),
    (
        U::Hour,
        Dim::TIME,
        Exact("3600"),
        &["h", "heure", "heures", "hour", "hours"],
    ),
    (
        U::Day,
        Dim::TIME,
        Exact("86400"),
        &["j", "jour", "jours", "d", "day", "days"],
    ),
    (
        U::Week,
        Dim::TIME,
        Exact("604800"),
        &["sem", "semaine", "semaines", "wk", "week", "weeks"],
    ),
    // The mean month and year of the Gregorian calendar; a date moves by calendar months.
    (
        U::Month,
        Dim::TIME,
        Exact("2629746"),
        &["mois", "month", "months", "mo"],
    ),
    (
        U::Year,
        Dim::TIME,
        Exact("31556952"),
        &["an", "ans", "année", "années", "y", "yr", "year", "years"],
    ),
    (
        U::Bit,
        Dim::DATA,
        Data(SizeUnit::Bit),
        &["b", "bit", "bits"],
    ),
    (
        U::Kilobit,
        Dim::DATA,
        Data(SizeUnit::Kilobit),
        &["Kb", "kbit", "Kbit"],
    ),
    (
        U::Megabit,
        Dim::DATA,
        Data(SizeUnit::Megabit),
        &["Mb", "Mbit", "mbit"],
    ),
    (
        U::Gigabit,
        Dim::DATA,
        Data(SizeUnit::Gigabit),
        &["Gb", "Gbit", "gbit"],
    ),
    (
        U::Terabit,
        Dim::DATA,
        Data(SizeUnit::Terabit),
        &["Tb", "Tbit", "tbit"],
    ),
    (
        U::Petabit,
        Dim::DATA,
        Exact("1000000000000000"),
        &["Pb", "Pbit", "pbit"],
    ),
    (
        U::Byte,
        Dim::DATA,
        Data(SizeUnit::Byte),
        &["o", "B", "octet", "octets", "byte", "bytes"],
    ),
    (
        U::Kilobyte,
        Dim::DATA,
        Data(SizeUnit::Kilobyte),
        &["Ko", "ko", "kB", "KB"],
    ),
    (
        U::Megabyte,
        Dim::DATA,
        Data(SizeUnit::Megabyte),
        &["Mo", "MB"],
    ),
    (
        U::Gigabyte,
        Dim::DATA,
        Data(SizeUnit::Gigabyte),
        &["Go", "go", "GB", "gb"],
    ),
    (
        U::Terabyte,
        Dim::DATA,
        Data(SizeUnit::Terabyte),
        &["To", "TB", "tb"],
    ),
    (
        U::Petabyte,
        Dim::DATA,
        Data(SizeUnit::Petabyte),
        &["Po", "PB"],
    ),
    (
        U::Exabyte,
        Dim::DATA,
        Exact("8000000000000000000"),
        &["Eo", "EB"],
    ),
    (
        U::Zettabyte,
        Dim::DATA,
        Exact("8000000000000000000000"),
        &["Zo", "ZB"],
    ),
    (
        U::Yottabyte,
        Dim::DATA,
        Exact("8000000000000000000000000"),
        &["Yo", "YB"],
    ),
    (
        U::Kibibyte,
        Dim::DATA,
        Data(SizeUnit::Kibibyte),
        &["Kio", "kio", "KiB", "kiB"],
    ),
    (
        U::Mebibyte,
        Dim::DATA,
        Data(SizeUnit::Mebibyte),
        &["Mio", "mio", "MiB"],
    ),
    (
        U::Gibibyte,
        Dim::DATA,
        Data(SizeUnit::Gibibyte),
        &["Gio", "gio", "GiB"],
    ),
    (
        U::Tebibyte,
        Dim::DATA,
        Data(SizeUnit::Tebibyte),
        &["Tio", "tio", "TiB"],
    ),
    (
        U::Pebibyte,
        Dim::DATA,
        Data(SizeUnit::Pebibyte),
        &["Pio", "pio", "PiB"],
    ),
    (
        U::Exbibyte,
        Dim::DATA,
        Exact("9223372036854775808"),
        &["Eio", "EiB"],
    ),
    (
        U::Zebibyte,
        Dim::DATA,
        Exact("9444732965739290427392"),
        &["Zio", "ZiB"],
    ),
    (
        U::Yobibyte,
        Dim::DATA,
        Exact("9671406556917033397649408"),
        &["Yio", "YiB"],
    ),
    (
        U::Celsius,
        Dim::TEMPERATURE,
        Exact("1"),
        &["°C", "°c", "celsius"],
    ),
    (
        U::Fahrenheit,
        Dim::TEMPERATURE,
        Exact("1"),
        &["°F", "°f", "fahrenheit"],
    ),
    (
        U::Kelvin,
        Dim::TEMPERATURE,
        Exact("1"),
        &["K", "kelvin", "kelvins"],
    ),
    (U::Hertz, Dim::FREQUENCY, Exact("1"), &["Hz", "hz", "hertz"]),
    (U::Kilohertz, Dim::FREQUENCY, Exact("1000"), &["kHz", "khz"]),
    (U::Megahertz, Dim::FREQUENCY, Exact("1000000"), &["MHz"]),
    (
        U::Gigahertz,
        Dim::FREQUENCY,
        Exact("1000000000"),
        &["GHz", "ghz"],
    ),
    (
        U::Terahertz,
        Dim::FREQUENCY,
        Exact("1000000000000"),
        &["THz", "thz"],
    ),
    (
        U::Euro,
        Dim::EURO,
        Exact("1"),
        &["€", "euro", "euros", "EUR"],
    ),
    (
        U::Dollar,
        Dim::DOLLAR,
        Exact("1"),
        &["$", "dollar", "dollars", "USD"],
    ),
];

pub fn table() -> Vec<UnitDef> {
    UNITS
        .iter()
        .map(|&(id, dim, size, symbols)| UnitDef {
            id,
            dim,
            factor: match size {
                Exact(text) => text.parse().unwrap_or(Decimal::ONE),
                Over(above, below) => {
                    let parse = |text: &str| text.parse().unwrap_or(Decimal::ONE);
                    parse(above) / parse(below)
                }
                Data(unit) => unit.bits(),
            },
            symbols,
        })
        .collect()
}

/// A unit and what it measures: the editor proposes, after `en` or `à`, the units of the kind
/// the line holds. Their symbols are the front's, in the language on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UnitEntry {
    pub id: UnitId,
    pub dimension: Dimension,
}

pub fn catalogue() -> Vec<UnitEntry> {
    UNITS
        .iter()
        .map(|&(id, dim, _, _)| UnitEntry {
            id,
            dimension: dim.dimension(),
        })
        .collect()
}

/// A sheet's screen settings, from the line that gives them down: `ppi = 326`, `em = 20 px`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Css {
    pub ppi: Decimal,
    /// In pixels, as `rem`.
    pub em: Decimal,
    pub rem: Decimal,
}

impl Default for Css {
    fn default() -> Self {
        Self {
            ppi: Decimal::from(96),
            em: Decimal::from(16),
            rem: Decimal::from(16),
        }
    }
}

impl Css {
    /// Metres in one pixel, `em` or `rem`; `None` for a unit the settings do not size.
    pub fn factor(&self, unit: UnitId) -> Option<Decimal> {
        let pixel = Decimal::new(254, 4).checked_div(self.ppi)?;
        match unit {
            UnitId::Pixel => Some(pixel),
            UnitId::Em => self.em.checked_mul(pixel),
            UnitId::Rem => self.rem.checked_mul(pixel),
            _ => None,
        }
    }
}

/// The one a quantity of a bare dimension is shown in when nothing chose another.
pub fn base_unit(dim: Dim) -> Option<UnitId> {
    Some(match dim {
        d if d == Dim::LENGTH => UnitId::Metre,
        d if d == Dim::MASS => UnitId::Kilogram,
        d if d == Dim::TIME => UnitId::Second,
        d if d == Dim::DATA => UnitId::Byte,
        d if d == Dim::TEMPERATURE => UnitId::Celsius,
        d if d == Dim::EURO => UnitId::Euro,
        d if d == Dim::DOLLAR => UnitId::Dollar,
        d if d == Dim::FREQUENCY => UnitId::Hertz,
        _ => return None,
    })
}

const CELSIUS_TO_KELVIN: Decimal = Decimal::from_parts(27315, 0, 0, false, 2);

/// Degrees in, Celsius out: a temperature is converted by formula, never by factor.
pub fn to_celsius(amount: Decimal, unit: UnitId) -> Decimal {
    match unit {
        UnitId::Fahrenheit => (amount - Decimal::from(32)) * Decimal::from(5) / Decimal::from(9),
        UnitId::Kelvin => amount - CELSIUS_TO_KELVIN,
        _ => amount,
    }
}

pub fn from_celsius(celsius: Decimal, unit: UnitId) -> Decimal {
    match unit {
        UnitId::Fahrenheit => celsius * Decimal::from(9) / Decimal::from(5) + Decimal::from(32),
        UnitId::Kelvin => celsius + CELSIUS_TO_KELVIN,
        _ => celsius,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_symbol_names_two_units() {
        let table = table();
        let mut seen = std::collections::HashSet::new();
        for unit in &table {
            for symbol in unit.symbols {
                assert!(seen.insert(*symbol), "{symbol} names two units");
            }
        }
    }

    #[test]
    fn a_rate_is_one_base_over_another() {
        assert_eq!(
            Dim::EURO.over(Dim::TIME).as_rate(),
            Some((Dim::EURO, Dim::TIME))
        );
        assert_eq!(Dim::LENGTH.times(Dim::LENGTH).as_rate(), None);
        assert_eq!(Dim::EURO.over(Dim::TIME).dimension(), Dimension::Rate);
    }

    #[test]
    fn a_temperature_goes_through_celsius_exactly() {
        assert_eq!(
            from_celsius(Decimal::from(38), UnitId::Fahrenheit),
            "100.4".parse().unwrap()
        );
        assert_eq!(
            to_celsius(Decimal::ZERO, UnitId::Kelvin),
            "-273.15".parse().unwrap()
        );
    }
}
