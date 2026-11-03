//! Numbers, dates, lists and country names in French and English, written by hand (D21): one
//! function per pattern the interface really uses, named by that use, each proved against
//! what `Intl` wrote for the reference (`golden/intl`). No ICU: two languages and a dozen
//! patterns do not need one.
//!
//! A date or a time arrives as the wall-clock value to show: the caller has put it in the
//! local zone, or in UTC where the screen says so.

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};

use crate::Language;
use crate::regions::COUNTRIES;

// Numbers ------------------------------------------------------------------------------------

/// `Intl.NumberFormat` with no option: up to three decimals.
pub fn number(value: f64, language: Language) -> String {
    decimal(value, language, Digits::new(0, 3), Sign::Negative, true)
}

/// Always `digits` decimals: « 0,0 », « 12,3 ».
pub fn fixed(value: f64, digits: usize, language: Language) -> String {
    decimal(
        value,
        language,
        Digits::new(digits, digits),
        Sign::Negative,
        true,
    )
}

/// Up to `max` decimals, none written when they are zeros.
pub fn at_most(value: f64, max: usize, language: Language) -> String {
    decimal(value, language, Digits::new(0, max), Sign::Negative, true)
}

/// At least `min` decimals, three at most unless `min` asks more.
pub fn at_least(value: f64, min: usize, language: Language) -> String {
    decimal(
        value,
        language,
        Digits::new(min, min.max(3)),
        Sign::Negative,
        true,
    )
}

/// A Calcul sheet's difference: « +1,25 », « -0,5 », « 0 ».
pub fn signed(value: f64, language: Language) -> String {
    decimal(value, language, Digits::new(0, 2), Sign::ExceptZero, true)
}

/// A Calcul sheet's value written without groups: « 1234567,89 ».
pub fn ungrouped(value: f64, language: Language) -> String {
    decimal(value, language, Digits::new(0, 2), Sign::Negative, false)
}

#[derive(Clone, Copy)]
struct Digits {
    min: usize,
    max: usize,
}

impl Digits {
    fn new(min: usize, max: usize) -> Self {
        Self { min, max }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sign {
    Negative,
    ExceptZero,
}

/// An exact decimal as the core wrote it (`-1234567.891`), between `min` and `max` decimals,
/// rounded half away from zero: text in, text out, never through a float, which would invent
/// figures past the fifteenth. `signed` writes `+` before what is not zero.
pub fn exact(
    value: &str,
    (min, max): (usize, usize),
    signed: bool,
    grouped: bool,
    language: Language,
) -> String {
    let magnitude = value.strip_prefix('-');
    let sign = if signed {
        Sign::ExceptZero
    } else {
        Sign::Negative
    };
    written(
        magnitude.unwrap_or(value),
        magnitude.is_some(),
        language,
        Digits::new(min, max),
        sign,
        grouped,
    )
}

fn decimal(value: f64, language: Language, digits: Digits, sign: Sign, grouped: bool) -> String {
    // The shortest decimal that reads back as `value`, as ICU rounds it: 12.345 is 12.35.
    let shortest = format!("{}", value.abs());
    let negative = value.is_sign_negative() && value != 0.0;
    written(&shortest, negative, language, digits, sign, grouped)
}

fn written(
    magnitude: &str,
    negative: bool,
    language: Language,
    digits: Digits,
    sign: Sign,
    grouped: bool,
) -> String {
    let (whole, fraction) = round(magnitude, digits);
    let is_zero =
        whole.bytes().all(|byte| byte == b'0') && fraction.bytes().all(|byte| byte == b'0');
    let mut out = String::new();
    if negative && !(sign == Sign::ExceptZero && is_zero) {
        out.push('-');
    } else if sign == Sign::ExceptZero && !is_zero {
        out.push('+');
    }
    out.push_str(&if grouped {
        group(&whole, language)
    } else {
        whole
    });
    if !fraction.is_empty() {
        out.push(match language {
            Language::Fr => ',',
            Language::En => '.',
        });
        out.push_str(&fraction);
    }
    out
}

/// `shortest` rounded half away from zero to `digits.max` decimals, then padded to `digits.min`.
fn round(shortest: &str, digits: Digits) -> (String, String) {
    let (whole, fraction) = shortest.split_once('.').unwrap_or((shortest, ""));
    let mut number: Vec<u8> = whole.bytes().chain(fraction.bytes()).collect();
    let point = whole.len();
    let kept = point + digits.max.min(fraction.len());
    let carry = fraction.len() > digits.max && number[kept] >= b'5';
    number.truncate(kept);
    let mut point = point;
    if carry {
        let mut at = number.len();
        loop {
            if at == 0 {
                number.insert(0, b'1');
                point += 1;
                break;
            }
            at -= 1;
            if number[at] == b'9' {
                number[at] = b'0';
            } else {
                number[at] += 1;
                break;
            }
        }
    }
    let whole = String::from_utf8_lossy(&number[..point]).into_owned();
    let mut fraction = String::from_utf8_lossy(&number[point..]).into_owned();
    while fraction.len() > digits.min && fraction.ends_with('0') {
        fraction.pop();
    }
    while fraction.len() < digits.min {
        fraction.push('0');
    }
    (whole, fraction)
}

/// Thousands: `,` in English, a narrow no-break space in French.
fn group(whole: &str, language: Language) -> String {
    let separator = match language {
        Language::Fr => '\u{202f}',
        Language::En => ',',
    };
    let mut out = String::new();
    for (at, digit) in whole.chars().enumerate() {
        if at > 0 && (whole.len() - at).is_multiple_of(3) {
            out.push(separator);
        }
        out.push(digit);
    }
    out
}

// Dates and times ------------------------------------------------------------------------------

const MONTHS_FR: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];
const MONTHS_FR_SHORT: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];
const MONTHS_EN: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn month(date: NaiveDate, language: Language) -> &'static str {
    let index = date.month0() as usize;
    match language {
        Language::Fr => MONTHS_FR[index],
        Language::En => MONTHS_EN[index],
    }
}

fn month_short(date: NaiveDate, language: Language) -> &'static str {
    let index = date.month0() as usize;
    match language {
        Language::Fr => MONTHS_FR_SHORT[index],
        Language::En => &MONTHS_EN[index][..3],
    }
}

fn weekday(date: NaiveDate, language: Language) -> &'static str {
    match (language, date.weekday()) {
        (Language::Fr, Weekday::Mon) => "lundi",
        (Language::Fr, Weekday::Tue) => "mardi",
        (Language::Fr, Weekday::Wed) => "mercredi",
        (Language::Fr, Weekday::Thu) => "jeudi",
        (Language::Fr, Weekday::Fri) => "vendredi",
        (Language::Fr, Weekday::Sat) => "samedi",
        (Language::Fr, Weekday::Sun) => "dimanche",
        (Language::En, Weekday::Mon) => "Monday",
        (Language::En, Weekday::Tue) => "Tuesday",
        (Language::En, Weekday::Wed) => "Wednesday",
        (Language::En, Weekday::Thu) => "Thursday",
        (Language::En, Weekday::Fri) => "Friday",
        (Language::En, Weekday::Sat) => "Saturday",
        (Language::En, Weekday::Sun) => "Sunday",
    }
}

/// « 25/07/2026 », « 07/25/2026 ».
pub fn date_numeric(date: NaiveDate, language: Language) -> String {
    let (day, month, year) = (date.day(), date.month(), date.year());
    match language {
        Language::Fr => format!("{day:02}/{month:02}/{year}"),
        Language::En => format!("{month:02}/{day:02}/{year}"),
    }
}

/// « 25 juil. 2026 », « Jul 25, 2026 ».
pub fn date_short(date: NaiveDate, language: Language) -> String {
    let (day, year) = (date.day(), date.year());
    let month = month_short(date, language);
    match language {
        Language::Fr => format!("{day} {month} {year}"),
        Language::En => format!("{month} {day}, {year}"),
    }
}

/// « 25 juillet 2026 », « July 25, 2026 ».
pub fn date_long(date: NaiveDate, language: Language) -> String {
    let (day, year) = (date.day(), date.year());
    let month = month(date, language);
    match language {
        Language::Fr => format!("{day} {month} {year}"),
        Language::En => format!("{month} {day}, {year}"),
    }
}

/// « samedi 25 juillet 2026 », « Saturday, July 25, 2026 ».
pub fn date_with_weekday(date: NaiveDate, language: Language) -> String {
    let weekday = weekday(date, language);
    match language {
        Language::Fr => format!("{weekday} {}", date_long(date, language)),
        Language::En => format!("{weekday}, {}", date_long(date, language)),
    }
}

/// « samedi 25 juillet », « Saturday, July 25 ».
pub fn day_with_weekday(date: NaiveDate, language: Language) -> String {
    let weekday = weekday(date, language);
    match language {
        Language::Fr => format!("{weekday} {}", day_month(date, language)),
        Language::En => format!("{weekday}, {}", day_month(date, language)),
    }
}

/// « 25 juillet », « July 25 ».
pub fn day_month(date: NaiveDate, language: Language) -> String {
    let day = date.day();
    let month = month(date, language);
    match language {
        Language::Fr => format!("{day} {month}"),
        Language::En => format!("{month} {day}"),
    }
}

/// « 11:05 », « 11:05 AM ».
pub fn time_minutes(time: NaiveTime, language: Language) -> String {
    clock(time, language, Precision::Minutes, true)
}

/// « 11:05:07 », « 11:05:07 AM ».
pub fn time_seconds(time: NaiveTime, language: Language) -> String {
    clock(time, language, Precision::Seconds, true)
}

/// « 11:05:07,123 », « 11:05:07.123 AM ».
pub fn time_millis(time: NaiveTime, language: Language) -> String {
    clock(time, language, Precision::Millis, true)
}

/// « 11:05:07 » in both languages: a 24-hour clock.
pub fn time_seconds_24h(time: NaiveTime) -> String {
    clock(time, Language::Fr, Precision::Seconds, true)
}

/// « 25 juil. 2026, 11:05 », « Jul 25, 2026, 1:00 AM ».
pub fn date_time(at: NaiveDateTime, language: Language) -> String {
    format!(
        "{}, {}",
        date_short(at.date(), language),
        clock(at.time(), language, Precision::Minutes, false)
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Precision {
    Minutes,
    Seconds,
    Millis,
}

/// French counts 24 hours; English 12, `AM`/`PM`. `padded` writes the hour on two digits.
fn clock(time: NaiveTime, language: Language, precision: Precision, padded: bool) -> String {
    let (hour, suffix) = match language {
        Language::Fr => (time.hour(), ""),
        Language::En => {
            let (pm, hour) = time.hour12();
            (hour, if pm { " PM" } else { " AM" })
        }
    };
    let mut out = if padded || language == Language::Fr {
        format!("{hour:02}:{:02}", time.minute())
    } else {
        format!("{hour}:{:02}", time.minute())
    };
    if precision != Precision::Minutes {
        let _ = std::fmt::Write::write_fmt(&mut out, format_args!(":{:02}", time.second()));
    }
    if precision == Precision::Millis {
        out.push(match language {
            Language::Fr => ',',
            Language::En => '.',
        });
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!("{:03}", time.nanosecond() / 1_000_000),
        );
    }
    out.push_str(suffix);
    out
}

// Lists and names ----------------------------------------------------------------------------

/// « un, deux et trois », « one, two, and three ».
pub fn and_list(items: &[impl AsRef<str>], language: Language) -> String {
    list(items, language, ("et", "and"))
}

/// « un ou deux », « one or two ».
pub fn or_list(items: &[impl AsRef<str>], language: Language) -> String {
    list(items, language, ("ou", "or"))
}

fn list(items: &[impl AsRef<str>], language: Language, (fr, en): (&str, &str)) -> String {
    let items: Vec<&str> = items.iter().map(AsRef::as_ref).collect();
    let Some((last, rest)) = items.split_last() else {
        return String::new();
    };
    if rest.is_empty() {
        return (*last).to_string();
    }
    let head = rest.join(", ");
    match language {
        Language::Fr => format!("{head} {fr} {last}"),
        // The serial comma, as English CLDR writes a list of three or more.
        Language::En if rest.len() > 1 => format!("{head}, {en} {last}"),
        Language::En => format!("{head} {en} {last}"),
    }
}

/// « Allemagne », « Germany »: the name of an IBAN's country. `None` for a code outside the
/// IBAN registry.
pub fn country(code: &str, language: Language) -> Option<&'static str> {
    COUNTRIES
        .iter()
        .find(|(known, _, _)| *known == code)
        .map(|(_, fr, en)| match language {
            Language::Fr => *fr,
            Language::En => *en,
        })
}

/// The CLDR plural category of `count`: `one`, `many` or `other`.
pub fn plural_category(count: f64, language: Language) -> &'static str {
    crate::plural::category(count, language)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};
    use chrono_tz::Europe::Paris;

    use super::*;

    /// What the reference's machine showed: Europe/Paris, unless the call said UTC.
    fn wall(input: &str, utc: bool) -> NaiveDateTime {
        let at: DateTime<Utc> = input.parse().unwrap();
        if utc {
            at.naive_utc()
        } else {
            Paris.from_utc_datetime(&at.naive_utc()).naive_local()
        }
    }

    fn written(
        site: &str,
        input: &serde_json::Value,
        language: Language,
        utc: bool,
    ) -> Option<String> {
        let at = || wall(input.as_str().unwrap(), utc);
        let value = || input.as_f64().unwrap();
        Some(match site {
            "calc-format DATE_STYLES.numeric" => date_numeric(at().date(), language),
            "calc-format DATE_STYLES.short" => date_short(at().date(), language),
            "calc-format DATE_STYLES.long" | "dates-tool day" => {
                date_with_weekday(at().date(), language)
            }
            "http-history.store draft name" => time_minutes(at().time(), language),
            "history-page day" => day_with_weekday(at().date(), language),
            "history-page time" => time_seconds(at().time(), language),
            "cookie-jar-dialog expiry" => date_time(at(), language),
            "socket-log time" => time_millis(at().time(), language),
            "dates-tool time" => time_seconds_24h(at().time()),
            "durations day" => day_month(at().date(), language),
            "identity-tool birth" => date_long(at().date(), language),
            "response-pane tenths" | "runner-results seconds" => fixed(value(), 1, language),
            "response-view size" => at_most(value(), 2, language),
            "search-bar count" | "transfer number" => number(value(), language),
            "json-explorer size" => at_most(value(), 1, language),
            "settings-page hint" => at_least(value(), 2, language),
            "colour-tool" => fixed(value(), 2, language),
            "durations number" => at_most(value(), 6, language),
            "calc-format signed" => signed(value(), language),
            "calc-format ungrouped" => ungrouped(value(), language),
            "cron-tool" | "note-picker" => {
                let items: Vec<&str> = input
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|item| item.as_str().unwrap())
                    .collect();
                if site == "cron-tool" {
                    and_list(&items, language)
                } else {
                    or_list(&items, language)
                }
            }
            "checks-tool" => country(input.as_str().unwrap(), language)?.to_string(),
            "plural-transpiler" => plural_category(value(), language).to_string(),
            _ => return None,
        })
    }

    /// Every call of the reference's `Intl`, with its exact options, at its edge values.
    #[test]
    fn every_pattern_writes_what_intl_wrote() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../../golden/intl/cases.json")).unwrap();
        let mut wrong = Vec::new();
        for case in corpus.as_array().unwrap() {
            let options = &case["options"];
            let site = options["site"].as_str().unwrap();
            let language = if options["lang"] == "fr" {
                Language::Fr
            } else {
                Language::En
            };
            let utc = options["timeZone"] == "UTC";
            let expected = case["output"].as_str().unwrap();
            match written(site, &case["input"], language, utc) {
                Some(text) if text == expected => {}
                Some(text) => wrong.push(format!(
                    "{site} {:?} {}: {text:?}, expected {expected:?}",
                    language, case["input"]
                )),
                None => wrong.push(format!("{site}: no function writes it")),
            }
        }

        assert!(wrong.is_empty(), "{} differ: {wrong:#?}", wrong.len());
    }

    #[test]
    fn every_iban_country_has_both_names() {
        for (code, fr, en) in COUNTRIES {
            assert!(
                code.len() == 2 && !fr.is_empty() && !en.is_empty(),
                "{code}"
            );
        }
        assert_eq!(country("ZZ", Language::Fr), None);
    }

    #[test]
    fn a_list_of_one_or_none_is_its_item_or_nothing() {
        assert_eq!(and_list(&["un"], Language::Fr), "un");
        let none: [&str; 0] = [];
        assert_eq!(or_list(&none, Language::En), "");
        assert_eq!(
            or_list(&["one", "two", "three"], Language::En),
            "one, two, or three"
        );
    }

    #[test]
    fn rounding_carries_into_the_whole_number() {
        assert_eq!(fixed(9.96, 1, Language::En), "10.0");
        assert_eq!(fixed(999.95, 1, Language::Fr), "1\u{202f}000,0");
        assert_eq!(signed(-0.001, Language::En), "0");
        assert_eq!(number(-0.0, Language::En), "0");
    }
}
