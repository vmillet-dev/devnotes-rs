//! The gap between two dates, counted by the calendar and by the clock, and ISO 8601 durations
//! read — or written from words such as « 1h30 ».

use std::fmt::Write;
use std::str::FromStr;

use chrono::{
    DateTime, Datelike, Duration, LocalResult, Months, NaiveDateTime, Offset, TimeZone, Utc,
};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use specta::Type;

use super::dates::{self, Parsed, Refusal};
use crate::count::saturating_u32;

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DurationsRequest {
    pub from: String,
    pub to: String,
    pub duration: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DurationsAnswer {
    /// Where a date written without an offset is read: the machine's zone, UTC without one.
    pub zone: String,
    /// `None` until both dates are typed.
    pub gap: Option<GapAnswer>,
    pub duration: Option<DurationAnswer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum GapField {
    From,
    To,
}

/// What a calendar counts: whole months first, each of its own length, then the wall clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CalendarGap {
    pub years: u32,
    pub months: u32,
    pub days: u32,
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
}

/// What a clock counts between the two instants, cut several ways.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Elapsed {
    pub days: u32,
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
    pub weeks: u32,
    pub week_days: u32,
    pub total_hours: u32,
    pub total_minutes: u32,
}

/// The two counts part when the zone's offset differs at either end: a daylight-saving change.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    /// The local date the clocks changed on — the last change, when there were several.
    pub date: Option<String>,
    /// The clocks went forward, to summer time.
    pub forward: bool,
    /// The calendar's count less the clock's.
    pub minutes: i32,
    /// The wall clock's total, beside `Elapsed::total_minutes`.
    pub wall_minutes: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GapAnswer {
    Measured {
        /// `to` before `from`: the parts are the gap's size, the ISO form carries the sign.
        negative: bool,
        calendar: CalendarGap,
        elapsed: Elapsed,
        /// The calendar gap, `P6M22DT7H41M`.
        iso: String,
        transition: Option<Transition>,
        /// The fields whose local time came twice: the earlier reading is taken.
        ambiguous: Vec<GapField>,
    },
    /// One-based, in characters.
    Unreadable { field: GapField, at: u32 },
    /// A local time the clocks skipped when they went forward.
    Skipped { field: GapField },
    /// Past the year 9999 either way.
    OutOfRange { field: GapField },
}

/// Each part as typed: a fraction is allowed on the last one only, so they are `f64`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DurationParts {
    pub years: f64,
    pub months: f64,
    pub weeks: f64,
    pub days: f64,
    pub hours: f64,
    pub minutes: f64,
    pub seconds: f64,
}

/// A day is counted as 24 hours, and a week as 7 of them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DurationTotals {
    pub seconds: f64,
    pub minutes: f64,
    pub hours: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DurationProblem {
    /// Not what an ISO duration or a unit of words may hold here.
    Unexpected,
    /// `P` or `PT` with nothing after it.
    Empty,
    /// A unit after a smaller one: `PT30M1H`.
    OutOfOrder,
    Repeated,
    /// A fraction on a part that is not the last.
    FractionNotLast,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DurationAnswer {
    Read {
        negative: bool,
        parts: DurationParts,
        iso: String,
        /// Typed as words, « 1h30 », rather than in ISO 8601.
        from_words: bool,
        /// `None` with years or months: neither has a fixed length in seconds.
        totals: Option<DurationTotals>,
    },
    /// One-based, in characters.
    Unreadable { at: u32, problem: DurationProblem },
}

/// `zone` is the machine's in the command, a fixed one in the tests.
pub fn measure(request: &DurationsRequest, zone: Tz) -> DurationsAnswer {
    let gap = (!request.from.trim().is_empty() && !request.to.trim().is_empty())
        .then(|| gap(&request.from, &request.to, zone));
    let duration =
        (!request.duration.trim().is_empty()).then(|| read_duration(request.duration.trim()));
    DurationsAnswer {
        zone: zone.name().to_owned(),
        gap,
        duration,
    }
}

/// The machine's zone by its IANA name, UTC when the system gives none the database knows.
pub fn zone_named(name: Option<&str>) -> Tz {
    name.and_then(|name| Tz::from_str(name).ok())
        .unwrap_or(Tz::UTC)
}

/// The instant a field names, and whether its local time came twice; or why it names none.
fn place(text: &str, zone: Tz) -> Result<(DateTime<Utc>, bool), Refusal> {
    let placed = match dates::parse(text, None)? {
        Parsed::Absolute(read) => (read.at, false),
        Parsed::Wall(naive, _) => match zone.from_local_datetime(&naive) {
            LocalResult::Single(at) => (at.with_timezone(&Utc), false),
            LocalResult::Ambiguous(earlier, _) => (earlier.with_timezone(&Utc), true),
            LocalResult::None => return Err(Refusal::Skipped),
        },
    };
    // ⚠️ chrono panics on a local time past its range: an offset can push a timestamp there.
    if placed.0.year().abs() > 9999 {
        return Err(Refusal::OutOfRange);
    }
    Ok(placed)
}

fn refused(field: GapField, refusal: Refusal) -> GapAnswer {
    match refusal {
        Refusal::Unreadable(at) => GapAnswer::Unreadable {
            field,
            at: saturating_u32(at).saturating_add(1),
        },
        Refusal::Skipped => GapAnswer::Skipped { field },
        Refusal::OutOfRange => GapAnswer::OutOfRange { field },
    }
}

fn gap(from: &str, to: &str, zone: Tz) -> GapAnswer {
    let (from, from_twice) = match place(from, zone) {
        Ok(placed) => placed,
        Err(refusal) => return refused(GapField::From, refusal),
    };
    let (to, to_twice) = match place(to, zone) {
        Ok(placed) => placed,
        Err(refusal) => return refused(GapField::To, refusal),
    };
    let negative = to < from;
    let (earlier, later) = if negative { (to, from) } else { (from, to) };
    let local = |at: DateTime<Utc>| at.with_timezone(&zone);
    let calendar = calendar(local(earlier).naive_local(), local(later).naive_local());

    GapAnswer::Measured {
        negative,
        calendar,
        elapsed: elapsed((later - earlier).num_seconds()),
        iso: write_iso(negative, &calendar_parts(calendar)),
        transition: transition(zone, earlier, later),
        ambiguous: [(from_twice, GapField::From), (to_twice, GapField::To)]
            .into_iter()
            .filter_map(|(twice, field)| twice.then_some(field))
            .collect(),
    }
}

/// Whole months from `earlier`, a day past the end of a month landing on its last (31 January
/// plus a month is 28 February), then what the wall clock shows beyond.
fn calendar(earlier: NaiveDateTime, later: NaiveDateTime) -> CalendarGap {
    let shifted = |months: u32| {
        earlier
            .checked_add_months(Months::new(months))
            .filter(|at| *at <= later)
    };
    let month = |at: NaiveDateTime| i32::try_from(at.month0()).unwrap_or(0);
    let span = (later.year() - earlier.year()) * 12 + month(later) - month(earlier);
    let mut months = u32::try_from(span).unwrap_or(0);
    let mut anchor = shifted(months);
    while anchor.is_none() && months > 0 {
        months -= 1;
        anchor = shifted(months);
    }
    let rest = (later - anchor.unwrap_or(earlier)).num_seconds().max(0);
    CalendarGap {
        years: months / 12,
        months: months % 12,
        days: saturating_u32(rest / 86_400),
        hours: saturating_u32(rest % 86_400 / 3600),
        minutes: saturating_u32(rest % 3600 / 60),
        seconds: saturating_u32(rest % 60),
    }
}

fn elapsed(seconds: i64) -> Elapsed {
    let days = seconds / 86_400;
    Elapsed {
        days: saturating_u32(days),
        hours: saturating_u32(seconds % 86_400 / 3600),
        minutes: saturating_u32(seconds % 3600 / 60),
        seconds: saturating_u32(seconds % 60),
        weeks: saturating_u32(days / 7),
        week_days: saturating_u32(days % 7),
        total_hours: saturating_u32(seconds / 3600),
        total_minutes: saturating_u32(seconds / 60),
    }
}

fn offset_at(zone: Tz, at: DateTime<Utc>) -> i32 {
    at.with_timezone(&zone).offset().fix().local_minus_utc()
}

/// The last change is looked for a day at a time back from `later`, a little over two years at
/// most, then down to the second.
fn transition(zone: Tz, earlier: DateTime<Utc>, later: DateTime<Utc>) -> Option<Transition> {
    let (before, after) = (offset_at(zone, earlier), offset_at(zone, later));
    if before == after {
        return None;
    }
    let mut date = None;
    let mut high = later;
    for _ in 0..800 {
        if high <= earlier {
            break;
        }
        let low = (high - Duration::days(1)).max(earlier);
        if offset_at(zone, low) != after {
            let (mut low, mut high) = (low, high);
            while (high - low).num_seconds() > 1 {
                let middle = low + (high - low) / 2;
                if offset_at(zone, middle) == after {
                    high = middle;
                } else {
                    low = middle;
                }
            }
            date = Some(high.with_timezone(&zone).date_naive().to_string());
            break;
        }
        high = low;
    }
    let minutes = (after - before) / 60;
    let elapsed = (later - earlier).num_seconds() / 60;
    Some(Transition {
        date,
        forward: after > before,
        minutes,
        wall_minutes: saturating_u32((elapsed + i64::from(minutes)).max(0)),
    })
}

fn calendar_parts(calendar: CalendarGap) -> DurationParts {
    DurationParts {
        years: f64::from(calendar.years),
        months: f64::from(calendar.months),
        weeks: 0.0,
        days: f64::from(calendar.days),
        hours: f64::from(calendar.hours),
        minutes: f64::from(calendar.minutes),
        seconds: f64::from(calendar.seconds),
    }
}

/// Shortest first: `1.5`, `90`, never `90.0`.
fn number(value: f64) -> String {
    format!("{value}")
}

fn write_iso(negative: bool, parts: &DurationParts) -> String {
    let date = [
        (parts.years, 'Y'),
        (parts.months, 'M'),
        (parts.weeks, 'W'),
        (parts.days, 'D'),
    ];
    let time = [
        (parts.hours, 'H'),
        (parts.minutes, 'M'),
        (parts.seconds, 'S'),
    ];
    let written = |units: &[(f64, char)]| -> String {
        units.iter().filter(|(value, _)| *value != 0.0).fold(
            String::new(),
            |mut text, (value, unit)| {
                let _ = write!(text, "{}{unit}", number(*value));
                text
            },
        )
    };
    let (date, time) = (written(&date), written(&time));
    let body = match (date.is_empty(), time.is_empty()) {
        (true, true) => "T0S".to_owned(),
        (_, true) => date,
        _ => format!("{date}T{time}"),
    };
    format!("{}P{body}", if negative { "-" } else { "" })
}

type Located = (usize, DurationProblem);

/// The seven units, in the order ISO 8601 writes them.
const YEARS: usize = 0;
const MONTHS: usize = 1;
const WEEKS: usize = 2;
const DAYS: usize = 3;
const HOURS: usize = 4;
const MINUTES: usize = 5;
const SECONDS: usize = 6;

fn set(parts: &mut DurationParts, rank: usize, value: f64) {
    *match rank {
        YEARS => &mut parts.years,
        MONTHS => &mut parts.months,
        WEEKS => &mut parts.weeks,
        DAYS => &mut parts.days,
        HOURS => &mut parts.hours,
        MINUTES => &mut parts.minutes,
        _ => &mut parts.seconds,
    } = value;
}

/// A number at `at`, its decimal comma or point included: where it stops, and whether it had one.
fn read_number(chars: &[char], at: usize) -> Option<(f64, usize, bool)> {
    let mut end = at;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    let mut fraction = false;
    if end > at && end < chars.len() && matches!(chars[end], '.' | ',') {
        let digits = end + 1;
        let mut past = digits;
        while past < chars.len() && chars[past].is_ascii_digit() {
            past += 1;
        }
        if past > digits {
            fraction = true;
            end = past;
        }
    }
    if end == at {
        return None;
    }
    let text: String = chars[at..end]
        .iter()
        .map(|c| if *c == ',' { '.' } else { *c })
        .collect();
    text.parse().ok().map(|value| (value, end, fraction))
}

/// `PnYnMnWnDTnHnMnS`, a fraction on the last part only, an optional leading `-`.
fn iso_duration(chars: &[char]) -> Result<(bool, DurationParts), Located> {
    let negative = chars.first() == Some(&'-');
    let mut at = usize::from(negative);
    if !matches!(chars.get(at), Some('P' | 'p')) {
        return Err((at, DurationProblem::Unexpected));
    }
    at += 1;
    let mut parts = DurationParts::default();
    let (mut in_time, mut last, mut fraction_seen) = (false, None::<usize>, false);
    while at < chars.len() {
        if matches!(chars[at], 'T' | 't') {
            if in_time {
                return Err((at, DurationProblem::Repeated));
            }
            in_time = true;
            at += 1;
            if at == chars.len() {
                return Err((at, DurationProblem::Empty));
            }
            continue;
        }
        let start = at;
        let (value, end, fraction) =
            read_number(chars, at).ok_or((at, DurationProblem::Unexpected))?;
        if fraction_seen {
            return Err((start, DurationProblem::FractionNotLast));
        }
        let rank = match (in_time, chars.get(end).map(char::to_ascii_uppercase)) {
            (false, Some('Y')) => YEARS,
            (false, Some('M')) => MONTHS,
            (false, Some('W')) => WEEKS,
            (false, Some('D')) => DAYS,
            (true, Some('H')) => HOURS,
            (true, Some('M')) => MINUTES,
            (true, Some('S')) => SECONDS,
            _ => return Err((end, DurationProblem::Unexpected)),
        };
        match last {
            Some(previous) if previous == rank => return Err((end, DurationProblem::Repeated)),
            Some(previous) if previous > rank => return Err((end, DurationProblem::OutOfOrder)),
            _ => {}
        }
        set(&mut parts, rank, value);
        (last, fraction_seen) = (Some(rank), fraction);
        at = end + 1;
    }
    if last.is_none() {
        return Err((at, DurationProblem::Empty));
    }
    Ok((negative, parts))
}

fn unit_of(word: &str) -> Option<usize> {
    Some(match word.to_lowercase().as_str() {
        "y" | "yr" | "yrs" | "year" | "years" | "an" | "ans" | "année" | "années" => YEARS,
        "mo" | "month" | "months" | "mois" => MONTHS,
        "w" | "wk" | "wks" | "week" | "weeks" | "sem" | "semaine" | "semaines" => WEEKS,
        "d" | "j" | "day" | "days" | "jour" | "jours" => DAYS,
        "h" | "hr" | "hrs" | "hour" | "hours" | "heure" | "heures" => HOURS,
        "m" | "mn" | "min" | "mins" | "minute" | "minutes" => MINUTES,
        "s" | "sec" | "secs" | "second" | "seconds" | "seconde" | "secondes" => SECONDS,
        _ => return None,
    })
}

/// « 1h30 », « 90 min », « 1d 2h », « 1:30 »: a bare number after hours is minutes, after
/// minutes seconds.
fn words_duration(chars: &[char]) -> Result<(bool, DurationParts), Located> {
    let negative = chars.first() == Some(&'-');
    let mut at = usize::from(negative);
    let mut parts = DurationParts::default();
    let mut seen = [false; 7];
    let mut last: Option<usize> = None;
    let mut fraction_seen = false;
    let skip_spaces = |at: &mut usize| {
        while *at < chars.len() && chars[*at].is_whitespace() {
            *at += 1;
        }
    };
    skip_spaces(&mut at);
    while at < chars.len() {
        let start = at;
        let (value, end, fraction) =
            read_number(chars, at).ok_or((at, DurationProblem::Unexpected))?;
        if fraction_seen {
            return Err((start, DurationProblem::FractionNotLast));
        }
        at = end;
        skip_spaces(&mut at);
        let rank = if at < chars.len() && chars[at] == ':' {
            at += 1;
            match last {
                None => HOURS,
                Some(HOURS) => MINUTES,
                _ => return Err((at - 1, DurationProblem::Unexpected)),
            }
        } else {
            let word_start = at;
            while at < chars.len() && chars[at].is_alphabetic() {
                at += 1;
            }
            if at > word_start {
                let word: String = chars[word_start..at].iter().collect();
                unit_of(&word).ok_or((word_start, DurationProblem::Unexpected))?
            } else {
                match last {
                    Some(HOURS) => MINUTES,
                    Some(MINUTES) => SECONDS,
                    _ => return Err((at, DurationProblem::Unexpected)),
                }
            }
        };
        if seen[rank] {
            return Err((start, DurationProblem::Repeated));
        }
        seen[rank] = true;
        set(&mut parts, rank, value);
        (last, fraction_seen) = (Some(rank), fraction);
        skip_spaces(&mut at);
    }
    if last.is_none() {
        return Err((at, DurationProblem::Empty));
    }
    Ok((negative, parts))
}

fn read_duration(text: &str) -> DurationAnswer {
    let chars: Vec<char> = text.chars().collect();
    let body = chars.get(usize::from(chars.first() == Some(&'-')));
    let iso = matches!(body, Some('P' | 'p'));
    let read = if iso {
        iso_duration(&chars)
    } else {
        words_duration(&chars)
    };
    match read {
        Ok((negative, parts)) => {
            let sign = if negative { -1.0 } else { 1.0 };
            let totals = (parts.years == 0.0 && parts.months == 0.0).then(|| {
                let seconds = sign
                    * (parts.weeks * 604_800.0
                        + parts.days * 86_400.0
                        + parts.hours * 3600.0
                        + parts.minutes * 60.0
                        + parts.seconds);
                DurationTotals {
                    seconds,
                    minutes: seconds / 60.0,
                    hours: seconds / 3600.0,
                }
            });
            DurationAnswer::Read {
                negative,
                parts,
                iso: write_iso(negative, &parts),
                from_words: !iso,
                totals,
            }
        }
        Err((at, problem)) => DurationAnswer::Unreadable {
            at: saturating_u32(at).saturating_add(1),
            problem,
        },
    }
}

// Every float compared here is a whole number or a short decimal, exact in binary.
#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const PARIS: Tz = chrono_tz::Europe::Paris;

    fn gap_of(from: &str, to: &str) -> GapAnswer {
        measure(
            &DurationsRequest {
                from: from.to_owned(),
                to: to.to_owned(),
                duration: String::new(),
            },
            PARIS,
        )
        .gap
        .expect("both dates typed")
    }

    fn measured(from: &str, to: &str) -> (bool, CalendarGap, Elapsed, String, Option<Transition>) {
        match gap_of(from, to) {
            GapAnswer::Measured {
                negative,
                calendar,
                elapsed,
                iso,
                transition,
                ..
            } => (negative, calendar, elapsed, iso, transition),
            other => panic!("{other:?}"),
        }
    }

    fn calendar_of(years: u32, months: u32, days: u32, hours: u32, minutes: u32) -> CalendarGap {
        CalendarGap {
            years,
            months,
            days,
            hours,
            minutes,
            seconds: 0,
        }
    }

    #[test]
    fn the_mockup_s_gap_counts_an_hour_more_on_the_calendar_than_on_the_clock() {
        let (negative, calendar, elapsed, iso, transition) =
            measured("2026-03-12 09:00", "2026-10-04 16:41");

        assert!(!negative);
        assert_eq!(calendar, calendar_of(0, 6, 22, 7, 41));
        assert_eq!(iso, "P6M22DT7H41M");
        assert_eq!((elapsed.days, elapsed.hours, elapsed.minutes), (206, 6, 41));
        assert_eq!((elapsed.weeks, elapsed.week_days), (29, 3));
        assert_eq!(elapsed.total_hours, 4950);
        assert_eq!(elapsed.total_minutes, 297_041);
        assert_eq!(
            transition,
            Some(Transition {
                date: Some("2026-03-29".into()),
                forward: true,
                minutes: 60,
                wall_minutes: 297_101,
            })
        );
    }

    #[test]
    fn falling_back_to_winter_time_counts_an_hour_less_on_the_calendar() {
        let (_, calendar, elapsed, _, transition) =
            measured("2026-10-24 12:00", "2026-10-26 12:00");

        assert_eq!(calendar, calendar_of(0, 0, 2, 0, 0));
        assert_eq!((elapsed.days, elapsed.hours), (2, 1));
        let transition = transition.unwrap();
        assert_eq!(transition.date.as_deref(), Some("2026-10-25"));
        assert!(!transition.forward);
        assert_eq!(transition.minutes, -60);
    }

    #[test]
    fn a_month_is_counted_by_the_calendar_and_ends_on_the_month_s_last_day() {
        assert_eq!(
            measured("2026-01-31", "2026-03-01").1,
            calendar_of(0, 1, 1, 0, 0)
        );
        assert_eq!(
            measured("2026-01-31", "2026-02-28").1,
            calendar_of(0, 1, 0, 0, 0)
        );
        assert_eq!(
            measured("2026-01-15", "2027-03-15").1,
            calendar_of(1, 2, 0, 0, 0)
        );
        assert_eq!(
            measured("2026-01-31 10:00", "2026-01-31 09:00").1,
            calendar_of(0, 0, 0, 1, 0)
        );
    }

    #[test]
    fn a_leap_day_is_a_day_of_its_own() {
        let (_, calendar, elapsed, iso, _) = measured("2028-02-28", "2028-03-01");

        assert_eq!(calendar, calendar_of(0, 0, 2, 0, 0));
        assert_eq!(elapsed.days, 2);
        assert_eq!(iso, "P2D");
        assert_eq!(
            measured("2028-02-29", "2029-02-28").1,
            calendar_of(1, 0, 0, 0, 0),
            "clamped"
        );
    }

    #[test]
    fn a_gap_backwards_is_its_size_with_a_sign() {
        let (negative, calendar, _, iso, _) = measured("2026-03-12 16:41", "2026-03-12 09:00");

        assert!(negative);
        assert_eq!(calendar, calendar_of(0, 0, 0, 7, 41));
        assert_eq!(iso, "-PT7H41M");
        assert_eq!(measured("2026-03-12", "2026-03-12").3, "PT0S");
    }

    #[test]
    fn a_time_the_clocks_skipped_is_refused_and_one_they_repeated_is_named() {
        assert_eq!(
            gap_of("2026-03-29 02:30", "2026-04-01"),
            GapAnswer::Skipped {
                field: GapField::From
            }
        );
        let GapAnswer::Measured {
            ambiguous, elapsed, ..
        } = gap_of("2026-10-25 00:00", "2026-10-25 02:30")
        else {
            panic!()
        };
        assert_eq!(ambiguous, [GapField::To]);
        assert_eq!(
            (elapsed.hours, elapsed.minutes),
            (2, 30),
            "the earlier 02:30"
        );
    }

    #[test]
    fn a_date_carrying_its_offset_or_a_timestamp_is_taken_as_written() {
        let (_, calendar, ..) = measured("2026-03-12T08:00:00Z", "1773306000");

        assert_eq!(calendar, calendar_of(0, 0, 0, 1, 0));
    }

    #[test]
    fn an_unreadable_date_is_located_in_its_own_field() {
        assert_eq!(
            gap_of("2026-03-12", "2026-13-01"),
            GapAnswer::Unreadable {
                field: GapField::To,
                at: 6
            }
        );
        assert_eq!(
            gap_of("99999-01-01", "2026-01-01"),
            GapAnswer::Unreadable {
                field: GapField::From,
                at: 5
            }
        );
    }

    #[test]
    fn without_both_dates_there_is_no_gap_and_the_zone_is_said() {
        let answer = measure(
            &DurationsRequest {
                from: "2026-03-12".into(),
                to: " ".into(),
                duration: String::new(),
            },
            PARIS,
        );

        assert_eq!(answer.gap, None);
        assert_eq!(answer.duration, None);
        assert_eq!(answer.zone, "Europe/Paris");
        assert_eq!(zone_named(Some("Nowhere/Else")), Tz::UTC);
        assert_eq!(zone_named(Some("Asia/Tokyo")), chrono_tz::Asia::Tokyo);
    }

    #[test]
    fn utc_has_no_transition() {
        let answer = measure(
            &DurationsRequest {
                from: "2026-03-12 09:00".into(),
                to: "2026-10-04 16:41".into(),
                duration: String::new(),
            },
            Tz::UTC,
        );
        let Some(GapAnswer::Measured {
            transition,
            elapsed,
            ..
        }) = answer.gap
        else {
            panic!()
        };
        assert_eq!(transition, None);
        assert_eq!((elapsed.days, elapsed.hours), (206, 7));
    }

    fn duration(text: &str) -> DurationAnswer {
        read_duration(text)
    }

    fn read(text: &str) -> (bool, DurationParts, String, bool, Option<DurationTotals>) {
        match duration(text) {
            DurationAnswer::Read {
                negative,
                parts,
                iso,
                from_words,
                totals,
            } => (negative, parts, iso, from_words, totals),
            other @ DurationAnswer::Unreadable { .. } => panic!("{text}: {other:?}"),
        }
    }

    fn refused(text: &str) -> (u32, DurationProblem) {
        match duration(text) {
            DurationAnswer::Unreadable { at, problem } => (at, problem),
            other @ DurationAnswer::Read { .. } => panic!("{text}: {other:?}"),
        }
    }

    #[test]
    fn an_iso_duration_is_read_and_totalled() {
        let (negative, parts, iso, words, totals) = read("PT1H30M");

        assert!(!negative && !words);
        assert_eq!((parts.hours, parts.minutes), (1.0, 30.0));
        assert_eq!(iso, "PT1H30M");
        assert_eq!(
            totals,
            Some(DurationTotals {
                seconds: 5400.0,
                minutes: 90.0,
                hours: 1.5
            })
        );
    }

    #[test]
    fn the_iso_table_reads_what_it_should() {
        for (text, written) in [
            ("P1Y2M3DT4H5M6S", "P1Y2M3DT4H5M6S"),
            ("p1w", "P1W"),
            ("P2W3D", "P2W3D"),
            ("PT1,5H", "PT1.5H"),
            ("PT0.25S", "PT0.25S"),
            ("-P1D", "-P1D"),
            ("P0D", "PT0S"),
            ("PT36H", "PT36H"),
        ] {
            assert_eq!(read(text).2, written, "{text}");
        }
        assert_eq!(read("P1DT12H").4.unwrap().hours, 36.0);
        assert_eq!(read("-PT90M").4.unwrap().seconds, -5400.0);
    }

    #[test]
    fn a_month_or_a_year_has_no_total_in_seconds() {
        assert_eq!(read("P1M").4, None);
        assert_eq!(read("P1Y").4, None);
        assert!(read("PT1M").4.is_some(), "a minute does");
    }

    #[test]
    fn what_is_no_iso_duration_is_refused_where_it_goes_wrong() {
        assert_eq!(refused("P"), (2, DurationProblem::Empty));
        assert_eq!(refused("PT"), (3, DurationProblem::Empty));
        assert_eq!(refused("P1DT"), (5, DurationProblem::Empty));
        assert_eq!(refused("PT30M1H"), (7, DurationProblem::OutOfOrder));
        assert_eq!(refused("P1D2D"), (5, DurationProblem::Repeated));
        assert_eq!(refused("P1.5DT2H"), (7, DurationProblem::FractionNotLast));
        assert_eq!(
            refused("P1H"),
            (3, DurationProblem::Unexpected),
            "an hour before the T"
        );
        assert_eq!(refused("PT1D"), (4, DurationProblem::Unexpected));
        assert_eq!(refused("P1DTT1H"), (5, DurationProblem::Repeated));
        assert_eq!(refused("PX"), (2, DurationProblem::Unexpected));
    }

    #[test]
    fn words_are_read_and_written_as_iso() {
        for (text, written) in [
            ("1h30", "PT1H30M"),
            ("90 min", "PT90M"),
            ("1d 2h", "P1DT2H"),
            ("1:30", "PT1H30M"),
            ("2 h 15", "PT2H15M"),
            ("3 jours", "P3D"),
            ("2 semaines 1 jour", "P2W1D"),
            ("1.5h", "PT1.5H"),
            ("2m30", "PT2M30S"),
            ("45 s", "PT45S"),
            ("1 heure 30 minutes", "PT1H30M"),
            ("- 2h", "-PT2H"),
        ] {
            let (_, _, iso, words, _) = read(text);
            assert_eq!(iso, written, "{text}");
            assert!(words, "{text}");
        }
    }

    #[test]
    fn words_that_are_no_duration_are_located() {
        assert_eq!(refused("1h 2h"), (4, DurationProblem::Repeated));
        assert_eq!(refused("12 parsecs"), (4, DurationProblem::Unexpected));
        assert_eq!(
            refused("30"),
            (3, DurationProblem::Unexpected),
            "a number alone"
        );
        assert_eq!(refused("1.5h 30"), (6, DurationProblem::FractionNotLast));
        assert_eq!(refused("abc"), (1, DurationProblem::Unexpected));
    }
}
