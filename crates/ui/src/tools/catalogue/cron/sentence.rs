//! A cron expression read aloud, from the fields Rust took apart: the words are translation keys,
//! the grammar the smallest that reads well — times written as times when the minute and the hour
//! are plain values, otherwise one clause per field.

use engine::tools::cron::{CronField, FieldReading, Piece};
use i18n::{Language, Message, Value};

const SAY: &str = "tools.cron.say";

struct Speech {
    language: Language,
}

impl Speech {
    fn say(&self, path: &str, params: Vec<(&str, Value)>) -> String {
        let Some(key) = i18n::Key::find(path) else {
            return String::new();
        };
        let mut message = Message::new(key);
        for (name, value) in params {
            message.params.push((name.to_owned(), value));
        }
        message.text(self.language)
    }

    fn join(&self, items: &[String]) -> String {
        i18n::format::and_list(items, self.language)
    }

    fn format(&self, field: CronField, value: u32) -> String {
        match field {
            CronField::Hour => self.say(&format!("{SAY}.hourValue"), vec![("hour", value.into())]),
            CronField::DayOfMonth => {
                self.say(&format!("{SAY}.dayValue"), vec![("day", value.into())])
            }
            CronField::Month => self.say(&format!("tools.cron.months.{value}"), vec![]),
            CronField::DayOfWeek => self.say(&format!("tools.cron.days.{}", value % 7), vec![]),
            CronField::Second | CronField::Minute => value.to_string(),
        }
    }

    /// One field's clause: its plain values together, then each range, step or special day.
    fn clause(&self, field: CronField, pieces: &[Piece]) -> String {
        let values: Vec<u32> = pieces
            .iter()
            .filter_map(|piece| match piece {
                Piece::Value { value } => Some(*value),
                _ => None,
            })
            .collect();
        let mut phrases = Vec::new();
        if !values.is_empty() {
            let shown: Vec<String> = values.iter().map(|it| self.format(field, *it)).collect();
            let count = u32::try_from(values.len()).unwrap_or(u32::MAX);
            phrases.push(self.say(
                &format!("{SAY}.{}.values", id_of(field)),
                vec![("count", count.into()), ("list", self.join(&shown).into())],
            ));
        }
        phrases.extend(pieces.iter().filter_map(|piece| self.phrase(field, piece)));
        self.join(&phrases)
    }

    fn phrase(&self, field: CronField, piece: &Piece) -> Option<String> {
        let id = id_of(field);
        Some(match piece {
            Piece::Range { from, to } => self.say(
                &format!("{SAY}.{id}.range"),
                vec![
                    ("from", self.format(field, *from).into()),
                    ("to", self.format(field, *to).into()),
                ],
            ),
            Piece::Step { step } => {
                self.say(&format!("{SAY}.{id}.step"), vec![("step", (*step).into())])
            }
            Piece::SteppedRange { step, from, to } => self.say(
                &format!("{SAY}.{id}.steppedRange"),
                vec![
                    ("step", (*step).into()),
                    ("from", self.format(field, *from).into()),
                    ("to", self.format(field, *to).into()),
                ],
            ),
            Piece::LastDayOfMonth => self.say(&format!("{SAY}.dayOfMonth.last"), vec![]),
            Piece::NearestWeekday { day } => self.say(
                &format!("{SAY}.dayOfMonth.nearest"),
                vec![("day", self.format(CronField::DayOfMonth, *day).into())],
            ),
            Piece::LastWeekday { weekday } => self.say(
                &format!("{SAY}.dayOfWeek.last"),
                vec![("day", self.format(CronField::DayOfWeek, *weekday).into())],
            ),
            Piece::Nth { weekday, nth } => self.say(
                &format!("{SAY}.dayOfWeek.nth"),
                vec![
                    (
                        "nth",
                        self.say(&format!("tools.cron.nths.{nth}"), vec![]).into(),
                    ),
                    ("day", self.format(CronField::DayOfWeek, *weekday).into()),
                ],
            ),
            Piece::Every | Piece::Value { .. } => return None,
        })
    }

    /// Seconds, minutes and hours: as times when they are plain values, a clause each otherwise.
    fn time_clauses(&self, fields: &[FieldReading]) -> Vec<String> {
        let (seconds, minutes, hours) = (
            pieces_of(fields, CronField::Second),
            pieces_of(fields, CronField::Minute),
            pieces_of(fields, CronField::Hour),
        );
        let second_values = seconds.and_then(values_of);
        let plain_seconds = seconds.is_none() || second_values.as_deref() == Some(&[0]);
        let minute_values = minutes.and_then(values_of);
        let hour_values = hours.and_then(values_of);
        let mut clauses = Vec::new();
        if !plain_seconds {
            clauses.push(if is_every(seconds) {
                self.say(&format!("{SAY}.second.every"), vec![])
            } else {
                self.clause(CronField::Second, seconds.unwrap_or_default())
            });
        }
        if plain_seconds
            && let (Some(minute_values), Some(hour_values)) = (&minute_values, &hour_values)
            && minute_values.len() * hour_values.len() <= 8
        {
            let mut times: Vec<String> = hour_values
                .iter()
                .flat_map(|hour| {
                    minute_values
                        .iter()
                        .map(move |it| format!("{hour:02}:{it:02}"))
                })
                .collect();
            times.sort();
            let count = u32::try_from(times.len()).unwrap_or(u32::MAX);
            let list = self.join(&times);
            return vec![self.say(
                &format!("{SAY}.at"),
                vec![("count", count.into()), ("list", list.into())],
            )];
        }
        if minute_values.as_deref() == Some(&[0]) && hour_values.is_none() {
            return vec![if is_every(hours) {
                self.say(&format!("{SAY}.hour.every"), vec![])
            } else {
                self.clause(CronField::Hour, hours.unwrap_or_default())
            }];
        }
        if !is_every(minutes) {
            clauses.push(self.clause(CronField::Minute, minutes.unwrap_or_default()));
        } else if plain_seconds {
            clauses.push(self.say(&format!("{SAY}.minute.every"), vec![]));
        }
        if !is_every(hours) {
            clauses.push(self.clause(CronField::Hour, hours.unwrap_or_default()));
        }
        clauses
    }
}

fn id_of(field: CronField) -> &'static str {
    match field {
        CronField::Second => "second",
        CronField::Minute => "minute",
        CronField::Hour => "hour",
        CronField::DayOfMonth => "dayOfMonth",
        CronField::Month => "month",
        CronField::DayOfWeek => "dayOfWeek",
    }
}

fn pieces_of(fields: &[FieldReading], field: CronField) -> Option<&[Piece]> {
    fields
        .iter()
        .find(|it| it.field == field)
        .map(|it| it.pieces.as_slice())
}

fn is_every(pieces: Option<&[Piece]>) -> bool {
    pieces.is_none_or(|it| it.iter().all(|piece| matches!(piece, Piece::Every)))
}

/// The values of a field made only of values.
fn values_of(pieces: &[Piece]) -> Option<Vec<u32>> {
    if pieces.is_empty() {
        return None;
    }
    pieces
        .iter()
        .map(|piece| match piece {
            Piece::Value { value } => Some(*value),
            _ => None,
        })
        .collect()
}

pub(super) fn of(fields: &[FieldReading], language: Language) -> String {
    let speech = Speech { language };
    let mut clauses = speech.time_clauses(fields);
    for field in [
        CronField::DayOfMonth,
        CronField::DayOfWeek,
        CronField::Month,
    ] {
        let pieces = pieces_of(fields, field);
        if !is_every(pieces) {
            clauses.push(speech.clause(field, pieces.unwrap_or_default()));
        }
    }
    let sentence = clauses.join(", ");
    let mut letters = sentence.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

#[cfg(test)]
mod tests {
    use engine::tools::cron::{CronRequest, describe};

    use super::*;

    fn said(expression: &str) -> String {
        let request = CronRequest {
            expression: expression.to_owned(),
            zone: Some("UTC".to_owned()),
            check: String::new(),
            from: String::new(),
            until: String::new(),
        };
        match describe(&request, None, chrono::Utc::now()) {
            engine::tools::cron::CronAnswer::Read { fields, .. } => of(&fields, Language::En),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn plain_values_read_as_a_time() {
        assert!(said("30 9 * * *").contains("09:30"));
    }

    #[test]
    fn a_step_and_a_range_each_have_their_clause() {
        let words = said("*/15 9-18 * * 1-5");

        assert!(words.contains("15"), "{words}");
        assert!(words.contains("Monday"), "{words}");
    }

    #[test]
    fn every_minute_says_so_with_a_capital() {
        let words = said("* * * * *");

        assert!(
            words.chars().next().is_some_and(char::is_uppercase),
            "{words}"
        );
    }
}
