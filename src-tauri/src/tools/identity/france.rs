//! What France asks of an identity: where its names come from, and its numbers — each one
//! computed, and the social security number read off the person it is the number of.

use super::data::{Place, Pool};
use super::{Bank, Birth, Identifier, IdentifierKind, Sex};
use crate::tools::checks::{iban_check_digits, luhn_check_digit};
use crate::tools::generate::Draw;

/// Where the names of people living in France come from, in shares of a hundred: the mix is
/// the diversity, and nothing else lists it.
pub(super) const POOLS: &[(Pool, u32)] = &[
    (Pool::French, 78),
    (Pool::Arabic, 9),
    (Pool::Portuguese, 5),
    (Pool::WestAfrican, 5),
    (Pool::Vietnamese, 3),
];

/// The bank's code in a RIB, its BIC and its name. ⚠️ Real banks: an IBAN has no range kept
/// for fiction, and a made-up code would not look like one.
const BANKS: &[(&str, &str, &str)] = &[
    ("30004", "BNPAFRPP", "BNP Paribas"),
    ("30003", "SOGEFRPP", "Société Générale"),
    ("30002", "CRLYFRPP", "LCL"),
    ("20041", "PSSTFRPP", "La Banque Postale"),
    ("30066", "CMCIFRPP", "CIC"),
];

/// ⚠️ The numbers ARCEP keeps for fiction, a block a zone and one for mobiles: they reach
/// nobody. The first digit after the `0` is the zone of the place.
const FICTION_LANDLINES: &[(&str, &str)] = &[
    ("1", "01 99 00"),
    ("2", "02 61 91"),
    ("3", "03 53 01"),
    ("4", "04 65 71"),
    ("5", "05 36 49"),
];
const FICTION_MOBILE: &str = "06 39 98";

const TRADES: &[&str] = &[
    "Conseil",
    "Services",
    "Bâtiment",
    "Informatique",
    "Transports",
    "Menuiserie",
    "Plomberie",
    "Électricité",
    "Paysages",
    "Formation",
];

fn digits(draw: &mut Draw, count: usize) -> String {
    (0..count)
        .map(|_| char::from_digit(u32::try_from(draw.below(10)).unwrap_or(0), 10).unwrap_or('0'))
        .collect()
}

fn as_digits(text: &str) -> Vec<u32> {
    text.chars()
        .filter_map(|digit| digit.to_digit(10))
        .collect()
}

fn pairs(text: &str) -> String {
    text.as_bytes()
        .chunks(2)
        .map(|pair| String::from_utf8_lossy(pair).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `0X XX XX XX XX`, in the block kept for fiction of the place's own zone.
pub(super) fn landline(place: &Place, draw: &mut Draw) -> String {
    let block = FICTION_LANDLINES
        .iter()
        .find(|(zone, _)| *zone == place.more)
        .map_or(FICTION_LANDLINES[0].1, |(_, block)| block);
    format!("{block} {}", pairs(&digits(draw, 4)))
}

pub(super) fn mobile(draw: &mut Draw) -> String {
    format!("{FICTION_MOBILE} {}", pairs(&digits(draw, 4)))
}

/// The key of a RIB: 97 less what `89 × bank + 15 × branch + 3 × account` leaves of 97.
pub(super) fn rib_key(bank: &str, branch: &str, account: &str) -> String {
    let number = |text: &str| text.parse::<u64>().unwrap_or(0);
    let sum = 89 * number(bank) + 15 * number(branch) + 3 * number(account);
    format!("{:02}", 97 - sum % 97)
}

/// An account in a real bank's code, with its RIB key and the IBAN's own two digits.
pub(super) fn bank(draw: &mut Draw) -> Bank {
    let (code, bic, name) = *draw.pick(BANKS);
    let (branch, account) = (digits(draw, 5), digits(draw, 11));
    let bban = format!(
        "{code}{branch}{account}{}",
        rib_key(code, &branch, &account)
    );
    let iban = format!("FR{}{bban}", iban_check_digits("FR", &bban));
    Bank {
        name: name.to_owned(),
        iban: iban
            .as_bytes()
            .chunks(4)
            .map(|group| String::from_utf8_lossy(group).into_owned())
            .collect::<Vec<_>>()
            .join(" "),
        bic: bic.to_owned(),
    }
}

/// Nine digits, the last one Luhn's.
pub(super) fn siren(draw: &mut Draw) -> String {
    let body = digits(draw, 8);
    format!("{body}{}", luhn_check_digit(&as_digits(&body)))
}

/// The SIREN, an establishment's number, and Luhn again over the fourteen.
pub(super) fn siret(siren: &str, draw: &mut Draw) -> String {
    let body = format!("{siren}000{}", draw.between(1, 9));
    format!("{body}{}", luhn_check_digit(&as_digits(&body)))
}

/// `FR`, a key of two digits — `(12 + 3 × (SIREN mod 97)) mod 97` — and the SIREN.
pub(super) fn vat(siren: &str) -> String {
    let key = (12 + 3 * (siren.parse::<u64>().unwrap_or(0) % 97)) % 97;
    format!("FR{key:02}{siren}")
}

/// The key of a social security number: 97 less what its thirteen digits leave of 97, Corsica's
/// `2A` and `2B` counted as 19 and 18.
pub(super) fn nir_key(thirteen: &str) -> String {
    let number: u64 = thirteen
        .replace("2A", "19")
        .replace("2B", "18")
        .parse()
        .unwrap_or(0);
    format!("{:02}", 97 - number % 97)
}

/// ⚠️ Read off the person, never drawn beside them: the sex, the year and the month of the
/// birth, the department and the commune it was in. Only the rank among the births of that
/// month is drawn.
pub(super) fn nir(sex: Sex, birth: &Birth, born_in: &Place, draw: &mut Draw) -> String {
    let (year, month) = (&birth.date[2..4], &birth.date[5..7]);
    let thirteen = format!(
        "{}{year}{month}{}{:03}",
        match sex {
            Sex::Male => 1,
            Sex::Female => 2,
        },
        born_in.code,
        draw.between(1, 999)
    );
    let key = nir_key(&thirteen);
    format!(
        "{} {} {} {} {} {} {key}",
        &thirteen[..1],
        &thirteen[1..3],
        &thirteen[3..5],
        &thirteen[5..7],
        &thirteen[7..10],
        &thirteen[10..13]
    )
}

pub(super) fn company(surname: &str, draw: &mut Draw) -> (String, Vec<Identifier>) {
    let siren = siren(draw);
    let establishment = siret(&siren, draw);
    let grouped = |text: &str, sizes: &[usize]| {
        let mut rest = text;
        let mut groups = Vec::new();
        for size in sizes {
            let (group, after) = rest.split_at((*size).min(rest.len()));
            groups.push(group);
            rest = after;
        }
        groups.join(" ")
    };
    let identifiers = vec![
        Identifier {
            kind: IdentifierKind::Siren,
            value: grouped(&siren, &[3, 3, 3]),
        },
        Identifier {
            kind: IdentifierKind::Siret,
            value: grouped(&establishment, &[3, 3, 3, 5]),
        },
        Identifier {
            kind: IdentifierKind::Vat,
            value: vat(&siren),
        },
    ];
    (format!("{surname} {}", draw.pick(TRADES)), identifiers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::checks::passes_luhn;

    /// The examples everyone checks a RIB against, and the published sample IBAN.
    #[test]
    fn a_rib_key_is_the_one_the_published_examples_carry() {
        // FR76 3000 6000 0112 3456 7890 189: bank 30006, branch 00001, account 12345678901.
        assert_eq!(rib_key("30006", "00001", "12345678901"), "89");
        assert_eq!(iban_check_digits("FR", "30006000011234567890189"), "76");
    }

    #[test]
    fn a_vat_number_carries_the_key_of_its_siren() {
        // The textbook pair: SIREN 404 833 048 is FR 83 404 833 048.
        assert_eq!(vat("404833048"), "FR83404833048");
        assert!(passes_luhn(&as_digits("404833048")));
    }

    #[test]
    fn a_social_security_key_counts_corsica_as_numbers() {
        // 2 69 05 49 588 157 80: the example the administration prints.
        assert_eq!(nir_key("2690549588157"), "80");
        // Born in Ajaccio: `2A` is read as 19 for the key, and stays written `2A`.
        assert_eq!(
            nir_key("185052A004123"),
            format!("{:02}", 97 - 1_850_519_004_123u64 % 97)
        );
    }
}
