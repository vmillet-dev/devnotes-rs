//! The lists an identity is drawn from: pools of names an origin, and a country's places and
//! streets. ⚠️ Plain text taken into the binary as it is and split when read: a table of
//! `&str` would cost sixteen bytes a name on top of the names themselves. Every list together
//! is held under [`BUDGET`] by a test — the lists do not have to be long, they have to be well
//! chosen.

use super::Country;

/// Every list together, in bytes. A megabyte of names a country was the obvious way, and
/// refused: a pool of two hundred names gives thousands of people, and a country is a mix of
/// pools, not a list of its own.
#[cfg(test)]
pub(super) const BUDGET: usize = 100 * 1024;

/// Names follow languages and histories, not borders: a pool an origin, shared by every
/// country whose profile draws from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Pool {
    French,
    Arabic,
    Portuguese,
    WestAfrican,
    Vietnamese,
    English,
    German,
    Dutch,
    Spanish,
    Italian,
    Polish,
    Nordic,
    Turkish,
    SouthAsian,
}

impl Pool {
    pub(super) const ALL: [Self; 14] = [
        Self::French,
        Self::Arabic,
        Self::Portuguese,
        Self::WestAfrican,
        Self::Vietnamese,
        Self::English,
        Self::German,
        Self::Dutch,
        Self::Spanish,
        Self::Italian,
        Self::Polish,
        Self::Nordic,
        Self::Turkish,
        Self::SouthAsian,
    ];

    fn text(self) -> &'static str {
        match self {
            Self::French => include_str!("data/pools/french.txt"),
            Self::Arabic => include_str!("data/pools/arabic.txt"),
            Self::Portuguese => include_str!("data/pools/portuguese.txt"),
            Self::WestAfrican => include_str!("data/pools/west_african.txt"),
            Self::Vietnamese => include_str!("data/pools/vietnamese.txt"),
            Self::English => include_str!("data/pools/english.txt"),
            Self::German => include_str!("data/pools/german.txt"),
            Self::Dutch => include_str!("data/pools/dutch.txt"),
            Self::Spanish => include_str!("data/pools/spanish.txt"),
            Self::Italian => include_str!("data/pools/italian.txt"),
            Self::Polish => include_str!("data/pools/polish.txt"),
            Self::Nordic => include_str!("data/pools/nordic.txt"),
            Self::Turkish => include_str!("data/pools/turkish.txt"),
            Self::SouthAsian => include_str!("data/pools/south_asian.txt"),
        }
    }

    /// Three lines: `f:`, `m:` and `s:`, each a list.
    pub(super) fn names(self) -> Names {
        let line = |prefix: &str| -> Vec<&'static str> {
            self.text()
                .lines()
                .find_map(|line| line.strip_prefix(prefix))
                .map(|names| names.split(',').collect())
                .unwrap_or_default()
        };
        Names {
            female: line("f:"),
            male: line("m:"),
            surnames: line("s:"),
        }
    }

    /// The surname as the person bears it. ⚠️ A Polish surname in `-ski`, `-cki` or `-dzki` is
    /// an adjective: a woman's ends in `-a`, in Poland and wherever she went.
    pub(super) fn borne(self, surname: &str, female: bool) -> String {
        match surname.strip_suffix('i') {
            Some(stem)
                if self == Self::Polish
                    && female
                    && ["sk", "ck", "dzk"].iter().any(|end| stem.ends_with(end)) =>
            {
                format!("{stem}a")
            }
            _ => surname.to_owned(),
        }
    }
}

pub(super) struct Names {
    pub(super) female: Vec<&'static str>,
    pub(super) male: Vec<&'static str>,
    pub(super) surnames: Vec<&'static str>,
}

/// A town with what its country says of it. `code` and `more` are the country's own — the
/// INSEE code of a French commune, the land-registry code of an Italian one, a state's two
/// letters; the prefix of its phone numbers, or the block kept for fiction there. `postcode`
/// may leave characters to draw (`#` a digit, `@` a letter), and `tag` names the streets it
/// takes where a country speaks several languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Place {
    pub(super) code: &'static str,
    pub(super) postcode: &'static str,
    pub(super) city: &'static str,
    pub(super) region: &'static str,
    pub(super) more: &'static str,
    pub(super) tag: &'static str,
}

pub(super) struct Land {
    pub(super) places: Vec<Place>,
    /// A list a tag, in the order of the file: the first one is the country's own.
    pub(super) streets: Vec<(&'static str, Vec<&'static str>)>,
}

impl Land {
    /// The streets of the place's own language, or the country's first list.
    pub(super) fn streets(&self, tag: &str) -> &[&'static str] {
        self.streets
            .iter()
            .find(|(named, _)| *named == tag)
            .or(self.streets.first())
            .map_or(&[], |(_, streets)| streets)
    }
}

fn land_text(country: Country) -> &'static str {
    match country {
        Country::Fr => include_str!("data/places/fr.txt"),
        Country::Be => include_str!("data/places/be.txt"),
        Country::De => include_str!("data/places/de.txt"),
        Country::Es => include_str!("data/places/es.txt"),
        Country::It => include_str!("data/places/it.txt"),
        Country::Pt => include_str!("data/places/pt.txt"),
        Country::Nl => include_str!("data/places/nl.txt"),
        Country::Ch => include_str!("data/places/ch.txt"),
        Country::Gb => include_str!("data/places/gb.txt"),
        Country::Pl => include_str!("data/places/pl.txt"),
        Country::Se => include_str!("data/places/se.txt"),
        Country::Us => include_str!("data/places/us.txt"),
        Country::Ca => include_str!("data/places/ca.txt"),
    }
}

/// The places, a line each, then a list of streets under each `---` and its tag. ⚠️ Read line
/// by line: a checkout may have turned every line ending into two characters.
pub(super) fn land(country: Country) -> Land {
    let mut land = Land {
        places: Vec::new(),
        streets: Vec::new(),
    };
    for line in land_text(country).lines().filter(|line| !line.is_empty()) {
        if let Some(tag) = line.strip_prefix("---") {
            land.streets.push((tag, Vec::new()));
        } else if let Some((_, streets)) = land.streets.last_mut() {
            streets.push(line);
        } else {
            let mut parts = line.split(';');
            let mut part = || parts.next().unwrap_or_default();
            land.places.push(Place {
                code: part(),
                postcode: part(),
                city: part(),
                region: part(),
                more: part(),
                tag: part(),
            });
        }
    }
    land
}

/// What the lists weigh in the binary.
#[cfg(test)]
pub(super) fn weight() -> usize {
    Pool::ALL
        .iter()
        .map(|pool| pool.text().len())
        .chain(Country::ALL.iter().map(|country| land_text(*country).len()))
        .sum()
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    /// The obvious way was refused: this is what keeps it out.
    #[test]
    fn every_list_together_holds_to_the_budget() {
        assert!(
            weight() <= BUDGET,
            "{} bytes of lists, {BUDGET} at most",
            weight()
        );
    }

    #[test]
    fn a_pool_has_enough_names_of_each_kind_each_once_and_none_empty() {
        for pool in Pool::ALL {
            let names = pool.names();
            for (kind, list) in [
                ("female", &names.female),
                ("male", &names.male),
                ("surnames", &names.surnames),
            ] {
                assert!(list.len() >= 40, "{pool:?} has {} {kind}", list.len());
                assert_eq!(
                    list.iter().collect::<HashSet<_>>().len(),
                    list.len(),
                    "{pool:?} repeats one of its {kind}"
                );
                assert!(
                    list.iter()
                        .all(|name| !name.is_empty() && name.trim() == *name),
                    "{pool:?} has an empty or padded name among its {kind}"
                );
            }
        }
    }

    /// A surname says which pool it came from: that is how a mix is counted.
    #[test]
    fn no_surname_sits_in_two_pools() {
        let mut seen: HashMap<&str, Pool> = HashMap::new();
        for pool in Pool::ALL {
            for surname in pool.names().surnames {
                if let Some(other) = seen.insert(surname, pool) {
                    panic!("{surname} is in {other:?} and in {pool:?}");
                }
            }
        }
    }

    #[test]
    fn a_polish_surname_that_is_an_adjective_follows_the_person() {
        assert_eq!(Pool::Polish.borne("Kowalski", true), "Kowalska");
        assert_eq!(Pool::Polish.borne("Nowicki", true), "Nowicka");
        assert_eq!(Pool::Polish.borne("Zawadzki", true), "Zawadzka");
        assert_eq!(Pool::Polish.borne("Kowalski", false), "Kowalski");
        assert_eq!(Pool::Polish.borne("Nowak", true), "Nowak");
        assert_eq!(Pool::Italian.borne("Rossi", true), "Rossi");
    }

    #[test]
    fn a_country_has_its_places_whole_and_streets_for_each_of_them() {
        for country in Country::ALL {
            let land = land(country);
            assert!(land.places.len() >= 20, "{country}: {}", land.places.len());
            assert!(!land.streets.is_empty(), "{country} has no street");
            for (tag, streets) in &land.streets {
                assert!(streets.len() >= 10, "{country} {tag}: {}", streets.len());
            }
            for place in &land.places {
                assert!(
                    ![
                        place.code,
                        place.postcode,
                        place.city,
                        place.region,
                        place.more
                    ]
                    .iter()
                    .any(|part| part.is_empty()),
                    "{country}: {place:?}"
                );
                assert!(
                    land.streets.iter().any(|(tag, _)| *tag == place.tag),
                    "{country}: no streets tagged « {} » for {}",
                    place.tag,
                    place.city
                );
            }
        }
    }
}
