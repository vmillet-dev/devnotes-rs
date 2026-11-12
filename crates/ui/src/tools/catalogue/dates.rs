//! An instant in every form, the gap between two dates, an ISO 8601 duration. Every reading and
//! every form is Rust's; how long ago is the page's, so it ages without a round trip.

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Utc};
use egui::Ui;
use engine::notes::language::Language as NoteLanguage;
use engine::tools::dates::{InstantAnswer, InstantForms, InstantRequest, Magnitude, ReadAs};
use engine::tools::durations::{
    CalendarGap, DurationAnswer, DurationParts, DurationsAnswer, DurationsRequest, GapAnswer,
    GapField, Transition,
};
use i18n::{Language, Message, keys};
use serde_json::{Value, json};

use crate::api;
use crate::backend::Pending;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{
    ResultRow, card, columns, group_title, hint, label, named, paragraph, problem, result_rows,
    section_title,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{Button, Segmented, TextField, address};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Convert,
    Durations,
}

pub(super) struct DatesTool {
    tab: Tab,
    text: String,
    magnitude: Option<Magnitude>,
    from: String,
    to: String,
    duration: String,
    answer: Live<InstantRequest, InstantAnswer>,
    durations: Live<DurationsRequest, DurationsAnswer>,
    wants_now: bool,
    now: Option<Pending<String>>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(DatesTool {
        tab: Tab::Convert,
        text: String::new(),
        magnitude: None,
        from: String::new(),
        to: String::new(),
        duration: String::new(),
        answer: Live::default(),
        durations: Live::default(),
        wants_now: false,
        now: None,
    })
}

fn said(language: Language, path: &str, params: &[(&str, i18n::Value)]) -> String {
    i18n::Key::find(path).map_or_else(String::new, |key| i18n::translate(language, key, params))
}

fn counted(language: Language, path: &str, count: u32) -> String {
    said(language, path, &[("count", count.into())])
}

fn number(value: f64, language: Language) -> String {
    i18n::format::at_most(value, 6, language)
}

fn whole(value: u32, language: Language) -> String {
    number(f64::from(value), language)
}

/// « Monday 8 November 1971, 05:25:55 »: the instant on screen, in the language on screen.
fn long_date(epoch_milliseconds: f64, language: Language) -> Option<String> {
    #[expect(clippy::cast_possible_truncation, reason = "milliseconds of a date")]
    let at = DateTime::from_timestamp_millis(epoch_milliseconds as i64)?;
    let local = at.with_timezone(&Local);
    let day = i18n::format::date_with_weekday(local.date_naive(), language);
    let time = i18n::format::time_seconds_24h(local.time());
    Some(format!("{day}, {time}"))
}

/// « 29 March », the date itself and no zone.
fn day_of_month(date: &str, language: Language) -> String {
    NaiveDate::parse_from_str(date, "%Y-%m-%d").map_or_else(
        |_| date.to_owned(),
        |it| i18n::format::day_month(it, language),
    )
}

/// A form of the instant: the key of its name, its value, and whether it is pasted anywhere.
type FormRow = (&'static str, String, bool);

fn groups_of(
    forms: &InstantForms,
    now: DateTime<Utc>,
    language: Language,
) -> [(&'static str, Vec<FormRow>); 3] {
    let plain = |id, text: &str| (id, text.to_owned(), true);
    let mut unix = vec![
        plain("unixSeconds", &forms.unix_seconds),
        plain("unixMilliseconds", &forms.unix_milliseconds),
        plain("unixMicroseconds", &forms.unix_microseconds),
    ];
    if let Some(nanos) = &forms.unix_nanoseconds {
        unix.push(plain("unixNanoseconds", nanos));
    }
    let mut formats = vec![
        plain("isoUtc", &forms.iso_utc),
        plain("isoLocal", &forms.iso_local),
    ];
    if let Some(rfc) = &forms.rfc2822 {
        formats.push(plain("rfc2822", rfc));
    }
    let mut landmarks = Vec::new();
    #[expect(clippy::cast_possible_truncation, reason = "milliseconds of a date")]
    if let Some(at) = DateTime::from_timestamp_millis(forms.epoch_milliseconds as i64) {
        let words = i18n::time::span(at, now).text(language);
        landmarks.push(("relative", words, false));
    }
    let weekday = said(
        language,
        &format!("tools.dates.weekdays.{}", forms.weekday),
        &[],
    );
    landmarks.extend([
        ("weekday", weekday, true),
        plain("weekDate", &forms.week_date),
        ("week", forms.week.to_string(), true),
        plain("ordinalDate", &forms.ordinal_date),
        ("dayOfYear", forms.day_of_year.to_string(), true),
    ]);
    [
        ("unix", unix),
        ("formats", formats),
        ("landmarks", landmarks),
    ]
}

/// « 6 months, 22 days, 7 h 41 min ».
fn spell_calendar(calendar: &CalendarGap, negative: bool, language: Language) -> String {
    let dates = [
        ("years", calendar.years),
        ("months", calendar.months),
        ("days", calendar.days),
    ];
    let clock = [
        ("hours", calendar.hours),
        ("minutes", calendar.minutes),
        ("seconds", calendar.seconds),
    ];
    let mut parts: Vec<String> = dates
        .iter()
        .filter(|(_, count)| *count > 0)
        .map(|(unit, count)| counted(language, &format!("tools.dates.calendar.{unit}"), *count))
        .collect();
    let time: Vec<String> = clock
        .iter()
        .filter(|(_, value)| *value > 0)
        .map(|(unit, value)| {
            let path = format!("tools.dates.clock.{unit}");
            said(language, &path, &[("value", (*value).into())])
        })
        .collect();
    if !time.is_empty() {
        parts.push(time.join(" "));
    }
    if parts.is_empty() {
        return i18n::translate(language, keys::tools::dates::calendar::NONE, &[]);
    }
    let words = parts.join(", ");
    if negative { format!("-{words}") } else { words }
}

/// « 1 hour 30 minutes ».
fn spell_parts(parts: &DurationParts, negative: bool, language: Language) -> String {
    let units = [
        ("years", parts.years),
        ("months", parts.months),
        ("weeks", parts.weeks),
        ("days", parts.days),
        ("hours", parts.hours),
        ("minutes", parts.minutes),
        ("seconds", parts.seconds),
    ];
    let words: Vec<String> = units
        .iter()
        .filter(|(_, value)| *value != 0.0)
        .map(|(unit, value)| {
            let path = format!("tools.dates.units.{unit}");
            let name = said(language, &path, &[("count", (*value).into())]);
            format!("{} {name}", number(*value, language))
        })
        .collect();
    if words.is_empty() {
        return i18n::translate(language, keys::tools::dates::units::NONE, &[]);
    }
    let words = words.join(" ");
    if negative { format!("-{words}") } else { words }
}

/// Where a daylight-saving change parts the calendar's count from the clock's.
fn transition_note(transition: &Transition, total_minutes: u32, language: Language) -> String {
    let hours = |minutes: u32| {
        let shown = format!("{:02}", minutes % 60);
        said(
            language,
            "tools.dates.elapsed.hours",
            &[
                ("hours", whole(minutes / 60, language).into()),
                ("minutes", shown.into()),
            ],
        )
    };
    let size = transition.minutes.unsigned_abs();
    let difference = if size.is_multiple_of(60) {
        counted(language, "tools.dates.transition.hoursApart", size / 60)
    } else {
        said(
            language,
            "tools.dates.transition.minutesApart",
            &[("minutes", size.into())],
        )
    };
    let way = if transition.forward {
        "forward"
    } else {
        "backward"
    };
    let undated = if transition.date.is_some() {
        ""
    } else {
        "Undated"
    };
    let date = transition
        .date
        .as_deref()
        .map_or_else(String::new, |it| day_of_month(it, language));
    said(
        language,
        &format!("tools.dates.transition.{way}{undated}"),
        &[
            ("date", date.into()),
            ("wall", hours(transition.wall_minutes).into()),
            ("elapsed", hours(total_minutes).into()),
            ("difference", difference.into()),
        ],
    )
}

fn field_name(field: GapField, language: Language) -> String {
    named(language, "tools.dates.fields", &json!(field))
}

impl DatesTool {
    fn convert(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(keys::tools::dates::INPUT_LABEL);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let room = ui.available_width() - 90.0;
            ui.scope(|ui| {
                ui.set_max_width(room);
                let hint_words = tr(keys::tools::dates::PLACEHOLDER);
                TextField::new(&mut self.text, &name, "dates-input")
                    .hint(&hint_words)
                    .show(ui);
            });
            if Button::new(&tr(keys::tools::dates::NOW), "dates-now")
                .show(ui)
                .clicked()
            {
                self.wants_now = true;
            }
        });
        self.reading(ui, language);
        self.forms(ui, drawn.now, language);
    }

    fn reading(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::dates::magnitudes;
        let tr = |key| i18n::translate(language, key, &[]);
        let mut unix = self.magnitude.is_some();
        if let Some(InstantAnswer::Read {
            read_as,
            magnitude,
            guessed,
            local_assumed,
            ambiguous,
            ..
        }) = self.answer.value()
        {
            let read: Value = match read_as {
                ReadAs::Unix => json!(magnitude),
                other => json!(other),
            };
            unix |= *read_as == ReadAs::Unix;
            let shown = named(language, "tools.dates.readings", &read);
            let mut words = format!("{} {shown}", tr(keys::tools::dates::READ_AS));
            if *guessed {
                words = format!("{words} {}", tr(keys::tools::dates::GUESSED));
            }
            let said = hint(ui, &words);
            address(ui, &said, "dates-reading");
            if *local_assumed {
                let said = hint(ui, &tr(keys::tools::dates::LOCAL_ASSUMED));
                address(ui, &said, "dates-local-assumed");
            }
            if *ambiguous {
                let said = hint(ui, &tr(keys::tools::dates::AMBIGUOUS));
                address(ui, &said, "dates-ambiguous");
            }
        }
        if unix {
            let options = [
                (None, tr(magnitudes::AUTO)),
                (Some(Magnitude::Seconds), tr(magnitudes::SECONDS)),
                (Some(Magnitude::Milliseconds), tr(magnitudes::MILLISECONDS)),
                (Some(Magnitude::Microseconds), tr(magnitudes::MICROSECONDS)),
                (Some(Magnitude::Nanoseconds), tr(magnitudes::NANOSECONDS)),
            ];
            let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
            Segmented::new(&options, "dates-magnitude").show(ui, &mut self.magnitude);
        }
        let refused = match self.answer.value() {
            Some(InstantAnswer::Unreadable { at }) => Some(
                Message::new(keys::tools::dates::UNREADABLE)
                    .with("at", *at)
                    .text(language),
            ),
            Some(InstantAnswer::Skipped) => Some(tr(keys::tools::dates::SKIPPED)),
            Some(InstantAnswer::OutOfRange) => Some(tr(keys::tools::dates::OUT_OF_RANGE)),
            _ => None,
        };
        if let Some(refused) = refused {
            problem(ui, &refused, "dates-problem");
        }
    }

    fn forms(&self, ui: &mut Ui, now: DateTime<Utc>, language: Language) {
        let p = palette(ui.ctx());
        let Some(InstantAnswer::Read { forms, .. }) = self.answer.value() else {
            if self.answer.value().is_none() {
                hint(
                    ui,
                    &i18n::translate(language, keys::tools::dates::EMPTY, &[]),
                );
            }
            return;
        };
        let heading = long_date(forms.epoch_milliseconds, language)
            .unwrap_or_else(|| forms.iso_local.clone());
        let zone = Message::new(keys::tools::dates::LOCAL_ZONE)
            .with("offset", forms.local_offset.as_str())
            .text(language);
        let groups = groups_of(forms, now, language);
        card(ui, "dates-forms", |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                let said = paragraph(ui, &heading, (14.0, 600.0), p.text_0);
                address(ui, &said, "dates-heading");
                ui.add_space(12.0);
                paragraph(ui, &zone, (11.5, 400.0), p.text_2);
            });
            for (group, rows) in &groups {
                group_title(
                    ui,
                    &said(language, &format!("tools.dates.groups.{group}"), &[]),
                );
                let names: Vec<String> = rows
                    .iter()
                    .map(|(id, _, _)| said(language, &format!("tools.dates.rows.{id}"), &[]))
                    .collect();
                let shown: Vec<_> = rows
                    .iter()
                    .zip(&names)
                    .map(|((_, value, pasted), name)| {
                        let row = ResultRow::new(name, value);
                        if *pasted { row } else { row.reading() }
                    })
                    .collect();
                result_rows(ui, &shown, language);
            }
        });
    }

    fn gap(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(keys::tools::dates::GAP_TITLE));
        let placeholder = tr(keys::tools::dates::DATE_PLACEHOLDER);
        let from = tr(keys::tools::dates::fields::FROM);
        label(ui, &from);
        TextField::new(&mut self.from, &from, "dates-from")
            .hint(&placeholder)
            .show(ui);
        let to = tr(keys::tools::dates::fields::TO);
        label(ui, &to);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let room = ui.available_width() - 90.0;
            ui.scope(|ui| {
                ui.set_max_width(room);
                TextField::new(&mut self.to, &to, "dates-to")
                    .hint(&placeholder)
                    .show(ui);
            });
            if Button::new(&tr(keys::tools::dates::NOW), "dates-to-now")
                .show(ui)
                .clicked()
            {
                let now = Local.from_utc_datetime(&drawn.now.naive_utc());
                self.to = format!(
                    "{}-{:02}-{:02} {}",
                    now.year(),
                    now.month(),
                    now.day(),
                    now.format("%H:%M")
                );
            }
        });
        let Some(answer) = self.durations.value() else {
            return;
        };
        let said = hint(
            ui,
            &Message::new(keys::tools::dates::TYPED_IN)
                .with("zone", answer.zone.as_str())
                .text(language),
        );
        address(ui, &said, "dates-zone");
        match &answer.gap {
            Some(GapAnswer::Measured { .. }) => measured(ui, answer, language),
            Some(refused) => {
                let words = gap_problem(refused, language);
                problem(ui, &words, "dates-gap-problem");
            }
            None => {}
        }
    }

    fn duration(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(keys::tools::dates::DURATION_TITLE);
        section_title(ui, &name);
        TextField::new(&mut self.duration, &name, "dates-duration")
            .hint("PT1H30M")
            .show(ui);
        match self.durations.value().and_then(|it| it.duration.as_ref()) {
            Some(DurationAnswer::Read {
                negative,
                parts,
                iso,
                from_words,
                totals,
            }) => {
                let p = palette(ui.ctx());
                let words = spell_parts(parts, *negative, language);
                let totals_shown: Vec<(String, String)> = totals
                    .iter()
                    .flat_map(|it| {
                        [
                            ("seconds", it.seconds),
                            ("minutes", it.minutes),
                            ("hours", it.hours),
                        ]
                    })
                    .map(|(id, value)| {
                        let path = format!("tools.dates.totals.{id}");
                        (said(language, &path, &[]), number(value, language))
                    })
                    .collect();
                let iso_name = said(language, "tools.dates.totals.iso", &[]);
                card(ui, "dates-duration-read", |ui| {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        let shown = paragraph(ui, &words, (14.0, 600.0), p.text_0);
                        address(ui, &shown, "dates-duration-words");
                    });
                    ui.add_space(8.0);
                    let mut rows = Vec::new();
                    if *from_words {
                        rows.push(ResultRow::new(&iso_name, iso));
                    }
                    rows.extend(totals_shown.iter().map(|(n, v)| ResultRow::new(n, v)));
                    result_rows(ui, &rows, language);
                });
                if totals.is_none() {
                    let shown = hint(ui, &tr(keys::tools::dates::NO_TOTALS));
                    address(ui, &shown, "dates-no-totals");
                }
            }
            Some(DurationAnswer::Unreadable {
                at,
                problem: reason,
            }) => {
                let path = format!(
                    "tools.dates.durationProblems.{}",
                    json!(reason).as_str().unwrap_or_default()
                );
                let words = said(language, &path, &[("at", (*at).into())]);
                problem(ui, &words, "dates-duration-problem");
            }
            None => {}
        }
        hint(ui, &tr(keys::tools::dates::DURATION_HINT));
        let explained = format!(
            "P1M {} PT1M {} T {}",
            tr(keys::tools::dates::EXPLAIN_MONTH),
            tr(keys::tools::dates::EXPLAIN_MINUTE),
            tr(keys::tools::dates::EXPLAIN_T)
        );
        hint(ui, &explained);
    }

    fn durations_result(&self, language: Language) -> Option<ToolResult> {
        let answer = self.durations.value()?;
        let mut lines = Vec::new();
        if let Some(GapAnswer::Measured { iso, .. }) = &answer.gap {
            lines.push(format!("{} → {}", self.from.trim(), self.to.trim()));
            lines.push(iso.clone());
        }
        if let Some(DurationAnswer::Read { iso, totals, .. }) = &answer.duration {
            lines.push(
                totals
                    .as_ref()
                    .map_or_else(|| iso.clone(), |it| format!("{iso} = {} s", it.seconds)),
            );
        }
        (!lines.is_empty()).then(|| ToolResult {
            title: i18n::translate(language, keys::tools::dates::NOTE_DURATIONS, &[]),
            language: NoteLanguage::Txt,
            content: lines.join("\n"),
            warning: None,
        })
    }

    fn converted(&self, language: Language) -> Option<ToolResult> {
        let Some(InstantAnswer::Read { forms, .. }) = self.answer.value() else {
            return None;
        };
        let mut lines = vec![
            format!("Unix: {}", forms.unix_seconds),
            format!("Unix ms: {}", forms.unix_milliseconds),
            format!("ISO 8601: {}", forms.iso_utc),
            format!("ISO 8601: {}", forms.iso_local),
        ];
        if let Some(rfc) = &forms.rfc2822 {
            lines.push(format!("RFC 2822: {rfc}"));
        }
        Some(ToolResult {
            title: Message::new(keys::tools::dates::NOTE_TITLE)
                .with("instant", forms.iso_utc.as_str())
                .text(language),
            language: NoteLanguage::Txt,
            content: lines.join("\n"),
            warning: None,
        })
    }
}

fn gap_problem(answer: &GapAnswer, language: Language) -> String {
    let (kind, field, at) = match answer {
        GapAnswer::Unreadable { field, at } => ("unreadable", *field, *at),
        GapAnswer::Skipped { field } => ("skipped", *field, 0),
        GapAnswer::OutOfRange { field } => ("outOfRange", *field, 0),
        GapAnswer::Measured { .. } => return String::new(),
    };
    said(
        language,
        &format!("tools.dates.gapProblems.{kind}"),
        &[
            ("field", field_name(field, language).into()),
            ("at", at.into()),
        ],
    )
}

fn measured(ui: &mut Ui, answer: &DurationsAnswer, language: Language) {
    let Some(GapAnswer::Measured {
        negative,
        calendar,
        elapsed,
        iso,
        transition,
        ambiguous,
    }) = &answer.gap
    else {
        return;
    };
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let minutes = format!("{:02}", elapsed.minutes);
    let rows = [
        (
            "days",
            said(
                language,
                "tools.dates.elapsed.days",
                &[
                    ("days", whole(elapsed.days, language).into()),
                    ("hours", elapsed.hours.into()),
                    ("minutes", minutes.clone().into()),
                ],
            ),
        ),
        (
            "weeks",
            said(
                language,
                "tools.dates.elapsed.weeks",
                &[
                    ("weeks", whole(elapsed.weeks, language).into()),
                    ("days", elapsed.week_days.into()),
                ],
            ),
        ),
        (
            "hours",
            said(
                language,
                "tools.dates.elapsed.hours",
                &[
                    ("hours", whole(elapsed.total_hours, language).into()),
                    ("minutes", minutes.into()),
                ],
            ),
        ),
        ("minutes", whole(elapsed.total_minutes, language)),
        ("iso", iso.clone()),
    ];
    let names = rows
        .each_ref()
        .map(|(id, _)| said(language, &format!("tools.dates.elapsedRows.{id}"), &[]));
    let words = spell_calendar(calendar, *negative, language);
    card(ui, "dates-gap", |ui| {
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            let shown = paragraph(ui, &words, (14.0, 600.0), p.text_0);
            address(ui, &shown, "dates-gap-calendar");
        });
        group_title(ui, &tr(keys::tools::dates::ELAPSED_TITLE));
        let shown: Vec<_> = rows
            .iter()
            .zip(&names)
            .map(|((_, value), name)| ResultRow::new(name, value))
            .collect();
        result_rows(ui, &shown, language);
    });
    if *negative {
        let shown = hint(ui, &tr(keys::tools::dates::NEGATIVE));
        address(ui, &shown, "dates-gap-negative");
    }
    for field in ambiguous {
        let words = Message::new(keys::tools::dates::AMBIGUOUS_FIELD)
            .with("field", field_name(*field, language))
            .text(language);
        let shown = hint(ui, &words);
        address(ui, &shown, "dates-gap-ambiguous");
    }
    if let Some(transition) = transition {
        let words = transition_note(transition, elapsed.total_minutes, language);
        let shown = hint(ui, &words);
        address(ui, &shown, "dates-transition");
    }
}

impl Tool for DatesTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        if std::mem::take(&mut self.wants_now) {
            self.now = Some(api::tools::current_instant(turn.backend));
        }
        if let Some(now) = &mut self.now
            && let Some(answer) = now.poll()
        {
            self.now = None;
            if let Ok(now) = answer {
                self.magnitude = None;
                self.text = now;
            }
        }
        let request = (!self.text.trim().is_empty()).then(|| InstantRequest {
            text: self.text.clone(),
            magnitude: self.magnitude,
        });
        self.answer
            .update(turn, request, api::tools::describe_instant);
        let typed = [&self.from, &self.to, &self.duration];
        let request = typed
            .iter()
            .any(|it| !it.trim().is_empty())
            .then(|| DurationsRequest {
                from: self.from.clone(),
                to: self.to.clone(),
                duration: self.duration.clone(),
            });
        self.durations
            .update(turn, request, api::tools::measure_durations);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::dates::tabs;
        let tr = |key| i18n::translate(drawn.language, key, &[]);
        let options = [
            (Tab::Convert, tr(tabs::CONVERT)),
            (Tab::Durations, tr(tabs::DURATIONS)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        Segmented::new(&options, "dates-tab").show(ui, &mut self.tab);
        ui.add_space(12.0);
        match self.tab {
            Tab::Convert => {
                egui::ScrollArea::vertical()
                    .id_salt("dates-convert")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 10.0;
                        self.convert(ui, drawn);
                    });
            }
            Tab::Durations => columns(ui, self, drawn, (Self::gap, Self::duration)),
        }
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        match self.tab {
            Tab::Convert => self.converted(language),
            Tab::Durations => self.durations_result(language),
        }
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        self.magnitude = None;
        "1700000000".clone_into(&mut self.text);
        "2026-03-12 09:00".clone_into(&mut self.from);
        "2026-10-04 16:41".clone_into(&mut self.to);
        "PT1H30M".clone_into(&mut self.duration);
    }

    fn clear(&mut self) {
        self.text.clear();
        self.magnitude = None;
        self.from.clear();
        self.to.clear();
        self.duration.clear();
    }
}
