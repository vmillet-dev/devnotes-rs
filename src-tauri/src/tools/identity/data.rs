//! The lists an identity is drawn from: pools of names an origin, and a country's places and
//! streets. ⚠️ Files shipped beside the application (`resources/identity/`, a Tauri resource),
//! read once and kept for the session: a name is corrected or a list completed without
//! building the application again. What was a constant is a file someone can remove, so each
//! list is checked when read, and a missing or damaged one is said — nothing divides by the
//! length of an empty list. Every list together is held under [`BUDGET`] by a test: the lists
//! do not have to be long, they have to be well chosen.

use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use specta::Type;

use super::Country;
use crate::closed_enum::closed_enum;

/// Every list together, in bytes. A megabyte of names a country was the obvious way, and
/// refused: a pool of two hundred names gives thousands of people, and a country is a mix of
/// pools, not a list of its own.
#[cfg(test)]
pub(super) const BUDGET: u64 = 100 * 1024;

closed_enum! {
    pub enum ListProblem {
        #[default]
        Missing = "missing",
        Damaged = "damaged",
    }
}

/// A list that could not be used: which one, and where it was looked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UnreadableList {
    pub problem: ListProblem,
    /// As it is named under the folder: `pools/french.txt`.
    pub file: String,
    pub folder: String,
}

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

    fn file(self) -> &'static str {
        match self {
            Self::French => "pools/french.txt",
            Self::Arabic => "pools/arabic.txt",
            Self::Portuguese => "pools/portuguese.txt",
            Self::WestAfrican => "pools/west_african.txt",
            Self::Vietnamese => "pools/vietnamese.txt",
            Self::English => "pools/english.txt",
            Self::German => "pools/german.txt",
            Self::Dutch => "pools/dutch.txt",
            Self::Spanish => "pools/spanish.txt",
            Self::Italian => "pools/italian.txt",
            Self::Polish => "pools/polish.txt",
            Self::Nordic => "pools/nordic.txt",
            Self::Turkish => "pools/turkish.txt",
            Self::SouthAsian => "pools/south_asian.txt",
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

fn land_file(country: Country) -> String {
    format!("places/{}.txt", country.as_str().to_lowercase())
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

/// Every list, as read from the folder the application ships.
pub(super) struct Library {
    pools: Vec<(Pool, Names)>,
    lands: Vec<(Country, Land)>,
}

impl Library {
    pub(super) fn names(&self, wanted: Pool) -> &Names {
        self.pools
            .iter()
            .find(|(pool, _)| *pool == wanted)
            .map_or(&self.pools[0].1, |(_, names)| names)
    }

    pub(super) fn land(&self, wanted: Country) -> &Land {
        self.lands
            .iter()
            .find(|(country, _)| *country == wanted)
            .map_or(&self.lands[0].1, |(_, land)| land)
    }
}

/// The folder as a person reads a path: Windows hands the resources over behind `\\?\`.
fn shown(root: &Path) -> String {
    let path = root.to_string_lossy();
    path.strip_prefix(r"\\?\").unwrap_or(&path).to_owned()
}

/// A file's text, for as long as the application runs. ⚠️ Given away on purpose: the lists
/// are read once and every name drawn afterwards points into them. An editor's byte-order
/// mark is not a name.
fn text(root: &Path, file: &str) -> Result<&'static str, UnreadableList> {
    let unreadable = |problem| UnreadableList {
        problem,
        file: file.to_owned(),
        folder: shown(root),
    };
    let bytes = std::fs::read(root.join(file)).map_err(|_| unreadable(ListProblem::Missing))?;
    let text = String::from_utf8(bytes).map_err(|_| unreadable(ListProblem::Damaged))?;
    let kept: &'static str = Box::leak(text.into_boxed_str());
    Ok(kept.trim_start_matches('\u{feff}'))
}

/// Every list read and checked, or the first one that cannot be used.
pub(super) fn read(root: &Path) -> Result<Library, UnreadableList> {
    let damaged = |file: &str| UnreadableList {
        problem: ListProblem::Damaged,
        file: file.to_owned(),
        folder: shown(root),
    };
    let mut library = Library {
        pools: Vec::new(),
        lands: Vec::new(),
    };
    for pool in Pool::ALL {
        let names = names(text(root, pool.file())?).ok_or_else(|| damaged(pool.file()))?;
        library.pools.push((pool, names));
    }
    for country in Country::ALL {
        let file = land_file(country);
        let land = land(text(root, &file)?);
        if !land.is_whole() {
            return Err(damaged(&file));
        }
        library.lands.push((country, land));
    }
    Ok(library)
}

/// The lists of the session: read at the first card, kept for the others. ⚠️ Only what was
/// read whole is kept — a file put back is found at the next draw, without a restart.
pub(super) fn library(root: &Path) -> Result<&'static Library, UnreadableList> {
    static KEPT: Mutex<Option<&'static Library>> = Mutex::new(None);
    let mut kept = KEPT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(library) = *kept {
        return Ok(library);
    }
    let library: &'static Library = Box::leak(Box::new(read(root)?));
    *kept = Some(library);
    Ok(library)
}

/// Where the repository keeps what the application ships.
#[cfg(test)]
pub(super) fn shipped_folder() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/identity")
}

/// The shipped lists, read once for every test.
#[cfg(test)]
pub(super) fn shipped() -> &'static Library {
    library(&shipped_folder()).expect("the shipped lists are whole")
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::fs;

    use super::*;

    /// Every file the application ships, with its name under the folder.
    fn files() -> Vec<String> {
        Pool::ALL
            .iter()
            .map(|pool| pool.file().to_owned())
            .chain(Country::ALL.iter().map(|country| land_file(*country)))
            .collect()
    }

    /// A copy of the shipped folder, to damage.
    fn copy() -> tempfile::TempDir {
        let folder = tempfile::tempdir().unwrap();
        for directory in ["pools", "places"] {
            fs::create_dir(folder.path().join(directory)).unwrap();
        }
        for file in files() {
            fs::copy(shipped_folder().join(&file), folder.path().join(&file)).unwrap();
        }
        folder
    }

    /// The obvious way was refused: this is what keeps it out.
    #[test]
    fn every_list_together_holds_to_the_budget() {
        let weight: u64 = files()
            .iter()
            .map(|file| fs::metadata(shipped_folder().join(file)).unwrap().len())
            .sum();
        assert!(
            weight <= BUDGET,
            "{weight} bytes of lists, {BUDGET} at most"
        );
    }

    /// ⚠️ What is shipped is what is read: a file beside the others that no pool and no country
    /// names would be dead weight in every installer.
    #[test]
    fn the_shipped_folder_holds_the_lists_and_nothing_else() {
        let mut found = Vec::new();
        for directory in ["pools", "places"] {
            for entry in fs::read_dir(shipped_folder().join(directory)).unwrap() {
                let name = entry.unwrap().file_name().to_string_lossy().into_owned();
                found.push(format!("{directory}/{name}"));
            }
        }
        let mut expected = files();
        found.sort();
        expected.sort();

        assert_eq!(found, expected);
        assert!(read(&shipped_folder()).is_ok());
    }

    #[test]
    fn a_list_that_is_missing_is_named_with_where_it_was_looked_for() {
        let folder = copy();
        fs::remove_file(folder.path().join("pools/italian.txt")).unwrap();

        let Err(unreadable) = read(folder.path()) else {
            panic!("a pool is missing");
        };
        assert_eq!(
            unreadable,
            UnreadableList {
                problem: ListProblem::Missing,
                file: "pools/italian.txt".to_owned(),
                folder: folder.path().to_string_lossy().into_owned(),
            }
        );
        assert_eq!(shown(Path::new(r"\\?\C:\app\identity")), r"C:\app\identity");
    }

    /// ⚠️ A list is no longer part of the program: emptied, cut short or no text at all, it is
    /// said to be damaged before anything is drawn from it.
    #[test]
    fn a_list_that_is_damaged_is_said_rather_than_drawn_from() {
        let damaged_by = |file: &str, content: &[u8]| {
            let folder = copy();
            fs::write(folder.path().join(file), content).unwrap();
            read(folder.path()).err().map(|unreadable| {
                assert_eq!(unreadable.problem, ListProblem::Damaged);
                unreadable.file
            })
        };

        // A pool with no surname, one with a list left empty, and one that is no text.
        for content in [
            &b"f:Anna\nm:Jan\n"[..],
            b"f:Anna\nm: , \ns:Nowak\n",
            &[0xff, 0xfe],
        ] {
            assert_eq!(
                damaged_by("pools/polish.txt", content).as_deref(),
                Some("pools/polish.txt")
            );
        }
        // A country with no place, a place cut short, and a place whose streets are not there.
        for content in [
            "---\nHigh Street\n",
            "ENG;M1 #@@;Manchester\n---\nHigh Street\n",
            "ENG;M1 #@@;Manchester;Greater Manchester;0161 496 0;cy\n---\nHigh Street\n",
            "ENG;M1 #@@;Manchester;Greater Manchester;0161 496 0\n---\n",
        ] {
            assert_eq!(
                damaged_by("places/gb.txt", content.as_bytes()).as_deref(),
                Some("places/gb.txt")
            );
        }
    }

    /// An editor may have saved a list with a byte-order mark and Windows line endings.
    #[test]
    fn a_list_saved_by_an_editor_is_still_read() {
        let folder = copy();
        let file = folder.path().join("pools/nordic.txt");
        let saved = format!(
            "\u{feff}{}",
            fs::read_to_string(&file).unwrap().replace('\n', "\r\n")
        );
        fs::write(&file, saved).unwrap();

        let library = read(folder.path()).ok().unwrap();
        assert_eq!(library.names(Pool::Nordic).female[0], "Anna");
        assert_eq!(
            library.names(Pool::Nordic).surnames,
            shipped().names(Pool::Nordic).surnames
        );
    }

    #[test]
    fn a_pool_has_enough_names_of_each_kind_each_once_and_none_empty() {
        for pool in Pool::ALL {
            let names = shipped().names(pool);
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
            }
        }
    }

    /// A surname says which pool it came from: that is how a mix is counted.
    #[test]
    fn no_surname_sits_in_two_pools() {
        let mut seen: HashMap<&str, Pool> = HashMap::new();
        for pool in Pool::ALL {
            for surname in &shipped().names(pool).surnames {
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
    fn a_country_has_enough_places_and_streets_for_each_of_them() {
        for country in Country::ALL {
            let land = shipped().land(country);
            assert!(land.places.len() >= 20, "{country}: {}", land.places.len());
            for (tag, streets) in &land.streets {
                assert!(streets.len() >= 10, "{country} {tag}: {}", streets.len());
            }
            assert!(land.is_whole(), "{country}");
        }
    }
}
