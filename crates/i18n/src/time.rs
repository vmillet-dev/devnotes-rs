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

/// What a card's corner has room for: « 4 h », « hier », « 3 j », « 2 sem. ».
pub fn short(at: DateTime<Utc>, now: DateTime<Utc>) -> Message {
    use keys::time::short;
    #[allow(clippy::cast_precision_loss)]
    let minutes = ((now - at).num_milliseconds() as f64 / 60_000.0).round();
    if minutes < 1.0 {
        return Message::new(short::JUST_NOW);
    }
    if minutes < 60.0 {
        return Message::new(short::MINUTES).with("count", minutes);
    }
    let hours = (minutes / 60.0).round();
    if hours < 24.0 {
        return Message::new(short::HOURS).with("count", hours);
    }
    let days = (hours / 24.0).round();
    match days {
        ..2.0 => Message::new(short::YESTERDAY),
        ..7.0 => Message::new(short::DAYS).with("count", days),
        ..30.0 => Message::new(short::WEEKS).with("count", (days / 7.0).floor()),
        ..365.0 => Message::new(short::MONTHS).with("count", (days / 30.0).floor()),
        _ => Message::new(short::YEARS).with("count", (days / 365.0).floor()),
    }
}

/// An expiry in a card's corner: « dans 2 h » the last day, then in local calendar days as
/// [`expiry`] counts them.
pub fn short_expiry<Tz: TimeZone>(at: DateTime<Utc>, now: DateTime<Utc>, local: &Tz) -> Message {
    use keys::time::short;
    if at <= now {
        return Message::new(short::EXPIRED);
    }
    #[allow(clippy::cast_precision_loss)]
    let minutes = ((at - now).num_milliseconds() as f64 / 60_000.0).ceil();
    if minutes < 60.0 {
        return Message::new(short::IN_MINUTES).with("count", minutes);
    }
    if minutes < 24.0 * 60.0 {
        return Message::new(short::IN_HOURS).with("count", (minutes / 60.0).round());
    }
    let day = |instant: DateTime<Utc>| instant.with_timezone(local).date_naive();
    #[allow(clippy::cast_precision_loss)]
    Message::new(short::IN_DAYS).with("count", (day(at) - day(now)).num_days() as f64)
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

    /// A card's corner, both languages: minutes, hours, yesterday, days, weeks, months, years.
    #[test]
    fn a_short_label_fits_a_cards_corner() {
        let now: DateTime<Utc> = "2026-11-09T12:00:00Z".parse().unwrap();
        let ago = |minutes: i64| now - chrono::TimeDelta::minutes(minutes);
        let read = |at, language| short(at, now).text(language);
        let cases = [
            (0, "à l’instant", "now"),
            (5, "5 min", "5 min"),
            (4 * 60, "4 h", "4 h"),
            (30 * 60, "hier", "yesterday"),
            (3 * 24 * 60, "3 j", "3 d"),
            (15 * 24 * 60, "2 sem.", "2 wk"),
            (65 * 24 * 60, "2 mois", "2 mo"),
            (400 * 24 * 60, "1 an", "1 y"),
        ];
        for (minutes, fr, en) in cases {
            assert_eq!(read(ago(minutes), Language::Fr), fr);
            assert_eq!(read(ago(minutes), Language::En), en);
        }
    }

    #[test]
    fn a_short_expiry_counts_hours_the_last_day_then_days() {
        let now: DateTime<Utc> = "2026-11-09T09:00:00Z".parse().unwrap();
        let later = |minutes: i64| now + chrono::TimeDelta::minutes(minutes);
        let read = |at| short_expiry(at, now, &Paris).text(Language::Fr);

        assert_eq!(read(now), "expirée");
        assert_eq!(read(later(20)), "dans 20 min");
        assert_eq!(read(later(2 * 60)), "dans 2 h");
        assert_eq!(read(later(3 * 24 * 60)), "dans 3 j");
    }
}
