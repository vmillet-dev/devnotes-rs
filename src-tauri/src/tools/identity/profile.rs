//! A country as the generator knows one: where its names come from, how it writes an address
//! and a phone number, what its firms are called. A profile is a few lines — the lists are
//! shared (`data.rs`) and the numbers are computed (`papers.rs`).

use super::Country;
use super::data::{Place, Pool};
use crate::tools::generate::Draw;

pub(super) struct Profile {
    /// Where the names of the people living there come from, in shares of a hundred: the mix
    /// is the diversity, and nothing else lists it.
    pub(super) pools: &'static [(Pool, u32)],
    /// `12 High Street` rather than `Hauptstraße 12`.
    number_first: bool,
    /// What follows the street: `{p}` the postcode, `{c}` the city, `{r}` the place's code.
    locality: &'static str,
    /// By the length of the place's own prefix, `0` for any: `~` is that prefix, `#` a digit.
    landline: &'static [(usize, &'static str)],
    mobile: &'static [&'static str],
    /// ⚠️ Whether those numbers are in a range the regulator keeps for fiction. Elsewhere a
    /// number of the right shape may ring at someone's, and the tool says so.
    pub(super) fiction_phones: bool,
    trades: &'static [&'static str],
    /// The form a firm takes, by the language of the place; the first one where it has none.
    forms: &'static [(&'static str, &'static str)],
    /// « Construcciones García » rather than « García Construcciones ».
    trade_first: bool,
}

const POSTCODE_THEN_CITY: &str = "{p} {c}";

const FRANCE: Profile = Profile {
    pools: &[
        (Pool::French, 78),
        (Pool::Arabic, 9),
        (Pool::Portuguese, 5),
        (Pool::WestAfrican, 5),
        (Pool::Vietnamese, 3),
    ],
    number_first: true,
    locality: POSTCODE_THEN_CITY,
    // ⚠️ The blocks ARCEP keeps for fiction, one a zone, written with each place.
    landline: &[(0, "~ ## ##")],
    mobile: &["06 39 98 ## ##"],
    fiction_phones: true,
    trades: &[
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
    ],
    forms: &[("", "")],
    trade_first: false,
};

const BELGIUM: Profile = Profile {
    pools: &[
        (Pool::Dutch, 45),
        (Pool::French, 32),
        (Pool::Arabic, 9),
        (Pool::Italian, 5),
        (Pool::Turkish, 5),
        (Pool::Portuguese, 4),
    ],
    number_first: false,
    locality: POSTCODE_THEN_CITY,
    landline: &[(2, "~ ### ## ##"), (3, "~ ## ## ##")],
    mobile: &["047# ## ## ##", "048# ## ## ##", "049# ## ## ##"],
    fiction_phones: false,
    trades: &[
        "Consult",
        "Services",
        "Construct",
        "Logistics",
        "Projects",
        "Solutions",
        "Partners",
        "Invest",
        "Trading",
        "Engineering",
    ],
    forms: &[("fr", "SRL"), ("nl", "BV")],
    trade_first: false,
};

const GERMANY: Profile = Profile {
    pools: &[
        (Pool::German, 76),
        (Pool::Turkish, 9),
        (Pool::Polish, 6),
        (Pool::Arabic, 5),
        (Pool::Italian, 4),
    ],
    number_first: false,
    locality: POSTCODE_THEN_CITY,
    landline: &[(0, "~ #######")],
    mobile: &[
        "0151 ########",
        "0160 #######",
        "0170 #######",
        "0172 #######",
        "0176 ########",
    ],
    fiction_phones: false,
    trades: &[
        "Bau",
        "Elektro",
        "Logistik",
        "Consulting",
        "Immobilien",
        "Software",
        "Maschinenbau",
        "Gartenbau",
        "Transporte",
        "Haustechnik",
    ],
    forms: &[("", "GmbH")],
    trade_first: false,
};

const SPAIN: Profile = Profile {
    pools: &[
        (Pool::Spanish, 87),
        (Pool::Arabic, 7),
        (Pool::Portuguese, 3),
        (Pool::English, 3),
    ],
    number_first: false,
    locality: POSTCODE_THEN_CITY,
    landline: &[(2, "~# ### ###"), (3, "~ ### ###")],
    mobile: &["6## ### ###"],
    fiction_phones: false,
    trades: &[
        "Construcciones",
        "Servicios",
        "Transportes",
        "Consultores",
        "Instalaciones",
        "Reformas",
        "Distribuciones",
        "Ingeniería",
        "Asesores",
        "Logística",
    ],
    forms: &[("", "S.L.")],
    trade_first: true,
};

const ITALY: Profile = Profile {
    pools: &[
        (Pool::Italian, 91),
        (Pool::Arabic, 5),
        (Pool::SouthAsian, 4),
    ],
    number_first: false,
    locality: POSTCODE_THEN_CITY,
    landline: &[(2, "~ ########"), (3, "~ #######"), (4, "~ ######")],
    mobile: &[
        "32# #######",
        "33# #######",
        "34# #######",
        "38# #######",
        "39# #######",
    ],
    fiction_phones: false,
    trades: &[
        "Costruzioni",
        "Servizi",
        "Trasporti",
        "Impianti",
        "Consulenze",
        "Immobiliare",
        "Informatica",
        "Arredamenti",
        "Logistica",
        "Ristorazione",
    ],
    forms: &[("", "S.r.l.")],
    trade_first: false,
};

const PORTUGAL: Profile = Profile {
    pools: &[
        (Pool::Portuguese, 91),
        (Pool::French, 3),
        (Pool::English, 3),
        (Pool::SouthAsian, 3),
    ],
    number_first: false,
    locality: POSTCODE_THEN_CITY,
    landline: &[(2, "~# ### ###"), (3, "~ ### ###")],
    mobile: &["91# ### ###", "92# ### ###", "93# ### ###", "96# ### ###"],
    fiction_phones: false,
    trades: &[
        "Construções",
        "Serviços",
        "Transportes",
        "Consultoria",
        "Instalações",
        "Imobiliária",
        "Informática",
        "Engenharia",
        "Logística",
        "Restauração",
    ],
    forms: &[("", "Lda.")],
    trade_first: false,
};

const NETHERLANDS: Profile = Profile {
    pools: &[
        (Pool::Dutch, 78),
        (Pool::Turkish, 7),
        (Pool::Arabic, 7),
        (Pool::German, 4),
        (Pool::English, 4),
    ],
    number_first: false,
    locality: POSTCODE_THEN_CITY,
    landline: &[(3, "~ ### ####"), (4, "~ ## ## ##")],
    mobile: &["06 ########"],
    fiction_phones: false,
    trades: &[
        "Bouw",
        "Advies",
        "Transport",
        "Installatie",
        "Vastgoed",
        "Software",
        "Techniek",
        "Hoveniers",
        "Logistiek",
        "Beheer",
    ],
    forms: &[("", "B.V.")],
    trade_first: false,
};

const SWITZERLAND: Profile = Profile {
    pools: &[
        (Pool::German, 57),
        (Pool::French, 20),
        (Pool::Italian, 10),
        (Pool::Portuguese, 6),
        (Pool::Turkish, 4),
        (Pool::Spanish, 3),
    ],
    number_first: false,
    locality: POSTCODE_THEN_CITY,
    landline: &[(0, "~ ### ## ##")],
    mobile: &[
        "076 ### ## ##",
        "077 ### ## ##",
        "078 ### ## ##",
        "079 ### ## ##",
    ],
    fiction_phones: false,
    trades: &[
        "Bau",
        "Treuhand",
        "Transport",
        "Consulting",
        "Immobilien",
        "Informatik",
        "Technik",
        "Services",
        "Logistik",
        "Engineering",
    ],
    forms: &[("de", "AG"), ("fr", "SA"), ("it", "SA")],
    trade_first: false,
};

const UNITED_KINGDOM: Profile = Profile {
    pools: &[
        (Pool::English, 79),
        (Pool::SouthAsian, 9),
        (Pool::Polish, 5),
        (Pool::WestAfrican, 4),
        (Pool::Arabic, 3),
    ],
    number_first: true,
    locality: "{c} {p}",
    // ⚠️ The numbers Ofcom keeps for drama: a block a large town, `01632 960` for the others.
    landline: &[(0, "~###")],
    mobile: &["07700 900###"],
    fiction_phones: true,
    trades: &[
        "Consulting",
        "Services",
        "Building",
        "Logistics",
        "Property",
        "Software",
        "Engineering",
        "Catering",
        "Design",
        "Trading",
    ],
    forms: &[("", "Ltd")],
    trade_first: false,
};

const POLAND: Profile = Profile {
    pools: &[(Pool::Polish, 100)],
    number_first: false,
    locality: POSTCODE_THEN_CITY,
    landline: &[(0, "~ ### ## ##")],
    mobile: &[
        "50# ### ###",
        "60# ### ###",
        "66# ### ###",
        "69# ### ###",
        "72# ### ###",
        "78# ### ###",
        "79# ### ###",
        "88# ### ###",
    ],
    fiction_phones: false,
    trades: &[
        "Transport",
        "Budownictwo",
        "Usługi",
        "Consulting",
        "Instalacje",
        "Nieruchomości",
        "Informatyka",
        "Handel",
        "Logistyka",
        "Projekt",
    ],
    forms: &[("", "sp. z o.o.")],
    trade_first: false,
};

const SWEDEN: Profile = Profile {
    pools: &[
        (Pool::Nordic, 80),
        (Pool::Arabic, 9),
        (Pool::Polish, 4),
        (Pool::Turkish, 4),
        (Pool::SouthAsian, 3),
    ],
    number_first: false,
    locality: POSTCODE_THEN_CITY,
    landline: &[(2, "~-### ### ##"), (3, "~-### ## ##"), (4, "~-## ## ##")],
    mobile: &[
        "070-### ## ##",
        "072-### ## ##",
        "073-### ## ##",
        "076-### ## ##",
    ],
    fiction_phones: false,
    trades: &[
        "Bygg",
        "Konsult",
        "Transport",
        "Fastigheter",
        "El",
        "Måleri",
        "Teknik",
        "Service",
        "Logistik",
        "Data",
    ],
    forms: &[("", "AB")],
    trade_first: false,
};

const NORTH_AMERICAN_TRADES: &[&str] = &[
    "Consulting",
    "Services",
    "Construction",
    "Logistics",
    "Properties",
    "Software",
    "Engineering",
    "Catering",
    "Design",
    "Holdings",
];

const UNITED_STATES: Profile = Profile {
    pools: &[
        (Pool::English, 62),
        (Pool::Spanish, 19),
        (Pool::German, 5),
        (Pool::Italian, 4),
        (Pool::SouthAsian, 4),
        (Pool::Vietnamese, 3),
        (Pool::WestAfrican, 3),
    ],
    number_first: true,
    locality: "{c}, {r} {p}",
    // ⚠️ `555-0100` to `555-0199`, kept for fiction in every area code of North America.
    landline: &[(0, "(~) 555-01##")],
    mobile: &["(~) 555-01##"],
    fiction_phones: true,
    trades: NORTH_AMERICAN_TRADES,
    forms: &[("", "LLC")],
    trade_first: false,
};

const CANADA: Profile = Profile {
    pools: &[
        (Pool::English, 55),
        (Pool::French, 21),
        (Pool::SouthAsian, 8),
        (Pool::Italian, 5),
        (Pool::Arabic, 4),
        (Pool::Portuguese, 4),
        (Pool::Vietnamese, 3),
    ],
    number_first: true,
    locality: "{c} {r} {p}",
    landline: &[(0, "(~) 555-01##")],
    mobile: &["(~) 555-01##"],
    fiction_phones: true,
    trades: NORTH_AMERICAN_TRADES,
    forms: &[("", "Inc.")],
    trade_first: false,
};

pub(super) fn of(country: Country) -> &'static Profile {
    match country {
        Country::Fr => &FRANCE,
        Country::Be => &BELGIUM,
        Country::De => &GERMANY,
        Country::Es => &SPAIN,
        Country::It => &ITALY,
        Country::Pt => &PORTUGAL,
        Country::Nl => &NETHERLANDS,
        Country::Ch => &SWITZERLAND,
        Country::Gb => &UNITED_KINGDOM,
        Country::Pl => &POLAND,
        Country::Se => &SWEDEN,
        Country::Us => &UNITED_STATES,
        Country::Ca => &CANADA,
    }
}

/// The letters every postcode drawn here may end on: none that the United Kingdom, the
/// Netherlands or Canada leaves out of theirs.
const LETTERS: &[u8] = b"ABEGHJLNPRTWXZ";

pub(super) fn digits(draw: &mut Draw, count: usize) -> String {
    (0..count)
        .map(|_| char::from_digit(u32::try_from(draw.below(10)).unwrap_or(0), 10).unwrap_or('0'))
        .collect()
}

/// A pattern written out: `#` a digit, `@` a letter, `~` the prefix, the rest as it is.
fn fill(pattern: &str, prefix: &str, draw: &mut Draw) -> String {
    let mut written = String::new();
    for character in pattern.chars() {
        match character {
            '#' => written.push_str(&digits(draw, 1)),
            '@' => written.push(char::from(*draw.pick(LETTERS))),
            '~' => written.push_str(prefix),
            other => written.push(other),
        }
    }
    written
}

impl Profile {
    pub(super) fn postcode(place: &Place, draw: &mut Draw) -> String {
        fill(place.postcode, "", draw)
    }

    pub(super) fn landline(&self, place: &Place, draw: &mut Draw) -> String {
        let pattern = self
            .landline
            .iter()
            .find(|(length, _)| *length == 0 || *length == place.more.len())
            .map_or(self.landline[0].1, |(_, pattern)| pattern);
        fill(pattern, place.more, draw)
    }

    pub(super) fn mobile(&self, place: &Place, draw: &mut Draw) -> String {
        let pattern = *draw.pick(self.mobile);
        fill(pattern, place.more, draw)
    }

    /// The number and the street, in the order the country writes them.
    pub(super) fn street(&self, number: i64, street: &str) -> String {
        if self.number_first {
            format!("{number} {street}")
        } else {
            format!("{street} {number}")
        }
    }

    /// The whole address on one line, as an envelope of the country carries it.
    pub(super) fn address(&self, street: &str, postcode: &str, place: &Place) -> String {
        let locality = self
            .locality
            .replace("{p}", postcode)
            .replace("{c}", place.city)
            .replace("{r}", place.code);
        format!("{street}, {locality}")
    }

    pub(super) fn firm(&self, surname: &str, place: &Place, draw: &mut Draw) -> String {
        let trade = *draw.pick(self.trades);
        let form = self
            .forms
            .iter()
            .find(|(tag, _)| *tag == place.tag)
            .map_or(self.forms[0].1, |(_, form)| form);
        let name = if self.trade_first {
            format!("{trade} {surname}")
        } else {
            format!("{surname} {trade}")
        };
        format!("{name} {form}").trim_end().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mix_is_a_hundred_and_names_its_first_pool_first() {
        for country in Country::ALL {
            let pools = of(country).pools;
            assert_eq!(
                pools.iter().map(|(_, share)| share).sum::<u32>(),
                100,
                "{country}"
            );
            assert!(
                pools.iter().all(|(_, share)| *share <= pools[0].1),
                "{country}: its first pool is not its largest"
            );
        }
    }

    #[test]
    fn an_address_is_written_in_the_order_of_its_country() {
        let place = |code, city| Place {
            code,
            postcode: "",
            city,
            region: "",
            more: "",
            tag: "",
        };
        let written = |country, street: &str, postcode, place: &Place| {
            let profile = of(country);
            profile.address(&profile.street(12, street), postcode, place)
        };

        assert_eq!(
            written(Country::Fr, "rue Pasteur", "38000", &place("", "Grenoble")),
            "12 rue Pasteur, 38000 Grenoble"
        );
        assert_eq!(
            written(Country::De, "Hauptstraße", "10115", &place("BE", "Berlin")),
            "Hauptstraße 12, 10115 Berlin"
        );
        assert_eq!(
            written(
                Country::Gb,
                "High Street",
                "M1 4BT",
                &place("ENG", "Manchester")
            ),
            "12 High Street, Manchester M1 4BT"
        );
        assert_eq!(
            written(Country::Us, "Main Street", "60601", &place("IL", "Chicago")),
            "12 Main Street, Chicago, IL 60601"
        );
        assert_eq!(
            written(
                Country::Ca,
                "King Street",
                "M5H 2N2",
                &place("ON", "Toronto")
            ),
            "12 King Street, Toronto ON M5H 2N2"
        );
    }

    #[test]
    fn a_firm_takes_the_form_of_its_place_and_the_order_of_its_country() {
        let place = |tag| Place {
            code: "",
            postcode: "",
            city: "",
            region: "",
            more: "",
            tag,
        };
        let firm = |country, tag| of(country).firm("Favre", &place(tag), &mut Draw::new(1));

        assert!(firm(Country::Ch, "de").ends_with(" AG"));
        assert!(firm(Country::Ch, "fr").ends_with(" SA"));
        assert!(firm(Country::Be, "nl").ends_with(" BV"));
        assert!(firm(Country::Es, "").ends_with(" Favre S.L."));
        assert!(firm(Country::Fr, "").starts_with("Favre "));
        assert!(!firm(Country::Fr, "").ends_with(' '));
    }
}
