//! The numbers a country gives a person, a firm and an account. ⚠️ One that encodes its bearer
//! — a birth, a sex, a name, a commune — is read off them, never drawn beside them; one that
//! carries a key gets it from `numbers.rs`. A country that has no number with a key for a
//! firm answers none, rather than a field made up.

use super::data::Place;
use super::numbers::{self, Born};
use super::profile::digits;
use super::{Bank, Country, Identifier, IdentifierKind, Sex};
use crate::tools::checks::iban_check_digits;
use crate::tools::generate::Draw;

/// The kinds that carry no key: a format and nothing to verify it by. The tool names those a
/// card shows, so that nobody takes one for checked.
pub(super) const UNKEYED: &[IdentifierKind] = &[
    IdentifierKind::Nino,
    IdentifierKind::Ssn,
    IdentifierKind::Crn,
    IdentifierKind::Kvk,
    IdentifierKind::SortCode,
    IdentifierKind::AccountNumber,
    IdentifierKind::TransitNumber,
];

/// Who the numbers are those of.
pub(super) struct Bearer<'a> {
    pub(super) sex: Sex,
    pub(super) surname: &'a str,
    pub(super) given: &'a str,
    pub(super) year: i32,
    pub(super) month: u32,
    pub(super) day: u32,
    pub(super) born_in: &'a Place,
    /// The year the card is drawn in: a Swedish number says who is a hundred.
    pub(super) this_year: i32,
}

fn one(kind: IdentifierKind, value: String) -> Identifier {
    Identifier { kind, value }
}

/// `text` cut in groups of these sizes, a separator between them; what is left stays whole.
fn grouped(text: &str, sizes: &[usize], separator: &str) -> String {
    let mut rest = text;
    let mut groups = Vec::new();
    for size in sizes {
        let (group, after) = rest.split_at((*size).min(rest.len()));
        groups.push(group);
        rest = after;
    }
    if !rest.is_empty() {
        groups.push(rest);
    }
    groups.join(separator)
}

/// A digit that is not `0`, then the others.
fn number(draw: &mut Draw, count: usize) -> String {
    format!(
        "{}{}",
        draw.between(1, 9),
        digits(draw, count.saturating_sub(1))
    )
}

/// A body drawn again until its key is a digit: modulo 11 leaves 10 once in eleven.
fn keyed(
    draw: &mut Draw,
    body: impl Fn(&mut Draw) -> String,
    key: fn(&str) -> Option<u32>,
) -> String {
    loop {
        let body = body(draw);
        if let Some(key) = key(&body) {
            return format!("{body}{key}");
        }
    }
}

fn serial_of(sex: Sex, draw: &mut Draw, width: usize) -> String {
    // Odd for a man, even for a woman: Belgium, Poland and Sweden agree on that much.
    let half = draw.between(0, 10_i64.pow(u32::try_from(width).unwrap_or(1)) / 2 - 2);
    let serial = half * 2
        + match sex {
            Sex::Male => 1,
            Sex::Female => 2,
        };
    format!("{serial:0width$}")
}

// --- The person

fn french(bearer: &Bearer<'_>, draw: &mut Draw) -> Vec<Identifier> {
    let thirteen = format!(
        "{}{:02}{:02}{}{:03}",
        match bearer.sex {
            Sex::Male => 1,
            Sex::Female => 2,
        },
        bearer.year.rem_euclid(100),
        bearer.month,
        bearer.born_in.code,
        draw.between(1, 999)
    );
    let key = numbers::nir_key(&thirteen);
    vec![one(
        IdentifierKind::Nir,
        format!("{} {key}", grouped(&thirteen, &[1, 2, 2, 2, 3, 3], " ")),
    )]
}

fn birth_digits(bearer: &Bearer<'_>) -> String {
    format!(
        "{:02}{:02}{:02}",
        bearer.year.rem_euclid(100),
        bearer.month,
        bearer.day
    )
}

fn belgian(bearer: &Bearer<'_>, draw: &mut Draw) -> Vec<Identifier> {
    let nine = format!("{}{}", birth_digits(bearer), serial_of(bearer.sex, draw, 3));
    let key = numbers::belgian_national_key(&nine, bearer.year >= 2000);
    vec![one(
        IdentifierKind::Nrn,
        format!(
            "{}-{}.{key}",
            grouped(&nine[..6], &[2, 2, 2], "."),
            &nine[6..]
        ),
    )]
}

/// Eleven digits, the first not `0`, where one digit is there twice and one is not there.
fn german(draw: &mut Draw) -> Vec<Identifier> {
    let mut ten: Vec<u32> = (0..10).collect();
    for at in (1..ten.len()).rev() {
        ten.swap(at, draw.below(at + 1));
    }
    ten[9] = ten[draw.below(9)];
    let moved = draw.below(10);
    ten.swap(9, moved);
    if ten[0] == 0 {
        let other = ten.iter().position(|digit| *digit != 0).unwrap_or(1);
        ten.swap(0, other);
    }
    let body: String = ten
        .iter()
        .filter_map(|digit| char::from_digit(*digit, 10))
        .collect();
    let whole = format!("{body}{}", numbers::mod_11_10(&body));
    vec![one(
        IdentifierKind::SteuerId,
        grouped(&whole, &[2, 3, 3, 3], " "),
    )]
}

fn spanish(draw: &mut Draw) -> Vec<Identifier> {
    let number = u32::try_from(draw.between(1_000_000, 99_999_999)).unwrap_or(1_000_000);
    vec![one(
        IdentifierKind::Dni,
        format!("{number:08}{}", numbers::dni_letter(number)),
    )]
}

fn italian(bearer: &Bearer<'_>) -> Vec<Identifier> {
    let born = Born {
        year: bearer.year,
        month: bearer.month,
        day: bearer.day,
        female: bearer.sex == Sex::Female,
        place: bearer.born_in.code,
    };
    vec![one(
        IdentifierKind::CodiceFiscale,
        numbers::codice_fiscale(bearer.surname, bearer.given, &born),
    )]
}

fn portuguese_number(first: &[u32], draw: &mut Draw) -> String {
    let eight = format!("{}{}", draw.pick(first), digits(draw, 7));
    let whole = format!("{eight}{}", numbers::portuguese_key(&eight));
    grouped(&whole, &[3, 3, 3], " ")
}

fn dutch(draw: &mut Draw) -> Vec<Identifier> {
    vec![one(
        IdentifierKind::Bsn,
        keyed(draw, |draw| number(draw, 8), numbers::dutch_citizen_key),
    )]
}

fn swiss(draw: &mut Draw) -> Vec<Identifier> {
    let twelve = format!("756{}", digits(draw, 9));
    let whole = format!("{twelve}{}", numbers::ean_13_key(&twelve));
    vec![one(
        IdentifierKind::Ahv,
        grouped(&whole, &[3, 4, 4, 2], "."),
    )]
}

/// ⚠️ No key: two letters the administration uses, six digits and a letter from A to D.
fn british(draw: &mut Draw) -> Vec<Identifier> {
    const FIRST: &[u8] = b"ABCEGHJKLMNOPRSTWXYZ";
    const SECOND: &[u8] = b"ABCEGHJKLMNPRSTWXYZ";
    const NEVER: &[&str] = &["BG", "GB", "KN", "NK", "NT", "TN", "ZZ"];
    let prefix = loop {
        let prefix = format!(
            "{}{}",
            char::from(*draw.pick(FIRST)),
            char::from(*draw.pick(SECOND))
        );
        if !NEVER.contains(&prefix.as_str()) {
            break prefix;
        }
    };
    vec![one(
        IdentifierKind::Nino,
        format!(
            "{prefix} {} {}",
            grouped(&digits(draw, 6), &[2, 2, 2], " "),
            char::from(*draw.pick(b"ABCD"))
        ),
    )]
}

fn polish(bearer: &Bearer<'_>, draw: &mut Draw) -> Vec<Identifier> {
    // The century is in the month: twenty higher for a birth from 2000 on.
    let month = bearer.month + if bearer.year >= 2000 { 20 } else { 0 };
    let ten = format!(
        "{:02}{month:02}{:02}{}{}",
        bearer.year.rem_euclid(100),
        bearer.day,
        digits(draw, 3),
        match bearer.sex {
            Sex::Male => draw.below(5) * 2 + 1,
            Sex::Female => draw.below(5) * 2,
        }
    );
    vec![one(
        IdentifierKind::Pesel,
        format!("{ten}{}", numbers::pesel_key(&ten)),
    )]
}

fn swedish(bearer: &Bearer<'_>, draw: &mut Draw) -> Vec<Identifier> {
    let nine = format!("{}{}", birth_digits(bearer), serial_of(bearer.sex, draw, 3));
    let whole = numbers::with_luhn(&nine);
    // A plus sign where there was a hyphen, from the year one turns a hundred.
    let separator = if bearer.this_year - bearer.year >= 100 {
        '+'
    } else {
        '-'
    };
    vec![one(
        IdentifierKind::Personnummer,
        format!("{}{separator}{}", &whole[..6], &whole[6..]),
    )]
}

/// ⚠️ Harmless by construction: `987-65-4320` to `4329` are the ten numbers kept for
/// advertisements, and a Canadian number starting with `0` is one Canada does not give.
fn north_american(country: Country, draw: &mut Draw) -> Vec<Identifier> {
    if country == Country::Us {
        return vec![one(
            IdentifierKind::Ssn,
            format!("987-65-432{}", draw.below(10)),
        )];
    }
    let whole = numbers::with_luhn(&format!("0{}", digits(draw, 7)));
    vec![one(IdentifierKind::Sin, grouped(&whole, &[3, 3, 3], " "))]
}

pub(super) fn personal(country: Country, bearer: &Bearer<'_>, draw: &mut Draw) -> Vec<Identifier> {
    match country {
        Country::Fr => french(bearer, draw),
        Country::Be => belgian(bearer, draw),
        Country::De => german(draw),
        Country::Es => spanish(draw),
        Country::It => italian(bearer),
        Country::Pt => vec![one(IdentifierKind::NifPt, portuguese_number(&[1, 2], draw))],
        Country::Nl => dutch(draw),
        Country::Ch => swiss(draw),
        Country::Gb => british(draw),
        Country::Pl => polish(bearer, draw),
        Country::Se => swedish(bearer, draw),
        Country::Us | Country::Ca => north_american(country, draw),
    }
}

// --- The firm

fn french_firm(draw: &mut Draw) -> Vec<Identifier> {
    let siren = numbers::with_luhn(&digits(draw, 8));
    let establishment = numbers::with_luhn(&format!("{siren}000{}", draw.between(1, 9)));
    vec![
        one(IdentifierKind::Siren, grouped(&siren, &[3, 3, 3], " ")),
        one(
            IdentifierKind::Siret,
            grouped(&establishment, &[3, 3, 3, 5], " "),
        ),
        one(IdentifierKind::Vat, numbers::french_vat(&siren)),
    ]
}

fn belgian_firm(draw: &mut Draw) -> Vec<Identifier> {
    let whole = numbers::belgian_enterprise(&format!("0{}", number(draw, 7)));
    vec![
        one(IdentifierKind::Bce, grouped(&whole, &[4, 3, 3], ".")),
        one(IdentifierKind::BeVat, format!("BE{whole}")),
    ]
}

fn german_firm(draw: &mut Draw) -> Vec<Identifier> {
    let eight = number(draw, 8);
    vec![one(
        IdentifierKind::UstIdNr,
        format!("DE{eight}{}", numbers::mod_11_10(&eight)),
    )]
}

fn spanish_firm(draw: &mut Draw) -> Vec<Identifier> {
    // `B` is a limited company, whose control is a digit; other letters take a letter.
    let seven = digits(draw, 7);
    vec![one(
        IdentifierKind::NifEs,
        format!("B{seven}{}", numbers::spanish_company_key(&seven)),
    )]
}

fn dutch_firm(draw: &mut Draw) -> Vec<Identifier> {
    let rsin = keyed(draw, |draw| number(draw, 8), numbers::dutch_citizen_key);
    vec![
        one(IdentifierKind::Kvk, number(draw, 8)),
        one(IdentifierKind::Btw, format!("NL{rsin}B01")),
    ]
}

fn swiss_firm(draw: &mut Draw) -> Vec<Identifier> {
    let whole = keyed(draw, |draw| number(draw, 8), numbers::swiss_company_key);
    vec![one(
        IdentifierKind::Uid,
        format!("CHE-{}", grouped(&whole, &[3, 3, 3], ".")),
    )]
}

fn british_firm(draw: &mut Draw) -> Vec<Identifier> {
    let seven = number(draw, 7);
    let whole = format!("{seven}{}", numbers::british_vat_key(&seven));
    vec![
        one(IdentifierKind::Crn, format!("0{}", number(draw, 7))),
        one(
            IdentifierKind::GbVat,
            format!("GB {}", grouped(&whole, &[3, 4, 2], " ")),
        ),
    ]
}

fn polish_firm(draw: &mut Draw) -> Vec<Identifier> {
    let nip = keyed(draw, |draw| number(draw, 9), numbers::nip_key);
    let eight = number(draw, 8);
    vec![
        one(IdentifierKind::Nip, grouped(&nip, &[3, 3, 2, 2], "-")),
        one(
            IdentifierKind::Regon,
            format!("{eight}{}", numbers::regon_key(&eight)),
        ),
    ]
}

fn swedish_firm(draw: &mut Draw) -> Vec<Identifier> {
    // A limited company starts with 5, and its third digit is never under 2.
    let whole = numbers::with_luhn(&format!(
        "5{}{}{}",
        draw.below(10),
        draw.between(2, 9),
        digits(draw, 6)
    ));
    vec![
        one(
            IdentifierKind::Orgnr,
            format!("{}-{}", &whole[..6], &whole[6..]),
        ),
        one(IdentifierKind::Moms, format!("SE{whole}01")),
    ]
}

pub(super) fn firm(country: Country, draw: &mut Draw) -> Vec<Identifier> {
    match country {
        Country::Fr => french_firm(draw),
        Country::Be => belgian_firm(draw),
        Country::De => german_firm(draw),
        Country::Es => spanish_firm(draw),
        Country::It => vec![one(
            IdentifierKind::PartitaIva,
            numbers::with_luhn(&number(draw, 10)),
        )],
        Country::Pt => vec![one(IdentifierKind::Nipc, portuguese_number(&[5], draw))],
        Country::Nl => dutch_firm(draw),
        Country::Ch => swiss_firm(draw),
        Country::Gb => british_firm(draw),
        Country::Pl => polish_firm(draw),
        Country::Se => swedish_firm(draw),
        // An EIN and a business number are formats: nothing computes them.
        Country::Us | Country::Ca => Vec::new(),
    }
}

// --- The account

/// A bank's own code where its country writes one in an account, its BIC and its name.
/// ⚠️ Real banks: an IBAN has no range kept for fiction, and a made-up code would not look
/// like one.
type House = (&'static str, &'static str, &'static str);

const FRENCH: &[House] = &[
    ("30004", "BNPAFRPP", "BNP Paribas"),
    ("30003", "SOGEFRPP", "Société Générale"),
    ("30002", "CRLYFRPP", "LCL"),
    ("20041", "PSSTFRPP", "La Banque Postale"),
    ("30066", "CMCIFRPP", "CIC"),
];
const BELGIAN: &[House] = &[
    ("001", "GEBABEBB", "BNP Paribas Fortis"),
    ("310", "BBRUBEBB", "ING België"),
    ("068", "GKCCBEBB", "Belfius"),
    ("734", "KREDBEBB", "KBC"),
    ("979", "ARSPBE22", "Argenta"),
];
const GERMAN: &[House] = &[
    ("10010010", "PBNKDEFF", "Postbank"),
    ("37040044", "COBADEFF", "Commerzbank"),
    ("50010517", "INGDDEFF", "ING"),
    ("70020270", "HYVEDEMM", "HypoVereinsbank"),
    ("10070000", "DEUTDEBB", "Deutsche Bank"),
];
const SPANISH: &[House] = &[
    ("2100", "CAIXESBB", "CaixaBank"),
    ("0049", "BSCHESMM", "Banco Santander"),
    ("0182", "BBVAESMM", "BBVA"),
    ("0081", "BSABESBB", "Banco Sabadell"),
    ("0128", "BKBKESMM", "Bankinter"),
];
const ITALIAN: &[House] = &[
    ("03069", "BCITITMM", "Intesa Sanpaolo"),
    ("02008", "UNCRITMM", "UniCredit"),
    ("05034", "BAPPIT21", "Banco BPM"),
    ("01030", "PASCITMM", "Monte dei Paschi di Siena"),
    ("07601", "BPPIITRR", "Poste Italiane"),
];
const PORTUGUESE: &[House] = &[
    ("0035", "CGDIPTPL", "Caixa Geral de Depósitos"),
    ("0033", "BCOMPTPL", "Millennium bcp"),
    ("0018", "TOTAPTPL", "Santander Totta"),
    ("0010", "BBPIPTPL", "Banco BPI"),
    ("0007", "BESCPTPL", "Novo Banco"),
];
const DUTCH: &[House] = &[
    ("ABNA", "ABNANL2A", "ABN AMRO"),
    ("INGB", "INGBNL2A", "ING"),
    ("RABO", "RABONL2U", "Rabobank"),
    ("SNSB", "SNSBNL2A", "SNS"),
    ("TRIO", "TRIONL2U", "Triodos Bank"),
];
const SWISS: &[House] = &[
    ("00230", "UBSWCHZH", "UBS"),
    ("09000", "POFICHBE", "PostFinance"),
    ("00700", "ZKBKCHZZ", "Zürcher Kantonalbank"),
    ("80808", "RAIFCH22", "Raiffeisen"),
    ("00762", "BLKBCH22", "Basellandschaftliche Kantonalbank"),
];
/// The four letters of the bank, and the two digits its sort codes start with.
const BRITISH: &[House] = &[
    ("NWBK60", "NWBKGB2L", "NatWest"),
    ("BARC20", "BARCGB22", "Barclays"),
    ("HBUK40", "HBUKGB4B", "HSBC UK"),
    ("LOYD30", "LOYDGB2L", "Lloyds Bank"),
    ("ABBY09", "ABBYGB2L", "Santander UK"),
];
const POLISH: &[House] = &[
    ("1020", "BPKOPLPW", "PKO Bank Polski"),
    ("1050", "INGBPLPW", "ING Bank Śląski"),
    ("1140", "BREXPLPW", "mBank"),
    ("1090", "WBKPPLPP", "Santander Bank Polska"),
    ("1240", "PKOPPLPW", "Bank Pekao"),
];
const SWEDISH: &[House] = &[
    ("500", "ESSESESS", "SEB"),
    ("600", "HANDSESS", "Handelsbanken"),
    ("800", "SWEDSESS", "Swedbank"),
    ("300", "NDEASESS", "Nordea"),
    ("120", "DABASESX", "Danske Bank"),
];
/// A routing number, which carries its own key; no BIC is written beside it.
pub(super) const AMERICAN: &[House] = &[
    ("021000021", "", "JPMorgan Chase"),
    ("026009593", "", "Bank of America"),
    ("121000248", "", "Wells Fargo"),
    ("021000089", "", "Citibank"),
    ("091000022", "", "U.S. Bank"),
];
/// The three digits of the institution.
const CANADIAN: &[House] = &[
    ("001", "", "BMO"),
    ("002", "", "Scotiabank"),
    ("003", "", "RBC"),
    ("004", "", "TD"),
    ("010", "", "CIBC"),
];

/// The account as the country writes it between the IBAN's key and its end.
fn bban(country: Country, code: &str, draw: &mut Draw) -> String {
    match country {
        Country::Fr => {
            let (branch, account) = (digits(draw, 5), digits(draw, 11));
            let key = numbers::rib_key(code, &branch, &account);
            format!("{code}{branch}{account}{key}")
        }
        Country::Be => numbers::belgian_account(&format!("{code}{}", digits(draw, 7))),
        Country::De => format!("{code}{}", digits(draw, 10)),
        Country::Es => {
            let (branch, account) = (digits(draw, 4), digits(draw, 10));
            let key = numbers::spanish_account_key(code, &branch, &account);
            format!("{code}{branch}{key}{account}")
        }
        Country::It => {
            let rest = format!("{code}{}{}", digits(draw, 5), digits(draw, 12));
            format!("{}{rest}", numbers::italian_control(&rest))
        }
        Country::Pt => {
            let nineteen = format!("{code}{}{}", digits(draw, 4), digits(draw, 11));
            let key = numbers::portuguese_account_key(&nineteen);
            format!("{nineteen}{key}")
        }
        Country::Nl => {
            let account = keyed(
                draw,
                |draw| format!("0{}", digits(draw, 8)),
                numbers::dutch_account_key,
            );
            format!("{code}{account}")
        }
        Country::Ch => format!("{code}{}", digits(draw, 12)),
        Country::Gb => format!("{code}{}{}", digits(draw, 4), digits(draw, 8)),
        Country::Pl => {
            let seven = format!("{code}{}", digits(draw, 3));
            let key = numbers::polish_branch_key(&seven);
            format!("{seven}{key}{}", digits(draw, 16))
        }
        Country::Se => format!("{code}{}", digits(draw, 17)),
        Country::Us | Country::Ca => String::new(),
    }
}

/// What the country writes beside an IBAN, or instead of one.
fn local(country: Country, code: &str, bban: &str, draw: &mut Draw) -> Vec<Identifier> {
    match country {
        Country::Gb => vec![
            one(
                IdentifierKind::SortCode,
                grouped(&bban[4..10], &[2, 2, 2], "-"),
            ),
            one(IdentifierKind::AccountNumber, bban[10..].to_owned()),
        ],
        Country::Us => vec![
            one(IdentifierKind::RoutingNumber, code.to_owned()),
            one(IdentifierKind::AccountNumber, number(draw, 10)),
        ],
        Country::Ca => vec![
            one(
                IdentifierKind::TransitNumber,
                format!("{}-{code}", digits(draw, 5)),
            ),
            one(IdentifierKind::AccountNumber, number(draw, 7)),
        ],
        _ => Vec::new(),
    }
}

pub(super) fn bank(country: Country, draw: &mut Draw) -> Bank {
    let houses = match country {
        Country::Fr => FRENCH,
        Country::Be => BELGIAN,
        Country::De => GERMAN,
        Country::Es => SPANISH,
        Country::It => ITALIAN,
        Country::Pt => PORTUGUESE,
        Country::Nl => DUTCH,
        Country::Ch => SWISS,
        Country::Gb => BRITISH,
        Country::Pl => POLISH,
        Country::Se => SWEDISH,
        Country::Us => AMERICAN,
        Country::Ca => CANADIAN,
    };
    let (code, bic, name) = *draw.pick(houses);
    let bban = bban(country, code, draw);
    let iban = (!bban.is_empty()).then(|| {
        let letters = country.as_str();
        let whole = format!("{letters}{}{bban}", iban_check_digits(letters, &bban));
        let every_four: Vec<usize> = vec![4; whole.len() / 4];
        grouped(&whole, &every_four, " ")
    });
    Bank {
        name: name.to_owned(),
        iban,
        bic: Some(bic.to_owned()).filter(|bic| !bic.is_empty()),
        local: local(country, code, &bban, draw),
    }
}
