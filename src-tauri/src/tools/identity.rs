//! A fictitious person whose every identifier passes its own check: test data for a form or a
//! database, drawn from a seed so that the same one comes back. Names come from pools shared
//! between countries and a country is a profile — a mix of pools, its places, its formats
//! (`identity/data.rs`). ⚠️ An identifier that encodes the person is computed from them, never
//! drawn beside them: the card does not contradict itself.

mod data;
mod france;

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use specta::Type;

use self::data::{Land, Names, Place, Pool};
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
    pub street: String,
    pub postcode: String,
    pub city: String,
    pub region: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Bank {
    pub name: String,
    pub iban: String,
    pub bic: String,
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
    Written { people: u32 },
    Failed { problem: FileProblem },
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

struct Lists {
    names: Vec<(Pool, Names)>,
    land: Land,
}

impl Lists {
    fn of(country: Country) -> Self {
        Self {
            names: Pool::ALL.iter().map(|pool| (*pool, pool.names())).collect(),
            land: data::land(country),
        }
    }

    fn names(&self, wanted: Pool) -> &Names {
        self.names
            .iter()
            .find(|(pool, _)| *pool == wanted)
            .map_or(&self.names[0].1, |(_, names)| names)
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
fn person(lists: &Lists, request: &IdentityRequest, year: i32, draw: &mut Draw) -> Identity {
    let drawn_sex = if draw.chance(0.5) {
        Sex::Female
    } else {
        Sex::Male
    };
    let sex = request.sex.unwrap_or(drawn_sex);

    let origin = pool(france::POOLS, draw);
    let surname = (*draw.pick(&lists.names(origin).surnames)).to_owned();
    // A given name mostly of the surname's own pool, sometimes of the country's first one.
    let given_from = if draw.chance(0.25) {
        france::POOLS[0].0
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
    let address = Address {
        street: format!(
            "{} {}",
            draw.between(1, 140),
            draw.pick(&lists.land.streets)
        ),
        postcode: home.postcode.to_owned(),
        city: home.city.to_owned(),
        region: home.region.to_owned(),
    };

    let (company, company_identifiers) = france::company(&surname, draw);
    let document: String = (0..9).map(|_| *draw.pick(&DOCUMENT_ALPHABET)).collect();
    Identity {
        country: request.country,
        sex,
        email: email(&given_names[0], &surname, draw),
        phone: france::landline(&home, draw),
        mobile: france::mobile(draw),
        bank: france::bank(draw),
        identifiers: vec![Identifier {
            kind: IdentifierKind::Nir,
            value: france::nir(sex, &birth, &born_in, draw),
        }],
        company: Company {
            name: company,
            identifiers: company_identifiers,
        },
        surname,
        given_names,
        birth,
        document,
        address,
    }
}

/// No `0`, `O`, `1` nor `I`: read aloud or typed again, each character is one character.
const DOCUMENT_ALPHABET: [char; 32] = [
    '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L',
    'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

/// `count` people from one seed: the first is the one the card shows for it.
fn people(request: &IdentityRequest, count: u32, today: NaiveDate) -> (String, Vec<Identity>) {
    let seed = seed_text(request.seed.as_deref());
    let mut draw = Draw::new(seed_number(&seed));
    let lists = Lists::of(request.country);
    let people = (0..count.clamp(1, MAX_PEOPLE))
        .map(|_| person(&lists, request, today.year(), &mut draw))
        .collect();
    (seed, people)
}

/// `today` is handed in, so the tests fix it. ⚠️ Only its year is read: the same seed gives
/// the same person all year, and the ages are the ones reached during it.
pub fn generate(request: &IdentityRequest, today: NaiveDate) -> IdentityAnswer {
    let (seed, mut people) = people(request, 1, today);
    let person = people.remove(0);
    IdentityAnswer {
        seed,
        json: serde_json::to_string_pretty(&person).unwrap_or_default(),
        person,
    }
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

/// One row a person, the columns named after the JSON's own fields.
fn csv(people: &[Identity]) -> String {
    let of = |identifiers: &[Identifier], wanted: IdentifierKind| {
        identifiers
            .iter()
            .find(|identifier| identifier.kind == wanted)
            .map(|identifier| identifier.value.clone())
            .unwrap_or_default()
    };
    let mut rows = vec![
        "country,sex,surname,givenNames,birthDate,birthCity,birthRegion,document,street,postcode,city,region,phone,mobile,email,bank,iban,bic,nir,company,siren,siret,vat"
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
            person.address.street.clone(),
            person.address.postcode.clone(),
            person.address.city.clone(),
            person.address.region.clone(),
            person.phone.clone(),
            person.mobile.clone(),
            person.email.clone(),
            person.bank.name.clone(),
            person.bank.iban.clone(),
            person.bank.bic.clone(),
            of(&person.identifiers, IdentifierKind::Nir),
            person.company.name.clone(),
            of(&person.company.identifiers, IdentifierKind::Siren),
            of(&person.company.identifiers, IdentifierKind::Siret),
            of(&person.company.identifiers, IdentifierKind::Vat),
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
pub fn exported(
    request: &IdentityRequest,
    count: u32,
    format: PeopleFormat,
    today: NaiveDate,
) -> (String, u32) {
    let (_, people) = people(request, count, today);
    let text = match format {
        PeopleFormat::Json => serde_json::to_string_pretty(&people).unwrap_or_default() + "\n",
        PeopleFormat::Csv => csv(&people),
    };
    (text, saturating_u32(people.len()))
}

/// A batch written where the user chose: never a note, a hundred people are a file.
pub fn export(
    request: &IdentityRequest,
    count: u32,
    format: PeopleFormat,
    path: &str,
    today: NaiveDate,
) -> PeopleExport {
    let (text, people) = exported(request, count, format, today);
    match files::write(path, text.as_bytes()) {
        Ok(()) => PeopleExport::Written { people },
        Err(problem) => PeopleExport::Failed { problem },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use crate::tools::checks::{
        CheckAnswer, CheckKind, CheckRequest, IbanVerdict, check, passes_luhn,
    };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 12).unwrap()
    }

    fn asked(seed: &str) -> IdentityRequest {
        IdentityRequest {
            country: Country::Fr,
            sex: None,
            age_min: 18,
            age_max: 80,
            seed: Some(seed.to_owned()),
        }
    }

    fn drawn(seed: &str) -> Identity {
        generate(&asked(seed), today()).person
    }

    fn of(identifiers: &[Identifier], wanted: IdentifierKind) -> String {
        identifiers
            .iter()
            .find(|identifier| identifier.kind == wanted)
            .map(|identifier| identifier.value.replace(' ', ""))
            .unwrap()
    }

    fn numbers(text: &str) -> Vec<u32> {
        text.chars()
            .filter_map(|digit| digit.to_digit(10))
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

    /// The tool that checks an IBAN and a Luhn number is asked, not a second implementation.
    #[test]
    fn every_identifier_passes_its_own_check_over_a_thousand_seeds() {
        for seed in 0..1000 {
            let person = drawn(&seed.to_string());

            let iban = check(&CheckRequest {
                text: person.bank.iban.clone(),
                kind: Some(CheckKind::Iban),
            });
            let CheckAnswer::Iban {
                verdict: IbanVerdict::Valid { rib_key, .. },
                ..
            } = iban
            else {
                panic!("{seed}: {} is no valid IBAN: {iban:?}", person.bank.iban);
            };
            let rib = rib_key.unwrap();
            assert_eq!(rib.given, rib.expected, "{seed}: the RIB key");
            assert_eq!(person.bank.bic.len(), 8);

            let siren = of(&person.company.identifiers, IdentifierKind::Siren);
            let establishment = of(&person.company.identifiers, IdentifierKind::Siret);
            let vat = of(&person.company.identifiers, IdentifierKind::Vat);
            assert_eq!(siren.len(), 9);
            assert!(passes_luhn(&numbers(&siren)), "{seed}: SIREN {siren}");
            assert_eq!(establishment.len(), 14);
            assert!(establishment.starts_with(&siren));
            assert!(
                passes_luhn(&numbers(&establishment)),
                "{seed}: SIRET {establishment}"
            );
            let key = (12 + 3 * (siren.parse::<u64>().unwrap() % 97)) % 97;
            assert_eq!(vat, format!("FR{key:02}{siren}"), "{seed}: VAT");

            let nir = of(&person.identifiers, IdentifierKind::Nir);
            assert_eq!(nir.len(), 15);
            assert_eq!(france::nir_key(&nir[..13]), &nir[13..], "{seed}: NIR {nir}");
        }
    }

    /// The card never contradicts itself: what the number encodes is the person's own.
    #[test]
    fn the_social_security_number_and_the_address_agree_with_the_person() {
        let lands = data::land(Country::Fr);
        for seed in 0..500 {
            let person = drawn(&seed.to_string());
            let nir = of(&person.identifiers, IdentifierKind::Nir);

            assert_eq!(
                &nir[..1],
                if person.sex == Sex::Male { "1" } else { "2" },
                "{seed}: the sex"
            );
            assert_eq!(&nir[1..3], &person.birth.date[2..4], "{seed}: the year");
            assert_eq!(&nir[3..5], &person.birth.date[5..7], "{seed}: the month");
            let born_in = lands
                .places
                .iter()
                .find(|place| place.city == person.birth.city)
                .unwrap();
            assert_eq!(&nir[5..10], born_in.code, "{seed}: the commune of birth");
            assert_eq!(person.birth.region, born_in.region);

            // The postcode belongs to the city, the department to the postcode, and the
            // landline to the zone of the place.
            let home = lands
                .places
                .iter()
                .find(|place| place.city == person.address.city)
                .unwrap();
            assert_eq!(person.address.postcode, home.postcode);
            assert_eq!(person.address.region, home.region);
            assert_eq!(&person.phone[1..2], home.more, "{seed}: the zone");
        }
    }

    /// ⚠️ Harmless: an address that reaches nobody, and the numbers kept for fiction.
    #[test]
    fn the_contact_reaches_nobody() {
        for seed in 0..300 {
            let person = drawn(&seed.to_string());
            let (local, domain) = person.email.split_once('@').unwrap();

            assert!(["example.com", "example.org", "example.net"].contains(&domain));
            assert!(
                local.chars().all(|character| character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || character == '.'),
                "{seed}: {}",
                person.email
            );
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
            assert_eq!(person.phone.len(), 14);
        }
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

    /// Diversity is checked, not hoped for: every pool the profile mixes shows, in about the
    /// stated proportions.
    #[test]
    fn a_thousand_people_carry_surnames_from_every_pool_in_about_its_share() {
        let lists = Lists::of(Country::Fr);
        let mut counts = [0u32; 5];
        for seed in 0..1000 {
            let surname = drawn(&seed.to_string()).surname;
            let from = Pool::ALL
                .iter()
                .position(|pool| lists.names(*pool).surnames.contains(&surname.as_str()))
                .unwrap();
            counts[from] += 1;
        }

        for (pool, share) in france::POOLS {
            let seen = counts[Pool::ALL.iter().position(|each| each == pool).unwrap()];
            let expected = share * 10;
            assert!(
                seen > expected / 2 && seen < expected * 2,
                "{pool:?}: {seen} of 1000, about {expected} expected"
            );
        }
    }

    #[test]
    fn a_thousand_people_are_born_in_every_place_and_none_takes_them_all() {
        let mut born: HashMap<String, u32> = HashMap::new();
        let mut moved = 0;
        for seed in 0..1000 {
            let person = drawn(&seed.to_string());
            if person.birth.city != person.address.city {
                moved += 1;
            }
            *born.entry(person.birth.city).or_default() += 1;
        }

        assert_eq!(born.len(), Lists::of(Country::Fr).land.places.len());
        let most = born.values().max().unwrap();
        assert!(*most < 60, "one place takes {most} of 1000");
        assert!(moved > 350 && moved < 650, "{moved} of 1000 moved");
    }

    #[test]
    fn a_person_has_one_to_three_given_names_each_once_and_a_document_number() {
        let mut counts = HashSet::new();
        for seed in 0..300 {
            let person = drawn(&seed.to_string());
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
        assert_eq!(json["bank"]["iban"], answer.person.bank.iban);
        assert_eq!(json["identifiers"][0]["kind"], "nir");
        assert_eq!(json["company"]["identifiers"][2]["kind"], "vat");
    }

    #[test]
    fn an_export_holds_as_many_people_as_asked_the_first_being_the_card_s() {
        let (json, count) = exported(&asked("devnotes"), 5, PeopleFormat::Json, today());
        let people: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!((people.len(), count), (5, 5));
        assert_eq!(people[0]["surname"], drawn("devnotes").surname);
        assert_ne!(people[0], people[1]);

        // No fewer than one, no more than a hundred.
        assert_eq!(exported(&asked("x"), 0, PeopleFormat::Json, today()).1, 1);
        assert_eq!(
            exported(&asked("x"), 5000, PeopleFormat::Json, today()).1,
            100
        );
    }

    #[test]
    fn a_csv_has_a_header_a_row_a_person_and_quotes_what_holds_a_comma() {
        let (csv, _) = exported(&asked("devnotes"), 3, PeopleFormat::Csv, today());
        let rows: Vec<&str> = csv.trim_end().split("\r\n").collect();

        assert_eq!(rows.len(), 4);
        assert!(rows[0].starts_with("country,sex,surname,givenNames,birthDate"));
        assert!(rows[0].ends_with("company,siren,siret,vat"));
        assert!(rows[1].starts_with("FR,"));
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("plain"), "plain");
    }

    #[test]
    fn an_export_is_written_where_asked_or_says_why_not() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("people.csv");

        assert_eq!(
            export(
                &asked("devnotes"),
                2,
                PeopleFormat::Csv,
                &path.to_string_lossy(),
                today()
            ),
            PeopleExport::Written { people: 2 }
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 3);
        assert_eq!(
            export(
                &asked("devnotes"),
                2,
                PeopleFormat::Json,
                &folder
                    .path()
                    .join("absent")
                    .join("people.json")
                    .to_string_lossy(),
                today()
            ),
            PeopleExport::Failed {
                problem: FileProblem::Unwritable
            }
        );
    }
}
