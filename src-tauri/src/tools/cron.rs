//! A cron expression taken apart field by field, for a sentence the front builds from translation
//! keys, and its next runs from `croner`. The fields are read here first so that a refusal names
//! its field and its token, which `croner` does not.

use std::str::FromStr;

use chrono::{DateTime, Datelike, Days, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use croner::parser::{CronParser, Seconds, Year};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::dates::{self, Parsed, Refusal};
use crate::count::saturating_u32;

pub const RUNS: usize = 10;

/// A window holding this many runs or fewer lists them all.
pub const LISTED: usize = 50;
/// Past it, the first and the last few, the total between them.
pub const EDGE: usize = 5;
/// The longest window counted: said, past it, rather than computed for ever.
pub const WINDOW_YEARS: u32 = 10;

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

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CronCheck {
    /// `before` and `after` are the nearest runs around a moment that is none: what a « no »
    /// leaves one asking.
    Checked {
        matches: bool,
        at: String,
        before: Option<Run>,
        after: Option<Run>,
    },
    Unreadable {
        at: u32,
    },
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WindowEnd {
    From,
    Until,
}

/// The runs between two moments: « will this run on Friday afternoon? ».
// One answer is built a request and sent: nothing stores these for a `Box` to make smaller.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CronWindow {
    /// ⚠️ `total` is counted, never the length of a list: a window of a year at every minute
    /// holds half a million runs, and ten of them cross the bridge. `last` is empty when
    /// `first` is all of them.
    Counted {
        from: Run,
        until: Run,
        total: u32,
        first: Vec<Run>,
        last: Vec<Run>,
    },
    Unreadable {
        end: WindowEnd,
        at: u32,
    },
    /// A date out of range, or an hour a change of clocks skipped.
    Skipped {
        end: WindowEnd,
    },
    /// The end is not after the start.
    Backwards,
    TooLong {
        years: u32,
    },
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CronRequest {
    pub expression: String,
    /// `None` is the machine's zone.
    pub zone: Option<String>,
    /// A date to test against the expression, read in `zone`; empty asks nothing.
    pub check: String,
    /// Where the runs are listed from, read in `zone`; empty is now.
    #[serde(default)]
    pub from: String,
    /// Where they stop; empty lists the next ten.
    #[serde(default)]
    pub until: String,
}

#[allow(clippy::large_enum_variant)]
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
        /// Asked for by an end, or by a start that does not read.
        window: Option<CronWindow>,
        /// Now, in `zone`: what « today » and « Friday afternoon » are counted from.
        now: Run,
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

/// A moment typed in a field, read in `zone`.
enum Moment {
    Unset,
    At(DateTime<Tz>),
    /// One-based, in characters of the text.
    Unreadable(u32),
    Skipped,
}

fn moment(text: &str, zone: Tz) -> Moment {
    if text.trim().is_empty() {
        return Moment::Unset;
    }
    match dates::parse(text, None) {
        Err(Refusal::Unreadable(at)) => Moment::Unreadable(saturating_u32(at) + 1),
        Err(Refusal::Skipped | Refusal::OutOfRange) => Moment::Skipped,
        Ok(Parsed::Absolute(read)) => Moment::At(read.at.with_timezone(&zone)),
        Ok(Parsed::Wall(naive, _)) => match zone.from_local_datetime(&naive) {
            LocalResult::Single(at) | LocalResult::Ambiguous(at, _) => Moment::At(at),
            LocalResult::None => Moment::Skipped,
        },
    }
}

fn check(cron: &croner::Cron, text: &str, zone: Tz) -> Option<CronCheck> {
    let at = match moment(text, zone) {
        Moment::Unset => return None,
        Moment::Unreadable(at) => return Some(CronCheck::Unreadable { at }),
        Moment::Skipped => return Some(CronCheck::Skipped),
        Moment::At(at) => at,
    };
    let matches = cron.is_time_matching(&at).unwrap_or(false);
    let nearest =
        |found: Result<DateTime<Tz>, _>| found.ok().filter(|_| !matches).map(|at| run(&at));
    Some(CronCheck::Checked {
        matches,
        at: at.format("%Y-%m-%d %H:%M:%S").to_string(),
        before: nearest(cron.find_previous_occurrence(&at, false)),
        after: nearest(cron.find_next_occurrence(&at, false)),
    })
}

/// How many values of `field` its pieces name. Only for the seconds, the minutes and the
/// hours, whose pieces are plain: values, ranges and steps.
fn slots(field: &FieldReading) -> u64 {
    let (low, high) = field.field.bounds();
    let mut named = 0u64;
    let mut mark = |from: u32, to: u32, step: u32| {
        for value in (from..=to.min(high)).step_by(usize::try_from(step.max(1)).unwrap_or(1)) {
            named |= 1 << value;
        }
    };
    for piece in &field.pieces {
        match *piece {
            Piece::Every => mark(low, high, 1),
            Piece::Value { value } => mark(value, value, 1),
            Piece::Range { from, to } => mark(from, to, 1),
            Piece::Step { step } => mark(low, high, step),
            Piece::SteppedRange { step, from, to } => mark(from, to, step),
            Piece::LastDayOfMonth
            | Piece::NearestWeekday { .. }
            | Piece::LastWeekday { .. }
            | Piece::Nth { .. } => {}
        }
    }
    u64::from(named.count_ones())
}

/// The runs of one whole day that matches: the same every such day, since the time fields
/// know nothing of the date.
fn runs_a_day(fields: &[FieldReading]) -> u64 {
    [CronField::Second, CronField::Minute, CronField::Hour]
        .into_iter()
        .map(|wanted| {
            fields
                .iter()
                .find(|field| field.field == wanted)
                // No seconds field: second 0 alone.
                .map_or(1, slots)
        })
        .product()
}

/// Midnight opening `date` in `zone`, when the clocks have one there.
fn midnight(zone: Tz, date: NaiveDate) -> Option<DateTime<Tz>> {
    zone.from_local_datetime(&date.and_time(NaiveTime::MIN))
        .single()
}

/// The runs in `[from, until)`, counted without being listed. ⚠️ By the day: a day that lies
/// whole in the window and that no change of clocks cuts holds `a_day` runs if it holds one,
/// so the search jumps from one matching day to the next. Only the two ends, and a day the
/// clocks change on, are walked run by run — where `croner` alone knows what a skipped or a
/// repeated hour does.
fn count(cron: &croner::Cron, a_day: u64, from: DateTime<Tz>, until: DateTime<Tz>) -> u64 {
    let zone = from.timezone();
    let mut total = 0u64;
    let mut cursor = from;
    let mut inclusive = true;
    while let Ok(next) = cron.find_next_occurrence(&cursor, inclusive) {
        if next >= until {
            break;
        }
        let day = next.date_naive();
        let whole = midnight(zone, day)
            .zip(
                day.checked_add_days(Days::new(1))
                    .and_then(|after| midnight(zone, after)),
            )
            .filter(|(opens, closes)| {
                *opens >= from
                    && *closes <= until
                    && chrono::Offset::fix(opens.offset()) == chrono::Offset::fix(closes.offset())
            });
        if let Some((_, closes)) = whole {
            total += a_day;
            cursor = closes;
            inclusive = true;
        } else {
            total += 1;
            cursor = next;
            inclusive = false;
        }
    }
    total
}

fn window(
    cron: &croner::Cron,
    fields: &[FieldReading],
    request: &CronRequest,
    zone: Tz,
    now: DateTime<Tz>,
) -> (DateTime<Tz>, Option<CronWindow>) {
    let from = match moment(&request.from, zone) {
        Moment::Unset => now,
        Moment::At(at) => at,
        Moment::Unreadable(at) => {
            return (
                now,
                Some(CronWindow::Unreadable {
                    end: WindowEnd::From,
                    at,
                }),
            );
        }
        Moment::Skipped => {
            return (
                now,
                Some(CronWindow::Skipped {
                    end: WindowEnd::From,
                }),
            );
        }
    };
    let until = match moment(&request.until, zone) {
        Moment::Unset => return (from, None),
        Moment::At(at) => at,
        Moment::Unreadable(at) => {
            return (
                from,
                Some(CronWindow::Unreadable {
                    end: WindowEnd::Until,
                    at,
                }),
            );
        }
        Moment::Skipped => {
            return (
                from,
                Some(CronWindow::Skipped {
                    end: WindowEnd::Until,
                }),
            );
        }
    };
    if until <= from {
        return (from, Some(CronWindow::Backwards));
    }
    if (until - from).num_days() > i64::from(WINDOW_YEARS) * 366 {
        return (
            from,
            Some(CronWindow::TooLong {
                years: WINDOW_YEARS,
            }),
        );
    }

    let total = count(cron, runs_a_day(fields), from, until);
    let within = usize::try_from(total).unwrap_or(usize::MAX);
    let listed = if within <= LISTED { LISTED } else { EDGE };
    let first: Vec<Run> = cron
        .iter_from(from, croner::Direction::Forward)
        .take_while(|at| *at < until)
        .take(listed)
        .map(|at| run(&at))
        .collect();
    let mut last: Vec<Run> = if within <= LISTED {
        Vec::new()
    } else {
        cron.iter_before(until)
            .take_while(|at| *at >= from)
            .take(EDGE)
            .map(|at| run(&at))
            .collect()
    };
    last.reverse();
    (
        from,
        Some(CronWindow::Counted {
            from: run(&from),
            until: run(&until),
            total: u32::try_from(total).unwrap_or(u32::MAX),
            first,
            last,
        }),
    )
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
    let now = now.with_timezone(&zone);
    let (from, window) = window(&cron, &fields, request, zone, now);
    CronAnswer::Read {
        expanded,
        both_days: restricted(CronField::DayOfMonth) && restricted(CronField::DayOfWeek),
        zone: zone_name,
        // From now, a run at this very second is not « next »; from a moment that was typed,
        // the run at that moment is the first of them.
        runs: if from == now {
            cron.iter_after(now)
        } else {
            cron.iter_from(from, croner::Direction::Forward)
        }
        .take(RUNS)
        .map(|at| run(&at))
        .collect(),
        check: check(&cron, &request.check, zone),
        window,
        now: run(&now),
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
                from: String::new(),
                until: String::new(),
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

        let verdict = |text: &str| match check(text) {
            Some(CronCheck::Checked {
                matches,
                at,
                before,
                after,
            }) => (
                matches,
                at,
                before.map(|run| format!("{} {}", run.date, run.time)),
                after.map(|run| format!("{} {}", run.date, run.time)),
            ),
            other => panic!("{text}: {other:?}"),
        };

        // A yes says nothing more.
        assert_eq!(
            verdict("2026-09-29 09:00"),
            (true, "2026-09-29 09:00:00".into(), None, None)
        );
        // A no says the nearest run on either side: Friday's and Monday's, around a Sunday.
        assert_eq!(
            verdict("2026-09-27 09:00"),
            (
                false,
                "2026-09-27 09:00:00".into(),
                Some("2026-09-25 09:00:00".into()),
                Some("2026-09-28 09:00:00".into())
            ),
            "a Sunday"
        );
        assert_eq!(
            verdict("2026-09-29T09:00:00+02:00"),
            (
                false,
                "2026-09-29 07:00:00".into(),
                Some("2026-09-28 09:00:00".into()),
                Some("2026-09-29 09:00:00".into())
            )
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

    fn between(expression: &str, zone: &str, from: &str, until: &str) -> CronAnswer {
        describe(
            &CronRequest {
                expression: expression.to_owned(),
                zone: Some(zone.to_owned()),
                check: String::new(),
                from: from.to_owned(),
                until: until.to_owned(),
            },
            None,
            now("2026-10-12T08:00:00Z"),
        )
    }

    fn counted(answer: CronAnswer) -> (u32, Vec<String>, Vec<String>) {
        let at = |runs: Vec<Run>| -> Vec<String> {
            runs.into_iter()
                .map(|run| format!("{} {}", run.date, run.time))
                .collect()
        };
        match answer {
            CronAnswer::Read {
                window:
                    Some(CronWindow::Counted {
                        total, first, last, ..
                    }),
                ..
            } => (total, at(first), at(last)),
            other => panic!("not counted: {other:?}"),
        }
    }

    /// « Will this run on Friday afternoon? » had no answer: the list was the next ten from
    /// now, and the check one exact minute.
    #[test]
    fn the_runs_between_two_moments_are_counted_and_listed() {
        // 2026-10-16 is a Friday, the 17th a Saturday.
        let friday = between(
            "*/5 * * * 1-5",
            "Europe/Paris",
            "2026-10-16 12:00",
            "2026-10-16 18:00",
        );
        let (total, first, last) = counted(friday);
        assert_eq!(total, 72);
        assert_eq!(first.len(), EDGE);
        assert_eq!(
            first[0], "2026-10-16 12:00:00",
            "the start is in the window"
        );
        assert_eq!(
            last.last().map(String::as_str),
            Some("2026-10-16 17:55:00"),
            "the end is not"
        );

        // A Saturday holds none: the answer one came for.
        assert_eq!(
            counted(between(
                "*/5 * * * 1-5",
                "Europe/Paris",
                "2026-10-17 00:00",
                "2026-10-18 00:00"
            )),
            (0, vec![], vec![])
        );

        // Few enough, they are all listed and nothing is cut.
        let (total, first, last) = counted(between(
            "0 9,14 * * *",
            "UTC",
            "2026-10-12 00:00",
            "2026-10-15 00:00",
        ));
        assert_eq!((total, first.len(), last.len()), (6, 6, 0));
        assert_eq!(first[5], "2026-10-14 14:00:00");
    }

    #[test]
    fn a_start_alone_lists_the_next_ten_from_there_the_run_at_that_moment_included() {
        let answer = between("0 9 * * *", "UTC", "2027-01-01 09:00", "");
        let CronAnswer::Read { runs, window, .. } = answer else {
            panic!("read");
        };

        assert_eq!(window, None);
        assert_eq!(runs.len(), RUNS);
        assert_eq!(
            (runs[0].date.as_str(), runs[0].time.as_str()),
            ("2027-01-01", "09:00:00")
        );
    }

    #[test]
    fn an_end_that_does_not_read_or_comes_first_is_said_rather_than_counted() {
        let window =
            |from: &str, until: &str| match between("0 9 * * *", "Europe/Paris", from, until) {
                CronAnswer::Read { window, .. } => window,
                other => panic!("not read: {other:?}"),
            };

        assert_eq!(
            window("2026-13-01", ""),
            Some(CronWindow::Unreadable {
                end: WindowEnd::From,
                at: 6
            })
        );
        assert_eq!(
            window("", "2026-13-01"),
            Some(CronWindow::Unreadable {
                end: WindowEnd::Until,
                at: 6
            })
        );
        // 02:30 on the night the clocks go forward does not exist in Paris.
        assert_eq!(
            window("2026-03-29 02:30", "2026-03-30 00:00"),
            Some(CronWindow::Skipped {
                end: WindowEnd::From
            })
        );
        assert_eq!(
            window("2026-10-16 18:00", "2026-10-16 12:00"),
            Some(CronWindow::Backwards)
        );
        assert_eq!(
            window("2026-10-16 12:00", "2026-10-16 12:00"),
            Some(CronWindow::Backwards)
        );
        assert_eq!(
            window("2026-01-01", "2040-01-01"),
            Some(CronWindow::TooLong { years: 10 })
        );
        // An end alone counts from now.
        assert!(matches!(
            window("", "2026-10-13 10:00"),
            Some(CronWindow::Counted { total: 1, .. })
        ));
    }

    /// The count jumps from one matching day to the next: it has to be what walking every run
    /// gives — over a change of clocks, a month's end, both day fields, a seconds field.
    #[test]
    fn the_count_is_exactly_what_walking_every_run_gives() {
        let zone = Tz::from_str("Europe/Paris").unwrap();
        let at = |text: &str| match moment(text, zone) {
            Moment::At(at) => at,
            _ => panic!("{text}"),
        };
        let windows = [
            // Across the night the clocks go forward, and the one they go back.
            ("2026-03-27 10:30", "2026-03-31 17:45"),
            ("2026-10-23 22:10", "2026-10-27 03:00"),
            // Whole days, a partial day at each end, a month's end and a year's.
            ("2026-10-12 00:00", "2026-10-19 00:00"),
            ("2026-12-28 13:37", "2027-01-04 06:01"),
            ("2026-02-27 23:59", "2026-03-02 00:01"),
            // Less than a day.
            ("2026-10-16 12:00", "2026-10-16 18:00"),
        ];
        for expression in [
            "*/5 * * * 1-5",
            "0 9 * * *",
            "30 2 * * *",
            "*/15 9-18/3 * * MON-FRI",
            "0,20,40 */6 1,15 * 5",
            "0 0 L * *",
            "15 10 * * 1#2",
            "*/20 * 8 * * *",
            "59 23 31 12 *",
        ] {
            let parser = CronParser::builder()
                .seconds(Seconds::Optional)
                .year(Year::Disallowed)
                .build();
            let cron = parser.parse(expression).unwrap();
            let Ok(fields) = read(expression) else {
                panic!("{expression} does not read");
            };
            for (from, until) in windows {
                let (from, until) = (at(from), at(until));
                let walked = cron
                    .iter_from(from, croner::Direction::Forward)
                    .take_while(|run| *run < until)
                    .count();

                assert_eq!(
                    count(&cron, runs_a_day(&fields), from, until),
                    u64::try_from(walked).unwrap(),
                    "{expression} from {from} until {until}"
                );
            }
        }
    }

    /// A year at every minute is half a million runs: counted in a few hundred searches, and
    /// five of each end handed over.
    #[test]
    fn a_long_window_is_counted_without_being_walked() {
        let (total, first, last) = counted(between(
            "* * * * *",
            "UTC",
            "2026-01-01 00:00",
            "2027-01-01 00:00",
        ));

        assert_eq!(total, 365 * 24 * 60);
        assert_eq!(first[0], "2026-01-01 00:00:00");
        assert_eq!(
            last,
            [
                "2026-12-31 23:55:00",
                "2026-12-31 23:56:00",
                "2026-12-31 23:57:00",
                "2026-12-31 23:58:00",
                "2026-12-31 23:59:00"
            ]
        );
    }

    #[test]
    fn the_answer_says_what_now_is_in_the_zone() {
        let CronAnswer::Read { now, .. } = between("0 9 * * *", "Asia/Tokyo", "", "") else {
            panic!("read");
        };

        // 08:00 UTC on Monday the 12th is 17:00 in Tokyo.
        assert_eq!(
            (now.date.as_str(), now.time.as_str(), now.weekday),
            ("2026-10-12", "17:00:00", 1)
        );
    }

    #[test]
    fn the_machine_s_zone_is_the_default_and_an_unknown_one_is_said() {
        let answer = describe(
            &CronRequest {
                expression: "0 20 * * *".into(),
                zone: None,
                check: String::new(),
                from: String::new(),
                until: String::new(),
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
