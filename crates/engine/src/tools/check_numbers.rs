//! Numbers drawn to pass a check: a series that ends or begins as asked and passes Luhn, an IBAN
//! of a country. Test data: no card and no account stands behind them.

use std::collections::BTreeSet;

use super::checks::{CardNetwork, by_four, passes_luhn};
use super::generate::Draw;
use super::identity::{self, Country};
use super::random::{NoRandomness, uniform};
use crate::count::saturating_u32;

pub const LENGTHS: (u32, u32) = (2, 30);
pub const MAX_NUMBERS: u32 = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumbersRequest {
    Luhn {
        prefix: String,
        suffix: String,
        length: u32,
        count: u32,
    },
    Iban {
        country: Country,
        count: u32,
    },
}

/// A number as it is stored, and as it is printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawnNumber {
    pub raw: String,
    pub printed: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Affix {
    Prefix,
    Suffix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumbersAnswer {
    Drawn(Vec<DrawnNumber>),
    /// Something else than a digit, one-based in characters.
    NotDigits {
        affix: Affix,
        at: u32,
    },
    /// The prefix and the suffix hold more digits than the length.
    TooLong {
        digits: u32,
    },
    /// They leave no digit to draw, and the number they make does not pass.
    Impossible,
}

/// The countries whose IBAN is drawn, its national keys right too.
pub fn iban_countries() -> Vec<Country> {
    Country::ALL
        .into_iter()
        .filter(|country| identity::has_iban(*country))
        .collect()
}

pub fn draw(request: &NumbersRequest) -> Result<NumbersAnswer, NoRandomness> {
    let mut draw = Draw::new(uniform(u32::MAX)?);
    Ok(match request {
        NumbersRequest::Luhn {
            prefix,
            suffix,
            length,
            count,
        } => luhn(&mut draw, (prefix, suffix), *length, *count),
        NumbersRequest::Iban { country, count } => {
            NumbersAnswer::Drawn(ibans(&mut draw, *country, *count))
        }
    })
}

/// The digits of what was typed, its spaces left out.
fn digits_of(text: &str, affix: Affix) -> Result<Vec<u32>, NumbersAnswer> {
    let mut digits = Vec::new();
    for (at, character) in text.chars().enumerate() {
        match character.to_digit(10) {
            Some(digit) => digits.push(digit),
            None if character.is_whitespace() => {}
            None => {
                let at = saturating_u32(at) + 1;
                return Err(NumbersAnswer::NotDigits { affix, at });
            }
        }
    }
    Ok(digits)
}

fn luhn(draw: &mut Draw, (prefix, suffix): (&str, &str), length: u32, count: u32) -> NumbersAnswer {
    let (prefix, suffix) = match (
        digits_of(prefix, Affix::Prefix),
        digits_of(suffix, Affix::Suffix),
    ) {
        (Ok(prefix), Ok(suffix)) => (prefix, suffix),
        (Err(refused), _) | (_, Err(refused)) => return refused,
    };
    let length = length.clamp(LENGTHS.0, LENGTHS.1) as usize;
    let given = prefix.len() + suffix.len();
    let Some(free) = length.checked_sub(given) else {
        let digits = saturating_u32(given);
        return NumbersAnswer::TooLong { digits };
    };
    if free == 0 {
        let whole = [prefix, suffix].concat();
        return if passes_luhn(&whole) {
            NumbersAnswer::Drawn(vec![drawn_number(&whole)])
        } else {
            NumbersAnswer::Impossible
        };
    }
    // One free digit is the check's: the others give this many numbers.
    let possible = 10u32.saturating_pow(saturating_u32(free - 1));
    let wanted = count.clamp(1, MAX_NUMBERS).min(possible) as usize;
    let mut numbers = BTreeSet::new();
    let mut drawn = Vec::with_capacity(wanted);
    while drawn.len() < wanted {
        let middle = (0..free).map(|_| saturating_u32(draw.below(10)));
        let mut digits: Vec<u32> = prefix.iter().copied().chain(middle).collect();
        let checked = digits.len() - 1;
        digits.extend(&suffix);
        // Doubled or not, a digit takes each of the ten remainders once: one of them passes.
        while !passes_luhn(&digits) {
            digits[checked] = (digits[checked] + 1) % 10;
        }
        if numbers.insert(digits.clone()) {
            drawn.push(drawn_number(&digits));
        }
    }
    NumbersAnswer::Drawn(drawn)
}

fn drawn_number(digits: &[u32]) -> DrawnNumber {
    let raw: String = digits
        .iter()
        .filter_map(|digit| char::from_digit(*digit, 10))
        .collect();
    let network = CardNetwork::of(&raw);
    DrawnNumber {
        printed: network.map_or_else(|| by_four(&raw), |network| network.grouped(&raw)),
        raw,
    }
}

fn ibans(draw: &mut Draw, country: Country, count: u32) -> Vec<DrawnNumber> {
    (0..count.clamp(1, MAX_NUMBERS))
        .filter_map(|_| identity::iban(country, draw))
        .map(|printed| DrawnNumber {
            raw: printed.replace(' ', ""),
            printed,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::checks::{CheckAnswer, CheckRequest, IbanVerdict, check};

    fn series(prefix: &str, suffix: &str, length: u32, count: u32) -> NumbersAnswer {
        draw(&NumbersRequest::Luhn {
            prefix: prefix.to_owned(),
            suffix: suffix.to_owned(),
            length,
            count,
        })
        .unwrap()
    }

    fn drawn(answer: NumbersAnswer) -> Vec<DrawnNumber> {
        match answer {
            NumbersAnswer::Drawn(numbers) => numbers,
            refused => panic!("{refused:?}"),
        }
    }

    fn valid(text: &str) -> bool {
        match check(&CheckRequest {
            text: text.to_owned(),
            kind: None,
        }) {
            CheckAnswer::Luhn { valid, .. } => valid,
            CheckAnswer::Iban { verdict, .. } => matches!(verdict, IbanVerdict::Valid { .. }),
            _ => false,
        }
    }

    #[test]
    fn every_number_passes_and_begins_and_ends_as_asked() {
        for length in 10..=30 {
            for (prefix, suffix) in [("", ""), ("41", ""), ("", "77"), ("0123", "456")] {
                let numbers = drawn(series(prefix, suffix, length, 10));
                assert_eq!(numbers.len(), 10);
                for number in numbers {
                    assert!(valid(&number.raw), "{}", number.raw);
                    assert_eq!(number.raw.len(), length as usize);
                    assert!(number.raw.starts_with(prefix) && number.raw.ends_with(suffix));
                    assert_eq!(number.printed.replace(' ', ""), number.raw);
                }
            }
        }
    }

    #[test]
    fn no_number_comes_twice_and_there_are_no_more_than_possible() {
        // One free digit, the check's: one number. Two: ten.
        assert_eq!(drawn(series("411111111111111", "", 16, 20)).len(), 1);
        let ten = drawn(series("41111111111111", "", 16, 20));
        let distinct: BTreeSet<_> = ten.iter().map(|it| it.raw.clone()).collect();
        assert_eq!((ten.len(), distinct.len()), (10, 10));
    }

    #[test]
    fn a_suffix_leaves_its_check_to_the_digits_before_it() {
        let [number] = &drawn(series("7992739", "713", 11, 1))[..] else {
            panic!("one free digit, one number");
        };
        assert_eq!(number.raw, "79927398713");
    }

    #[test]
    fn a_whole_number_is_kept_if_it_passes_and_impossible_if_not() {
        let whole = drawn(series("4111 1111", "1111 1111", 16, 5));
        assert_eq!(whole.len(), 1);
        assert_eq!(whole[0].printed, "4111 1111 1111 1111");
        assert_eq!(
            series("4111111111111111", "2", 17, 1),
            NumbersAnswer::Impossible
        );
    }

    #[test]
    fn what_cannot_be_drawn_says_why() {
        assert_eq!(
            series("41a", "", 16, 1),
            NumbersAnswer::NotDigits {
                affix: Affix::Prefix,
                at: 3
            }
        );
        assert_eq!(
            series("", "-1", 16, 1),
            NumbersAnswer::NotDigits {
                affix: Affix::Suffix,
                at: 1
            }
        );
        assert_eq!(
            series("12345", "678", 6, 1),
            NumbersAnswer::TooLong { digits: 8 }
        );
    }

    #[test]
    fn the_bounds_are_kept() {
        assert_eq!(drawn(series("", "", 1, 0))[0].raw.len(), 2);
        let many = drawn(series("", "", 99, 99));
        assert_eq!((many.len(), many[0].raw.len()), (50, 30));
    }

    #[test]
    fn an_iban_drawn_is_valid_in_its_country() {
        let countries = iban_countries();
        assert!(countries.contains(&Country::Fr) && !countries.contains(&Country::Us));
        for country in countries {
            let request = NumbersRequest::Iban { country, count: 20 };
            for iban in drawn(draw(&request).unwrap()) {
                assert!(iban.raw.starts_with(country.as_str()), "{}", iban.raw);
                assert!(valid(&iban.raw), "{}", iban.raw);
                assert_eq!(iban.printed.replace(' ', ""), iban.raw);
            }
        }
    }
}
