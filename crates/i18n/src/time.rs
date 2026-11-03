//! The labels that age on their own — « il y a 3 min », « expire dans 2j », « dans 2 jours » —
//! as messages. `now` is always a parameter: the clock is the caller's, which also decides when
//! to draw the label again.

use chrono::{DateTime, TimeZone, Utc};

use crate::{Message, keys};

/// How long ago `at` was: minutes, then hours, then days, each rounded.
pub fn relative(at: DateTime<Utc>, now: DateTime<Utc>) -> Message {
    #[allow(clippy::cast_precision_loss)]
    let minutes = ((now - at).num_milliseconds() as f64 / 60_000.0).round();
    if minutes < 1.0 {
        return Message::new(keys::time::JUST_NOW);
    }
    if minutes < 60.0 {
        return Message::new(keys::time::MINUTES_AGO).with("count", minutes);
    }
    let hours = (minutes / 60.0).round();
    if hours < 24.0 {
        return Message::new(keys::time::HOURS_AGO).with("count", hours);
    }
    Message::new(keys::time::DAYS_AGO).with("count", (hours / 24.0).round())
}

/// In local calendar days, not hours: an expiry is the end of its day, and counted in hours
/// the third day away read « 4j » every evening.
pub fn expiry<Tz: TimeZone>(at: DateTime<Utc>, now: DateTime<Utc>, local: &Tz) -> Message {
    if at <= now {
        return Message::new(keys::time::EXPIRED);
    }
    let day = |instant: DateTime<Utc>| instant.with_timezone(local).date_naive();
    let days = (day(at) - day(now)).num_days();
    if days == 0 {
        return Message::new(keys::time::EXPIRES_TODAY);
    }
    #[allow(clippy::cast_precision_loss)]
    Message::new(keys::time::EXPIRES_IN).with("count", days as f64)
}

/// « il y a 3 ans », « dans 2 jours »: in the largest unit the gap holds a whole one of. Under
/// a minute it says so rather than count seconds a clock of thirty seconds cannot follow.
pub fn span(at: DateTime<Utc>, now: DateTime<Utc>) -> Message {
    const DAY: f64 = 86_400_000.0;
    let units = [
        (
            keys::time::span::past::YEARS,
            keys::time::span::future::YEARS,
            365.2425 * DAY,
        ),
        (
            keys::time::span::past::MONTHS,
            keys::time::span::future::MONTHS,
            30.436_875 * DAY,
        ),
        (
            keys::time::span::past::DAYS,
            keys::time::span::future::DAYS,
            DAY,
        ),
        (
            keys::time::span::past::HOURS,
            keys::time::span::future::HOURS,
            3_600_000.0,
        ),
        (
            keys::time::span::past::MINUTES,
            keys::time::span::future::MINUTES,
            60_000.0,
        ),
    ];
    #[allow(clippy::cast_precision_loss)]
    let gap = (at - now).num_milliseconds() as f64;
    let Some((past, future, length)) = units
        .into_iter()
        .find(|(_, _, length)| gap.abs() >= *length)
    else {
        return Message::new(keys::time::span::NOW);
    };
    let key = if gap < 0.0 { past } else { future };
    Message::new(key).with("count", (gap.abs() / length).floor())
}

#[cfg(test)]
mod tests {
    use chrono_tz::Europe::Paris;

    use super::*;
    use crate::Language;

    /// The reference's labels, from a second to a year, both ways, both languages
    /// (`golden/relative-time`), on the machine's zone, Europe/Paris.
    #[test]
    fn every_label_reads_as_the_reference_wrote_it() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../../golden/relative-time/cases.json")).unwrap();
        let mut wrong = Vec::new();
        for case in corpus.as_array().unwrap() {
            let instant = |name: &str| case["input"][name].as_str().unwrap().parse().unwrap();
            let (now, at): (DateTime<Utc>, DateTime<Utc>) = (instant("now"), instant("at"));
            let language = if case["options"]["lang"] == "fr" {
                Language::Fr
            } else {
                Language::En
            };
            let message = match case["options"]["kind"].as_str().unwrap() {
                "relative" => relative(at, now),
                "expiry" => expiry(at, now, &Paris),
                _ => span(at, now),
            };
            let (key, text) = (message.key.path(), message.text(language));
            if key != case["output"]["key"] || text != case["output"]["text"] {
                wrong.push(format!("{case}: {key} {text:?}"));
            }
        }

        assert!(wrong.is_empty(), "{} differ: {wrong:#?}", wrong.len());
    }
}
