//! An instant read from whatever was typed — a Unix timestamp of any magnitude, an ISO 8601
//! date, an RFC 2822 one — and written back in every form.

use chrono::{
    DateTime, Datelike, FixedOffset, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, Offset,
    SecondsFormat, TimeZone, Utc,
};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::count::saturating_u32;

const NANOS_PER_SECOND: i128 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Magnitude {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

impl Magnitude {
    fn nanos(self) -> i128 {
        match self {
            Self::Seconds => NANOS_PER_SECOND,
            Self::Milliseconds => 1_000_000,
            Self::Microseconds => 1_000,
            Self::Nanoseconds => 1,
        }
    }

    /// Each unit read from 1973 on: a timestamp in seconds reaches `1e11` only in the year 5138.
    fn guessed(whole: i128) -> Self {
        match whole.unsigned_abs() {
            magnitude if magnitude < 100_000_000_000 => Self::Seconds,
            magnitude if magnitude < 100_000_000_000_000 => Self::Milliseconds,
            magnitude if magnitude < 100_000_000_000_000_000 => Self::Microseconds,
            _ => Self::Nanoseconds,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InstantRequest {
    pub text: String,
    /// A number read in this unit rather than by its magnitude.
    pub magnitude: Option<Magnitude>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ReadAs {
    Unix,
    Iso8601,
    WeekDate,
    OrdinalDate,
    Rfc2822,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InstantForms {
    pub unix_seconds: String,
    pub unix_milliseconds: String,
    pub unix_microseconds: String,
    /// An `i64` of nanoseconds reaches from 1677 to 2262 only.
    pub unix_nanoseconds: Option<String>,
    pub iso_utc: String,
    pub iso_local: String,
    pub local_offset: String,
    /// RFC 2822 writes a year of four digits, and none before year 0.
    pub rfc2822: Option<String>,
    /// The calendar facts are the local date's. 1 is Monday.
    pub weekday: u32,
    pub week_date: String,
    pub week: u32,
    pub ordinal_date: String,
    pub day_of_year: u32,
    /// For the relative time the front formats: exact, since chrono's range stays under 2⁵³ ms.
    pub epoch_milliseconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum InstantAnswer {
    Read {
        read_as: ReadAs,
        /// The unit a number was read in, guessed from its magnitude or forced.
        magnitude: Option<Magnitude>,
        guessed: bool,
        /// Written without an offset, it was taken in the local zone.
        local_assumed: bool,
        /// A local time the clocks went through twice: the earlier reading is kept.
        ambiguous: bool,
        forms: Box<InstantForms>,
    },
    /// One-based, in characters: where the text stops making sense.
    Unreadable { at: u32 },
    /// A local time the clocks skipped when they went forward.
    Skipped,
    /// Beyond chrono's range, some 262 000 years either side of year 0.
    OutOfRange,
}

pub fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub(crate) enum Parsed {
    /// A count from the epoch, or a date that carries its offset.
    Absolute(Reading),
    /// A wall-clock time, for a zone to place.
    Wall(NaiveDateTime, ReadAs),
}

/// What a text says, before any zone: an `Unreadable` is located in the text as given.
pub(crate) fn parse(text: &str, magnitude: Option<Magnitude>) -> Result<Parsed, Refusal> {
    let trimmed = text.trim_start();
    let skipped = text.chars().count() - trimmed.chars().count();
    let text = trimmed.trim_end();

    let parsed = if looks_numeric(text) {
        unix(text, magnitude).map(Parsed::Absolute)
    } else if text.len() > 4 && text.as_bytes()[..4].iter().all(u8::is_ascii_digit) {
        iso(text)
    } else {
        DateTime::parse_from_rfc2822(text)
            .map(|at| Parsed::Absolute(Reading::exact(ReadAs::Rfc2822, at.with_timezone(&Utc))))
            .map_err(|_| Refusal::Unreadable(0))
    };
    parsed.map_err(|refusal| match refusal {
        Refusal::Unreadable(at) => Refusal::Unreadable(skipped + at),
        other => other,
    })
}

/// `zone` is the machine's in the command, a fixed one in the tests.
pub fn describe<Z: TimeZone>(request: &InstantRequest, zone: &Z) -> InstantAnswer {
    let read = parse(&request.text, request.magnitude).and_then(|parsed| match parsed {
        Parsed::Absolute(read) => Ok(read),
        Parsed::Wall(naive, read_as) => local(naive, read_as, zone),
    });

    match read.and_then(|read| within_range(read, zone)) {
        Ok(read) => InstantAnswer::Read {
            read_as: read.read_as,
            magnitude: read.magnitude,
            guessed: read.guessed,
            local_assumed: read.local_assumed,
            ambiguous: read.ambiguous,
            forms: Box::new(forms(read.at, zone)),
        },
        Err(Refusal::Unreadable(at)) => InstantAnswer::Unreadable {
            at: saturating_u32(at) + 1,
        },
        Err(Refusal::Skipped) => InstantAnswer::Skipped,
        Err(Refusal::OutOfRange) => InstantAnswer::OutOfRange,
    }
}

/// ⚠️ chrono panics on a local time past its range, which an offset can push the edges into.
fn within_range<Z: TimeZone>(read: Reading, zone: &Z) -> Result<Reading, Refusal> {
    let offset = read.at.with_timezone(zone).offset().fix();
    match read.at.naive_utc().checked_add_offset(offset) {
        Some(_) => Ok(read),
        None => Err(Refusal::OutOfRange),
    }
}

pub(crate) struct Reading {
    read_as: ReadAs,
    pub(crate) at: DateTime<Utc>,
    magnitude: Option<Magnitude>,
    guessed: bool,
    local_assumed: bool,
    ambiguous: bool,
}

impl Reading {
    fn exact(read_as: ReadAs, at: DateTime<Utc>) -> Self {
        Self {
            read_as,
            at,
            magnitude: None,
            guessed: false,
            local_assumed: false,
            ambiguous: false,
        }
    }
}

pub(crate) enum Refusal {
    /// Zero-based, in characters.
    Unreadable(usize),
    Skipped,
    OutOfRange,
}

fn looks_numeric(text: &str) -> bool {
    let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    !whole.is_empty()
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
}

fn unix(text: &str, forced: Option<Magnitude>) -> Result<Reading, Refusal> {
    let negative = text.starts_with('-');
    let digits = text.trim_start_matches(['-', '+']);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    // 38 digits is the most an `i128` holds whole: past it, no unit brings it back in range.
    let whole: i128 = whole.parse().map_err(|_| Refusal::OutOfRange)?;
    let magnitude = forced.unwrap_or_else(|| Magnitude::guessed(whole));
    let unit = magnitude.nanos();

    let mut below = 0i128;
    let mut scale = unit;
    for digit in fraction.bytes().map(|byte| i128::from(byte - b'0')) {
        scale /= 10;
        below += digit * scale;
    }
    let nanos = whole
        .checked_mul(unit)
        .and_then(|nanos| nanos.checked_add(below))
        .ok_or(Refusal::OutOfRange)?;
    let nanos = if negative { -nanos } else { nanos };

    let seconds =
        i64::try_from(nanos.div_euclid(NANOS_PER_SECOND)).map_err(|_| Refusal::OutOfRange)?;
    let within = u32::try_from(nanos.rem_euclid(NANOS_PER_SECOND)).unwrap_or(0);
    let at = DateTime::from_timestamp(seconds, within).ok_or(Refusal::OutOfRange)?;
    Ok(Reading {
        magnitude: Some(magnitude),
        guessed: forced.is_none(),
        ..Reading::exact(ReadAs::Unix, at)
    })
}

/// A cursor over the ASCII of an ISO 8601 date: anything else stops it where it stands.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn eat(&mut self, wanted: u8) -> bool {
        let found = self.peek() == Some(wanted);
        if found {
            self.at += 1;
        }
        found
    }

    fn expect(&mut self, wanted: u8) -> Result<(), Refusal> {
        if self.eat(wanted) {
            Ok(())
        } else {
            Err(Refusal::Unreadable(self.at))
        }
    }

    fn digits(&mut self, count: usize) -> Result<u32, Refusal> {
        let start = self.at;
        let mut value = 0u32;
        for _ in 0..count {
            match self.peek() {
                Some(byte) if byte.is_ascii_digit() => {
                    value = value * 10 + u32::from(byte - b'0');
                    self.at += 1;
                }
                _ => return Err(Refusal::Unreadable(start)),
            }
        }
        Ok(value)
    }

    /// The field's value, or the field's position when the calendar refuses it.
    fn field<T>(
        &mut self,
        count: usize,
        check: impl FnOnce(u32) -> Option<T>,
    ) -> Result<T, Refusal> {
        let start = self.at;
        let value = self.digits(count)?;
        check(value).ok_or(Refusal::Unreadable(start))
    }

    fn done(&self) -> bool {
        self.at == self.bytes.len()
    }
}

fn iso(text: &str) -> Result<Parsed, Refusal> {
    if let Some(stop) = text.char_indices().find(|(_, c)| !c.is_ascii()) {
        return Err(Refusal::Unreadable(text[..stop.0].chars().count()));
    }
    let mut cursor = Cursor {
        bytes: text.as_bytes(),
        at: 0,
    };
    let year = i32::try_from(cursor.digits(4)?).unwrap_or(0);
    cursor.expect(b'-')?;

    let (date, read_as) = if cursor.eat(b'W') {
        let start = cursor.at;
        let week = cursor.digits(2)?;
        let day = if cursor.eat(b'-') {
            cursor.field(1, |day| {
                chrono::Weekday::try_from(u8::try_from(day).ok()?.checked_sub(1)?).ok()
            })?
        } else {
            chrono::Weekday::Mon
        };
        let date = NaiveDate::from_isoywd_opt(year, week, day).ok_or(Refusal::Unreadable(start))?;
        (date, ReadAs::WeekDate)
    } else {
        let start = cursor.at;
        let first = cursor.digits(2)?;
        match cursor.peek() {
            Some(b'-') => {
                cursor.at += 1;
                let month = first;
                let day_at = cursor.at;
                let day = cursor.digits(2)?;
                let date = NaiveDate::from_ymd_opt(year, month, 1)
                    .ok_or(Refusal::Unreadable(start))?
                    .with_day(day)
                    .ok_or(Refusal::Unreadable(day_at))?;
                (date, ReadAs::Iso8601)
            }
            Some(byte) if byte.is_ascii_digit() => {
                cursor.at = start;
                let date = cursor.field(3, |day| NaiveDate::from_yo_opt(year, day))?;
                (date, ReadAs::OrdinalDate)
            }
            _ => return Err(Refusal::Unreadable(cursor.at)),
        }
    };

    if cursor.done() {
        return Ok(Parsed::Wall(date.and_time(NaiveTime::MIN), read_as));
    }
    if !(cursor.eat(b'T') || cursor.eat(b't') || cursor.eat(b' ')) {
        return Err(Refusal::Unreadable(cursor.at));
    }
    let time = time(&mut cursor)?;
    let naive = date.and_time(time);

    if cursor.done() {
        return Ok(Parsed::Wall(naive, read_as));
    }
    let offset = offset(&mut cursor)?;
    if !cursor.done() {
        return Err(Refusal::Unreadable(cursor.at));
    }
    let at = offset
        .from_local_datetime(&naive)
        .single()
        .ok_or(Refusal::OutOfRange)?;
    Ok(Parsed::Absolute(Reading::exact(
        read_as,
        at.with_timezone(&Utc),
    )))
}

fn time(cursor: &mut Cursor<'_>) -> Result<NaiveTime, Refusal> {
    let hour = cursor.field(2, |hour| (hour < 24).then_some(hour))?;
    cursor.expect(b':')?;
    let minute = cursor.field(2, |minute| (minute < 60).then_some(minute))?;
    let second = if cursor.eat(b':') {
        cursor.field(2, |second| (second < 60).then_some(second))?
    } else {
        0
    };
    let mut nanos = 0u32;
    if cursor.eat(b'.') || cursor.eat(b',') {
        let start = cursor.at;
        let mut scale = 100_000_000u32;
        while let Some(byte) = cursor.peek().filter(u8::is_ascii_digit) {
            nanos += u32::from(byte - b'0') * scale;
            scale /= 10;
            cursor.at += 1;
        }
        if cursor.at == start {
            return Err(Refusal::Unreadable(start));
        }
    }
    NaiveTime::from_hms_nano_opt(hour, minute, second, nanos).ok_or(Refusal::Unreadable(0))
}

fn offset(cursor: &mut Cursor<'_>) -> Result<FixedOffset, Refusal> {
    if cursor.eat(b'Z') || cursor.eat(b'z') {
        return FixedOffset::east_opt(0).ok_or(Refusal::Unreadable(cursor.at));
    }
    let start = cursor.at;
    let sign = match cursor.peek() {
        Some(b'+') => 1,
        Some(b'-') => -1,
        _ => return Err(Refusal::Unreadable(start)),
    };
    cursor.at += 1;
    let hours = cursor.field(2, |hours| (hours < 24).then_some(hours))?;
    let minutes = if cursor.done() {
        0
    } else {
        cursor.eat(b':');
        cursor.field(2, |minutes| (minutes < 60).then_some(minutes))?
    };
    let seconds = i32::try_from(hours * 3600 + minutes * 60).unwrap_or(0);
    FixedOffset::east_opt(sign * seconds).ok_or(Refusal::Unreadable(start))
}

fn local<Z: TimeZone>(naive: NaiveDateTime, read_as: ReadAs, zone: &Z) -> Result<Reading, Refusal> {
    let (at, ambiguous) = match zone.from_local_datetime(&naive) {
        LocalResult::Single(at) => (at, false),
        LocalResult::Ambiguous(earlier, _) => (earlier, true),
        LocalResult::None => return Err(Refusal::Skipped),
    };
    Ok(Reading {
        local_assumed: true,
        ambiguous,
        ..Reading::exact(read_as, at.with_timezone(&Utc))
    })
}

fn forms<Z: TimeZone>(at: DateTime<Utc>, zone: &Z) -> InstantForms {
    let local = at.with_timezone(zone).fixed_offset();
    let date = local.date_naive();
    let week = date.iso_week();
    let micros = i128::from(at.timestamp()) * 1_000_000 + i128::from(at.timestamp_subsec_micros());

    #[allow(clippy::cast_precision_loss)]
    let epoch_milliseconds = at.timestamp_millis() as f64;

    InstantForms {
        unix_seconds: at.timestamp().to_string(),
        unix_milliseconds: at.timestamp_millis().to_string(),
        unix_microseconds: micros.to_string(),
        unix_nanoseconds: at.timestamp_nanos_opt().map(|nanos| nanos.to_string()),
        iso_utc: at.to_rfc3339_opts(SecondsFormat::AutoSi, true),
        iso_local: local.to_rfc3339_opts(SecondsFormat::AutoSi, false),
        local_offset: local.offset().to_string(),
        rfc2822: (0..=9999)
            .contains(&local.year())
            .then(|| local.to_rfc2822()),
        weekday: date.weekday().number_from_monday(),
        week_date: format!(
            "{}-W{:02}-{}",
            week.year(),
            week.week(),
            date.weekday().number_from_monday()
        ),
        week: week.week(),
        ordinal_date: format!("{}-{:03}", date.year(), date.ordinal()),
        day_of_year: date.ordinal(),
        epoch_milliseconds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paris_summer() -> FixedOffset {
        FixedOffset::east_opt(2 * 3600).unwrap()
    }

    fn ask(text: &str) -> InstantAnswer {
        describe(
            &InstantRequest {
                text: text.to_owned(),
                magnitude: None,
            },
            &paris_summer(),
        )
    }

    fn read(text: &str) -> (ReadAs, Option<Magnitude>, bool, InstantForms) {
        match ask(text) {
            InstantAnswer::Read {
                read_as,
                magnitude,
                local_assumed,
                forms,
                ..
            } => (read_as, magnitude, local_assumed, *forms),
            other => panic!("{text} was not read: {other:?}"),
        }
    }

    fn utc(text: &str) -> String {
        read(text).3.iso_utc
    }

    #[test]
    fn the_epoch_is_read_in_every_form() {
        let (read_as, magnitude, local_assumed, forms) = read("0");

        assert_eq!(read_as, ReadAs::Unix);
        assert_eq!(magnitude, Some(Magnitude::Seconds));
        assert!(!local_assumed);
        assert_eq!(
            forms,
            InstantForms {
                unix_seconds: "0".into(),
                unix_milliseconds: "0".into(),
                unix_microseconds: "0".into(),
                unix_nanoseconds: Some("0".into()),
                iso_utc: "1970-01-01T00:00:00Z".into(),
                iso_local: "1970-01-01T02:00:00+02:00".into(),
                local_offset: "+02:00".into(),
                rfc2822: Some("Thu, 1 Jan 1970 02:00:00 +0200".into()),
                weekday: 4,
                week_date: "1970-W01-4".into(),
                week: 1,
                ordinal_date: "1970-001".into(),
                day_of_year: 1,
                epoch_milliseconds: 0.0,
            }
        );
    }

    #[test]
    fn the_last_second_of_a_32_bit_clock_is_read() {
        assert_eq!(utc("2147483647"), "2038-01-19T03:14:07Z");
        assert_eq!(read("2038-01-19T03:14:07Z").3.unix_seconds, "2147483647");
    }

    #[test]
    fn each_magnitude_is_read_by_its_size() {
        for (text, magnitude) in [
            ("1790000000", Magnitude::Seconds),
            ("1790000000123", Magnitude::Milliseconds),
            ("1790000000123456", Magnitude::Microseconds),
            ("1790000000123456789", Magnitude::Nanoseconds),
        ] {
            let (_, read, _, forms) = read(text);

            assert_eq!(read, Some(magnitude), "{text}");
            assert!(forms.iso_utc.starts_with("2026-09-21T14:13:20"), "{text}");
        }
        assert_eq!(utc("1790000000123456789"), "2026-09-21T14:13:20.123456789Z");
    }

    #[test]
    fn a_forced_magnitude_overrides_the_guess() {
        let answer = describe(
            &InstantRequest {
                text: "1790000000".into(),
                magnitude: Some(Magnitude::Milliseconds),
            },
            &Utc,
        );

        let InstantAnswer::Read {
            magnitude,
            guessed,
            forms,
            ..
        } = answer
        else {
            panic!("not read");
        };
        assert_eq!((magnitude, guessed), (Some(Magnitude::Milliseconds), false));
        assert_eq!(forms.iso_utc, "1970-01-21T17:13:20Z");
    }

    #[test]
    fn a_negative_timestamp_is_before_1970_and_a_fraction_is_kept() {
        assert_eq!(utc("-86400"), "1969-12-31T00:00:00Z");
        assert_eq!(utc("-1.5"), "1969-12-31T23:59:58.500Z");
        assert_eq!(read("-1.5").3.unix_seconds, "-2");
        assert_eq!(utc("1790000000.25"), "2026-09-21T14:13:20.250Z");
    }

    #[test]
    fn the_range_s_limits_are_answered_not_overflowed() {
        assert_eq!(
            ask(&format!("1{}", "0".repeat(29))),
            InstantAnswer::OutOfRange
        );
        assert_eq!(
            ask(&"9".repeat(60)),
            InstantAnswer::OutOfRange,
            "past an i128"
        );
        let far = describe(
            &InstantRequest {
                text: "8000000000000".into(),
                magnitude: Some(Magnitude::Seconds),
            },
            &Utc,
        );
        let InstantAnswer::Read { forms, .. } = far else {
            panic!("read");
        };
        assert_eq!(forms.unix_nanoseconds, None);
        assert_eq!(forms.rfc2822, None);
        assert!(forms.iso_utc.starts_with("+255479-"));

        for edge in [DateTime::<Utc>::MIN_UTC, DateTime::<Utc>::MAX_UTC] {
            for hours in [-14, 14] {
                let zone = FixedOffset::east_opt(hours * 3600).unwrap();
                let text = edge.timestamp().to_string();
                let answer = describe(
                    &InstantRequest {
                        text,
                        magnitude: Some(Magnitude::Seconds),
                    },
                    &zone,
                );
                assert!(
                    matches!(
                        answer,
                        InstantAnswer::Read { .. } | InstantAnswer::OutOfRange
                    ),
                    "{edge} {hours}"
                );
            }
        }
    }

    #[test]
    fn every_iso_form_is_read() {
        assert_eq!(utc("2026-09-29T14:03:12+02:00"), "2026-09-29T12:03:12Z");
        assert_eq!(utc("2026-09-29T12:03:12Z"), "2026-09-29T12:03:12Z");
        assert_eq!(utc("2026-09-29 12:03:12.5z"), "2026-09-29T12:03:12.500Z");
        assert_eq!(utc("2026-09-29T12:03-0530"), "2026-09-29T17:33:00Z");
        assert_eq!(utc("2026-09-29T12:03:12,25+01"), "2026-09-29T11:03:12.250Z");

        let (read_as, _, local_assumed, forms) = read("2026-W40-2");
        assert_eq!((read_as, local_assumed), (ReadAs::WeekDate, true));
        assert_eq!(forms.iso_local, "2026-09-29T00:00:00+02:00");
        assert_eq!(read("2026-W40").3.weekday, 1);

        let (read_as, _, _, forms) = read("2026-272");
        assert_eq!(read_as, ReadAs::OrdinalDate);
        assert_eq!(forms.week_date, "2026-W40-2");
        assert_eq!((forms.day_of_year, forms.week), (272, 40));
    }

    #[test]
    fn a_date_without_an_offset_is_local_and_says_so() {
        let (read_as, _, local_assumed, forms) = read("2026-09-29T14:03:12");

        assert_eq!((read_as, local_assumed), (ReadAs::Iso8601, true));
        assert_eq!(forms.iso_utc, "2026-09-29T12:03:12Z");
        assert!(!read("2026-09-29T14:03:12Z").2);
    }

    #[test]
    fn a_leap_day_is_a_day_and_the_28th_of_february_is_not_the_29th() {
        assert_eq!(read("2024-02-29").3.day_of_year, 60);
        assert_eq!(ask("2026-02-29"), InstantAnswer::Unreadable { at: 9 });
    }

    #[test]
    fn rfc_2822_is_read_too() {
        let (read_as, _, _, forms) = read("Tue, 29 Sep 2026 14:03:12 +0200");

        assert_eq!(read_as, ReadAs::Rfc2822);
        assert_eq!(forms.iso_utc, "2026-09-29T12:03:12Z");
    }

    #[test]
    fn an_unreadable_text_says_where_it_stops() {
        assert_eq!(ask("2026-13-01"), InstantAnswer::Unreadable { at: 6 });
        assert_eq!(ask("  2026/09/29"), InstantAnswer::Unreadable { at: 7 });
        assert_eq!(
            ask("2026-09-29T25:00"),
            InstantAnswer::Unreadable { at: 12 }
        );
        assert_eq!(
            ask("2026-09-29T12:00Zut"),
            InstantAnswer::Unreadable { at: 18 }
        );
        assert_eq!(
            ask("2026-09-29T12:00+2"),
            InstantAnswer::Unreadable { at: 18 }
        );
        assert_eq!(ask("2026-W54"), InstantAnswer::Unreadable { at: 7 });
        assert_eq!(
            ask("2026-09-29T12:00:00.x"),
            InstantAnswer::Unreadable { at: 21 }
        );
        assert_eq!(ask("2026-09-2é"), InstantAnswer::Unreadable { at: 10 });
        assert_eq!(ask("demain"), InstantAnswer::Unreadable { at: 1 });
        assert_eq!(ask("12:00"), InstantAnswer::Unreadable { at: 1 });
    }

    #[test]
    fn a_skipped_local_time_and_a_repeated_one_are_told_apart() {
        let in_paris = |text: &str| {
            describe(
                &InstantRequest {
                    text: text.to_owned(),
                    magnitude: None,
                },
                &chrono_tz::Europe::Paris,
            )
        };

        assert_eq!(in_paris("2026-03-29T02:30"), InstantAnswer::Skipped);
        let InstantAnswer::Read {
            ambiguous, forms, ..
        } = in_paris("2026-10-25T02:30")
        else {
            panic!("read");
        };
        assert!(ambiguous);
        assert_eq!(
            forms.iso_utc, "2026-10-25T00:30:00Z",
            "the earlier, still in summer time"
        );
    }

    #[test]
    fn now_is_an_iso_instant_in_utc_that_reads_back() {
        let text = now();

        assert!(text.ends_with('Z'));
        assert_eq!(read(&text).0, ReadAs::Iso8601);
    }
}
