//! People who do not exist, for a test or a demo: their names, emails, phones, addresses, IBANs
//! and card numbers, harmless by construction — reserved domains and fiction's phone ranges, and
//! numbers whose checksums are right but whose banks are made up. Drawn from a seed.

use deunicode::deunicode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use specta::Type;

use super::checks::{self, CardNetwork};
use super::generate::{Draw, seed_or_draw};

pub const MAX_ROWS: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FakeColumn {
    Name,
    Email,
    Phone,
    Address,
    Iban,
    Card,
}

const COLUMNS: [FakeColumn; 6] = [
    FakeColumn::Name,
    FakeColumn::Email,
    FakeColumn::Phone,
    FakeColumn::Address,
    FakeColumn::Iban,
    FakeColumn::Card,
];

impl FakeColumn {
    /// The key of a JSON object and the header of a CSV column, the same in every language.
    fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Address => "address",
            Self::Iban => "iban",
            Self::Card => "card",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FakeLocale {
    Fr,
    En,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FakeRequest {
    pub columns: Vec<FakeColumn>,
    pub count: u32,
    pub locale: FakeLocale,
    /// The same seed draws the same rows; none draws one and says which.
    pub seed: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FakeAnswer {
    pub seed: u32,
    pub columns: Vec<FakeColumn>,
    pub rows: Vec<Vec<String>>,
    pub csv: String,
    pub json: String,
}

const FR_FIRST: [&str; 40] = [
    "Camille",
    "Léa",
    "Manon",
    "Chloé",
    "Inès",
    "Jade",
    "Louise",
    "Zoé",
    "Emma",
    "Alice",
    "Juliette",
    "Anaïs",
    "Clémence",
    "Élise",
    "Margaux",
    "Sarah",
    "Lucie",
    "Noémie",
    "Agathe",
    "Héloïse",
    "Lucas",
    "Hugo",
    "Louis",
    "Gabriel",
    "Arthur",
    "Jules",
    "Raphaël",
    "Adam",
    "Théo",
    "Nathan",
    "Maël",
    "Noé",
    "Paul",
    "Victor",
    "Étienne",
    "Antoine",
    "Bastien",
    "Mathis",
    "Quentin",
    "Rémi",
];
const FR_LAST: [&str; 40] = [
    "Martin",
    "Bernard",
    "Dubois",
    "Thomas",
    "Robert",
    "Richard",
    "Petit",
    "Durand",
    "Leroy",
    "Moreau",
    "Simon",
    "Laurent",
    "Lefèvre",
    "Michel",
    "Garcia",
    "David",
    "Bertrand",
    "Roux",
    "Vincent",
    "Fournier",
    "Morel",
    "Girard",
    "André",
    "Mercier",
    "Dupont",
    "Lambert",
    "Bonnet",
    "François",
    "Martinez",
    "Legrand",
    "Garnier",
    "Faure",
    "Rousseau",
    "Blanc",
    "Guérin",
    "Muller",
    "Henry",
    "Roussel",
    "Nicolas",
    "Perrin",
];
const FR_STREETS: [&str; 20] = [
    "rue des Lilas",
    "rue de la République",
    "avenue Victor-Hugo",
    "boulevard Voltaire",
    "rue du Moulin",
    "place de l’Église",
    "chemin des Vignes",
    "rue Pasteur",
    "allée des Tilleuls",
    "rue de la Gare",
    "impasse des Acacias",
    "rue Jean-Jaurès",
    "quai des Brumes",
    "rue du Four",
    "avenue des Peupliers",
    "rue Émile-Zola",
    "rue des Écoles",
    "cours Lafayette",
    "rue de la Paix",
    "route de la Forêt",
];
const FR_CITIES: [(&str, &str); 20] = [
    ("75011", "Paris"),
    ("69003", "Lyon"),
    ("13006", "Marseille"),
    ("31000", "Toulouse"),
    ("06000", "Nice"),
    ("44000", "Nantes"),
    ("67000", "Strasbourg"),
    ("34000", "Montpellier"),
    ("33000", "Bordeaux"),
    ("59000", "Lille"),
    ("35000", "Rennes"),
    ("51100", "Reims"),
    ("42000", "Saint-Étienne"),
    ("83000", "Toulon"),
    ("38000", "Grenoble"),
    ("21000", "Dijon"),
    ("49000", "Angers"),
    ("37000", "Tours"),
    ("63000", "Clermont-Ferrand"),
    ("25000", "Besançon"),
];
/// ARCEP's six blocks for fiction: none is ever given to a subscriber.
const FR_PHONE_BLOCKS: [&str; 6] = [
    "01 99 00", "02 61 91", "03 53 01", "04 65 71", "05 36 49", "06 39 98",
];

const EN_FIRST: [&str; 40] = [
    "Olivia", "Emma", "Ava", "Sophia", "Isabella", "Mia", "Amelia", "Harper", "Evelyn", "Abigail",
    "Emily", "Ella", "Grace", "Chloe", "Nora", "Hazel", "Lily", "Zoe", "Stella", "Aurora", "Liam",
    "Noah", "Oliver", "Elijah", "James", "William", "Benjamin", "Lucas", "Henry", "Theodore",
    "Jack", "Levi", "Owen", "Ethan", "Wyatt", "Leo", "Julian", "Isaac", "Caleb", "Nathan",
];
const EN_LAST: [&str; 40] = [
    "Smith", "Johnson", "Williams", "Brown", "Jones", "Miller", "Davis", "Wilson", "Anderson",
    "Taylor", "Thomas", "Moore", "Jackson", "Martin", "Lee", "Thompson", "White", "Harris",
    "Clark", "Lewis", "Robinson", "Walker", "Young", "Allen", "King", "Wright", "Scott", "Hill",
    "Green", "Adams", "Baker", "Nelson", "Carter", "Mitchell", "Roberts", "Turner", "Phillips",
    "Campbell", "Parker", "Evans",
];
const EN_STREETS: [&str; 20] = [
    "Maple Street",
    "Oak Avenue",
    "Cedar Lane",
    "Pine Road",
    "Elm Street",
    "Washington Avenue",
    "Lake Drive",
    "Hillcrest Road",
    "Park Place",
    "Sunset Boulevard",
    "Main Street",
    "Church Street",
    "River Road",
    "Highland Avenue",
    "Willow Way",
    "Spring Street",
    "Forest Drive",
    "Meadow Lane",
    "Chestnut Street",
    "Valley Road",
];
const EN_CITIES: [(&str, &str, &str, &str); 20] = [
    ("New York", "NY", "10001", "212"),
    ("Chicago", "IL", "60601", "312"),
    ("San Francisco", "CA", "94103", "415"),
    ("Boston", "MA", "02108", "617"),
    ("Seattle", "WA", "98101", "206"),
    ("Austin", "TX", "78701", "512"),
    ("Denver", "CO", "80202", "303"),
    ("Portland", "OR", "97205", "503"),
    ("Atlanta", "GA", "30303", "404"),
    ("Miami", "FL", "33130", "305"),
    ("Phoenix", "AZ", "85004", "602"),
    ("Minneapolis", "MN", "55401", "612"),
    ("Nashville", "TN", "37203", "615"),
    ("Philadelphia", "PA", "19103", "215"),
    ("San Diego", "CA", "92101", "619"),
    ("Detroit", "MI", "48226", "313"),
    ("Salt Lake City", "UT", "84111", "801"),
    ("Pittsburgh", "PA", "15222", "412"),
    ("Raleigh", "NC", "27601", "919"),
    ("Madison", "WI", "53703", "608"),
];
/// RFC 2606 keeps these for examples: a mail sent to one reaches no one.
const DOMAINS: [&str; 3] = ["example.com", "example.org", "example.net"];

struct Person {
    first: &'static str,
    last: &'static str,
}

fn digits(draw: &mut Draw, count: usize) -> String {
    (0..count)
        .map(|_| char::from(b'0' + u8::try_from(draw.below(10)).unwrap_or(0)))
        .collect()
}

fn email(draw: &mut Draw, person: &Person) -> String {
    let local = |part: &str| {
        deunicode(part)
            .to_lowercase()
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>()
    };
    let suffix = if draw.chance(0.3) {
        draw.between(1, 99).to_string()
    } else {
        String::new()
    };
    format!(
        "{}.{}{suffix}@{}",
        local(person.first),
        local(person.last),
        draw.pick(&DOMAINS)
    )
}

fn phone(draw: &mut Draw, locale: FakeLocale, area: &str) -> String {
    let line = digits(draw, 4);
    match locale {
        FakeLocale::Fr => format!(
            "{} {} {}",
            draw.pick(&FR_PHONE_BLOCKS),
            &line[..2],
            &line[2..]
        ),
        // 555-0100 to 555-0199 are the numbers North America keeps for fiction.
        FakeLocale::En => format!("({area}) 555-01{}", &line[..2]),
    }
}

/// The French RIB key: 97 less the bank, branch and account weighed 89, 15 and 3, mod 97.
fn rib_key(bank: &str, branch: &str, account: &str) -> String {
    let number = |text: &str| text.parse::<u64>().unwrap_or(0);
    let sum = 89 * number(bank) + 15 * number(branch) + 3 * number(account);
    format!("{:02}", 97 - sum % 97)
}

fn iban(draw: &mut Draw, locale: FakeLocale) -> String {
    let (country, bban) = match locale {
        FakeLocale::Fr => {
            let (bank, branch, account) = (digits(draw, 5), digits(draw, 5), digits(draw, 11));
            let key = rib_key(&bank, &branch, &account);
            ("FR", format!("{bank}{branch}{account}{key}"))
        }
        FakeLocale::En => ("DE", digits(draw, 18)),
    };
    let electronic = format!(
        "{country}{}{bban}",
        checks::iban_check_digits(country, &bban)
    );
    checks::by_four(&electronic)
}

/// In the network's prefix and length, its last digit Luhn's: it passes a form, and pays nothing.
fn card(draw: &mut Draw) -> String {
    let (network, prefix, length) = match draw.below(3) {
        0 => (CardNetwork::Visa, "4".to_owned(), 16),
        1 => (
            CardNetwork::Mastercard,
            format!("5{}", draw.between(1, 5)),
            16,
        ),
        _ => (
            CardNetwork::AmericanExpress,
            draw.pick(&["34", "37"]).to_string(),
            15,
        ),
    };
    let mut number = prefix;
    number.push_str(&digits(draw, length - 1 - number.len()));
    let values: Vec<u32> = number.chars().filter_map(|c| c.to_digit(10)).collect();
    number.push(char::from(
        b'0' + u8::try_from(checks::luhn_check_digit(&values)).unwrap_or(0),
    ));
    network.grouped(&number)
}

fn row(draw: &mut Draw, locale: FakeLocale, columns: &[FakeColumn]) -> Vec<String> {
    let (first, last): (&[&'static str], &[&'static str]) = match locale {
        FakeLocale::Fr => (&FR_FIRST, &FR_LAST),
        FakeLocale::En => (&EN_FIRST, &EN_LAST),
    };
    let person = Person {
        first: draw.pick(first),
        last: draw.pick(last),
    };
    let number = draw.between(1, 180);
    let (address, area) = match locale {
        FakeLocale::Fr => {
            let (postcode, city) = draw.pick(&FR_CITIES);
            (
                format!("{number} {}, {postcode} {city}", draw.pick(&FR_STREETS)),
                "",
            )
        }
        FakeLocale::En => {
            let (city, state, zip, area) = draw.pick(&EN_CITIES);
            (
                format!("{number} {}, {city}, {state} {zip}", draw.pick(&EN_STREETS)),
                *area,
            )
        }
    };
    columns
        .iter()
        .map(|column| match column {
            FakeColumn::Name => format!("{} {}", person.first, person.last),
            FakeColumn::Email => email(draw, &person),
            FakeColumn::Phone => phone(draw, locale, area),
            FakeColumn::Address => address.clone(),
            FakeColumn::Iban => iban(draw, locale),
            FakeColumn::Card => card(draw),
        })
        .collect()
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

pub fn generate(request: &FakeRequest) -> FakeAnswer {
    let seed = seed_or_draw(request.seed);
    let mut draw = Draw::new(seed);
    let columns: Vec<FakeColumn> = COLUMNS
        .into_iter()
        .filter(|column| request.columns.contains(column))
        .collect();
    let rows: Vec<Vec<String>> = if columns.is_empty() {
        Vec::new()
    } else {
        (0..request.count.clamp(1, MAX_ROWS))
            .map(|_| row(&mut draw, request.locale, &columns))
            .collect()
    };

    let header = columns
        .iter()
        .map(|column| column.key())
        .collect::<Vec<_>>()
        .join(",");
    let csv = std::iter::once(header)
        .chain(rows.iter().map(|row| {
            row.iter()
                .map(|field| csv_field(field))
                .collect::<Vec<_>>()
                .join(",")
        }))
        .collect::<Vec<_>>()
        .join("\n");
    let objects: Vec<Value> = rows
        .iter()
        .map(|row| {
            Value::Object(
                columns
                    .iter()
                    .zip(row)
                    .map(|(column, field)| (column.key().to_owned(), Value::String(field.clone())))
                    .collect::<Map<_, _>>(),
            )
        })
        .collect();

    FakeAnswer {
        seed,
        json: serde_json::to_string_pretty(&objects).unwrap_or_default(),
        csv,
        columns,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn ask(locale: FakeLocale, seed: Option<u32>, count: u32) -> FakeAnswer {
        generate(&FakeRequest {
            columns: COLUMNS.to_vec(),
            count,
            locale,
            seed,
        })
    }

    fn column(answer: &FakeAnswer, wanted: FakeColumn) -> Vec<String> {
        let index = answer.columns.iter().position(|c| *c == wanted).unwrap();
        answer.rows.iter().map(|row| row[index].clone()).collect()
    }

    fn passes_luhn(number: &str) -> bool {
        let digits: Vec<u32> = number.chars().filter_map(|c| c.to_digit(10)).collect();
        checks::luhn_check_digit(&digits[..digits.len() - 1]) == *digits.last().unwrap()
    }

    #[test]
    fn a_seed_draws_the_same_rows_and_none_says_which_it_drew() {
        assert_eq!(
            ask(FakeLocale::Fr, Some(7), 20),
            ask(FakeLocale::Fr, Some(7), 20)
        );
        assert_ne!(
            ask(FakeLocale::Fr, Some(7), 20).rows,
            ask(FakeLocale::Fr, Some(8), 20).rows
        );
        let drawn = ask(FakeLocale::En, None, 3);
        assert_eq!(ask(FakeLocale::En, Some(drawn.seed), 3), drawn);
    }

    #[test]
    fn every_iban_is_valid_and_french_ones_carry_a_right_rib_key() {
        for locale in [FakeLocale::Fr, FakeLocale::En] {
            for iban in column(&ask(locale, Some(1), MAX_ROWS), FakeColumn::Iban) {
                let electronic: String = iban.chars().filter(|c| !c.is_whitespace()).collect();
                assert!(iban::Iban::from_str(&electronic).is_ok(), "{iban}");
                if locale == FakeLocale::Fr {
                    assert!(electronic.starts_with("FR"));
                    let bban = &electronic[4..];
                    assert_eq!(
                        rib_key(&bban[..5], &bban[5..10], &bban[10..21]),
                        &bban[21..]
                    );
                } else {
                    assert!(electronic.starts_with("DE"));
                }
            }
        }
    }

    #[test]
    fn every_card_passes_luhn_in_its_network_s_prefix_and_length() {
        for number in column(&ask(FakeLocale::En, Some(3), MAX_ROWS), FakeColumn::Card) {
            let digits: String = number.chars().filter(char::is_ascii_digit).collect();
            assert!(passes_luhn(&digits), "{number}");
            match digits.len() {
                16 => assert!(
                    digits.starts_with('4')
                        || (51..=55).contains(&digits[..2].parse::<u32>().unwrap())
                ),
                15 => assert!(digits.starts_with("34") || digits.starts_with("37")),
                other => panic!("{other} digits: {number}"),
            }
        }
    }

    #[test]
    fn every_phone_is_in_a_range_kept_for_fiction() {
        for phone in column(&ask(FakeLocale::Fr, Some(4), MAX_ROWS), FakeColumn::Phone) {
            assert!(
                FR_PHONE_BLOCKS.iter().any(|block| phone.starts_with(block)),
                "{phone}"
            );
            assert_eq!(phone.len(), 14);
        }
        for phone in column(&ask(FakeLocale::En, Some(4), MAX_ROWS), FakeColumn::Phone) {
            assert!(phone.contains(") 555-01"), "{phone}");
            assert_eq!(phone.len(), 14);
        }
    }

    #[test]
    fn every_email_is_on_a_reserved_domain_and_made_of_the_name() {
        let answer = ask(FakeLocale::Fr, Some(5), MAX_ROWS);
        for (email, name) in column(&answer, FakeColumn::Email)
            .iter()
            .zip(column(&answer, FakeColumn::Name))
        {
            let (local, domain) = email.split_once('@').unwrap();
            assert!(DOMAINS.contains(&domain), "{email}");
            assert!(local.is_ascii(), "{email}");
            let first = deunicode(name.split(' ').next().unwrap()).to_lowercase();
            assert!(
                local.starts_with(&first.replace(|c: char| !c.is_ascii_alphanumeric(), "")),
                "{email} for {name}"
            );
        }
    }

    #[test]
    fn both_locales_write_their_own_addresses() {
        let french = column(&ask(FakeLocale::Fr, Some(6), 10), FakeColumn::Address);
        let english = column(&ask(FakeLocale::En, Some(6), 10), FakeColumn::Address);

        assert!(french.iter().all(|address| {
            FR_CITIES
                .iter()
                .any(|(code, city)| address.ends_with(&format!("{code} {city}")))
        }));
        assert!(english.iter().all(|address| {
            EN_CITIES
                .iter()
                .any(|(city, state, zip, _)| address.ends_with(&format!("{city}, {state} {zip}")))
        }));
    }

    #[test]
    fn only_the_columns_asked_in_their_order_and_as_csv_and_json() {
        let answer = generate(&FakeRequest {
            columns: vec![FakeColumn::Email, FakeColumn::Name, FakeColumn::Address],
            count: 2,
            locale: FakeLocale::En,
            seed: Some(9),
        });

        assert_eq!(
            answer.columns,
            [FakeColumn::Name, FakeColumn::Email, FakeColumn::Address]
        );
        assert!(answer.csv.starts_with("name,email,address\n"));
        assert_eq!(answer.csv.lines().count(), 3);
        assert!(
            answer.csv.lines().nth(1).unwrap().contains(",\""),
            "the address holds commas, and is quoted"
        );
        let json: Vec<Value> = serde_json::from_str(&answer.json).unwrap();
        assert_eq!(json[0]["name"], Value::String(answer.rows[0][0].clone()));
        assert_eq!(csv_field("say \"hi\", then"), "\"say \"\"hi\"\", then\"");
    }

    #[test]
    fn no_column_draws_no_row_and_a_count_is_kept_within_its_bounds() {
        let none = generate(&FakeRequest {
            columns: vec![],
            count: 5,
            locale: FakeLocale::Fr,
            seed: Some(1),
        });
        assert!(none.rows.is_empty());
        assert_eq!(ask(FakeLocale::Fr, Some(1), 0).rows.len(), 1);
        assert_eq!(
            ask(FakeLocale::Fr, Some(1), 1_000).rows.len(),
            MAX_ROWS as usize
        );
    }
}
