//! A fictitious person whose every identifier passes its own check: test data for a form or a
//! database, drawn from a seed so that the same one comes back. Names come from pools shared
//! between countries — files shipped beside the application and read there
//! (`identity/data.rs`) — and a country is a profile — a mix of pools, its
//! places, its formats (`identity/profile.rs`) — with the numbers it gives a person, a firm
//! and an account (`identity/papers.rs`), each key computed in `identity/numbers.rs`. ⚠️ An
//! identifier that encodes the person is computed from them, never drawn beside them: the
//! card does not contradict itself.

mod data;
mod numbers;
mod papers;
mod profile;

use std::path::Path;

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use specta::Type;

use self::data::{Land, Library, Names, Place, Pool};
pub use self::data::{ListProblem, UnreadableList};
use self::papers::Bearer;
use self::profile::Profile;
use super::files::{self, FileProblem};
use super::generate::Draw;
use crate::closed_enum::closed_enum;
use crate::count::saturating_u32;

/// How many people one export holds at most.
pub const MAX_PEOPLE: u32 = 100;
/// The ages a person is drawn between, whatever is asked.
pub const AGES: (u32, u32) = (0, 110);

closed_enum! {
    pub enum Country {
        #[default]
        Fr = "FR",
        Be = "BE",
        De = "DE",
        Es = "ES",
        It = "IT",
        Pt = "PT",
        Nl = "NL",
        Ch = "CH",
        Gb = "GB",
        Pl = "PL",
        Se = "SE",
        Us = "US",
        Ca = "CA",
    }
}

closed_enum! {
    pub enum Sex {
        #[default]
        Female = "F",
        Male = "M",
    }
}

closed_enum! {
    /// Their own names in their own country: a label, never a translation.
    pub enum IdentifierKind {
        #[default]
        Nir = "nir",
        Siren = "siren",
        Siret = "siret",
        Vat = "vat",
        Nrn = "nrn",
        Bce = "bce",
        BeVat = "beVat",
        SteuerId = "steuerId",
        UstIdNr = "ustIdNr",
        Dni = "dni",
        NifEs = "nifEs",
        CodiceFiscale = "codiceFiscale",
        PartitaIva = "partitaIva",
        NifPt = "nifPt",
        Nipc = "nipc",
        Bsn = "bsn",
        Kvk = "kvk",
        Btw = "btw",
        Ahv = "ahv",
        Uid = "uid",
        Nino = "nino",
        Crn = "crn",
        GbVat = "gbVat",
        Pesel = "pesel",
        Nip = "nip",
        Regon = "regon",
        Personnummer = "personnummer",
        Orgnr = "orgnr",
        Moms = "moms",
        Ssn = "ssn",
        Sin = "sin",
        SortCode = "sortCode",
        AccountNumber = "accountNumber",
        RoutingNumber = "routingNumber",
        TransitNumber = "transitNumber",
    }
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IdentityRequest {
    pub country: Country,
    /// `None` draws it.
    pub sex: Option<Sex>,
    /// The age reached this year, between the two.
    pub age_min: u32,
    pub age_max: u32,
    /// The same seed and options give the same person; none draws one, and says which.
    pub seed: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Identifier {
    pub kind: IdentifierKind,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Birth {
    /// `YYYY-MM-DD`.
    pub date: String,
    pub city: String,
    pub region: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    /// The number and the street, in the order the country writes them.
    pub street: String,
    pub postcode: String,
    pub city: String,
    pub region: String,
    /// The whole of it on one line, as an envelope of the country carries it.
    pub formatted: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Bank {
    pub name: String,
    /// None in a country that has no IBAN.
    pub iban: Option<String>,
    pub bic: Option<String>,
    /// What the country writes beside an IBAN or instead of one: a sort code, a routing number.
    pub local: Vec<Identifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Company {
    pub name: String,
    pub identifiers: Vec<Identifier>,
}

/// ⚠️ These names are what « Copier en JSON » and an export write: renaming one breaks whoever
/// reads the files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub country: Country,
    pub sex: Sex,
    pub surname: String,
    pub given_names: Vec<String>,
    pub birth: Birth,
    /// A number as a card would carry one: nine characters, of no format that exists.
    pub document: String,
    pub address: Address,
    pub phone: String,
    pub mobile: String,
    pub email: String,
    pub bank: Bank,
    pub identifiers: Vec<Identifier>,
    pub company: Company,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IdentityAnswer {
    /// The seed that was used: the one given, or the one drawn.
    pub seed: String,
    pub person: Identity,
    /// The person as an export writes one, for the clipboard and for a note.
    pub json: String,
    /// Whether the phone numbers are in a range the country keeps for fiction.
    pub fiction_phones: bool,
    /// The numbers of this card that carry no key: a format, and nothing to verify it by.
    pub unkeyed: Vec<IdentifierKind>,
}

closed_enum! {
    pub enum PeopleFormat {
        #[default]
        Json = "json",
        Csv = "csv",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PeopleExport {
    Written {
        people: u32,
    },
    Failed {
        problem: FileProblem,
    },
    /// Nothing was written: a list the people are drawn from cannot be read.
    Unreadable {
        list: UnreadableList,
    },
}

/// A person, or the list that kept one from being drawn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum IdentityOutcome {
    Drawn { answer: Box<IdentityAnswer> },
    Unreadable { list: UnreadableList },
}

/// FNV-1a over the seed as it is written: « devnotes » is a seed like `42` is.
fn seed_number(seed: &str) -> u32 {
    seed.bytes().fold(0x811c_9dc5, |hash: u32, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    })
}

/// The seed asked for, or one drawn from the system and written as a number.
fn seed_text(seed: Option<&str>) -> String {
    if let Some(seed) = seed.map(str::trim).filter(|seed| !seed.is_empty()) {
        return seed.to_owned();
    }
    let mut bytes = [0u8; 4];
    // Without the system's randomness, the seed is still a seed: 0 is as good as any.
    let _ = getrandom::fill(&mut bytes);
    u32::from_le_bytes(bytes).to_string()
}

/// What a person of one country is drawn from.
struct Lists<'a> {
    country: Country,
    profile: &'static Profile,
    library: &'a Library,
    land: &'a Land,
}

impl<'a> Lists<'a> {
    fn of(library: &'a Library, country: Country) -> Self {
        Self {
            country,
            profile: profile::of(country),
            library,
            land: library.land(country),
        }
    }

    fn names(&self, wanted: Pool) -> &Names {
        self.library.names(wanted)
    }
}

/// One of the pools the country draws from, by its share.
fn pool(mix: &[(Pool, u32)], draw: &mut Draw) -> Pool {
    let total: u32 = mix.iter().map(|(_, share)| share).sum();
    let mut at = u32::try_from(draw.below(usize::try_from(total).unwrap_or(1))).unwrap_or(0);
    for (pool, share) in mix {
        if at < *share {
            return *pool;
        }
        at -= share;
    }
    mix[0].0
}

fn days_in(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    next.and_then(|first| first.pred_opt())
        .map_or(28, |last| last.day())
}

/// An address that reaches nobody: the three domains kept for examples.
fn email(given: &str, surname: &str, draw: &mut Draw) -> String {
    let plain = |name: &str| -> String {
        deunicode::deunicode(name)
            .to_lowercase()
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect()
    };
    let domain = draw.pick(&["example.com", "example.org", "example.net"]);
    format!("{}.{}@{domain}", plain(given), plain(surname))
}

/// ⚠️ Every draw is made in one order, whatever was asked: a sex that is imposed still costs
/// its draw, so that the same seed gives the same person but for what the options change.
fn person(lists: &Lists<'_>, request: &IdentityRequest, year: i32, draw: &mut Draw) -> Identity {
    let profile = lists.profile;
    let drawn_sex = if draw.chance(0.5) {
        Sex::Female
    } else {
        Sex::Male
    };
    let sex = request.sex.unwrap_or(drawn_sex);

    let origin = pool(profile.pools, draw);
    let surname = origin.borne(draw.pick(&lists.names(origin).surnames), sex == Sex::Female);
    // A given name mostly of the surname's own pool, sometimes of the country's first one.
    let given_from = if draw.chance(0.25) {
        profile.pools[0].0
    } else {
        origin
    };
    let given = match sex {
        Sex::Female => &lists.names(given_from).female,
        Sex::Male => &lists.names(given_from).male,
    };
    // Three draws whatever the list and however many names are kept: one already taken is
    // stepped past, not drawn again.
    let count = 1 + usize::from(draw.chance(0.5)) + usize::from(draw.chance(0.2));
    let slots = [(); 3].map(|()| draw.below(1 << 20));
    let mut given_names: Vec<String> = Vec::new();
    for slot in slots.into_iter().take(count) {
        let mut at = slot % given.len();
        while given_names.iter().any(|kept| kept == given[at]) {
            at = (at + 1) % given.len();
        }
        given_names.push(given[at].to_owned());
    }

    let (youngest, oldest) = (
        request.age_min.min(request.age_max).clamp(AGES.0, AGES.1),
        request.age_min.max(request.age_max).clamp(AGES.0, AGES.1),
    );
    let age = draw.between(i64::from(youngest), i64::from(oldest));
    let birth_year = year - i32::try_from(age).unwrap_or(0);
    let month = u32::try_from(draw.between(1, 12)).unwrap_or(1);
    let day = u32::try_from(draw.between(1, i64::from(days_in(birth_year, month)))).unwrap_or(1);
    let born_in: Place = *draw.pick(&lists.land.places);
    let birth = Birth {
        date: format!("{birth_year:04}-{month:02}-{day:02}"),
        city: born_in.city.to_owned(),
        region: born_in.region.to_owned(),
    };

    // Half the people still live where they were born.
    let elsewhere: Place = *draw.pick(&lists.land.places);
    let home = if draw.chance(0.5) { born_in } else { elsewhere };
    let street = profile.street(
        draw.between(1, 140),
        draw.pick(lists.land.streets(home.tag)),
    );
    let postcode = Profile::postcode(&home, draw);
    let address = Address {
        formatted: profile.address(&street, &postcode, &home),
        street,
        postcode,
        city: home.city.to_owned(),
        region: home.region.to_owned(),
    };

    let company = Company {
        name: profile.firm(&surname, &home, draw),
        identifiers: papers::firm(lists.country, draw),
    };
    let document: String = (0..9).map(|_| *draw.pick(&DOCUMENT_ALPHABET)).collect();
    let email = email(&given_names[0], &surname, draw);
    let phone = profile.landline(&home, draw);
    let mobile = profile.mobile(&home, draw);
    let bank = papers::bank(lists.country, draw);
    let identifiers = papers::personal(
        lists.country,
        &Bearer {
            sex,
            surname: &surname,
            given: &given_names[0],
            year: birth_year,
            month,
            day,
            born_in: &born_in,
            this_year: year,
        },
        draw,
    );
    Identity {
        country: lists.country,
        sex,
        surname,
        given_names,
        birth,
        document,
        address,
        phone,
        mobile,
        email,
        bank,
        identifiers,
        company,
    }
}

/// No `0`, `O`, `1` nor `I`: read aloud or typed again, each character is one character.
const DOCUMENT_ALPHABET: [char; 32] = [
    '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L',
    'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

/// `count` people from one seed: the first is the one the card shows for it. Several
/// countries take their turn, the request's own first.
fn people(
    request: &IdentityRequest,
    countries: &[Country],
    count: u32,
    today: NaiveDate,
    library: &Library,
) -> (String, Vec<Identity>) {
    let seed = seed_text(request.seed.as_deref());
    let mut draw = Draw::new(seed_number(&seed));
    let mut turns = vec![request.country];
    for country in countries {
        if !turns.contains(country) {
            turns.push(*country);
        }
    }
    let lists: Vec<Lists<'_>> = turns
        .into_iter()
        .map(|country| Lists::of(library, country))
        .collect();
    let count = usize::try_from(count.clamp(1, MAX_PEOPLE)).unwrap_or(1);
    let people = (0..count)
        .map(|at| person(&lists[at % lists.len()], request, today.year(), &mut draw))
        .collect();
    (seed, people)
}

/// `today` is handed in, so the tests fix it. ⚠️ Only its year is read: the same seed gives
/// the same person all year, and the ages are the ones reached during it.
fn drawn(request: &IdentityRequest, today: NaiveDate, library: &Library) -> IdentityAnswer {
    let (seed, mut people) = people(request, &[], 1, today, library);
    let person = people.remove(0);
    let unkeyed = person
        .identifiers
        .iter()
        .chain(&person.bank.local)
        .chain(&person.company.identifiers)
        .map(|identifier| identifier.kind)
        .filter(|kind| papers::UNKEYED.contains(kind))
        .collect();
    IdentityAnswer {
        seed,
        json: serde_json::to_string_pretty(&person).unwrap_or_default(),
        fiction_phones: profile::of(person.country).fiction_phones,
        unkeyed,
        person,
    }
}

/// A person drawn from the lists in `lists`, the folder the application ships — or the list
/// that cannot be read there.
pub fn generate(request: &IdentityRequest, today: NaiveDate, lists: &Path) -> IdentityOutcome {
    match data::library(lists) {
        Ok(library) => IdentityOutcome::Drawn {
            answer: Box::new(drawn(request, today, library)),
        },
        Err(list) => IdentityOutcome::Unreadable { list },
    }
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

/// One row a person, the columns named after the JSON's own fields. ⚠️ The numbers differ
/// from a country to the next, so a column holds them all, `kind:value` and a bar between.
fn csv(people: &[Identity]) -> String {
    let all = |identifiers: &[Identifier]| {
        identifiers
            .iter()
            .map(|identifier| format!("{}:{}", identifier.kind, identifier.value))
            .collect::<Vec<_>>()
            .join("|")
    };
    let mut rows = vec![
        "country,sex,surname,givenNames,birthDate,birthCity,birthRegion,document,address,street,postcode,city,region,phone,mobile,email,bank,iban,bic,account,identifiers,company,companyIdentifiers"
            .to_owned(),
    ];
    for person in people {
        let fields = [
            person.country.to_string(),
            person.sex.to_string(),
            person.surname.clone(),
            person.given_names.join(" "),
            person.birth.date.clone(),
            person.birth.city.clone(),
            person.birth.region.clone(),
            person.document.clone(),
            person.address.formatted.clone(),
            person.address.street.clone(),
            person.address.postcode.clone(),
            person.address.city.clone(),
            person.address.region.clone(),
            person.phone.clone(),
            person.mobile.clone(),
            person.email.clone(),
            person.bank.name.clone(),
            person.bank.iban.clone().unwrap_or_default(),
            person.bank.bic.clone().unwrap_or_default(),
            all(&person.bank.local),
            all(&person.identifiers),
            person.company.name.clone(),
            all(&person.company.identifiers),
        ];
        rows.push(
            fields
                .iter()
                .map(|field| csv_field(field))
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    rows.join("\r\n") + "\r\n"
}

/// What an export writes: a JSON array, or a CSV with its header.
fn exported(
    request: &IdentityRequest,
    batch: &PeopleBatch,
    today: NaiveDate,
    library: &Library,
) -> (String, u32) {
    let (_, people) = people(request, &batch.countries, batch.count, today, library);
    let format = batch.format;
    let text = match format {
        PeopleFormat::Json => serde_json::to_string_pretty(&people).unwrap_or_default() + "\n",
        PeopleFormat::Csv => csv(&people),
    };
    (text, saturating_u32(people.len()))
}

/// What a batch is made of, beside the person asked for.
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PeopleBatch {
    /// The countries that take their turn after the request's own; none is that one alone.
    pub countries: Vec<Country>,
    pub count: u32,
    pub format: PeopleFormat,
}

/// A batch written where the user chose: never a note, a hundred people are a file.
pub fn export(
    request: &IdentityRequest,
    batch: &PeopleBatch,
    path: &str,
    today: NaiveDate,
    lists: &Path,
) -> PeopleExport {
    let library = match data::library(lists) {
        Ok(library) => library,
        Err(list) => return PeopleExport::Unreadable { list },
    };
    let (text, people) = exported(request, batch, today, library);
    match files::write(path, text.as_bytes()) {
        Ok(()) => PeopleExport::Written { people },
        Err(problem) => PeopleExport::Failed { problem },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::numbers::as_digits;
    use super::*;
    use crate::tools::checks::{
        CheckAnswer, CheckKind, CheckRequest, IbanVerdict, check, passes_luhn,
    };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 12).unwrap()
    }

    /// A person from the lists the application ships.
    fn generate(request: &IdentityRequest, today: NaiveDate) -> IdentityAnswer {
        super::drawn(request, today, data::shipped())
    }

    fn exported(
        request: &IdentityRequest,
        countries: &[Country],
        count: u32,
        format: PeopleFormat,
        today: NaiveDate,
    ) -> (String, u32) {
        let batch = PeopleBatch {
            countries: countries.to_vec(),
            count,
            format,
        };
        super::exported(request, &batch, today, data::shipped())
    }

    fn asked_in(country: Country, seed: &str) -> IdentityRequest {
        IdentityRequest {
            country,
            sex: None,
            age_min: 18,
            age_max: 80,
            seed: Some(seed.to_owned()),
        }
    }

    fn asked(seed: &str) -> IdentityRequest {
        asked_in(Country::Fr, seed)
    }

    fn drawn(seed: &str) -> Identity {
        generate(&asked(seed), today()).person
    }

    /// As many people of a country as asked, a seed each, its lists read once.
    fn crowd(country: Country, count: u32) -> Vec<Identity> {
        let lists = Lists::of(data::shipped(), country);
        (0..count)
            .map(|seed| {
                let seed = seed.to_string();
                let mut draw = Draw::new(seed_number(&seed));
                person(&lists, &asked_in(country, &seed), 2026, &mut draw)
            })
            .collect()
    }

    /// The number of that kind, with nothing but its letters and digits.
    fn of(identifiers: &[Identifier], wanted: IdentifierKind) -> String {
        let Some(identifier) = identifiers
            .iter()
            .find(|identifier| identifier.kind == wanted)
        else {
            panic!("no {wanted}");
        };
        identifier
            .value
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect()
    }

    /// Digits as `9`, letters as `A`: what a format looks like.
    fn shape(text: &str) -> String {
        text.chars()
            .map(|character| match character {
                digit if digit.is_ascii_digit() => '9',
                letter if letter.is_ascii_alphabetic() => 'A',
                other => other,
            })
            .collect()
    }

    #[test]
    fn the_same_seed_gives_the_same_person_and_another_gives_another() {
        assert_eq!(drawn("devnotes"), drawn("devnotes"));
        assert_ne!(drawn("devnotes"), drawn("devnotes2"));
        // All year: only the year of « today » is read.
        assert_eq!(
            generate(
                &asked("devnotes"),
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
            )
            .person,
            generate(
                &asked("devnotes"),
                NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()
            )
            .person
        );
    }

    #[test]
    fn a_seed_that_was_drawn_is_said_and_gives_that_person_again() {
        let first = generate(
            &IdentityRequest {
                seed: None,
                ..asked("")
            },
            today(),
        );

        assert!(first.seed.parse::<u32>().is_ok(), "{}", first.seed);
        assert_eq!(drawn(&first.seed), first.person);
        // An empty seed is no seed.
        let blank = generate(&asked("  "), today());
        assert!(blank.seed.parse::<u32>().is_ok());
    }

    /// The tool that checks an IBAN is asked, not a second implementation: its length for the
    /// country, its two digits, and in France the RIB key inside it.
    #[test]
    fn every_iban_is_one_the_checks_tool_finds_valid_and_a_country_without_says_what_it_has() {
        for country in Country::ALL {
            for (seed, person) in crowd(country, 300).iter().enumerate() {
                let Some(iban) = &person.bank.iban else {
                    assert!(matches!(country, Country::Us | Country::Ca), "{country}");
                    assert_eq!(person.bank.bic, None);
                    assert!(person.bank.local.len() >= 2, "{country}");
                    continue;
                };
                let answer = check(&CheckRequest {
                    text: iban.clone(),
                    kind: Some(CheckKind::Iban),
                });
                let CheckAnswer::Iban {
                    verdict: IbanVerdict::Valid { rib_key, .. },
                    ..
                } = answer
                else {
                    panic!("{country} {seed}: {iban} is no valid IBAN: {answer:?}");
                };
                assert!(iban.starts_with(country.as_str()));
                if let Some(rib) = rib_key {
                    assert_eq!(rib.given, rib.expected, "{country} {seed}: the RIB key");
                }
                assert!(matches!(person.bank.bic.as_deref().map(str::len), Some(8)));
            }
        }
    }

    fn holds_in_france(person: &Identity) {
        let siren = of(&person.company.identifiers, IdentifierKind::Siren);
        let establishment = of(&person.company.identifiers, IdentifierKind::Siret);
        assert_eq!(siren.len(), 9);
        assert!(passes_luhn(&as_digits(&siren)), "SIREN {siren}");
        assert_eq!(establishment.len(), 14);
        assert!(establishment.starts_with(&siren));
        assert!(passes_luhn(&as_digits(&establishment)), "{establishment}");
        assert_eq!(
            of(&person.company.identifiers, IdentifierKind::Vat),
            numbers::french_vat(&siren)
        );
        let nir = of(&person.identifiers, IdentifierKind::Nir);
        assert_eq!(nir.len(), 15);
        assert_eq!(numbers::nir_key(&nir[..13]), &nir[13..], "NIR {nir}");
    }

    fn holds_in_the_low_countries(person: &Identity) {
        if person.country == Country::Be {
            let national = of(&person.identifiers, IdentifierKind::Nrn);
            assert_eq!(national.len(), 11);
            assert_eq!(
                numbers::belgian_national_key(&national[..9], person.birth.date.as_str() >= "2000"),
                &national[9..],
                "{national}"
            );
            let enterprise = of(&person.company.identifiers, IdentifierKind::Bce);
            assert_eq!(numbers::belgian_enterprise(&enterprise[..8]), enterprise);
            assert_eq!(
                of(&person.company.identifiers, IdentifierKind::BeVat),
                format!("BE{enterprise}")
            );
            return;
        }
        let citizen = of(&person.identifiers, IdentifierKind::Bsn);
        assert_eq!(citizen.len(), 9);
        assert_eq!(
            numbers::dutch_citizen_key(&citizen[..8]),
            citizen[8..].parse().ok(),
            "BSN {citizen}"
        );
        let vat = of(&person.company.identifiers, IdentifierKind::Btw);
        assert_eq!((&vat[..2], &vat[11..]), ("NL", "B01"), "{vat}");
        assert_eq!(
            numbers::dutch_citizen_key(&vat[2..10]),
            vat[10..11].parse().ok()
        );
        assert_eq!(
            of(&person.company.identifiers, IdentifierKind::Kvk).len(),
            8
        );
        // The account inside the IBAN passes the same proof, weighted from 10 to 1.
        let iban: String = person.bank.iban.clone().unwrap().replace(' ', "");
        assert_eq!(
            numbers::dutch_account_key(&iban[8..17]),
            iban[17..].parse().ok(),
            "{iban}"
        );
    }

    fn holds_in_germany_and_switzerland(person: &Identity) {
        if person.country == Country::De {
            let tax = of(&person.identifiers, IdentifierKind::SteuerId);
            assert_eq!(tax.len(), 11);
            assert!(!tax.starts_with('0'));
            assert_eq!(numbers::mod_11_10(&tax[..10]).to_string(), &tax[10..]);
            // One digit is there twice, one is not there, the eight others once.
            let mut times = [0; 10];
            for digit in as_digits(&tax[..10]) {
                times[digit as usize] += 1;
            }
            times.sort_unstable();
            assert_eq!(times, [0, 1, 1, 1, 1, 1, 1, 1, 1, 2], "{tax}");
            let vat = of(&person.company.identifiers, IdentifierKind::UstIdNr);
            assert_eq!((vat.len(), &vat[..2]), (11, "DE"));
            assert_eq!(numbers::mod_11_10(&vat[2..10]).to_string(), &vat[10..]);
            return;
        }
        let insured = of(&person.identifiers, IdentifierKind::Ahv);
        assert_eq!((insured.len(), &insured[..3]), (13, "756"));
        assert_eq!(
            numbers::ean_13_key(&insured[..12]).to_string(),
            &insured[12..]
        );
        let company = of(&person.company.identifiers, IdentifierKind::Uid);
        assert_eq!((company.len(), &company[..3]), (12, "CHE"));
        assert_eq!(
            numbers::swiss_company_key(&company[3..11]),
            company[11..].parse().ok()
        );
    }

    fn holds_in_the_south(person: &Identity) {
        match person.country {
            Country::Es => {
                let dni = of(&person.identifiers, IdentifierKind::Dni);
                assert_eq!(dni.len(), 9);
                assert_eq!(
                    numbers::dni_letter(dni[..8].parse().unwrap()).to_string(),
                    &dni[8..]
                );
                let company = of(&person.company.identifiers, IdentifierKind::NifEs);
                assert_eq!((company.len(), &company[..1]), (9, "B"));
                assert_eq!(
                    numbers::spanish_company_key(&company[1..8]).to_string(),
                    &company[8..]
                );
                let iban = person.bank.iban.clone().unwrap().replace(' ', "");
                assert_eq!(
                    numbers::spanish_account_key(&iban[4..8], &iban[8..12], &iban[14..]),
                    &iban[12..14],
                    "{iban}"
                );
            }
            Country::It => {
                let code = of(&person.identifiers, IdentifierKind::CodiceFiscale);
                assert_eq!(code.len(), 16);
                assert_eq!(
                    numbers::italian_control(&code[..15]).to_string(),
                    &code[15..]
                );
                let vat = of(&person.company.identifiers, IdentifierKind::PartitaIva);
                assert_eq!(vat.len(), 11);
                assert!(passes_luhn(&as_digits(&vat)), "{vat}");
                let iban = person.bank.iban.clone().unwrap().replace(' ', "");
                assert_eq!(
                    numbers::italian_control(&iban[5..]).to_string(),
                    &iban[4..5]
                );
            }
            _ => {
                for (number, first) in [
                    (of(&person.identifiers, IdentifierKind::NifPt), "12"),
                    (of(&person.company.identifiers, IdentifierKind::Nipc), "5"),
                ] {
                    assert_eq!(number.len(), 9);
                    assert!(first.contains(&number[..1]), "{number}");
                    assert_eq!(
                        numbers::portuguese_key(&number[..8]).to_string(),
                        &number[8..]
                    );
                }
                let iban = person.bank.iban.clone().unwrap().replace(' ', "");
                assert_eq!(numbers::portuguese_account_key(&iban[4..23]), &iban[23..]);
            }
        }
    }

    fn holds_in_the_north(person: &Identity) {
        match person.country {
            Country::Gb => {
                let vat = of(&person.company.identifiers, IdentifierKind::GbVat);
                assert_eq!((vat.len(), &vat[..2]), (11, "GB"));
                assert_eq!(numbers::british_vat_key(&vat[2..9]), &vat[9..]);
                assert_eq!(
                    shape(&person.identifiers[0].value),
                    "AA 99 99 99 A",
                    "{:?}",
                    person.identifiers
                );
                assert_eq!(
                    of(&person.company.identifiers, IdentifierKind::Crn).len(),
                    8
                );
                // The sort code and the account are the ones inside the IBAN.
                let iban = person.bank.iban.clone().unwrap().replace(' ', "");
                assert_eq!(
                    of(&person.bank.local, IdentifierKind::SortCode),
                    &iban[8..14]
                );
                assert_eq!(
                    of(&person.bank.local, IdentifierKind::AccountNumber),
                    &iban[14..]
                );
            }
            Country::Pl => {
                let pesel = of(&person.identifiers, IdentifierKind::Pesel);
                assert_eq!(pesel.len(), 11);
                assert_eq!(numbers::pesel_key(&pesel[..10]).to_string(), &pesel[10..]);
                let nip = of(&person.company.identifiers, IdentifierKind::Nip);
                assert_eq!(nip.len(), 10);
                assert_eq!(numbers::nip_key(&nip[..9]), nip[9..].parse().ok());
                let regon = of(&person.company.identifiers, IdentifierKind::Regon);
                assert_eq!(regon.len(), 9);
                assert_eq!(numbers::regon_key(&regon[..8]).to_string(), &regon[8..]);
                let iban = person.bank.iban.clone().unwrap().replace(' ', "");
                assert_eq!(
                    numbers::polish_branch_key(&iban[4..11]).to_string(),
                    &iban[11..12]
                );
            }
            _ => {
                let number = of(&person.identifiers, IdentifierKind::Personnummer);
                assert_eq!(number.len(), 10);
                assert!(passes_luhn(&as_digits(&number)), "{number}");
                let company = of(&person.company.identifiers, IdentifierKind::Orgnr);
                assert_eq!(company.len(), 10);
                assert!(passes_luhn(&as_digits(&company)), "{company}");
                assert!(&company[2..3] >= "2");
                assert_eq!(
                    of(&person.company.identifiers, IdentifierKind::Moms),
                    format!("SE{company}01")
                );
            }
        }
    }

    /// ⚠️ Harmless by construction, where a country allows it: the ten social security numbers
    /// kept for advertisements, and a Canadian number no one is given.
    fn holds_in_north_america(person: &Identity) {
        assert!(person.company.identifiers.is_empty());
        if person.country == Country::Us {
            let number = &person.identifiers[0];
            assert_eq!(number.kind, IdentifierKind::Ssn);
            assert!(number.value.starts_with("987-65-432"), "{}", number.value);
            assert_eq!(number.value.len(), 11);
            let routing = of(&person.bank.local, IdentifierKind::RoutingNumber);
            assert!(numbers::routing_number_holds(&routing), "{routing}");
            return;
        }
        let number = of(&person.identifiers, IdentifierKind::Sin);
        assert_eq!(number.len(), 9);
        assert!(number.starts_with('0'));
        assert!(passes_luhn(&as_digits(&number)), "{number}");
        assert_eq!(
            shape(&person.bank.local[0].value),
            "99999-999",
            "{:?}",
            person.bank.local
        );
    }

    /// Each number with a key carries the one its country computes — verified by the key's own
    /// function, itself held to a published example in `numbers.rs`.
    #[test]
    fn every_number_with_a_key_carries_it_over_a_thousand_seeds_a_country() {
        for country in Country::ALL {
            for person in &crowd(country, 1000) {
                match country {
                    Country::Fr => holds_in_france(person),
                    Country::Be | Country::Nl => holds_in_the_low_countries(person),
                    Country::De | Country::Ch => holds_in_germany_and_switzerland(person),
                    Country::Es | Country::It | Country::Pt => holds_in_the_south(person),
                    Country::Gb | Country::Pl | Country::Se => holds_in_the_north(person),
                    Country::Us | Country::Ca => holds_in_north_america(person),
                }
            }
        }
    }

    /// The card never contradicts itself: what a number encodes is the person's own.
    #[test]
    fn a_number_that_encodes_the_person_is_read_off_them() {
        let year = |person: &Identity| person.birth.date[2..4].to_owned();
        let month = |person: &Identity| person.birth.date[5..7].parse::<u32>().unwrap();
        let day = |person: &Identity| person.birth.date[8..10].to_owned();
        let odd = |digit: &str| digit.parse::<u32>().unwrap() % 2 == 1;

        for person in &crowd(Country::Fr, 500) {
            let nir = of(&person.identifiers, IdentifierKind::Nir);
            assert_eq!(&nir[..1], if person.sex == Sex::Male { "1" } else { "2" });
            assert_eq!(&nir[1..3], year(person));
            assert_eq!(nir[3..5].parse::<u32>().unwrap(), month(person));
            let born_in = place_of(Country::Fr, &person.birth.city);
            assert_eq!(&nir[5..10], born_in.code, "the commune of birth");
        }
        for person in &crowd(Country::Be, 500) {
            let national = of(&person.identifiers, IdentifierKind::Nrn);
            assert_eq!(&national[..2], year(person));
            assert_eq!(national[2..4].parse::<u32>().unwrap(), month(person));
            assert_eq!(&national[4..6], day(person));
            assert_eq!(odd(&national[6..9]), person.sex == Sex::Male, "{national}");
        }
        for person in &crowd(Country::It, 500) {
            let code = of(&person.identifiers, IdentifierKind::CodiceFiscale);
            assert_eq!(&code[6..8], year(person));
            let letter = "ABCDEHLMPRST"
                .chars()
                .nth(usize::try_from(month(person)).unwrap() - 1)
                .unwrap();
            assert_eq!(&code[8..9], letter.to_string());
            let written: u32 = code[9..11].parse().unwrap();
            let born: u32 = day(person).parse().unwrap();
            assert_eq!(
                written,
                born + if person.sex == Sex::Female { 40 } else { 0 }
            );
            assert_eq!(
                &code[11..15],
                place_of(Country::It, &person.birth.city).code
            );
        }
        for person in &crowd(Country::Pl, 500) {
            let pesel = of(&person.identifiers, IdentifierKind::Pesel);
            let century = if person.birth.date.as_str() >= "2000" {
                20
            } else {
                0
            };
            assert_eq!(&pesel[..2], year(person));
            assert_eq!(pesel[2..4].parse::<u32>().unwrap(), month(person) + century);
            assert_eq!(&pesel[4..6], day(person));
            assert_eq!(odd(&pesel[9..10]), person.sex == Sex::Male, "{pesel}");
        }
        for person in &crowd(Country::Se, 500) {
            let number = of(&person.identifiers, IdentifierKind::Personnummer);
            assert_eq!(&number[..2], year(person));
            assert_eq!(number[2..4].parse::<u32>().unwrap(), month(person));
            assert_eq!(&number[4..6], day(person));
            assert_eq!(odd(&number[8..9]), person.sex == Sex::Male, "{number}");
            assert!(person.identifiers[0].value.contains('-'));
        }
    }

    fn place_of(country: Country, city: &str) -> Place {
        *data::shipped()
            .land(country)
            .places
            .iter()
            .find(|place| place.city == city)
            .unwrap()
    }

    #[test]
    fn a_swede_of_a_hundred_writes_a_plus_where_there_was_a_hyphen() {
        let request = IdentityRequest {
            age_min: 100,
            age_max: 105,
            ..asked_in(Country::Se, "old")
        };
        let person = generate(&request, today()).person;
        assert!(person.identifiers[0].value.contains('+'), "{person:?}");
    }

    /// The postcode belongs to the city and the region to the postcode, in the country's own
    /// format; the address is written in its order.
    #[test]
    fn an_address_is_of_its_country_in_format_and_in_order() {
        const POSTCODES: &[(Country, &[&str])] = &[
            (Country::Fr, &["99999"]),
            (Country::Be, &["9999"]),
            (Country::De, &["99999"]),
            (Country::Es, &["99999"]),
            (Country::It, &["99999"]),
            (Country::Pt, &["9999-999"]),
            (Country::Nl, &["9999 AA"]),
            (Country::Ch, &["9999"]),
            (
                Country::Gb,
                &["A9 9AA", "A99 9AA", "AA9 9AA", "AA99 9AA", "AA9A 9AA"],
            ),
            (Country::Pl, &["99-999"]),
            (Country::Se, &["999 99"]),
            (Country::Us, &["99999"]),
            (Country::Ca, &["A9A 9A9"]),
        ];
        for (country, shapes) in POSTCODES {
            let land = data::shipped().land(*country);
            for person in &crowd(*country, 300) {
                let address = &person.address;
                assert!(
                    shapes.contains(&shape(&address.postcode).as_str()),
                    "{country}: {}",
                    address.postcode
                );
                let home = place_of(*country, &address.city);
                assert_eq!(address.region, home.region);
                assert_eq!(
                    address.postcode.split(['#', '@']).next(),
                    Some(&address.postcode[..]),
                    "nothing is left to draw"
                );
                assert!(address.formatted.starts_with(&address.street));
                assert!(address.formatted.contains(&address.postcode));
                assert!(address.formatted.contains(&address.city));
                assert!(
                    land.streets(home.tag).iter().any(|street| {
                        address.street.starts_with(&format!("{street} "))
                            || address.street.ends_with(&format!(" {street}"))
                    }),
                    "{country}: « {} » is on no street of {}",
                    address.street,
                    home.city
                );
            }
        }
    }

    #[test]
    fn a_phone_number_has_the_shape_its_country_writes() {
        const PHONES: &[(Country, &[&str], &[&str])] = &[
            (Country::Fr, &["99 99 99 99 99"], &["99 99 99 99 99"]),
            (
                Country::Be,
                &["99 999 99 99", "999 99 99 99"],
                &["9999 99 99 99"],
            ),
            (
                Country::De,
                &["999 9999999", "9999 9999999", "99999 9999999"],
                &["9999 99999999", "9999 9999999"],
            ),
            (Country::Es, &["999 999 999"], &["999 999 999"]),
            (
                Country::It,
                &["99 99999999", "999 9999999", "9999 999999"],
                &["999 9999999"],
            ),
            (Country::Pt, &["999 999 999"], &["999 999 999"]),
            (
                Country::Nl,
                &["999 999 9999", "9999 99 99 99"],
                &["99 99999999"],
            ),
            (Country::Ch, &["999 999 99 99"], &["999 999 99 99"]),
            (
                Country::Gb,
                &["999 9999 9999", "9999 999 9999", "99999 999999"],
                &["99999 999999"],
            ),
            (Country::Pl, &["99 999 99 99"], &["999 999 999"]),
            (
                Country::Se,
                &["99-999 999 99", "999-999 99 99", "9999-99 99 99"],
                &["999-999 99 99"],
            ),
            (Country::Us, &["(999) 999-9999"], &["(999) 999-9999"]),
            (Country::Ca, &["(999) 999-9999"], &["(999) 999-9999"]),
        ];
        for (country, landlines, mobiles) in PHONES {
            for person in &crowd(*country, 300) {
                assert!(
                    landlines.contains(&shape(&person.phone).as_str()),
                    "{country}: {}",
                    person.phone
                );
                assert!(
                    mobiles.contains(&shape(&person.mobile).as_str()),
                    "{country}: {}",
                    person.mobile
                );
                // The landline starts with what its own town writes first.
                let home = place_of(*country, &person.address.city);
                assert!(
                    person.phone.trim_start_matches('(').starts_with(home.more),
                    "{country}: {} in {}",
                    person.phone,
                    home.city
                );
            }
        }
    }

    /// ⚠️ Harmless where a range is kept for it: an address that reaches nobody everywhere, and
    /// the numbers a regulator keeps for fiction in the four countries that have some.
    #[test]
    fn the_contact_reaches_nobody_where_a_range_is_kept_for_that() {
        for country in Country::ALL {
            for person in &crowd(country, 200) {
                let (local, domain) = person.email.split_once('@').unwrap();
                assert!(["example.com", "example.org", "example.net"].contains(&domain));
                assert!(
                    !local.is_empty()
                        && local.chars().all(|character| character.is_ascii_lowercase()
                            || character.is_ascii_digit()
                            || character == '.'),
                    "{country}: {}",
                    person.email
                );
            }
        }
        for person in &crowd(Country::Fr, 300) {
            assert!(person.mobile.starts_with("06 39 98 "), "{}", person.mobile);
            assert!(
                [
                    "01 99 00 ",
                    "02 61 91 ",
                    "03 53 01 ",
                    "04 65 71 ",
                    "05 36 49 "
                ]
                .iter()
                .any(|block| person.phone.starts_with(block)),
                "{}",
                person.phone
            );
        }
        for person in &crowd(Country::Gb, 300) {
            assert!(person.mobile.starts_with("07700 900"), "{}", person.mobile);
            assert!(
                [
                    "01632 960",
                    " 496 0",
                    " 498 0",
                    "020 7946 0",
                    "028 9649 6",
                    "029 2018 0"
                ]
                .iter()
                .any(|block| person.phone.contains(block)),
                "{}",
                person.phone
            );
        }
        for country in [Country::Us, Country::Ca] {
            for person in &crowd(country, 300) {
                assert!(person.phone.contains(" 555-01"), "{}", person.phone);
                assert!(person.mobile.contains(" 555-01"), "{}", person.mobile);
            }
        }
        let fiction: Vec<Country> = Country::ALL
            .into_iter()
            .filter(|country| generate(&asked_in(*country, "a"), today()).fiction_phones)
            .collect();
        assert_eq!(
            fiction,
            [Country::Fr, Country::Gb, Country::Us, Country::Ca]
        );
    }

    /// What has no key is said, so that nobody takes a format for a number that was checked.
    #[test]
    fn a_card_names_its_numbers_that_carry_no_key() {
        let unkeyed = |country| generate(&asked_in(country, "a"), today()).unkeyed;

        assert!(unkeyed(Country::Fr).is_empty());
        assert!(unkeyed(Country::It).is_empty());
        assert_eq!(
            unkeyed(Country::Gb),
            [
                IdentifierKind::Nino,
                IdentifierKind::SortCode,
                IdentifierKind::AccountNumber,
                IdentifierKind::Crn
            ]
        );
        assert_eq!(unkeyed(Country::Nl), [IdentifierKind::Kvk]);
        assert_eq!(
            unkeyed(Country::Us),
            [IdentifierKind::Ssn, IdentifierKind::AccountNumber]
        );
        assert_eq!(
            unkeyed(Country::Ca),
            [IdentifierKind::TransitNumber, IdentifierKind::AccountNumber]
        );
    }

    #[test]
    fn the_age_reached_this_year_stays_between_the_two_bounds_whichever_comes_first() {
        let years = |min: u32, max: u32| -> HashSet<i32> {
            (0..400)
                .map(|seed| {
                    let request = IdentityRequest {
                        age_min: min,
                        age_max: max,
                        ..asked(&seed.to_string())
                    };
                    2026 - generate(&request, today()).person.birth.date[..4]
                        .parse::<i32>()
                        .unwrap()
                })
                .collect()
        };

        assert_eq!(years(30, 30), HashSet::from([30]));
        assert_eq!(years(20, 22), HashSet::from([20, 21, 22]));
        assert_eq!(years(22, 20), HashSet::from([20, 21, 22]));
        // Past what a person lives, the bound is the tool's own.
        assert!(years(109, 500).iter().all(|age| (109..=110).contains(age)));
    }

    #[test]
    fn a_sex_that_is_asked_is_the_sex_and_changes_nothing_else_of_the_draw() {
        for (seed, sex) in [("a", Sex::Female), ("b", Sex::Male), ("c", Sex::Female)] {
            let free = drawn(seed);
            let asked = generate(
                &IdentityRequest {
                    sex: Some(sex),
                    ..asked(seed)
                },
                today(),
            )
            .person;

            assert_eq!(asked.sex, sex);
            assert_eq!(asked.surname, free.surname);
            assert_eq!(asked.birth.date, free.birth.date);
            assert_eq!(asked.address, free.address);
        }
        let women: Vec<Sex> = (0..50)
            .map(|seed| {
                generate(
                    &IdentityRequest {
                        sex: Some(Sex::Female),
                        ..asked(&seed.to_string())
                    },
                    today(),
                )
                .person
                .sex
            })
            .collect();
        assert!(women.iter().all(|sex| *sex == Sex::Female));
    }

    /// Diversity is checked, not hoped for: in every country, every pool its profile mixes
    /// shows, in about the stated proportions.
    #[test]
    fn two_thousand_people_of_each_country_carry_surnames_from_every_pool_in_about_its_share() {
        let pools: Vec<(Pool, &Names)> = Pool::ALL
            .iter()
            .map(|pool| (*pool, data::shipped().names(*pool)))
            .collect();
        // A Polish woman's surname is looked up as the list writes it.
        let origin = |surname: &str| {
            let listed = surname
                .strip_suffix('a')
                .map_or_else(|| surname.to_owned(), |stem| format!("{stem}i"));
            let Some((pool, _)) = pools.iter().find(|(_, names)| {
                names.surnames.contains(&surname) || names.surnames.contains(&listed.as_str())
            }) else {
                panic!("{surname} is in no pool");
            };
            *pool
        };

        for country in Country::ALL {
            let mut counts: HashMap<Pool, u32> = HashMap::new();
            for person in crowd(country, 2000) {
                *counts.entry(origin(&person.surname)).or_default() += 1;
            }

            let mix = profile::of(country).pools;
            assert_eq!(counts.len(), mix.len(), "{country}: {counts:?}");
            for (pool, share) in mix {
                let seen = counts.get(pool).copied().unwrap_or(0);
                let expected = share * 20;
                assert!(
                    seen * 2 > expected && seen < expected * 2,
                    "{country} {pool:?}: {seen} of 2000, about {expected} expected"
                );
            }
        }
    }

    #[test]
    fn a_polish_woman_bears_the_surname_in_its_feminine() {
        let women: Vec<String> = crowd(Country::Pl, 400)
            .into_iter()
            .filter(|person| person.sex == Sex::Female)
            .map(|person| person.surname)
            .collect();

        assert!(women.iter().any(|surname| surname.ends_with("ska")));
        assert!(
            !women
                .iter()
                .any(|surname| surname.ends_with("ski") || surname.ends_with("cki")),
            "{women:?}"
        );
    }

    #[test]
    fn a_thousand_people_are_born_in_every_place_and_none_takes_them_all() {
        for country in Country::ALL {
            let mut born: HashMap<String, u32> = HashMap::new();
            let mut moved = 0;
            for person in crowd(country, 1000) {
                if person.birth.city != person.address.city {
                    moved += 1;
                }
                *born.entry(person.birth.city).or_default() += 1;
            }

            assert_eq!(
                born.len(),
                data::shipped().land(country).places.len(),
                "{country}"
            );
            let most = born.values().max().unwrap();
            assert!(*most < 80, "{country}: one place takes {most} of 1000");
            assert!(moved > 350 && moved < 650, "{country}: {moved} moved");
        }
    }

    #[test]
    fn a_person_has_one_to_three_given_names_each_once_and_a_document_number() {
        let mut counts = HashSet::new();
        for person in crowd(Country::Fr, 300) {
            counts.insert(person.given_names.len());
            assert_eq!(
                person.given_names.iter().collect::<HashSet<_>>().len(),
                person.given_names.len()
            );
            assert_eq!(person.document.chars().count(), 9);
            assert!(!person.document.contains(['0', 'O', '1', 'I']));
            assert!(person.company.name.starts_with(&person.surname));
        }
        assert_eq!(counts, HashSet::from([1, 2, 3]));
    }

    /// What « Copier en JSON » writes: stable names, and the very person of the card.
    #[test]
    fn the_json_names_its_fields_as_the_note_documents_them() {
        let answer = generate(&asked("devnotes"), today());
        let json: serde_json::Value = serde_json::from_str(&answer.json).unwrap();

        for field in [
            "country",
            "sex",
            "surname",
            "givenNames",
            "birth",
            "document",
            "address",
            "phone",
            "mobile",
            "email",
            "bank",
            "identifiers",
            "company",
        ] {
            assert!(json.get(field).is_some(), "{field} is missing");
        }
        assert_eq!(json["country"], "FR");
        assert_eq!(
            json["bank"]["iban"].as_str(),
            answer.person.bank.iban.as_deref()
        );
        assert_eq!(
            json["address"]["formatted"],
            answer.person.address.formatted
        );
        assert_eq!(json["identifiers"][0]["kind"], "nir");
        assert_eq!(json["company"]["identifiers"][2]["kind"], "vat");

        // A country without an IBAN writes none, and what it has instead.
        let american = generate(&asked_in(Country::Us, "devnotes"), today());
        let json: serde_json::Value = serde_json::from_str(&american.json).unwrap();
        assert!(json["bank"]["iban"].is_null());
        assert_eq!(json["bank"]["local"][0]["kind"], "routingNumber");
        assert_eq!(json["company"]["identifiers"], serde_json::json!([]));
    }

    #[test]
    fn an_export_holds_as_many_people_as_asked_the_first_being_the_card_s() {
        let (json, count) = exported(&asked("devnotes"), &[], 5, PeopleFormat::Json, today());
        let people: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!((people.len(), count), (5, 5));
        assert_eq!(people[0]["surname"], drawn("devnotes").surname);
        assert_ne!(people[0], people[1]);
        assert!(people.iter().all(|person| person["country"] == "FR"));

        // No fewer than one, no more than a hundred.
        let held = |count| exported(&asked("x"), &[], count, PeopleFormat::Json, today()).1;
        assert_eq!((held(0), held(5000)), (1, 100));
    }

    /// Several countries take their turn, the card's own first — named again, it is not twice
    /// in the round.
    #[test]
    fn an_export_of_several_countries_gives_each_its_turn() {
        let (json, _) = exported(
            &asked("devnotes"),
            &[Country::It, Country::Fr, Country::Us],
            7,
            PeopleFormat::Json,
            today(),
        );
        let people: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        let countries: Vec<&str> = people
            .iter()
            .map(|person| person["country"].as_str().unwrap())
            .collect();

        assert_eq!(countries, ["FR", "IT", "US", "FR", "IT", "US", "FR"]);
        assert_eq!(people[0]["surname"], drawn("devnotes").surname);
        assert_eq!(people[1]["identifiers"][0]["kind"], "codiceFiscale");
        assert!(people[2]["bank"]["iban"].is_null());
    }

    #[test]
    fn a_csv_has_a_header_a_row_a_person_and_quotes_what_holds_a_comma() {
        let (csv, _) = exported(
            &asked("devnotes"),
            &[Country::Gb],
            4,
            PeopleFormat::Csv,
            today(),
        );
        let rows: Vec<&str> = csv.trim_end().split("\r\n").collect();

        assert_eq!(rows.len(), 5);
        assert!(rows[0].starts_with("country,sex,surname,givenNames,birthDate"));
        assert!(rows[0].ends_with("account,identifiers,company,companyIdentifiers"));
        assert!(rows[1].starts_with("FR,"));
        // The address holds a comma, and is quoted; the numbers are named, a bar between.
        assert!(
            rows[1].contains(",\"") && rows[1].contains("nir:"),
            "{}",
            rows[1]
        );
        assert!(rows[1].contains("|siret:") && rows[1].contains("|vat:FR"));
        assert!(rows[2].starts_with("GB,") && rows[2].contains("sortCode:"));
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("plain"), "plain");
    }

    #[test]
    fn an_export_is_written_where_asked_or_says_why_not() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("people.csv");
        let batch = |format| PeopleBatch {
            countries: Vec::new(),
            count: 2,
            format,
        };

        let lists = data::shipped_folder();
        assert_eq!(
            export(
                &asked("devnotes"),
                &batch(PeopleFormat::Csv),
                &path.to_string_lossy(),
                today(),
                &lists
            ),
            PeopleExport::Written { people: 2 }
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 3);
        assert_eq!(
            export(
                &asked("devnotes"),
                &batch(PeopleFormat::Json),
                &folder
                    .path()
                    .join("absent")
                    .join("people.json")
                    .to_string_lossy(),
                today(),
                &lists
            ),
            PeopleExport::Failed {
                problem: FileProblem::Unwritable
            }
        );
    }

    /// ⚠️ The lists are files beside the application, no longer part of it: a folder that does
    /// not hold them is said — to the card and to an export, which then writes nothing.
    #[test]
    fn a_folder_without_its_lists_is_said_and_the_shipped_one_draws_the_same_person() {
        let request = asked("devnotes");
        assert_eq!(
            super::generate(&request, today(), &data::shipped_folder()),
            IdentityOutcome::Drawn {
                answer: Box::new(generate(&request, today()))
            }
        );

        // Asked first of a folder that holds nothing: nothing is kept of a failed reading.
        let empty = tempfile::tempdir().unwrap();
        let Err(list) = data::read(empty.path()) else {
            panic!("an empty folder holds no list");
        };
        assert_eq!(list.problem, ListProblem::Missing);
        assert_eq!(list.file, "pools/french.txt");
        assert_eq!(list.folder, empty.path().to_string_lossy());
    }
}
