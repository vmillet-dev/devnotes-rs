//! The lists an identity is drawn from: the names, and a country's places and streets.
//! ⚠️ **The names are one list**: no origin and no country is attached to a name — not in the
//! data, not in a file's name, not in the order of the file, which is alphabetical. Sorting
//! people's names by where they come from is not something this tool does; what is a
//! country's own is its places, its formats and its numbers. ⚠️ Compiled into the application
//! (`resources/identity/`, `include_str!`): the standalone executable is one file and draws
//! people with nothing beside it. Each list is held whole by a test, and every list together
//! under [`BUDGET`]: the lists do not have to be long, they have to be well chosen.

use std::sync::LazyLock;

use super::Country;

/// Every list together, in bytes. A megabyte of names a country was the obvious way, and
/// refused: a few hundred given names and a thousand surnames give hundreds of thousands of
/// people.
#[cfg(test)]
pub(super) const BUDGET: usize = 100 * 1024;

const NAMES: &str = include_str!("../../../resources/identity/names.txt");

/// A country's places and streets.
fn land_text(country: Country) -> &'static str {
    match country {
        Country::Fr => include_str!("../../../resources/identity/places/fr.txt"),
        Country::Be => include_str!("../../../resources/identity/places/be.txt"),
        Country::De => include_str!("../../../resources/identity/places/de.txt"),
        Country::Es => include_str!("../../../resources/identity/places/es.txt"),
        Country::It => include_str!("../../../resources/identity/places/it.txt"),
        Country::Pt => include_str!("../../../resources/identity/places/pt.txt"),
        Country::Nl => include_str!("../../../resources/identity/places/nl.txt"),
        Country::Ch => include_str!("../../../resources/identity/places/ch.txt"),
        Country::Gb => include_str!("../../../resources/identity/places/gb.txt"),
        Country::Pl => include_str!("../../../resources/identity/places/pl.txt"),
        Country::Se => include_str!("../../../resources/identity/places/se.txt"),
        Country::Us => include_str!("../../../resources/identity/places/us.txt"),
        Country::Ca => include_str!("../../../resources/identity/places/ca.txt"),
    }
}

/// The surname as the person bears it. ⚠️ One in `-ski`, `-cki` or `-dzki` is an adjective:
/// a woman's ends in `-a`. The grammar of the name, read off its ending and nothing else.
pub(super) fn borne(surname: &str, female: bool) -> String {
    match surname.strip_suffix('i') {
        Some(stem) if female && ["sk", "ck", "dzk"].iter().any(|end| stem.ends_with(end)) => {
            format!("{stem}a")
        }
        _ => surname.to_owned(),
    }
}

pub(super) struct Names {
    pub(super) female: Vec<&'static str>,
    pub(super) male: Vec<&'static str>,
    pub(super) surnames: Vec<&'static str>,
}

/// Three lines: `f:`, `m:` and `s:`, each a list — none of them empty.
fn names(text: &'static str) -> Option<Names> {
    let line = |prefix: &str| -> Option<Vec<&'static str>> {
        let names: Vec<&'static str> = text
            .lines()
            .find_map(|line| line.strip_prefix(prefix))?
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .collect();
        Some(names).filter(|names| !names.is_empty())
    };
    Some(Names {
        female: line("f:")?,
        male: line("m:")?,
        surnames: line("s:")?,
    })
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

    /// Whole enough to draw from: a place, every part of each, and streets of its own tag.
    fn is_whole(&self) -> bool {
        !self.places.is_empty()
            && self.places.iter().all(|place| {
                ![
                    place.code,
                    place.postcode,
                    place.city,
                    place.region,
                    place.more,
                ]
                .iter()
                .any(|part| part.is_empty())
                    && self
                        .streets
                        .iter()
                        .any(|(tag, streets)| *tag == place.tag && !streets.is_empty())
            })
    }
}

/// The places, a line each, then a list of streets under each `---` and its tag. ⚠️ Read line
/// by line: a checkout may have turned every line ending into two characters.
fn land(text: &'static str) -> Land {
    let mut land = Land {
        places: Vec::new(),
        streets: Vec::new(),
    };
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
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

/// Every list, as compiled in.
pub(super) struct Library {
    pub(super) names: Names,
    lands: Vec<(Country, Land)>,
}

impl Library {
    pub(super) fn land(&self, wanted: Country) -> &Land {
        self.lands
            .iter()
            .find(|(country, _)| *country == wanted)
            .map_or(&self.lands[0].1, |(_, land)| land)
    }
}

/// Every list read with these names, or nothing when one is not whole.
pub(super) fn read_with(names_text: &'static str) -> Option<Library> {
    let mut library = Library {
        names: names(names_text)?,
        lands: Vec::new(),
    };
    for country in Country::ALL {
        let land = land(land_text(country));
        if !land.is_whole() {
            return None;
        }
        library.lands.push((country, land));
    }
    Some(library)
}

/// The lists, read at the first card and kept. ⚠️ Whole by construction: a test reads every one
/// of them, so the `expect` cannot be reached by a build that passed its tests.
pub(super) fn library() -> &'static Library {
    static KEPT: LazyLock<Library> =
        LazyLock::new(|| read_with(NAMES).expect("the lists compiled in are whole"));
    &KEPT
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// Where the repository keeps the lists.
    fn folder() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/identity")
    }

    /// The obvious way was refused: this is what keeps it out.
    #[test]
    fn every_list_together_holds_to_the_budget() {
        let weight: usize = NAMES.len()
            + Country::ALL
                .iter()
                .map(|country| land_text(*country).len())
                .sum::<usize>();
        assert!(
            weight <= BUDGET,
            "{weight} bytes of lists, {BUDGET} at most"
        );
    }

    /// ⚠️ What is compiled in is the whole folder — and the names are **one file**: a second one
    /// beside it would be names sorted by something, which is what this folder does not do.
    #[test]
    fn the_folder_holds_one_file_of_names_a_file_a_country_and_nothing_else() {
        let entries = |folder: std::path::PathBuf| -> Vec<String> {
            std::fs::read_dir(folder)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        };
        let mut found: Vec<String> = entries(folder())
            .into_iter()
            .filter(|name| name != "places")
            .chain(
                entries(folder().join("places"))
                    .into_iter()
                    .map(|name| format!("places/{name}")),
            )
            .collect();
        let mut expected: Vec<String> = std::iter::once("names.txt".to_owned())
            .chain(
                Country::ALL
                    .iter()
                    .map(|country| format!("places/{}.txt", country.as_str().to_lowercase())),
            )
            .collect();
        found.sort();
        expected.sort();

        assert_eq!(found, expected);
    }

    /// What a list was checked for when it was read at run time, now checked here.
    #[test]
    fn every_list_compiled_in_is_whole() {
        assert!(names(NAMES).is_some());
        for country in Country::ALL {
            assert!(land(land_text(country)).is_whole(), "{country}");
        }
        assert!(read_with(NAMES).is_some());
    }

    /// Emptied or cut short, a list is not whole: the test above would fail on it.
    #[test]
    fn a_list_cut_short_is_not_whole() {
        // Names with no surname, and with a list left empty.
        for text in [
            "f:Anna
m:Jan
",
            "f:Anna
m: , 
s:Nowak
",
        ] {
            assert!(names(text).is_none(), "{text}");
        }
        // A country with no place, a place cut short, and a place whose streets are not there.
        for text in [
            "---
High Street
",
            "ENG;M1 #@@;Manchester
---
High Street
",
            "ENG;M1 #@@;Manchester;Greater Manchester;0161 496 0;cy
---
High Street
",
            "ENG;M1 #@@;Manchester;Greater Manchester;0161 496 0
---
",
        ] {
            assert!(!land(text).is_whole(), "{text}");
        }
    }

    /// The order a list is kept in: its letters, an accent or a stroke aside.
    fn rank(name: &str) -> (String, &str) {
        (deunicode::deunicode(name).to_lowercase(), name)
    }

    /// ⚠️ Alphabetical, and nothing else: an order that groups the names would say where they
    /// come from, in a file that says it of none. Each name is there once.
    #[test]
    fn the_names_are_in_alphabetical_order_each_once_and_enough_of_each_kind() {
        let names = &library().names;
        for (kind, list, at_least) in [
            ("female", &names.female, 400),
            ("male", &names.male, 400),
            ("surnames", &names.surnames, 800),
        ] {
            assert!(list.len() >= at_least, "{} {kind}", list.len());
            for pair in list.windows(2) {
                assert!(
                    rank(pair[0]) < rank(pair[1]),
                    "{kind}: « {} » is before « {} »",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    #[test]
    fn a_surname_that_is_an_adjective_follows_the_person() {
        assert_eq!(borne("Kowalski", true), "Kowalska");
        assert_eq!(borne("Nowicki", true), "Nowicka");
        assert_eq!(borne("Zawadzki", true), "Zawadzka");
        assert_eq!(borne("Kowalski", false), "Kowalski");
        assert_eq!(borne("Nowak", true), "Nowak");
        assert_eq!(borne("Rossi", true), "Rossi");
    }

    #[test]
    fn a_country_has_enough_places_and_streets_for_each_of_them() {
        for country in Country::ALL {
            let land = library().land(country);
            assert!(land.places.len() >= 20, "{country}: {}", land.places.len());
            for (tag, streets) in &land.streets {
                assert!(streets.len() >= 10, "{country} {tag}: {}", streets.len());
            }
            assert!(land.is_whole(), "{country}");
        }
    }
}
