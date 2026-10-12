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
}

impl Pool {
    pub(super) const ALL: [Self; 5] = [
        Self::French,
        Self::Arabic,
        Self::Portuguese,
        Self::WestAfrican,
        Self::Vietnamese,
    ];

    fn text(self) -> &'static str {
        match self {
            Self::French => include_str!("data/pools/french.txt"),
            Self::Arabic => include_str!("data/pools/arabic.txt"),
            Self::Portuguese => include_str!("data/pools/portuguese.txt"),
            Self::WestAfrican => include_str!("data/pools/west_african.txt"),
            Self::Vietnamese => include_str!("data/pools/vietnamese.txt"),
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
}

pub(super) struct Names {
    pub(super) female: Vec<&'static str>,
    pub(super) male: Vec<&'static str>,
    pub(super) surnames: Vec<&'static str>,
}

/// A town with what its country says of it. `code` and `more` are the country's own: in
/// France the INSEE code of the commune and the first digit after the `0` of its phone numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Place {
    pub(super) code: &'static str,
    pub(super) postcode: &'static str,
    pub(super) city: &'static str,
    pub(super) region: &'static str,
    pub(super) more: &'static str,
}

pub(super) struct Land {
    pub(super) places: Vec<Place>,
    pub(super) streets: Vec<&'static str>,
}

fn land_text(country: Country) -> &'static str {
    match country {
        Country::Fr => include_str!("data/places/fr.txt"),
    }
}

/// The places, a line each with its five parts, then `---`, then the streets. ⚠️ Read line by
/// line: a checkout may have turned every line ending into two characters.
pub(super) fn land(country: Country) -> Land {
    let mut lines = land_text(country).lines();
    Land {
        places: lines
            .by_ref()
            .take_while(|line| *line != "---")
            .filter_map(|line| {
                let mut parts = line.split(';');
                Some(Place {
                    code: parts.next()?,
                    postcode: parts.next()?,
                    city: parts.next()?,
                    region: parts.next()?,
                    more: parts.next().unwrap_or_default(),
                })
            })
            .collect(),
        streets: lines.filter(|line| !line.is_empty()).collect(),
    }
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
    use std::collections::HashSet;

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

    #[test]
    fn a_country_has_its_places_whole_and_its_streets() {
        for country in Country::ALL {
            let land = land(country);
            assert!(land.places.len() >= 20, "{country}: {}", land.places.len());
            assert!(
                land.streets.len() >= 20,
                "{country}: {}",
                land.streets.len()
            );
            for place in &land.places {
                assert!(
                    ![place.code, place.postcode, place.city, place.region]
                        .iter()
                        .any(|part| part.is_empty()),
                    "{country}: {place:?}"
                );
            }
        }
    }
}
