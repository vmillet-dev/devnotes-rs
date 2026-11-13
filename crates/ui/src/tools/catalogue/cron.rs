//! Rust takes the expression apart and counts its runs; the sentence is built here, from keys.

mod sentence;

use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use egui::{CornerRadius, FontFamily, Margin, Pos2, Role, Sense, Stroke, Ui, Vec2, WidgetInfo};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::cron::{
    CronAnswer, CronCheck, CronRequest, CronWindow, FieldReading, Run, WindowEnd,
};
use engine::tools::zones::ZoneEntry;
use i18n::{Language, Message, keys};
use serde_json::json;

use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{columns, hint, label, paragraph, problem, section_title};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{self, Button, TextField, address, hovered, interact, towards};

const SHORTCUTS: [&str; 4] = ["today", "tomorrow", "weekend", "fridayAfternoon"];

#[derive(Default)]
pub(super) struct CronTool {
    expression: String,
    /// `None` is the machine's zone.
    zone: Option<String>,
    check: String,
    /// The two ends of the window the runs are listed in; empty is now, and the next ten.
    from: String,
    until: String,
    query: String,
    answer: Live<CronRequest, CronAnswer>,
    found: Live<String, Vec<ZoneEntry>>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::<CronTool>::default()
}

fn days_on(date: &str, days: i64) -> String {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .ok()
        .and_then(|it| it.checked_add_signed(TimeDelta::days(days)))
        .map_or_else(|| date.to_owned(), |it| it.format("%Y-%m-%d").to_string())
}

/// The two ends of a shortcut, as they are typed. `weekday` is 1 for Monday: on a Saturday or a
/// Sunday « this weekend » is the one under way, and « Friday » the next.
fn shortcut_window(shortcut: &str, now: &Run) -> (String, String) {
    let at = |days: i64, time: &str| format!("{} {time}", days_on(&now.date, days));
    let weekday = i64::from(now.weekday);
    match shortcut {
        "today" => (at(0, "00:00"), at(1, "00:00")),
        "tomorrow" => (at(1, "00:00"), at(2, "00:00")),
        "weekend" => {
            let saturday = 6 - weekday;
            (at(saturday, "00:00"), at(saturday + 2, "00:00"))
        }
        _ => {
            let friday = if weekday <= 5 {
                5 - weekday
            } else {
                12 - weekday
            };
            (at(friday, "12:00"), at(friday, "18:00"))
        }
    }
}

fn say(language: Language, path: &str) -> String {
    i18n::Key::find(path).map_or_else(String::new, |key| i18n::translate(language, key, &[]))
}

fn day_name(weekday: u32, language: Language) -> String {
    say(language, &format!("tools.cron.days.{}", weekday % 7))
}

fn run_row(ui: &mut Ui, run: &Run, now: DateTime<Utc>, language: Language) {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::hover());
    let at = format!("{} {}", run.date, run.time);
    response.widget_info(|| WidgetInfo::labeled(Role::Row, true, &at));
    address(ui, &response, "cron-run");
    ui.painter()
        .hline(rect.x_range(), rect.top() + 0.5, (1.0, p.line_soft));
    #[expect(clippy::cast_possible_truncation, reason = "milliseconds of a date")]
    let relative = DateTime::from_timestamp_millis(run.epoch_milliseconds as i64)
        .map(|it| i18n::time::span(it, now).text(language));
    let offset = format!("UTC{}", run.offset);
    let cells = [
        (
            0.0,
            day_name(run.weekday, language),
            FontFamily::Proportional,
            p.text_2,
        ),
        (0.16, at.clone(), FontFamily::Monospace, p.text_0),
        (0.5, offset, FontFamily::Monospace, p.text_2),
        (
            0.68,
            relative.unwrap_or_default(),
            FontFamily::Proportional,
            p.text_2,
        ),
    ];
    for (share, words, family, ink) in cells {
        let galley = widgets::text(ui, &words, 12.0, 400.0, family, ink);
        let left = rect.left() + 12.0 + (width - 24.0) * share;
        let spot = Pos2::new(left, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(spot, galley, ink);
    }
}

fn runs_card(ui: &mut Ui, draw: impl FnOnce(&mut Ui)) {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line_soft))
        .corner_radius(CornerRadius::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            draw(ui);
        });
}

/// What the expression's fields are, each under its name.
fn field_chips(ui: &mut Ui, fields: &[FieldReading], language: Language) {
    let p = palette(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(6.0);
        for field in fields {
            let name = named_field(field, language);
            let shown = egui::Frame::new()
                .fill(p.bg_2)
                .stroke(Stroke::new(1.0, p.line_soft))
                .corner_radius(CornerRadius::same(7))
                .inner_margin(Margin::symmetric(10, 5))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        let code = egui::RichText::new(&field.text)
                            .font(egui::FontId::monospace(13.0))
                            .color(p.text_0);
                        ui.label(code);
                        let under = egui::RichText::new(&name).size(10.5).color(p.text_2);
                        ui.label(under);
                    });
                });
            address(ui, &shown.response, "cron-field");
        }
    });
}

fn named_field(field: &FieldReading, language: Language) -> String {
    let id = json!(field.field);
    say(
        language,
        &format!("tools.cron.fields.{}.name", id.as_str().unwrap_or_default()),
    )
}

fn window_problem(window: &CronWindow, language: Language) -> Option<String> {
    use keys::tools::cron::window_problems as words;
    let end_id = |end: &WindowEnd| json!(end).as_str().unwrap_or_default().to_owned();
    Some(match window {
        CronWindow::Unreadable { end, at } => {
            let path = format!("tools.cron.windowProblems.unreadable.{}", end_id(end));
            i18n::Key::find(&path).map_or_else(String::new, |key| {
                Message::new(key).with("at", *at).text(language)
            })
        }
        CronWindow::Skipped { end } => say(
            language,
            &format!("tools.cron.windowProblems.skipped.{}", end_id(end)),
        ),
        CronWindow::TooLong { years } => Message::new(words::TOO_LONG)
            .with("years", *years)
            .text(language),
        CronWindow::Backwards => i18n::translate(language, words::BACKWARDS, &[]),
        CronWindow::Counted { .. } => return None,
    })
}

fn zone_entry(ui: &mut Ui, entry: &ZoneEntry) -> egui::Response {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let corner = CornerRadius::same(7);
    let response = interact(
        ui,
        Vec2::new(width, 30.0),
        Sense::click(),
        &format!("cron-zone-found-{}", entry.zone),
        || WidgetInfo::labeled(Role::Button, true, &entry.zone),
        corner,
    );
    let rect = response.rect;
    let fill = towards(egui::Color32::TRANSPARENT, p.bg_2, hovered(ui, &response));
    ui.painter().rect_filled(rect, corner, fill);
    let name = widgets::text(
        ui,
        &entry.zone,
        12.5,
        400.0,
        FontFamily::Proportional,
        p.text_0,
    );
    let at = Pos2::new(rect.left() + 10.0, rect.center().y - name.size().y / 2.0);
    ui.painter().galley(at, name, p.text_0);
    response
}

impl CronTool {
    fn input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(keys::tools::INPUT));
        let name = tr(keys::tools::cron::INPUT_LABEL);
        TextField::new(&mut self.expression, &name, "cron-input")
            .mono()
            .hint(&tr(keys::tools::cron::PLACEHOLDER))
            .show(ui);
        self.reading(ui, language);
        ui.add_space(8.0);
        self.zone_picker(ui, language);
    }

    fn reading(&self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        let p = palette(ui.ctx());
        match self.answer.value() {
            Some(CronAnswer::Read {
                expanded,
                fields,
                both_days,
                ..
            }) => {
                if let Some(expression) = expanded {
                    let words = Message::new(keys::tools::cron::EXPANDED)
                        .with("expression", expression.as_str())
                        .text(language);
                    let said = hint(ui, &words);
                    address(ui, &said, "cron-expanded");
                }
                field_chips(ui, fields, language);
                let words = sentence::of(fields, language);
                let said = paragraph(ui, &words, (13.5, 500.0), p.text_0);
                address(ui, &said, "cron-sentence");
                if *both_days {
                    let said = hint(ui, &tr(keys::tools::cron::EITHER_DAY));
                    address(ui, &said, "cron-either-day");
                }
            }
            Some(CronAnswer::Reboot) => {
                let said = hint(ui, &tr(keys::tools::cron::REBOOT));
                address(ui, &said, "cron-reboot");
            }
            Some(CronAnswer::Refused {
                field,
                token,
                at,
                problem: reason,
            }) => {
                let field_id = json!(field);
                let field_id = field_id.as_str().unwrap_or_default();
                let range = say(language, &format!("tools.cron.fields.{field_id}.range"));
                let reason_id = json!(reason);
                let path = format!(
                    "tools.cron.problems.{}",
                    reason_id.as_str().unwrap_or_default()
                );
                let message = i18n::Key::find(&path).map_or_else(String::new, |key| {
                    Message::new(key)
                        .with("token", token.as_str())
                        .with("range", range)
                        .text(language)
                });
                let place = Message::new(keys::tools::cron::REFUSED_AT)
                    .with("at", *at)
                    .text(language);
                let name = say(language, &format!("tools.cron.fields.{field_id}.name"));
                let head = if name.is_empty() {
                    place
                } else {
                    format!("{name} · {place}")
                };
                problem(ui, &format!("{head} {message}"), "cron-problem");
            }
            Some(CronAnswer::UnknownZone { zone }) => {
                let words = Message::new(keys::tools::cron::UNKNOWN_ZONE)
                    .with("zone", zone.as_str())
                    .text(language);
                problem(ui, &words, "cron-unknown-zone");
            }
            None => {}
        }
    }

    fn zone_picker(&mut self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(keys::tools::cron::ZONE);
        section_title(ui, &name);
        let shown = match self.answer.value() {
            Some(CronAnswer::Read { zone, .. }) => zone.clone(),
            _ => self
                .zone
                .clone()
                .unwrap_or_else(|| tr(keys::tools::cron::ZONE_LOCAL)),
        };
        let mut back = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let said = label(ui, &shown);
            address(ui, &said, "cron-zone");
            if self.zone.is_some() {
                let words = tr(keys::tools::cron::BACK_TO_LOCAL);
                back = Button::new(&words, "cron-zone-local").show(ui).clicked();
            } else {
                hint(ui, &tr(keys::tools::cron::ZONE_LOCAL));
            }
        });
        if back {
            self.zone = None;
        }
        TextField::new(&mut self.query, &name, "cron-zone-search")
            .hint(&tr(keys::tools::cron::ZONE_SEARCH))
            .show(ui);
        if self.query.trim().is_empty() {
            return;
        }
        let mut picked = None;
        for entry in self.found.value().into_iter().flatten() {
            if zone_entry(ui, entry).clicked() {
                picked = Some(entry.zone.clone());
            }
        }
        if picked.is_some() {
            self.zone = picked;
            self.query.clear();
        }
    }

    fn output(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(keys::tools::cron::RUNS));
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let half = (ui.available_width() - 10.0) / 2.0;
            for (value, key, hint_key, id) in [
                (
                    &mut self.from,
                    keys::tools::cron::FROM,
                    keys::tools::cron::FROM_PLACEHOLDER,
                    "cron-from",
                ),
                (
                    &mut self.until,
                    keys::tools::cron::UNTIL,
                    keys::tools::cron::UNTIL_PLACEHOLDER,
                    "cron-until",
                ),
            ] {
                ui.allocate_ui(Vec2::new(half, 0.0), |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(half);
                        let name = tr(key);
                        label(ui, &name);
                        TextField::new(value, &name, id)
                            .hint(&tr(hint_key))
                            .show(ui);
                    });
                });
            }
        });
        let picked = self.shortcuts(ui, language);
        if let Some((from, until)) = picked {
            self.from = from;
            self.until = until;
        }
        self.runs(ui, drawn.now, language);
        section_title(ui, &tr(keys::tools::cron::CHECK));
        let name = tr(keys::tools::cron::CHECK);
        TextField::new(&mut self.check, &name, "cron-check")
            .mono()
            .hint(&tr(keys::tools::cron::CHECK_PLACEHOLDER))
            .show(ui);
        self.verdict(ui, language);
    }

    fn shortcuts(&self, ui: &mut Ui, language: Language) -> Option<(String, String)> {
        let Some(CronAnswer::Read { now, .. }) = self.answer.value() else {
            return None;
        };
        let mut picked = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for shortcut in SHORTCUTS {
                let words = say(language, &format!("tools.cron.shortcut.{shortcut}"));
                let id = format!("cron-shortcut-{shortcut}");
                if Button::new(&words, &id).show(ui).clicked() {
                    picked = Some(shortcut_window(shortcut, now));
                }
            }
        });
        picked
    }

    fn runs(&self, ui: &mut Ui, now: DateTime<Utc>, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        let Some(CronAnswer::Read { runs, window, .. }) = self.answer.value() else {
            return;
        };
        if let Some(words) = window.as_ref().and_then(|it| window_problem(it, language)) {
            let said = hint(ui, &words);
            address(ui, &said, "cron-window-problem");
        }
        let counted = match window {
            Some(CronWindow::Counted {
                from,
                until,
                total,
                first,
                last,
            }) => Some((from, until, *total, first, last)),
            _ => None,
        };
        if let Some((from, until, total, _, _)) = counted {
            let said = |end: &Run| {
                let time: String = end.time.chars().take(5).collect();
                format!("{} {} {time}", day_name(end.weekday, language), end.date)
            };
            let words = if total == 0 {
                tr(keys::tools::cron::WINDOW_NONE)
            } else {
                Message::new(keys::tools::cron::WINDOW_COUNT)
                    .with("count", total)
                    .with("from", said(from))
                    .with("until", said(until))
                    .text(language)
            };
            let shown = hint(ui, &words);
            address(ui, &shown, "cron-window-count");
            if total == 0 {
                return;
            }
        }
        let (first, last, between) = match counted {
            Some((_, _, total, first, last)) => {
                let listed = u32::try_from(first.len() + last.len()).unwrap_or(total);
                let between = if last.is_empty() {
                    0
                } else {
                    total.saturating_sub(listed)
                };
                (first.as_slice(), last.as_slice(), between)
            }
            None => (runs.as_slice(), [].as_slice(), 0),
        };
        if first.is_empty() {
            let said = hint(ui, &tr(keys::tools::cron::NO_RUNS));
            address(ui, &said, "cron-no-runs");
            return;
        }
        runs_card(ui, |ui| {
            for run in first {
                run_row(ui, run, now, language);
            }
            if between > 0 {
                let words = Message::new(keys::tools::cron::WINDOW_GAP)
                    .with("count", between)
                    .text(language);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    let said = hint(ui, &words);
                    address(ui, &said, "cron-window-gap");
                });
                ui.add_space(4.0);
            }
            for run in last {
                run_row(ui, run, now, language);
            }
        });
    }

    fn verdict(&self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        let Some(CronAnswer::Read {
            check: Some(check), ..
        }) = self.answer.value()
        else {
            return;
        };
        let p = palette(ui.ctx());
        let at_message = |key, at: &str| Message::new(key).with("at", at).text(language);
        match check {
            CronCheck::Checked {
                matches,
                at,
                before,
                after,
            } => {
                let (key, ink) = if *matches {
                    (keys::tools::cron::MATCHES, p.green)
                } else {
                    (keys::tools::cron::NOT_MATCHES, p.text_1)
                };
                let said = paragraph(ui, &at_message(key, at), (12.5, 500.0), ink);
                address(ui, &said, "cron-verdict");
                if *matches {
                    return;
                }
                // A « no » leaves one asking when, then: the nearest run on either side.
                let at_of = |run: &Run| format!("{} {}", run.date, run.time);
                if let Some(run) = before {
                    let words = at_message(keys::tools::cron::NEAREST_BEFORE, &at_of(run));
                    let said = hint(ui, &words);
                    address(ui, &said, "cron-nearest-before");
                }
                if let Some(run) = after {
                    let words = at_message(keys::tools::cron::NEAREST_AFTER, &at_of(run));
                    let said = hint(ui, &words);
                    address(ui, &said, "cron-nearest-after");
                }
            }
            CronCheck::Unreadable { at } => {
                let words = Message::new(keys::tools::cron::CHECK_UNREADABLE)
                    .with("at", *at)
                    .text(language);
                let said = hint(ui, &words);
                address(ui, &said, "cron-verdict");
            }
            CronCheck::Skipped => {
                let said = hint(ui, &tr(keys::tools::cron::CHECK_SKIPPED));
                address(ui, &said, "cron-verdict");
            }
        }
    }
}

impl Tool for CronTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = (!self.expression.trim().is_empty()).then(|| CronRequest {
            expression: self.expression.clone(),
            zone: self.zone.clone(),
            check: self.check.clone(),
            from: self.from.clone(),
            until: self.until.clone(),
        });
        self.answer.update(turn, request, api::tools::describe_cron);
        let query = (!self.query.trim().is_empty()).then(|| self.query.clone());
        self.found
            .update(turn, query, api::tools::search_time_zones);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::input, Self::output));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let Some(CronAnswer::Read {
            fields, zone, runs, ..
        }) = self.answer.value()
        else {
            return None;
        };
        let expression = self.answer.answered()?.expression.trim().to_owned();
        let mut lines = vec![
            expression.clone(),
            format!("{}.", sentence::of(fields, language)),
        ];
        if !runs.is_empty() {
            lines.push(String::new());
            lines.push(
                Message::new(keys::tools::cron::NOTE_RUNS)
                    .with("zone", zone.as_str())
                    .text(language),
            );
            lines.extend(runs.iter().map(|run| format!("{} {}", run.date, run.time)));
        }
        Some(ToolResult {
            title: Message::new(keys::tools::cron::NOTE_TITLE)
                .with("expression", expression)
                .text(language),
            language: NoteLanguage::Txt,
            content: lines.join("\n"),
            warning: None,
        })
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        "*/15 9-18 * * 1-5".clone_into(&mut self.expression);
    }

    fn clear(&mut self) {
        self.expression.clear();
        self.check.clear();
        self.from.clear();
        self.until.clear();
        self.query.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(date: &str, weekday: u32) -> Run {
        Run {
            date: date.to_owned(),
            time: "10:00:00".to_owned(),
            weekday,
            offset: "+00:00".to_owned(),
            epoch_milliseconds: 0.0,
        }
    }

    #[test]
    fn friday_afternoon_is_the_next_one_from_a_saturday() {
        let saturday = run("2026-10-17", 6);

        let (from, until) = shortcut_window("fridayAfternoon", &saturday);

        assert_eq!(from, "2026-10-23 12:00");
        assert_eq!(until, "2026-10-23 18:00");
    }

    #[test]
    fn this_weekend_on_a_wednesday_is_the_next_one() {
        let wednesday = run("2026-10-14", 3);

        let (from, until) = shortcut_window("weekend", &wednesday);

        assert_eq!(from, "2026-10-17 00:00");
        assert_eq!(until, "2026-10-19 00:00");
    }

    #[test]
    fn this_weekend_on_a_sunday_is_the_one_under_way_counted_from_its_saturday() {
        let sunday = run("2026-10-18", 7);

        let (from, until) = shortcut_window("weekend", &sunday);

        assert_eq!(from, "2026-10-17 00:00");
        assert_eq!(until, "2026-10-19 00:00");
    }
}
