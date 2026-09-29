//! A cron expression taken apart field by field, for a sentence the front builds from translation
//! keys, and its next runs from `croner`. The fields are read here first so that a refusal names
//! its field and its token, which `croner` does not.

use std::str::FromStr;

use chrono::{DateTime, Datelike, LocalResult, TimeZone, Utc};
use chrono_tz::Tz;
use croner::parser::{CronParser, Seconds, Year};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::dates::{self, Parsed, Refusal};
use crate::count::saturating_u32;

pub const RUNS: usize = 10;

const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];
const DAYS: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CronField {
    Second,
    Minute,
    Hour,
    DayOfMonth,
    Month,
    DayOfWeek,
}

impl CronField {
    fn bounds(self) -> (u32, u32) {
        match self {
            Self::Second | Self::Minute => (0, 59),
            Self::Hour => (0, 23),
            Self::DayOfMonth => (1, 31),
            Self::Month => (1, 12),
            Self::DayOfWeek => (0, 7),
        }
    }

    fn names(self) -> &'static [&'static str] {
        match self {
            Self::Month => &MONTHS,
            Self::DayOfWeek => &DAYS,
            _ => &[],
        }
    }

    fn is_day(self) -> bool {
        matches!(self, Self::DayOfMonth | Self::DayOfWeek)
    }
}

/// One item of a field's list. A day of the week is 0 (Sunday) to 7 (Sunday again).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Piece {
    Every,
    Value {
        value: u32,
    },
    Range {
        from: u32,
        to: u32,
    },
    /// `*/15`.
    Step {
        step: u32,
    },
    /// `9-18/2`, `5-59/10`.
    SteppedRange {
        step: u32,
        from: u32,
        to: u32,
    },
    LastDayOfMonth,
    /// `15W`: the weekday nearest the 15th.
    NearestWeekday {
        day: u32,
    },
    /// `5L`: the last Friday of the month.
    LastWeekday {
        weekday: u32,
    },
    /// `1#2`: the second Monday of the month.
    Nth {
        weekday: u32,
        nth: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FieldReading {
    pub field: CronField,
    pub text: String,
    /// Zero-based character offsets in the expression read (the expansion of a macro).
    pub start: u32,
    pub end: u32,
    pub pieces: Vec<Piece>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CronProblem {
    Empty,
    FieldCount,
    UnknownMacro,
    UnknownName,
    NotANumber,
    OutOfRange,
    ReversedRange,
    ZeroStep,
    /// A special character this field does not take: `L` in the minutes.
    NotHere,
    /// Read here, refused by the scheduler: nothing more precise to say.
    Refused,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub date: String,
    pub time: String,
    /// 1 is Monday.
    pub weekday: u32,
    pub offset: String,
    pub epoch_milliseconds: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CronCheck {
    Checked { matches: bool, at: String },
    Unreadable { at: u32 },
    Skipped,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CronRequest {
    pub expression: String,
    /// `None` is the machine's zone.
    pub zone: Option<String>,
    /// A date to test against the expression, read in `zone`; empty asks nothing.
    pub check: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CronAnswer {
    Read {
        /// What a macro stands for: `@daily` is `0 0 * * *`.
        expanded: Option<String>,
        fields: Vec<FieldReading>,
        /// Both day fields restricted: either one matching is enough, as in cron.
        both_days: bool,
        zone: String,
        runs: Vec<Run>,
        check: Option<CronCheck>,
    },
    /// `@reboot` runs when the machine starts: no time to give.
    Reboot,
    Refused {
        field: Option<CronField>,
        token: String,
        /// One-based, in characters of the expression as typed.
        at: u32,
        problem: CronProblem,
    },
    UnknownZone {
        zone: String,
    },
}

// The macros `croner` knows, and `@midnight`, which it does not.
fn expand(macro_name: &str) -> Option<&'static str> {
    match macro_name.to_ascii_lowercase().as_str() {
        "@yearly" | "@annually" => Some("0 0 1 1 *"),
        "@monthly" => Some("0 0 1 * *"),
        "@weekly" => Some("0 0 * * 0"),
        "@daily" | "@midnight" => Some("0 0 * * *"),
        "@hourly" => Some("0 * * * *"),
        _ => None,
    }
}

struct Fault {
    problem: CronProblem,
    /// Zero-based, in characters of the field.
    at: usize,
    token: String,
}

fn fault(problem: CronProblem, at: usize, token: &str) -> Fault {
    Fault {
        problem,
        at,
        token: token.to_owned(),
    }
}

fn value(field: CronField, text: &str, at: usize) -> Result<u32, Fault> {
    let upper = text.to_ascii_uppercase();
    let number = if let Some(index) = field.names().iter().position(|name| *name == upper) {
        u32::try_from(index).unwrap_or(0) + u32::from(field == CronField::Month)
    } else if text.bytes().all(|byte| byte.is_ascii_digit()) && !text.is_empty() {
        text.parse()
            .map_err(|_| fault(CronProblem::OutOfRange, at, text))?
    } else if text.bytes().all(|byte| byte.is_ascii_alphabetic()) && !text.is_empty() {
        return Err(fault(CronProblem::UnknownName, at, text));
    } else {
        return Err(fault(CronProblem::NotANumber, at, text));
    };
    let (low, high) = field.bounds();
    if (low..=high).contains(&number) {
        Ok(number)
    } else {
        Err(fault(CronProblem::OutOfRange, at, text))
    }
}

fn item(field: CronField, text: &str, at: usize) -> Result<Piece, Fault> {
    if text.is_empty() {
        return Err(fault(CronProblem::Empty, at, text));
    }
    let upper = text.to_ascii_uppercase();
    if text == "*" || (text == "?" && field.is_day()) {
        return Ok(Piece::Every);
    }
    if field == CronField::DayOfMonth {
        if upper == "L" {
            return Ok(Piece::LastDayOfMonth);
        }
        if let Some(day) = upper.strip_suffix('W') {
            return Ok(Piece::NearestWeekday {
                day: value(field, day, at)?,
            });
        }
    }
    if field == CronField::DayOfWeek {
        if let Some((weekday, nth)) = text.split_once('#') {
            let nth_at = at + weekday.chars().count() + 1;
            let nth = match nth.parse::<u32>() {
                Ok(nth) if (1..=5).contains(&nth) => nth,
                _ => return Err(fault(CronProblem::OutOfRange, nth_at, nth)),
            };
            return Ok(Piece::Nth {
                weekday: value(field, weekday, at)? % 7,
                nth,
            });
        }
        if let Some(weekday) = upper
            .strip_suffix('L')
            .filter(|weekday| !weekday.is_empty())
        {
            return Ok(Piece::LastWeekday {
                weekday: value(field, weekday, at)? % 7,
            });
        }
    }
    if let Some(stop) = text
        .find(['L', 'W', '#', '?', 'l', 'w'])
        .filter(|_| !field.names().iter().any(|name| upper.contains(name)))
    {
        return Err(fault(
            CronProblem::NotHere,
            at + text[..stop].chars().count(),
            &text[stop..=stop],
        ));
    }

    let (base, step) = match text.split_once('/') {
        Some((base, step)) => {
            let step_at = at + base.chars().count() + 1;
            match step.parse::<u32>() {
                Ok(0) => return Err(fault(CronProblem::ZeroStep, step_at, step)),
                Ok(step) => (base, Some(step)),
                Err(_) => return Err(fault(CronProblem::NotANumber, step_at, step)),
            }
        }
        None => (text, None),
    };
    let bounds = if base == "*" {
        None
    } else if let Some((from, to)) = base.split_once('-') {
        let from_value = value(field, from, at)?;
        let to_value = value(field, to, at + from.chars().count() + 1)?;
        if from_value > to_value {
            return Err(fault(CronProblem::ReversedRange, at, base));
        }
        Some((from_value, Some(to_value)))
    } else if step.is_some() {
        // `5/15` is Quartz's, and cron's own parsers refuse it.
        return Err(fault(CronProblem::NotHere, at + base.chars().count(), "/"));
    } else {
        Some((value(field, base, at)?, None))
    };

    Ok(match (bounds, step) {
        (None, Some(step)) => Piece::Step { step },
        (Some((from, Some(to))), Some(step)) => Piece::SteppedRange { step, from, to },
        (Some((from, Some(to))), None) => Piece::Range { from, to },
        (Some((value, _)), _) => Piece::Value { value },
        (None, None) => Piece::Every,
    })
}

fn field(field: CronField, text: &str) -> Result<Vec<Piece>, Fault> {
    let mut at = 0;
    text.split(',')
        .map(|part| {
            let piece = item(field, part, at);
            at += part.chars().count() + 1;
            piece
        })
        .collect()
}

/// Each field with its offsets, whitespace between them.
fn split(expression: &str) -> Vec<(usize, &str)> {
    let mut fields = Vec::new();
    let mut start = None;
    for (index, (byte, character)) in expression.char_indices().enumerate() {
        match (character.is_whitespace(), start) {
            (false, None) => start = Some((index, byte)),
            (true, Some((first, from))) => {
                fields.push((first, &expression[from..byte]));
                start = None;
            }
            _ => {}
        }
    }
    if let Some((first, from)) = start {
        fields.push((first, &expression[from..]));
    }
    fields
}

/// A refusal, zero-based in the expression.
struct Located {
    field: Option<CronField>,
    token: String,
    at: usize,
    problem: CronProblem,
}

fn located(field: Option<CronField>, token: &str, at: usize, problem: CronProblem) -> Located {
    Located {
        field,
        token: token.to_owned(),
        at,
        problem,
    }
}

impl From<Located> for CronAnswer {
    fn from(refusal: Located) -> Self {
        Self::Refused {
            field: refusal.field,
            token: refusal.token,
            at: saturating_u32(refusal.at) + 1,
            problem: refusal.problem,
        }
    }
}

fn read(expression: &str) -> Result<Vec<FieldReading>, Located> {
    let parts = split(expression);
    let names: &[CronField] = match parts.len() {
        5 => &[
            CronField::Minute,
            CronField::Hour,
            CronField::DayOfMonth,
            CronField::Month,
            CronField::DayOfWeek,
        ],
        6 => &[
            CronField::Second,
            CronField::Minute,
            CronField::Hour,
            CronField::DayOfMonth,
            CronField::Month,
            CronField::DayOfWeek,
        ],
        _ => {
            let (at, token) = parts.get(6).or(parts.last()).copied().unwrap_or((0, ""));
            return Err(located(None, token, at, CronProblem::FieldCount));
        }
    };
    names
        .iter()
        .zip(parts)
        .map(|(&name, (start, text))| {
            field(name, text)
                .map(|pieces| FieldReading {
                    field: name,
                    text: text.to_owned(),
                    start: saturating_u32(start),
                    end: saturating_u32(start + text.chars().count()),
                    pieces,
                })
                .map_err(|fault| located(Some(name), &fault.token, start + fault.at, fault.problem))
        })
        .collect()
}

fn offset_text(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let seconds = seconds.unsigned_abs();
    format!("{sign}{:02}:{:02}", seconds / 3600, seconds % 3600 / 60)
}

fn run(at: &DateTime<Tz>) -> Run {
    #[allow(clippy::cast_precision_loss)]
    let epoch_milliseconds = at.timestamp_millis() as f64;
    Run {
        date: at.format("%Y-%m-%d").to_string(),
        time: at.format("%H:%M:%S").to_string(),
        weekday: at.weekday().number_from_monday(),
        offset: offset_text(chrono::Offset::fix(at.offset()).local_minus_utc()),
        epoch_milliseconds,
    }
}

fn check(cron: &croner::Cron, text: &str, zone: Tz) -> Option<CronCheck> {
    if text.trim().is_empty() {
        return None;
    }
    let at = match dates::parse(text, None) {
        Err(Refusal::Unreadable(at)) => {
            return Some(CronCheck::Unreadable {
                at: saturating_u32(at) + 1,
            });
        }
        Err(Refusal::Skipped | Refusal::OutOfRange) => return Some(CronCheck::Skipped),
        Ok(Parsed::Absolute(read)) => read.at.with_timezone(&zone),
        Ok(Parsed::Wall(naive, _)) => match zone.from_local_datetime(&naive) {
            LocalResult::Single(at) | LocalResult::Ambiguous(at, _) => at,
            LocalResult::None => return Some(CronCheck::Skipped),
        },
    };
    Some(CronCheck::Checked {
        matches: cron.is_time_matching(&at).unwrap_or(false),
        at: at.format("%Y-%m-%d %H:%M:%S").to_string(),
    })
}

/// `now` and the machine's `local` zone are handed in, so the tests fix both.
pub fn describe(request: &CronRequest, local: Option<&str>, now: DateTime<Utc>) -> CronAnswer {
    let zone_name = request
        .zone
        .clone()
        .or_else(|| local.map(str::to_owned))
        .unwrap_or_else(|| "UTC".to_owned());
    let Ok(zone) = Tz::from_str(&zone_name) else {
        return CronAnswer::UnknownZone { zone: zone_name };
    };

    let typed = request.expression.trim();
    let lead = request.expression.chars().count() - request.expression.trim_start().chars().count();
    if typed.eq_ignore_ascii_case("@reboot") {
        return CronAnswer::Reboot;
    }
    let (expression, expanded) = if typed.starts_with('@') {
        match expand(typed) {
            Some(expansion) => (expansion, Some(expansion.to_owned())),
            None => return located(None, typed, lead, CronProblem::UnknownMacro).into(),
        }
    } else {
        (request.expression.as_str(), None)
    };
    if expression.trim().is_empty() {
        return located(None, "", 0, CronProblem::Empty).into();
    }

    let fields = match read(expression) {
        Ok(fields) => fields,
        Err(refusal) => return refusal.into(),
    };
    let parser = CronParser::builder()
        .seconds(Seconds::Optional)
        .year(Year::Disallowed)
        .build();
    let Ok(cron) = parser.parse(expression) else {
        return located(None, typed, lead, CronProblem::Refused).into();
    };

    let restricted = |wanted: CronField| {
        fields
            .iter()
            .any(|field| field.field == wanted && field.pieces != [Piece::Every])
    };
    CronAnswer::Read {
        expanded,
        both_days: restricted(CronField::DayOfMonth) && restricted(CronField::DayOfWeek),
        zone: zone_name,
        runs: cron
            .iter_after(now.with_timezone(&zone))
            .take(RUNS)
            .map(|at| run(&at))
            .collect(),
        check: check(&cron, &request.check, zone),
        fields,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now(iso: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(iso)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn ask_at(expression: &str, zone: &str, check: &str, at: &str) -> CronAnswer {
        describe(
            &CronRequest {
                expression: expression.to_owned(),
                zone: Some(zone.to_owned()),
                check: check.to_owned(),
            },
            Some("Europe/Paris"),
            now(at),
        )
    }

    fn ask(expression: &str) -> CronAnswer {
        ask_at(expression, "UTC", "", "2026-09-29T10:00:00Z")
    }

    fn fields(expression: &str) -> Vec<(CronField, Vec<Piece>)> {
        match ask(expression) {
            CronAnswer::Read { fields, .. } => fields
                .into_iter()
                .map(|field| (field.field, field.pieces))
                .collect(),
            other => panic!("{expression} was not read: {other:?}"),
        }
    }

    fn pieces(expression: &str, wanted: CronField) -> Vec<Piece> {
        fields(expression)
            .into_iter()
            .find(|(field, _)| *field == wanted)
            .map(|(_, pieces)| pieces)
            .unwrap_or_default()
    }

    fn runs(answer: CronAnswer) -> Vec<String> {
        match answer {
            CronAnswer::Read { runs, .. } => runs
                .into_iter()
                .map(|run| format!("{} {} {}", run.date, run.time, run.offset))
                .collect(),
            other => panic!("not read: {other:?}"),
        }
    }

    fn refusal(expression: &str) -> (Option<CronField>, String, u32, CronProblem) {
        match ask(expression) {
            CronAnswer::Refused {
                field,
                token,
                at,
                problem,
            } => (field, token, at, problem),
            other => panic!("{expression} was not refused: {other:?}"),
        }
    }

    #[test]
    fn every_quarter_hour_of_the_working_day() {
        assert_eq!(
            fields("*/15 9-18 * * MON-FRI"),
            [
                (CronField::Minute, vec![Piece::Step { step: 15 }]),
                (CronField::Hour, vec![Piece::Range { from: 9, to: 18 }]),
                (CronField::DayOfMonth, vec![Piece::Every]),
                (CronField::Month, vec![Piece::Every]),
                (CronField::DayOfWeek, vec![Piece::Range { from: 1, to: 5 }]),
            ]
        );
    }

    #[test]
    fn every_minute_lists_names_and_steps_from_a_start() {
        assert!(
            fields("* * * * *")
                .iter()
                .all(|(_, pieces)| pieces == &[Piece::Every])
        );
        assert_eq!(
            pieces("0 9,12,18 * * *", CronField::Hour),
            [
                Piece::Value { value: 9 },
                Piece::Value { value: 12 },
                Piece::Value { value: 18 }
            ]
        );
        assert_eq!(
            pieces("0 0 1 JAN,jul *", CronField::Month),
            [Piece::Value { value: 1 }, Piece::Value { value: 7 }]
        );
        assert_eq!(
            pieces("5-59/10 * * * *", CronField::Minute),
            [Piece::SteppedRange {
                step: 10,
                from: 5,
                to: 59
            }]
        );
        assert_eq!(
            pieces("0 0 * * 7", CronField::DayOfWeek),
            [Piece::Value { value: 7 }]
        );
    }

    #[test]
    fn the_special_days_are_read() {
        assert_eq!(
            pieces("0 0 L * *", CronField::DayOfMonth),
            [Piece::LastDayOfMonth]
        );
        assert_eq!(
            pieces("0 0 15W * *", CronField::DayOfMonth),
            [Piece::NearestWeekday { day: 15 }]
        );
        assert_eq!(
            pieces("0 0 * * 5L", CronField::DayOfWeek),
            [Piece::LastWeekday { weekday: 5 }]
        );
        assert_eq!(
            pieces("0 0 * * MON#2", CronField::DayOfWeek),
            [Piece::Nth { weekday: 1, nth: 2 }]
        );
        assert_eq!(pieces("0 0 ? * MON", CronField::DayOfMonth), [Piece::Every]);
    }

    #[test]
    fn a_sixth_field_is_the_seconds() {
        let read = fields("*/30 * * * * *");

        assert_eq!(read[0].0, CronField::Second);
        assert_eq!(read.len(), 6);
    }

    #[test]
    fn both_day_fields_set_are_either_one() {
        let CronAnswer::Read { both_days, .. } = ask("30 4 1,15 * 5") else {
            panic!("read");
        };
        assert!(both_days);
        let CronAnswer::Read { both_days, .. } = ask("30 4 1,15 * *") else {
            panic!("read");
        };
        assert!(!both_days);
        assert_eq!(
            runs(ask("30 4 1,15 * 5"))[..3],
            [
                "2026-10-01 04:30:00 +00:00",
                "2026-10-02 04:30:00 +00:00",
                "2026-10-09 04:30:00 +00:00"
            ],
            "the 1st, then a Friday, then a Friday"
        );
    }

    #[test]
    fn a_macro_is_read_as_what_it_stands_for() {
        let CronAnswer::Read {
            expanded, fields, ..
        } = ask("@daily")
        else {
            panic!("read");
        };
        assert_eq!(expanded.as_deref(), Some("0 0 * * *"));
        assert_eq!(fields.len(), 5);
        assert_eq!(runs(ask("@MIDNIGHT"))[0], "2026-09-30 00:00:00 +00:00");
        assert_eq!(runs(ask("@yearly"))[0], "2027-01-01 00:00:00 +00:00");
        assert_eq!(ask(" @reboot "), CronAnswer::Reboot);
        assert_eq!(
            refusal("@often"),
            (None, "@often".into(), 1, CronProblem::UnknownMacro)
        );
    }

    #[test]
    fn the_next_runs_cross_a_month_end() {
        assert_eq!(
            runs(ask("0 12 * * *"))[..3],
            [
                "2026-09-29 12:00:00 +00:00",
                "2026-09-30 12:00:00 +00:00",
                "2026-10-01 12:00:00 +00:00"
            ]
        );
        assert_eq!(runs(ask("0 12 * * *")).len(), RUNS);
    }

    #[test]
    fn a_leap_day_comes_every_four_years() {
        assert_eq!(
            runs(ask("0 0 29 2 *"))[..2],
            ["2028-02-29 00:00:00 +00:00", "2032-02-29 00:00:00 +00:00"]
        );
        assert!(
            runs(ask("0 0 30 2 *")).is_empty(),
            "the 30th of February never comes"
        );
    }

    #[test]
    fn the_runs_are_read_in_the_zone_through_a_change_of_offset() {
        let spring = runs(ask_at(
            "30 2 * * *",
            "Europe/Paris",
            "",
            "2026-03-27T12:00:00Z",
        ));
        assert_eq!(spring[0], "2026-03-28 02:30:00 +01:00");
        assert!(
            !spring.iter().any(|run| run.starts_with("2026-03-29 02:30")),
            "02:30 does not exist that night: {spring:?}"
        );
        assert!(spring.iter().any(|run| run.ends_with("+02:00")));

        let evening = runs(ask_at(
            "0 20 * * *",
            "Asia/Tokyo",
            "",
            "2026-09-29T10:00:00Z",
        ));
        assert_eq!(evening[0], "2026-09-29 20:00:00 +09:00");
    }

    #[test]
    fn a_date_is_checked_in_the_zone() {
        let check =
            |text: &str| match ask_at("0 9 * * MON-FRI", "UTC", text, "2026-09-29T10:00:00Z") {
                CronAnswer::Read { check, .. } => check,
                other => panic!("not read: {other:?}"),
            };

        assert_eq!(
            check("2026-09-29 09:00"),
            Some(CronCheck::Checked {
                matches: true,
                at: "2026-09-29 09:00:00".into()
            })
        );
        assert_eq!(
            check("2026-09-27 09:00"),
            Some(CronCheck::Checked {
                matches: false,
                at: "2026-09-27 09:00:00".into()
            }),
            "a Sunday"
        );
        assert_eq!(
            check("2026-09-29T09:00:00+02:00"),
            Some(CronCheck::Checked {
                matches: false,
                at: "2026-09-29 07:00:00".into()
            })
        );
        assert_eq!(check("2026-13-01"), Some(CronCheck::Unreadable { at: 6 }));
        assert_eq!(check("  "), None);
        let skipped = match ask_at(
            "* * * * *",
            "Europe/Paris",
            "2026-03-29 02:30",
            "2026-03-27T12:00:00Z",
        ) {
            CronAnswer::Read { check, .. } => check,
            other => panic!("not read: {other:?}"),
        };
        assert_eq!(skipped, Some(CronCheck::Skipped));
    }

    #[test]
    fn the_machine_s_zone_is_the_default_and_an_unknown_one_is_said() {
        let answer = describe(
            &CronRequest {
                expression: "0 20 * * *".into(),
                zone: None,
                check: String::new(),
            },
            Some("Europe/Paris"),
            now("2026-09-29T10:00:00Z"),
        );
        let CronAnswer::Read { zone, runs, .. } = answer else {
            panic!("read");
        };
        assert_eq!(zone, "Europe/Paris");
        assert_eq!(runs[0].offset, "+02:00");
        assert_eq!(runs[0].weekday, 2);
        assert_eq!(
            ask_at("* * * * *", "Nowhere/City", "", "2026-09-29T10:00:00Z"),
            CronAnswer::UnknownZone {
                zone: "Nowhere/City".into()
            }
        );
    }

    #[test]
    fn every_refusal_names_its_field_and_where() {
        use CronField::{DayOfWeek, Hour, Minute, Month};
        use CronProblem::{
            Empty, FieldCount, NotANumber, NotHere, OutOfRange, ReversedRange, UnknownName,
            ZeroStep,
        };

        assert_eq!(refusal("*/15 9-18 * *"), (None, "*".into(), 13, FieldCount));
        assert_eq!(refusal("* * * * * * *"), (None, "*".into(), 13, FieldCount));
        assert_eq!(
            refusal("61 * * * *"),
            (Some(Minute), "61".into(), 1, OutOfRange)
        );
        assert_eq!(
            refusal("* 9-25 * * *"),
            (Some(Hour), "25".into(), 5, OutOfRange)
        );
        assert_eq!(
            refusal("* * * FOO *"),
            (Some(Month), "FOO".into(), 7, UnknownName)
        );
        assert_eq!(
            refusal("*/0 * * * *"),
            (Some(Minute), "0".into(), 3, ZeroStep)
        );
        assert_eq!(
            refusal("5L * * * *"),
            (Some(Minute), "L".into(), 2, NotHere)
        );
        assert_eq!(
            refusal("* 18-9 * * *"),
            (Some(Hour), "18-9".into(), 3, ReversedRange)
        );
        assert_eq!(
            refusal("* * * * MON#6"),
            (Some(DayOfWeek), "6".into(), 13, OutOfRange)
        );
        assert_eq!(
            refusal("1,,2 * * * *"),
            (Some(Minute), String::new(), 3, Empty)
        );
        assert_eq!(
            refusal("5/15 * * * *"),
            (Some(Minute), "/".into(), 2, NotHere)
        );
        assert_eq!(
            refusal("1.5 * * * *"),
            (Some(Minute), "1.5".into(), 1, NotANumber)
        );
        assert_eq!(refusal("   "), (None, String::new(), 1, Empty));
    }
}
