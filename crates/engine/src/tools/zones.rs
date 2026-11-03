//! One instant in several zones, from the IANA database compiled into the binary: nothing is read
//! from the system but the name of its own zone, so every machine answers the same.

use std::str::FromStr;

use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::{OffsetComponents, OffsetName, TZ_VARIANTS, Tz};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::dates::{self, Parsed, Refusal};
use crate::count::saturating_u32;
use crate::notes::view::fold;

pub const MAX_FOUND: usize = 40;

/// Kept apart from the rest: backward links to a region's old names, and the POSIX-style ones.
const NOT_A_PLACE: [&str; 9] = [
    "Etc/",
    "SystemV/",
    "US/",
    "Brazil/",
    "Canada/",
    "Chile/",
    "Mexico/",
    "posixrules",
    "Factory",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ZoneEntry {
    pub zone: String,
    pub city: String,
    pub offset: String,
    pub abbreviation: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ZonesRequest {
    /// A date and time on the wall clock of `from`, or an instant that carries its offset.
    pub text: String,
    /// `None` is this machine's zone.
    pub from: Option<String>,
    pub zones: Vec<String>,
    /// For a time the clocks went through twice: the second pass rather than the first.
    pub later: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ZoneTime {
    pub zone: String,
    pub city: String,
    pub local: bool,
    /// The zone the time was typed in.
    pub source: bool,
    pub date: String,
    pub time: String,
    pub iso: String,
    pub offset: String,
    pub abbreviation: Option<String>,
    pub summer_time: bool,
    /// Days ahead of (or behind) the date typed: `1` in Tokyo for an evening in Paris.
    pub day_shift: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ZoneReading {
    pub offset: String,
    pub abbreviation: Option<String>,
    pub iso_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ZonesAnswer {
    Placed {
        from: String,
        /// Both readings of a time the clocks went through twice, the earlier first.
        ambiguous: Option<[ZoneReading; 2]>,
        times: Vec<ZoneTime>,
        unknown: Vec<String>,
    },
    /// The clocks jumped over it, from one offset to the other.
    Skipped {
        from: String,
        before: String,
        after: String,
    },
    /// One-based, in characters.
    Unreadable {
        at: u32,
    },
    OutOfRange,
    UnknownZone {
        zone: String,
    },
}

fn city(zone: &str) -> String {
    zone.rsplit('/').next().unwrap_or(zone).replace('_', " ")
}

fn is_place(zone: &str) -> bool {
    zone == "UTC"
        || (zone.contains('/') && !NOT_A_PLACE.iter().any(|prefix| zone.starts_with(prefix)))
}

/// An abbreviation the database has no letters for is written as its offset (`+0530`): no name.
fn abbreviation(name: Option<&str>) -> Option<String> {
    name.filter(|name| name.starts_with(|c: char| c.is_ascii_alphabetic()))
        .map(str::to_owned)
}

fn offset_text(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let seconds = seconds.unsigned_abs();
    format!("{sign}{:02}:{:02}", seconds / 3600, seconds % 3600 / 60)
}

fn entry(zone: Tz, at: DateTime<Utc>) -> ZoneEntry {
    let offset = zone.offset_from_utc_datetime(&at.naive_utc());
    ZoneEntry {
        zone: zone.name().to_owned(),
        city: city(zone.name()),
        offset: offset_text(offset.fix().local_minus_utc()),
        abbreviation: abbreviation(offset.abbreviation()),
    }
}

/// By name, city or abbreviation, case and accents aside; an abbreviation met whole comes first.
pub fn search(query: &str, at: DateTime<Utc>) -> Vec<ZoneEntry> {
    let wanted = fold(query.trim());
    if wanted.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<(bool, ZoneEntry)> = TZ_VARIANTS
        .iter()
        .filter(|zone| is_place(zone.name()))
        .map(|zone| entry(*zone, at))
        .filter_map(|entry| {
            let named = entry
                .abbreviation
                .as_deref()
                .is_some_and(|name| fold(name) == wanted);
            let matches = named || fold(&entry.zone.replace('_', " ")).contains(&wanted);
            matches.then_some((named, entry))
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.zone.cmp(&b.1.zone)));
    found
        .into_iter()
        .take(MAX_FOUND)
        .map(|(_, entry)| entry)
        .collect()
}

fn zone_reading(at: DateTime<Tz>) -> ZoneReading {
    ZoneReading {
        offset: offset_text(at.offset().fix().local_minus_utc()),
        abbreviation: abbreviation(at.offset().abbreviation()),
        iso_utc: at
            .with_timezone(&Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
    }
}

fn time_in(
    zone: Tz,
    at: DateTime<Utc>,
    typed: NaiveDateTime,
    from: Tz,
    local: Option<Tz>,
) -> ZoneTime {
    let there = at.with_timezone(&zone);
    let offset = there.offset();
    ZoneTime {
        zone: zone.name().to_owned(),
        city: city(zone.name()),
        local: Some(zone) == local,
        source: zone == from,
        date: there.format("%Y-%m-%d").to_string(),
        time: there.format("%H:%M:%S").to_string(),
        iso: there.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, false),
        offset: offset_text(offset.fix().local_minus_utc()),
        abbreviation: abbreviation(offset.abbreviation()),
        summer_time: !offset.dst_offset().is_zero(),
        day_shift: there.date_naive().num_days_from_ce() - typed.date().num_days_from_ce(),
    }
}

/// `local` is this machine's zone by name, when the system gives one the database knows.
pub fn place(request: &ZonesRequest, local: Option<&str>) -> ZonesAnswer {
    let local = local.and_then(|name| Tz::from_str(name).ok());
    let from_name = request
        .from
        .clone()
        .or_else(|| local.map(|zone| zone.name().to_owned()))
        .unwrap_or_else(|| "UTC".to_owned());
    let Ok(from) = Tz::from_str(&from_name) else {
        return ZonesAnswer::UnknownZone { zone: from_name };
    };

    let (at, ambiguous) = match dates::parse(&request.text, None) {
        Err(Refusal::Unreadable(at)) => {
            return ZonesAnswer::Unreadable {
                at: saturating_u32(at) + 1,
            };
        }
        Err(Refusal::OutOfRange | Refusal::Skipped) => return ZonesAnswer::OutOfRange,
        Ok(Parsed::Absolute(read)) => (read.at, None),
        Ok(Parsed::Wall(naive, _)) => match from.from_local_datetime(&naive) {
            LocalResult::Single(at) => (at.with_timezone(&Utc), None),
            LocalResult::Ambiguous(earlier, later) => (
                if request.later { later } else { earlier }.with_timezone(&Utc),
                Some([zone_reading(earlier), zone_reading(later)]),
            ),
            LocalResult::None => {
                let around = |hours: i64| {
                    from.from_local_datetime(&(naive + Duration::hours(hours)))
                        .earliest()
                        .map_or_else(String::new, |at| {
                            offset_text(at.offset().fix().local_minus_utc())
                        })
                };
                return ZonesAnswer::Skipped {
                    from: from_name,
                    before: around(-24),
                    after: around(24),
                };
            }
        },
    };
    // ⚠️ chrono panics on a local time past its range: a timestamp can reach its edges.
    if at.year().abs() > 9999 {
        return ZonesAnswer::OutOfRange;
    }
    let typed = at.with_timezone(&from).naive_local();

    let mut unknown = Vec::new();
    let mut seen: Vec<Tz> = Vec::new();
    let mut times = Vec::new();
    let listed = local.into_iter().chain([from]).map(Ok).chain(
        request
            .zones
            .iter()
            .map(|name| Tz::from_str(name).map_err(|_| name.clone())),
    );
    for zone in listed {
        match zone {
            Ok(zone) if !seen.contains(&zone) => {
                seen.push(zone);
                times.push(time_in(zone, at, typed, from, local));
            }
            Ok(_) => {}
            Err(name) => unknown.push(name),
        }
    }

    ZonesAnswer::Placed {
        from: from_name,
        ambiguous,
        times,
        unknown,
    }
}

/// The wall clock of a zone now, as the field takes it.
pub fn now_in(zone: Option<&str>, local: Option<&str>) -> String {
    let zone = zone
        .or(local)
        .and_then(|name| Tz::from_str(name).ok())
        .unwrap_or(Tz::UTC);
    Utc::now()
        .with_timezone(&zone)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(text: &str, from: &str, zones: &[&str], later: bool) -> ZonesAnswer {
        place(
            &ZonesRequest {
                text: text.to_owned(),
                from: Some(from.to_owned()),
                zones: zones.iter().map(|zone| (*zone).to_owned()).collect(),
                later,
            },
            Some("Europe/Paris"),
        )
    }

    fn placed(answer: ZonesAnswer) -> (Option<[ZoneReading; 2]>, Vec<ZoneTime>, Vec<String>) {
        match answer {
            ZonesAnswer::Placed {
                ambiguous,
                times,
                unknown,
                ..
            } => (ambiguous, times, unknown),
            other => panic!("not placed: {other:?}"),
        }
    }

    fn row(times: &[ZoneTime], zone: &str) -> ZoneTime {
        times
            .iter()
            .find(|time| time.zone == zone)
            .cloned()
            .unwrap_or_else(|| panic!("no {zone}"))
    }

    #[test]
    fn an_evening_in_paris_is_the_next_morning_in_tokyo() {
        let (ambiguous, times, _) = placed(ask(
            "2026-09-29 20:30",
            "Europe/Paris",
            &["America/New_York", "Asia/Tokyo", "UTC"],
            false,
        ));

        assert_eq!(ambiguous, None);
        let zones: Vec<&str> = times.iter().map(|time| time.zone.as_str()).collect();
        assert_eq!(
            zones,
            ["Europe/Paris", "America/New_York", "Asia/Tokyo", "UTC"]
        );
        assert_eq!(
            row(&times, "Europe/Paris"),
            ZoneTime {
                zone: "Europe/Paris".into(),
                city: "Paris".into(),
                local: true,
                source: true,
                date: "2026-09-29".into(),
                time: "20:30:00".into(),
                iso: "2026-09-29T20:30:00+02:00".into(),
                offset: "+02:00".into(),
                abbreviation: Some("CEST".into()),
                summer_time: true,
                day_shift: 0,
            }
        );
        let new_york = row(&times, "America/New_York");
        assert_eq!(
            (
                new_york.time.as_str(),
                new_york.offset.as_str(),
                new_york.abbreviation.as_deref()
            ),
            ("14:30:00", "-04:00", Some("EDT"))
        );
        assert_eq!(new_york.city, "New York");
        let tokyo = row(&times, "Asia/Tokyo");
        assert_eq!(
            (tokyo.date.as_str(), tokyo.time.as_str()),
            ("2026-09-30", "03:30:00")
        );
        assert_eq!(
            (tokyo.day_shift, tokyo.summer_time),
            (1, false),
            "no summer time in Japan"
        );
        assert_eq!(row(&times, "UTC").abbreviation.as_deref(), Some("UTC"));
    }

    #[test]
    fn a_half_hour_and_a_quarter_hour_zone_keep_their_minutes() {
        let (_, times, _) = placed(ask(
            "2026-09-29T12:00:00Z",
            "UTC",
            &["Asia/Kolkata", "Asia/Kathmandu"],
            false,
        ));

        let kolkata = row(&times, "Asia/Kolkata");
        assert_eq!(
            (kolkata.time.as_str(), kolkata.offset.as_str()),
            ("17:30:00", "+05:30")
        );
        assert_eq!(kolkata.abbreviation.as_deref(), Some("IST"));
        let kathmandu = row(&times, "Asia/Kathmandu");
        assert_eq!(
            (kathmandu.time.as_str(), kathmandu.offset.as_str()),
            ("17:45:00", "+05:45")
        );
        assert_eq!(
            kathmandu.abbreviation, None,
            "the database writes +0545, which is no name"
        );
    }

    #[test]
    fn a_skipped_hour_says_between_which_offsets() {
        assert_eq!(
            ask("2026-03-29 02:30", "Europe/Paris", &[], false),
            ZonesAnswer::Skipped {
                from: "Europe/Paris".into(),
                before: "+01:00".into(),
                after: "+02:00".into(),
            }
        );
        assert_eq!(
            ask("2026-03-08T02:30", "America/New_York", &[], false),
            ZonesAnswer::Skipped {
                from: "America/New_York".into(),
                before: "-05:00".into(),
                after: "-04:00".into(),
            }
        );
    }

    #[test]
    fn a_repeated_hour_gives_both_readings_and_places_the_one_chosen() {
        let (ambiguous, times, _) =
            placed(ask("2026-10-25 02:30", "Europe/Paris", &["UTC"], false));
        assert_eq!(
            ambiguous,
            Some([
                ZoneReading {
                    offset: "+02:00".into(),
                    abbreviation: Some("CEST".into()),
                    iso_utc: "2026-10-25T00:30:00Z".into(),
                },
                ZoneReading {
                    offset: "+01:00".into(),
                    abbreviation: Some("CET".into()),
                    iso_utc: "2026-10-25T01:30:00Z".into(),
                },
            ])
        );
        assert_eq!(row(&times, "UTC").time, "00:30:00");

        let (_, times, _) = placed(ask("2026-10-25 02:30", "Europe/Paris", &["UTC"], true));
        assert_eq!(row(&times, "UTC").time, "01:30:00");
        assert!(!row(&times, "Europe/Paris").summer_time);

        let (ambiguous, _, _) = placed(ask("2026-11-01 01:30", "America/New_York", &[], false));
        let [first, second] = ambiguous.expect("repeated in New York");
        assert_eq!(
            (first.iso_utc.as_str(), second.iso_utc.as_str()),
            ("2026-11-01T05:30:00Z", "2026-11-01T06:30:00Z")
        );
    }

    #[test]
    fn an_instant_with_its_offset_is_placed_whatever_the_zone_typed_in() {
        let (_, times, _) = placed(ask("2026-09-29T12:00:00Z", "Asia/Tokyo", &["UTC"], false));

        assert_eq!(row(&times, "UTC").time, "12:00:00");
        assert_eq!(row(&times, "Asia/Tokyo").day_shift, 0);
    }

    #[test]
    fn an_unknown_zone_is_said_and_the_local_one_listed_once() {
        assert_eq!(
            ask("2026-09-29 12:00", "Mars/Olympus_Mons", &[], false),
            ZonesAnswer::UnknownZone {
                zone: "Mars/Olympus_Mons".into()
            }
        );
        let (_, times, unknown) = placed(ask(
            "2026-09-29 12:00",
            "UTC",
            &["Europe/Paris", "Nowhere/City", "UTC", "UTC"],
            false,
        ));
        assert_eq!(unknown, ["Nowhere/City"]);
        assert_eq!(times.len(), 2);
        assert!(times[0].local);
    }

    #[test]
    fn without_a_local_zone_the_time_is_read_in_utc() {
        let answer = place(
            &ZonesRequest {
                text: "2026-09-29 12:00".into(),
                from: None,
                zones: vec!["Asia/Tokyo".into()],
                later: false,
            },
            None,
        );

        let ZonesAnswer::Placed { from, times, .. } = answer else {
            panic!("placed");
        };
        assert_eq!(from, "UTC");
        assert!(times[0].source && !times[0].local);
        assert_eq!(times[1].time, "21:00:00");
    }

    #[test]
    fn a_timestamp_at_the_edge_of_the_range_is_refused_not_panicked_on() {
        assert_eq!(
            ask("8210298412799000000000", "Pacific/Kiritimati", &[], false),
            ZonesAnswer::OutOfRange
        );
        assert_eq!(
            ask(&"9".repeat(50), "UTC", &[], false),
            ZonesAnswer::OutOfRange
        );
    }

    #[test]
    fn an_unreadable_time_says_where() {
        assert_eq!(
            ask("2026-09-29 25:00", "UTC", &[], false),
            ZonesAnswer::Unreadable { at: 12 }
        );
    }

    #[test]
    fn an_abbreviation_several_zones_use_lists_them_all_rather_than_guessing() {
        let winter = DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let zones = |query: &str| -> Vec<String> {
            search(query, winter)
                .into_iter()
                .filter(|entry| entry.abbreviation.as_deref() == Some(query))
                .map(|entry| entry.zone)
                .collect()
        };

        let ist = zones("IST");
        assert!(ist.contains(&"Asia/Kolkata".to_owned()), "{ist:?}");
        assert!(ist.contains(&"Asia/Jerusalem".to_owned()), "{ist:?}");
        let cst = zones("CST");
        assert!(cst.contains(&"America/Chicago".to_owned()), "{cst:?}");
        assert!(cst.contains(&"Asia/Shanghai".to_owned()), "{cst:?}");
    }

    #[test]
    fn a_zone_is_found_by_its_city_its_name_or_its_abbreviation() {
        let summer = DateTime::parse_from_rfc3339("2026-07-01T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let names = |query: &str| -> Vec<String> {
            search(query, summer)
                .into_iter()
                .map(|entry| entry.zone)
                .collect()
        };

        assert_eq!(names("tokyo"), ["Asia/Tokyo"]);
        assert!(names("new york").contains(&"America/New_York".to_owned()));
        assert!(names("São Paulo").contains(&"America/Sao_Paulo".to_owned()));
        let cest = search("cest", summer);
        assert!(cest.iter().any(|entry| entry.zone == "Europe/Paris"));
        assert!(
            cest.iter()
                .all(|entry| entry.abbreviation.as_deref() == Some("CEST"))
        );
        assert_eq!(search("UTC", summer)[0].zone, "UTC");
        assert!(names("etc").iter().all(|zone| !zone.starts_with("Etc/")));
        assert!(names("  ").is_empty());
        assert!(search("a", summer).len() <= MAX_FOUND);
    }

    #[test]
    fn now_in_a_zone_is_what_the_field_reads_back() {
        let text = now_in(Some("Asia/Tokyo"), None);

        assert!(matches!(dates::parse(&text, None), Ok(Parsed::Wall(..))));
        assert_eq!(now_in(Some("Nowhere"), None).len(), 19);
    }
}
