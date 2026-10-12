//! The keys: each number's own arithmetic, a function alone, held to an example its country
//! publishes. Nothing here draws: what is drawn is the body, and the key is computed from it.

use crate::tools::checks::luhn_check_digit;

pub(super) fn as_digits(text: &str) -> Vec<u32> {
    text.chars()
        .filter_map(|digit| digit.to_digit(10))
        .collect()
}

fn weighted(digits: &[u32], weights: &[u32]) -> u32 {
    digits
        .iter()
        .zip(weights)
        .map(|(digit, weight)| digit * weight)
        .sum()
}

/// What a string of digits leaves of 97, however long it is.
fn of_97(digits: &str) -> u64 {
    as_digits(digits)
        .iter()
        .fold(0, |left, digit| (left * 10 + u64::from(*digit)) % 97)
}

/// The body and Luhn's digit after it: a SIREN, a partita IVA, an organisationsnummer, a SIN.
pub(super) fn with_luhn(body: &str) -> String {
    format!("{body}{}", luhn_check_digit(&as_digits(body)))
}

// --- France

/// The key of a RIB: 97 less what `89 × bank + 15 × branch + 3 × account` leaves of 97.
pub(super) fn rib_key(bank: &str, branch: &str, account: &str) -> String {
    let number = |text: &str| text.parse::<u64>().unwrap_or(0);
    let sum = 89 * number(bank) + 15 * number(branch) + 3 * number(account);
    format!("{:02}", 97 - sum % 97)
}

/// `FR`, a key of two digits — `(12 + 3 × (SIREN mod 97)) mod 97` — and the SIREN.
pub(super) fn french_vat(siren: &str) -> String {
    let key = (12 + 3 * (siren.parse::<u64>().unwrap_or(0) % 97)) % 97;
    format!("FR{key:02}{siren}")
}

/// The key of a social security number: 97 less what its thirteen digits leave of 97, Corsica's
/// `2A` and `2B` counted as 19 and 18.
pub(super) fn nir_key(thirteen: &str) -> String {
    let number = thirteen.replace("2A", "19").replace("2B", "18");
    format!("{:02}", 97 - of_97(&number))
}

// --- Belgium

/// The two digits of a national number: 97 less what the nine before leave of 97 — a `2` set
/// before them for a birth from 2000 on, which is how one century is told from the other.
pub(super) fn belgian_national_key(nine: &str, born_from_2000: bool) -> String {
    let number = if born_from_2000 {
        format!("2{nine}")
    } else {
        nine.to_owned()
    };
    format!("{:02}", 97 - of_97(&number))
}

/// An enterprise number: eight digits and 97 less what they leave of 97.
pub(super) fn belgian_enterprise(eight: &str) -> String {
    format!("{eight}{:02}", 97 - of_97(eight))
}

/// The account of a Belgian IBAN: ten digits and what they leave of 97, `97` for nothing.
pub(super) fn belgian_account(ten: &str) -> String {
    let left = of_97(ten);
    format!("{ten}{:02}", if left == 0 { 97 } else { left })
}

// --- Germany

/// ISO 7064 MOD 11,10: the digit a Steuer-ID and a USt-IdNr end on.
pub(super) fn mod_11_10(digits: &str) -> u32 {
    let product = as_digits(digits).iter().fold(10, |product, digit| {
        let sum = match (digit + product) % 10 {
            0 => 10,
            sum => sum,
        };
        (sum * 2) % 11
    });
    (11 - product) % 10
}

// --- Spain

/// The letter of a DNI: its eight digits, modulo 23, in a table everyone knows by heart.
pub(super) fn dni_letter(number: u32) -> char {
    const LETTERS: &[u8] = b"TRWAGMYFPDXBNJZSQVHLCKE";
    char::from(LETTERS[(number % 23) as usize])
}

/// The control digit of a company's NIF: the digits of even rank, and those of odd rank
/// doubled and their own digits added.
pub(super) fn spanish_company_key(seven: &str) -> u32 {
    let sum: u32 = as_digits(seven)
        .iter()
        .enumerate()
        .map(|(at, digit)| {
            if at % 2 == 0 {
                let doubled = digit * 2;
                doubled / 10 + doubled % 10
            } else {
                *digit
            }
        })
        .sum();
    (10 - sum % 10) % 10
}

/// The two control digits of a Spanish account: one over the bank and the branch, one over
/// the account, each `11 − (weighted sum mod 11)`, 10 written `1` and 11 written `0`.
pub(super) fn spanish_account_key(bank: &str, branch: &str, account: &str) -> String {
    const WEIGHTS: [u32; 10] = [1, 2, 4, 8, 5, 10, 9, 7, 3, 6];
    let digit = |text: &str| match 11 - weighted(&as_digits(text), &WEIGHTS) % 11 {
        10 => 1,
        11 => 0,
        digit => digit,
    };
    format!("{}{}", digit(&format!("00{bank}{branch}")), digit(account))
}

// --- Italy

/// What a character of odd rank is worth in a codice fiscale and in an account's CIN.
fn italian_odd(character: char) -> u32 {
    const ODD: [u32; 26] = [
        1, 0, 5, 7, 9, 13, 15, 17, 19, 21, 2, 4, 18, 20, 11, 3, 6, 8, 12, 14, 16, 10, 22, 25, 24,
        23,
    ];
    let rank = character
        .to_digit(10)
        .unwrap_or_else(|| u32::from(character).saturating_sub(u32::from('A')));
    ODD[(rank % 26) as usize]
}

fn italian_even(character: char) -> u32 {
    character
        .to_digit(10)
        .unwrap_or_else(|| u32::from(character).saturating_sub(u32::from('A')))
}

/// The letter both end on: the characters of odd rank through their table, the others as
/// they count, modulo 26.
pub(super) fn italian_control(text: &str) -> char {
    let sum: u32 = text
        .chars()
        .enumerate()
        .map(|(at, character)| {
            if at % 2 == 0 {
                italian_odd(character)
            } else {
                italian_even(character)
            }
        })
        .sum();
    char::from(b'A' + u8::try_from(sum % 26).unwrap_or(0))
}

fn letters(name: &str) -> (Vec<char>, Vec<char>) {
    deunicode::deunicode(name)
        .to_uppercase()
        .chars()
        .filter(char::is_ascii_alphabetic)
        .partition(|letter| !"AEIOU".contains(*letter))
}

/// Three letters of a name: its consonants, then its vowels, then `X`. ⚠️ A given name of
/// four consonants or more gives its first, third and fourth.
fn italian_name(name: &str, given: bool) -> String {
    let (mut consonants, vowels) = letters(name);
    if given && consonants.len() >= 4 {
        consonants.remove(1);
    }
    consonants
        .into_iter()
        .chain(vowels)
        .chain(['X', 'X', 'X'])
        .take(3)
        .collect()
}

pub(super) struct Born<'a> {
    pub(super) year: i32,
    pub(super) month: u32,
    pub(super) day: u32,
    pub(super) female: bool,
    /// The commune's code in the land registry.
    pub(super) place: &'a str,
}

/// ⚠️ Read off the person: the names, the birth — a woman's day is written forty higher — and
/// the commune. Nothing of it is drawn.
pub(super) fn codice_fiscale(surname: &str, given: &str, born: &Born<'_>) -> String {
    const MONTHS: &[u8] = b"ABCDEHLMPRST";
    let month = char::from(MONTHS[(born.month.clamp(1, 12) - 1) as usize]);
    let day = born.day + if born.female { 40 } else { 0 };
    let body = format!(
        "{}{}{:02}{month}{day:02}{}",
        italian_name(surname, false),
        italian_name(given, true),
        born.year.rem_euclid(100),
        born.place
    );
    format!("{body}{}", italian_control(&body))
}

// --- Portugal

/// The ninth digit of a NIF: `11 − (weighted sum mod 11)`, and `0` for 10 or 11.
pub(super) fn portuguese_key(eight: &str) -> u32 {
    match 11 - weighted(&as_digits(eight), &[9, 8, 7, 6, 5, 4, 3, 2]) % 11 {
        key if key >= 10 => 0,
        key => key,
    }
}

/// The two digits a Portuguese account ends on: 98 less what its nineteen digits and `00`
/// leave of 97.
pub(super) fn portuguese_account_key(nineteen: &str) -> String {
    format!("{:02}", 98 - of_97(&format!("{nineteen}00")))
}

// --- The Netherlands

/// The « elfproef » of a BSN: its eight first digits weighted 9 to 2, modulo 11, are its
/// ninth — which no digit can be when that is 10.
pub(super) fn dutch_citizen_key(eight: &str) -> Option<u32> {
    Some(weighted(&as_digits(eight), &[9, 8, 7, 6, 5, 4, 3, 2]) % 11).filter(|key| *key < 10)
}

/// The same proof for an account of ten digits, weighted 10 to 1: the tenth makes the sum a
/// multiple of 11, when a digit can.
pub(super) fn dutch_account_key(nine: &str) -> Option<u32> {
    let sum = weighted(&as_digits(nine), &[10, 9, 8, 7, 6, 5, 4, 3, 2]);
    Some((11 - sum % 11) % 11).filter(|key| *key < 10)
}

// --- Switzerland

/// The thirteenth digit of an AHV number, which is an EAN-13.
pub(super) fn ean_13_key(twelve: &str) -> u32 {
    let sum: u32 = as_digits(twelve)
        .iter()
        .enumerate()
        .map(|(at, digit)| if at % 2 == 0 { *digit } else { digit * 3 })
        .sum();
    (10 - sum % 10) % 10
}

/// The ninth digit of a UID, modulo 11 — none when the sum leaves 1.
pub(super) fn swiss_company_key(eight: &str) -> Option<u32> {
    Some((11 - weighted(&as_digits(eight), &[5, 4, 3, 2, 7, 6, 5, 4]) % 11) % 11)
        .filter(|key| *key < 10)
}

// --- The United Kingdom

/// The two digits of a VAT number: 97 less what its seven digits, weighted 8 to 2, leave.
pub(super) fn british_vat_key(seven: &str) -> String {
    format!(
        "{:02}",
        97 - weighted(&as_digits(seven), &[8, 7, 6, 5, 4, 3, 2]) % 97
    )
}

// --- Poland

/// The eleventh digit of a PESEL.
pub(super) fn pesel_key(ten: &str) -> u32 {
    (10 - weighted(&as_digits(ten), &[1, 3, 7, 9, 1, 3, 7, 9, 1, 3]) % 10) % 10
}

/// The tenth digit of a NIP, modulo 11 — none when that is 10.
pub(super) fn nip_key(nine: &str) -> Option<u32> {
    Some(weighted(&as_digits(nine), &[6, 5, 7, 2, 3, 4, 5, 6, 7]) % 11).filter(|key| *key < 10)
}

/// The ninth digit of a REGON: modulo 11, and `0` for 10.
pub(super) fn regon_key(eight: &str) -> u32 {
    weighted(&as_digits(eight), &[8, 9, 2, 3, 4, 5, 6, 7]) % 11 % 10
}

/// The eighth digit of the number that names a Polish bank and its branch in an IBAN.
pub(super) fn polish_branch_key(seven: &str) -> u32 {
    (10 - weighted(&as_digits(seven), &[3, 9, 7, 1, 3, 9, 7]) % 10) % 10
}

// --- The United States

/// Whether a routing number holds: `3, 7, 1` over its nine digits, a multiple of ten.
#[cfg(test)]
pub(super) fn routing_number_holds(nine: &str) -> bool {
    weighted(&as_digits(nine), &[3, 7, 1, 3, 7, 1, 3, 7, 1]).is_multiple_of(10)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::checks::{iban_check_digits, passes_luhn};

    /// The examples everyone checks a RIB against, and the published sample IBAN.
    #[test]
    fn france_a_rib_key_a_vat_key_and_a_social_security_key() {
        // FR76 3000 6000 0112 3456 7890 189: bank 30006, branch 00001, account 12345678901.
        assert_eq!(rib_key("30006", "00001", "12345678901"), "89");
        assert_eq!(iban_check_digits("FR", "30006000011234567890189"), "76");
        // The textbook pair: SIREN 404 833 048 is FR 83 404 833 048.
        assert_eq!(french_vat("404833048"), "FR83404833048");
        assert!(passes_luhn(&as_digits("404833048")));
        assert_eq!(with_luhn("40483304"), "404833048");
        // 2 69 05 49 588 157 80: the example the administration prints.
        assert_eq!(nir_key("2690549588157"), "80");
        // Born in Ajaccio: `2A` is read as 19 for the key, and stays written `2A`.
        assert_eq!(
            nir_key("185052A004123"),
            format!("{:02}", 97 - 1_850_519_004_123u64 % 97)
        );
    }

    #[test]
    fn belgium_a_national_number_an_enterprise_number_and_an_account() {
        // 85.07.30-033.28, the number every explanation takes.
        assert_eq!(belgian_national_key("850730033", false), "28");
        // Born in 2000 or after, the same nine digits are read behind a 2.
        assert_eq!(
            belgian_national_key("170730033", true),
            format!("{:02}", 97 - 2_170_730_033u64 % 97)
        );
        assert_eq!(belgian_enterprise("04287594"), "0428759497");
        // BE68 5390 0754 7034, the registry's example.
        assert_eq!(belgian_account("5390075470"), "539007547034");
        assert_eq!(iban_check_digits("BE", "539007547034"), "68");
    }

    #[test]
    fn germany_a_tax_number_and_a_vat_number() {
        assert_eq!(mod_11_10("8609574271"), 9);
        assert_eq!(mod_11_10("4703689281"), 6);
        // DE136695976.
        assert_eq!(mod_11_10("13669597"), 6);
        // DE89 3704 0044 0532 0130 00.
        assert_eq!(iban_check_digits("DE", "370400440532013000"), "89");
    }

    #[test]
    fn spain_a_dni_a_company_number_and_an_account() {
        assert_eq!(dni_letter(12_345_678), 'Z');
        assert_eq!(dni_letter(0), 'T');
        // A58818501.
        assert_eq!(spanish_company_key("5881850"), 1);
        // ES91 2100 0418 4502 0005 1332.
        assert_eq!(spanish_account_key("2100", "0418", "0200051332"), "45");
        assert_eq!(iban_check_digits("ES", "21000418450200051332"), "91");
    }

    #[test]
    fn italy_a_codice_fiscale_a_partita_iva_and_an_account() {
        // Mario Rossi, born in San Giuliano Terme on 10 December 1985.
        let born = Born {
            year: 1985,
            month: 12,
            day: 10,
            female: false,
            place: "A562",
        };
        assert_eq!(codice_fiscale("Rossi", "Mario", &born), "RSSMRA85T10A562S");
        // A woman's day is forty higher, and four consonants give the first, third and fourth.
        let born = Born {
            year: 2001,
            month: 1,
            day: 5,
            female: true,
            place: "H501",
        };
        let code = codice_fiscale("D'Angelo", "Francesca", &born);
        assert_eq!(&code[..15], "DNGFNC01A45H501");
        // Too short a name is filled with X.
        assert_eq!(italian_name("Yao", false), "YAO");
        assert_eq!(italian_name("Ly", false), "LYX");
        assert_eq!(with_luhn("1234567890"), "12345678903");
        // IT60 X054 2811 1010 0000 0123 456.
        assert_eq!(italian_control("0542811101000000123456"), 'X');
        assert_eq!(iban_check_digits("IT", "X0542811101000000123456"), "60");
    }

    #[test]
    fn portugal_a_nif_and_an_account() {
        assert_eq!(portuguese_key("12345678"), 9);
        // PT50 0002 0123 1234 5678 9015 4.
        assert_eq!(portuguese_account_key("0002012312345678901"), "54");
        assert_eq!(iban_check_digits("PT", "000201231234567890154"), "50");
    }

    #[test]
    fn the_netherlands_a_bsn_and_an_account() {
        assert_eq!(dutch_citizen_key("11122233"), Some(3));
        assert_eq!(dutch_citizen_key("12345678"), Some(2));
        // NL91 ABNA 0417 1643 00.
        assert_eq!(dutch_account_key("041716430"), Some(0));
        assert_eq!(iban_check_digits("NL", "ABNA0417164300"), "91");
    }

    #[test]
    fn switzerland_an_ahv_number_and_a_uid() {
        // 756.9217.0769.85, the sample the administration prints.
        assert_eq!(ean_13_key("756921707698"), 5);
        // CHE-116.281.710.
        assert_eq!(swiss_company_key("11628171"), Some(0));
        // CH93 0076 2011 6238 5295 7.
        assert_eq!(iban_check_digits("CH", "00762011623852957"), "93");
    }

    #[test]
    fn the_united_kingdom_a_vat_number_and_an_iban() {
        assert_eq!(british_vat_key("1234567"), "82");
        // GB29 NWBK 6016 1331 9268 19.
        assert_eq!(iban_check_digits("GB", "NWBK60161331926819"), "29");
    }

    #[test]
    fn poland_a_pesel_a_nip_a_regon_and_a_branch() {
        assert_eq!(pesel_key("4405140135"), 9);
        assert_eq!(nip_key("123456321"), Some(8));
        assert_eq!(regon_key("12345678"), 5);
        // PL61 1090 1014 0000 0712 1981 2874.
        assert_eq!(polish_branch_key("1090101"), 4);
        assert_eq!(iban_check_digits("PL", "109010140000071219812874"), "61");
    }

    #[test]
    fn sweden_a_personnummer_and_an_organisationsnummer() {
        assert_eq!(with_luhn("811218987"), "8112189876");
        assert_eq!(with_luhn("556036079"), "5560360793");
        // SE45 5000 0000 0583 9825 7466.
        assert_eq!(iban_check_digits("SE", "50000000058398257466"), "45");
    }

    #[test]
    fn north_america_a_sin_and_a_routing_number() {
        // 046 454 286, the number written wherever one must be shown.
        assert_eq!(with_luhn("04645428"), "046454286");
        assert!(routing_number_holds("021000021"));
        assert!(!routing_number_holds("021000022"));
    }
}
