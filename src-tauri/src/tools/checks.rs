//! Check digits: Luhn for a card or any number that carries one, and ISO 13616's mod 97 for an
//! IBAN, whose country's length and BBAN format come from `iban_validate`'s registry.

use std::str::FromStr;

use iban::{Iban, ParseIbanError};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::count::saturating_u32;

/// Each country's IBAN length, summed from the registry `iban_validate` checks the BBAN against:
/// the crate keeps its formats to itself, and a wrong length has to say the right one.
const IBAN_LENGTHS: [(&str, usize); 87] = [
    ("AD", 24),
    ("AE", 23),
    ("AL", 28),
    ("AT", 20),
    ("AZ", 28),
    ("BA", 20),
    ("BE", 16),
    ("BG", 22),
    ("BH", 22),
    ("BI", 27),
    ("BR", 29),
    ("BY", 28),
    ("CH", 21),
    ("CR", 22),
    ("CY", 28),
    ("CZ", 24),
    ("DE", 22),
    ("DJ", 27),
    ("DK", 18),
    ("DO", 28),
    ("EE", 20),
    ("EG", 29),
    ("ES", 24),
    ("FI", 18),
    ("FK", 18),
    ("FO", 18),
    ("FR", 27),
    ("GB", 22),
    ("GE", 22),
    ("GI", 23),
    ("GL", 18),
    ("GR", 27),
    ("GT", 28),
    ("HR", 21),
    ("HU", 28),
    ("IE", 22),
    ("IL", 23),
    ("IQ", 23),
    ("IS", 26),
    ("IT", 27),
    ("JO", 30),
    ("KW", 30),
    ("KZ", 20),
    ("LB", 28),
    ("LC", 32),
    ("LI", 21),
    ("LT", 20),
    ("LU", 20),
    ("LV", 21),
    ("LY", 25),
    ("MC", 27),
    ("MD", 24),
    ("ME", 22),
    ("MK", 19),
    ("MN", 20),
    ("MR", 27),
    ("MT", 31),
    ("MU", 30),
    ("NI", 28),
    ("NL", 18),
    ("NO", 15),
    ("OM", 23),
    ("PL", 28),
    ("PS", 29),
    ("PT", 25),
    ("QA", 29),
    ("RO", 24),
    ("RS", 22),
    ("RU", 33),
    ("SA", 24),
    ("SC", 31),
    ("SD", 18),
    ("SE", 24),
    ("SI", 19),
    ("SK", 24),
    ("SM", 27),
    ("SO", 23),
    ("ST", 25),
    ("SV", 28),
    ("TL", 23),
    ("TN", 24),
    ("TR", 26),
    ("UA", 29),
    ("VA", 22),
    ("VG", 24),
    ("XK", 20),
    ("YE", 30),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CheckKind {
    Luhn,
    Iban,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CardNetwork {
    Visa,
    Mastercard,
    AmericanExpress,
    Discover,
    DinersClub,
    Jcb,
    UnionPay,
    Maestro,
}

impl CardNetwork {
    /// By the prefix its issuers are given and a length it issues: a hint, never a proof.
    fn of(number: &str) -> Option<Self> {
        let within = |low: u32, high: u32, digits: usize| {
            number
                .get(..digits)
                .and_then(|head| head.parse::<u32>().ok())
                .is_some_and(|prefix| (low..=high).contains(&prefix))
        };
        let network = if within(34, 34, 2) || within(37, 37, 2) {
            Self::AmericanExpress
        } else if within(51, 55, 2) || within(2221, 2720, 4) {
            Self::Mastercard
        } else if number.starts_with('4') {
            Self::Visa
        } else if within(6011, 6011, 4) || within(644, 649, 3) || within(65, 65, 2) {
            Self::Discover
        } else if within(3528, 3589, 4) {
            Self::Jcb
        } else if within(300, 305, 3) || within(36, 36, 2) || within(38, 39, 2) {
            Self::DinersClub
        } else if within(62, 62, 2) {
            Self::UnionPay
        } else if within(50, 50, 2) || within(56, 69, 2) {
            Self::Maestro
        } else {
            return None;
        };
        let lengths: &[usize] = match network {
            Self::AmericanExpress => &[15],
            Self::Mastercard => &[16],
            Self::Visa => &[13, 16, 19],
            Self::DinersClub => &[14, 15, 16, 17, 18, 19],
            Self::Maestro => &[12, 13, 14, 15, 16, 17, 18, 19],
            Self::Discover | Self::Jcb | Self::UnionPay => &[16, 17, 18, 19],
        };
        lengths.contains(&number.len()).then_some(network)
    }

    /// American Express groups 4-6-5; the others by four.
    fn grouped(self, digits: &str) -> String {
        match self {
            Self::AmericanExpress if digits.len() == 15 => {
                format!("{} {} {}", &digits[..4], &digits[4..10], &digits[10..])
            }
            _ => by_four(digits),
        }
    }
}

fn by_four(text: &str) -> String {
    text.as_bytes()
        .chunks(4)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The Luhn sum, the last digit doubled when `appending`: a check digit about to be added will
/// push it one place left.
fn luhn_sum(digits: &[u32], appending: bool) -> u32 {
    digits
        .iter()
        .rev()
        .enumerate()
        .map(|(index, digit)| {
            if (index % 2 == 0) == appending {
                let doubled = digit * 2;
                if doubled > 9 { doubled - 9 } else { doubled }
            } else {
                *digit
            }
        })
        .sum()
}

pub(crate) fn luhn_check_digit(digits: &[u32]) -> u32 {
    (10 - luhn_sum(digits, true) % 10) % 10
}

pub(crate) fn passes_luhn(digits: &[u32]) -> bool {
    luhn_sum(digits, false).is_multiple_of(10)
}

/// ISO 13616: the country and `00` moved behind the BBAN, letters as 10 to 35, mod 97.
pub(crate) fn iban_check_digits(country: &str, bban: &str) -> String {
    let remainder = bban
        .chars()
        .chain(country.chars())
        .chain("00".chars())
        .fold(0u32, |remainder, character| {
            let value = character.to_digit(36).unwrap_or(0);
            if value > 9 {
                (remainder * 100 + value) % 97
            } else {
                (remainder * 10 + value) % 97
            }
        });
    format!("{:02}", 98 - remainder)
}

/// The French RIB's key, over the bank, the branch and the account, a letter of the account read
/// as a digit (`A` and `J` as 1, `B`, `K` and `S` as 2…).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RibKey {
    pub given: String,
    pub expected: String,
}

fn rib_digit(character: char) -> Option<u64> {
    let digit = match character {
        '0'..='9' => return character.to_digit(10).map(u64::from),
        'A' | 'J' => 1,
        'B' | 'K' | 'S' => 2,
        'C' | 'L' | 'T' => 3,
        'D' | 'M' | 'U' => 4,
        'E' | 'N' | 'V' => 5,
        'F' | 'O' | 'W' => 6,
        'G' | 'P' | 'X' => 7,
        'H' | 'Q' | 'Y' => 8,
        'I' | 'R' | 'Z' => 9,
        _ => return None,
    };
    Some(digit)
}

fn rib_number(text: &str) -> Option<u64> {
    text.chars().try_fold(0u64, |number, character| {
        Some(number * 10 + rib_digit(character)?)
    })
}

/// The account and the RIB key of a French (or Monégasque) BBAN: 5 + 5 + 11 + 2 characters.
fn french_account(bban: &str) -> Option<(String, RibKey)> {
    if bban.len() != 23 || !bban.is_ascii() {
        return None;
    }
    let (bank, branch, account, given) = (&bban[..5], &bban[5..10], &bban[10..21], &bban[21..]);
    let sum = 89 * rib_number(bank)? + 15 * rib_number(branch)? + 3 * rib_number(account)?;
    let expected = format!("{:02}", 97 - sum % 97);
    Some((
        account.to_owned(),
        RibKey {
            given: given.to_owned(),
            expected,
        },
    ))
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CheckRequest {
    pub text: String,
    /// Read as this kind rather than by its shape.
    pub kind: Option<CheckKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum IbanVerdict {
    Valid {
        bban: String,
        bank: Option<String>,
        branch: Option<String>,
        /// Where the country's format names one: France and Monaco.
        account: Option<String>,
        rib_key: Option<RibKey>,
    },
    /// The check digits the rest asks for, and the IBAN written with them.
    WrongChecksum {
        expected: String,
        corrected: String,
    },
    WrongLength {
        expected: u32,
        found: u32,
    },
    /// The length is right, the characters are not the country's: letters where digits go.
    WrongFormat,
    UnknownCountry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CheckAnswer {
    Luhn {
        grouped: String,
        /// Its digits, counted.
        length: u32,
        valid: bool,
        /// The last digit the rest asks for.
        expected_last: u32,
        /// The number with a check digit appended, as if it had none yet.
        completed: String,
        network: Option<CardNetwork>,
        guessed: bool,
    },
    Iban {
        country: String,
        printed: String,
        /// The two digits after the country, as typed.
        check_digits: String,
        /// Its characters, spaces left out.
        length: u32,
        verdict: IbanVerdict,
        guessed: bool,
    },
    /// One-based, in characters.
    Unreadable {
        at: u32,
    },
    TooShort,
}

/// What is kept of the text: its letters and digits, with where each stood.
fn cleaned(text: &str, letters: bool) -> Result<String, u32> {
    let mut kept = String::new();
    for (index, character) in text.chars().enumerate() {
        if character.is_ascii_digit() || (letters && character.is_ascii_alphabetic()) {
            kept.push(character.to_ascii_uppercase());
        } else if !matches!(character, ' ' | '-' | '.' | '\u{a0}' | '\u{202f}') {
            return Err(saturating_u32(index) + 1);
        }
    }
    Ok(kept)
}

fn looks_like_iban(text: &str) -> bool {
    let kept: Vec<char> = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .take(4)
        .collect();
    kept.len() == 4
        && kept[..2].iter().all(char::is_ascii_alphabetic)
        && kept[2..].iter().all(char::is_ascii_digit)
}

fn luhn(text: &str, guessed: bool) -> CheckAnswer {
    let digits_text = match cleaned(text, false) {
        Ok(digits) => digits,
        Err(at) => return CheckAnswer::Unreadable { at },
    };
    if digits_text.len() < 2 {
        return CheckAnswer::TooShort;
    }
    let digits: Vec<u32> = digits_text.chars().filter_map(|c| c.to_digit(10)).collect();
    let network = CardNetwork::of(&digits_text);
    let completed = format!("{digits_text}{}", luhn_check_digit(&digits));
    CheckAnswer::Luhn {
        grouped: network.map_or_else(|| by_four(&digits_text), |n| n.grouped(&digits_text)),
        length: saturating_u32(digits_text.len()),
        valid: passes_luhn(&digits),
        expected_last: luhn_check_digit(&digits[..digits.len() - 1]),
        completed: by_four(&completed),
        network,
        guessed,
    }
}

fn iban(text: &str, guessed: bool) -> CheckAnswer {
    let electronic = match cleaned(text, true) {
        Ok(kept) => kept,
        Err(at) => return CheckAnswer::Unreadable { at },
    };
    if electronic.len() < 5 {
        return CheckAnswer::TooShort;
    }
    if !electronic[..2].chars().all(|c| c.is_ascii_alphabetic()) {
        return CheckAnswer::Unreadable { at: 1 };
    }
    let country = electronic[..2].to_owned();
    let (given, bban) = (&electronic[2..4], &electronic[4..]);
    let printed = by_four(&electronic);
    let verdict = match IBAN_LENGTHS.iter().find(|(code, _)| *code == country) {
        None => IbanVerdict::UnknownCountry,
        Some((_, expected)) if *expected != electronic.len() => IbanVerdict::WrongLength {
            expected: saturating_u32(*expected),
            found: saturating_u32(electronic.len()),
        },
        Some(_) => {
            let expected = iban_check_digits(&country, bban);
            if expected == given {
                match Iban::from_str(&electronic) {
                    Ok(iban) => {
                        let french = matches!(country.as_str(), "FR" | "MC")
                            .then(|| french_account(iban.bban()))
                            .flatten();
                        // The registry names no branch for France; its BBAN always carries one.
                        let branch = iban
                            .branch_identifier()
                            .map(str::to_owned)
                            .or_else(|| french.as_ref().map(|_| iban.bban()[5..10].to_owned()));
                        let (account, rib_key) = french.unzip();
                        IbanVerdict::Valid {
                            bban: iban.bban().to_owned(),
                            bank: iban.bank_identifier().map(str::to_owned),
                            branch,
                            account,
                            rib_key,
                        }
                    }
                    Err(ParseIbanError::UnknownCountry(_)) => IbanVerdict::UnknownCountry,
                    Err(_) => IbanVerdict::WrongFormat,
                }
            } else {
                IbanVerdict::WrongChecksum {
                    corrected: by_four(&format!("{country}{expected}{bban}")),
                    expected,
                }
            }
        }
    };
    CheckAnswer::Iban {
        check_digits: given.to_owned(),
        length: saturating_u32(electronic.len()),
        country,
        printed,
        verdict,
        guessed,
    }
}

pub fn check(request: &CheckRequest) -> CheckAnswer {
    match request.kind {
        Some(CheckKind::Luhn) => luhn(&request.text, false),
        Some(CheckKind::Iban) => iban(&request.text, false),
        None if looks_like_iban(&request.text) => iban(&request.text, true),
        None => luhn(&request.text, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(text: &str) -> CheckAnswer {
        check(&CheckRequest {
            text: text.to_owned(),
            kind: None,
        })
    }

    fn luhn_of(text: &str) -> (bool, u32, String, Option<CardNetwork>) {
        match ask(text) {
            CheckAnswer::Luhn {
                valid,
                expected_last,
                completed,
                network,
                ..
            } => (valid, expected_last, completed, network),
            other => panic!("{text} was not read as Luhn: {other:?}"),
        }
    }

    fn verdict(text: &str) -> IbanVerdict {
        match ask(text) {
            CheckAnswer::Iban { verdict, .. } => verdict,
            other => panic!("{text} was not read as an IBAN: {other:?}"),
        }
    }

    #[test]
    fn the_published_test_numbers_of_each_network_pass() {
        for (number, network) in [
            ("4111 1111 1111 1111", CardNetwork::Visa),
            ("4012888888881881", CardNetwork::Visa),
            ("5555 5555 5555 4444", CardNetwork::Mastercard),
            ("2223003122003222", CardNetwork::Mastercard),
            ("3782 822463 10005", CardNetwork::AmericanExpress),
            ("371449635398431", CardNetwork::AmericanExpress),
            ("6011111111111117", CardNetwork::Discover),
            ("30569309025904", CardNetwork::DinersClub),
            ("3530111333300000", CardNetwork::Jcb),
            ("6200000000000005", CardNetwork::UnionPay),
            ("6759649826438453", CardNetwork::Maestro),
        ] {
            let (valid, _, _, found) = luhn_of(number);
            assert!(valid, "{number}");
            assert_eq!(found, Some(network), "{number}");
        }
    }

    #[test]
    fn a_wrong_digit_and_swapped_digits_are_caught() {
        let (valid, expected_last, _, _) = luhn_of("4111 1111 1111 1112");
        assert!(!valid);
        assert_eq!(expected_last, 1);
        assert!(!luhn_of("4111111111111111".replacen("41", "14", 1).as_str()).0);
        assert!(!luhn_of("79927398710").0);
        assert!(luhn_of("79927398713").0);
    }

    #[test]
    fn a_check_digit_is_computed_for_a_number_without_one() {
        let (_, _, completed, _) = luhn_of("7992739871");

        assert_eq!(completed, "7992 7398 713");
        assert_eq!(luhn_of("411111111111111").2, "4111 1111 1111 1111");
    }

    #[test]
    fn a_card_number_is_grouped_as_its_network_prints_it() {
        let CheckAnswer::Luhn { grouped, .. } = ask("378282246310005") else {
            panic!("luhn");
        };
        assert_eq!(grouped, "3782 822463 10005");
        assert!(matches!(
            ask("1234-5678"),
            CheckAnswer::Luhn { network: None, .. }
        ));
    }

    #[test]
    fn valid_ibans_of_a_dozen_countries() {
        for iban in [
            "FR14 2004 1010 0505 0001 3M02 606",
            "DE89 3704 0044 0532 0130 00",
            "GB29 NWBK 6016 1331 9268 19",
            "ES91 2100 0418 4502 0005 1332",
            "IT60 X054 2811 1010 0000 0123 456",
            "NL91 ABNA 0417 1643 00",
            "BE68 5390 0754 7034",
            "CH93 0076 2011 6238 5295 7",
            "LU28 0019 4006 4475 0000",
            "PT50 0002 0123 1234 5678 9015 4",
            "AT61 1904 3002 3457 3201",
            "PL61 1090 1014 0000 0712 1981 2874",
            "SE45 5000 0000 0583 9825 7466",
            "NO93 8601 1117 947",
            "MC58 1122 2000 0101 2345 6789 030",
        ] {
            assert!(
                matches!(verdict(iban), IbanVerdict::Valid { .. }),
                "{iban}: {:?}",
                verdict(iban)
            );
        }
        let CheckAnswer::Iban {
            country,
            printed,
            verdict,
            ..
        } = ask("de89370400440532013000")
        else {
            panic!("iban");
        };
        assert_eq!(
            (country.as_str(), printed.as_str()),
            ("DE", "DE89 3704 0044 0532 0130 00")
        );
        assert_eq!(
            verdict,
            IbanVerdict::Valid {
                bban: "370400440532013000".into(),
                bank: Some("37040044".into()),
                branch: None,
                account: None,
                rib_key: None,
            }
        );
    }

    #[test]
    fn a_french_iban_gives_its_bank_branch_account_and_rib_key() {
        let CheckAnswer::Iban {
            check_digits,
            length,
            verdict: french,
            ..
        } = ask("FR14 2004 1010 0505 0001 3M02 606")
        else {
            panic!("iban");
        };

        assert_eq!((check_digits.as_str(), length), ("14", 27));
        assert_eq!(
            french,
            IbanVerdict::Valid {
                bban: "20041010050500013M02606".into(),
                bank: Some("20041".into()),
                branch: Some("01005".into()),
                account: Some("0500013M026".into()),
                rib_key: Some(RibKey {
                    given: "06".into(),
                    expected: "06".into()
                }),
            }
        );
        let IbanVerdict::Valid { rib_key, .. } = verdict("FR76 3000 6000 0112 3456 7890 189")
        else {
            panic!("valid");
        };
        assert_eq!(rib_key.map(|key| key.expected), Some("89".into()));
    }

    #[test]
    fn a_wrong_rib_key_is_said_where_the_iban_still_adds_up() {
        let bban = "20041010050500013M02607";
        let iban = format!("FR{}{bban}", iban_check_digits("FR", bban));

        let IbanVerdict::Valid { rib_key, .. } = verdict(&iban) else {
            panic!("the IBAN's own key is right");
        };
        assert_eq!(
            rib_key,
            Some(RibKey {
                given: "07".into(),
                expected: "06".into()
            })
        );
    }

    #[test]
    fn a_card_says_how_many_digits_it_has() {
        let CheckAnswer::Luhn { length, .. } = ask("4111 1111 1111 1111") else {
            panic!("luhn");
        };
        assert_eq!(length, 16);
    }

    #[test]
    fn a_wrong_checksum_gives_the_right_one() {
        assert_eq!(
            verdict("FR15 2004 1010 0505 0001 3M02 606"),
            IbanVerdict::WrongChecksum {
                expected: "14".into(),
                corrected: "FR14 2004 1010 0505 0001 3M02 606".into(),
            }
        );
    }

    #[test]
    fn a_wrong_length_says_the_right_one_and_an_unknown_country_is_named() {
        assert_eq!(
            verdict("FR14 2004 1010 0505 0001 3M02 60"),
            IbanVerdict::WrongLength {
                expected: 27,
                found: 26
            }
        );
        assert_eq!(verdict("ZZ12 3456 7890"), IbanVerdict::UnknownCountry);
        let digits_for_letters = "123456987654321012";
        let gb = format!(
            "GB{}{digits_for_letters}",
            iban_check_digits("GB", digits_for_letters)
        );
        assert_eq!(verdict(&gb), IbanVerdict::WrongFormat);
    }

    #[test]
    fn every_length_is_the_registry_s() {
        for (country, length) in IBAN_LENGTHS {
            let bban = "0".repeat(length - 4);
            let electronic = format!("{country}{}{bban}", iban_check_digits(country, &bban));
            match Iban::from_str(&electronic) {
                Ok(_) | Err(ParseIbanError::InvalidBban(_)) => {}
                Err(problem) => panic!("{country}: {problem:?}"),
            }
        }
    }

    #[test]
    fn the_kind_is_read_from_the_shape_or_forced() {
        assert!(matches!(
            ask("FR76 3000"),
            CheckAnswer::Iban { guessed: true, .. }
        ));
        assert!(matches!(
            ask("4111"),
            CheckAnswer::Luhn { guessed: true, .. }
        ));
        assert!(matches!(
            check(&CheckRequest {
                text: "12345678".into(),
                kind: Some(CheckKind::Iban),
            }),
            CheckAnswer::Unreadable { at: 1 }
        ));
        assert!(matches!(
            check(&CheckRequest {
                text: "FR76".into(),
                kind: Some(CheckKind::Luhn),
            }),
            CheckAnswer::Unreadable { at: 1 }
        ));
    }

    #[test]
    fn what_cannot_be_read_says_where() {
        assert_eq!(ask("4111 1111 x111"), CheckAnswer::Unreadable { at: 11 });
        assert_eq!(ask("7"), CheckAnswer::TooShort);
        assert_eq!(ask("FR76 30*0"), CheckAnswer::Unreadable { at: 8 });
        assert_eq!(ask("FR76"), CheckAnswer::TooShort);
    }
}
