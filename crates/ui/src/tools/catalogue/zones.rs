//! A time placed in the zones of a list kept with the application.

use egui::{CornerRadius, FontFamily, Margin, Pos2, Role, Sense, Stroke, Ui, Vec2, WidgetInfo};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::zones::{ZoneEntry, ZoneReading, ZoneTime, ZonesAnswer, ZonesRequest};
use i18n::{Language, Message, keys};

use crate::api;
use crate::backend::Pending;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{CopyValue, check, columns, hint, problem, section_title};
use crate::tools::{Action, Drawn, Tool, ToolResult};
use crate::widgets::{self, Button, Kind, TextField, address, hovered, interact, towards};

pub(super) struct ZonesTool {
    text: String,
    from: Option<String>,
    later: bool,
    query: String,
    /// The application's list, as last given.
    zones: Vec<String>,
    answer: Live<ZonesRequest, ZonesAnswer>,
    found: Live<String, Vec<ZoneEntry>>,
    wants_now: bool,
    now: Option<Pending<String>>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(ZonesTool {
        text: String::new(),
        from: None,
        later: false,
        query: String::new(),
        zones: Vec::new(),
        answer: Live::default(),
        found: Live::default(),
        wants_now: false,
        now: None,
    })
}

fn reading_words(reading: &ZoneReading) -> String {
    let name = reading.abbreviation.as_deref().unwrap_or_default();
    format!("{name} UTC{} · {}", reading.offset, reading.iso_utc)
}

impl ZonesTool {
    fn input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(keys::tools::zones::INPUT_LABEL);
        section_title(ui, &name);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let room = ui.available_width() - 90.0;
            ui.scope(|ui| {
                ui.set_max_width(room);
                let words = tr(keys::tools::zones::PLACEHOLDER);
                let field = TextField::new(&mut self.text, &name, "zones-input").hint(&words);
                if field.show(ui).changed() {
                    self.later = false;
                }
            });
            if Button::new(&tr(keys::tools::zones::NOW), "zones-now")
                .show(ui)
                .clicked()
            {
                self.wants_now = true;
            }
        });
        self.reading(ui, language);
        ui.add_space(8.0);
        let name = tr(keys::tools::zones::ADD);
        section_title(ui, &name);
        TextField::new(&mut self.query, &name, "zones-search")
            .hint(&tr(keys::tools::zones::ADD_PLACEHOLDER))
            .show(ui);
        self.search_results(ui, drawn);
    }

    fn reading(&mut self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        match self.answer.value() {
            Some(ZonesAnswer::Placed {
                from,
                ambiguous,
                unknown,
                ..
            }) => {
                let mut words = format!(
                    "{}{from}{}",
                    tr(keys::tools::zones::TYPED_IN_BEFORE),
                    tr(keys::tools::zones::TYPED_IN_AFTER)
                );
                if ambiguous.is_none() {
                    words = format!("{words} {}", tr(keys::tools::zones::NOT_AMBIGUOUS));
                }
                let said = hint(ui, &words);
                address(ui, &said, "zones-from");
                let readings = ambiguous.clone();
                let from = from.clone();
                if !unknown.is_empty() {
                    let words = Message::new(keys::tools::zones::UNKNOWN)
                        .with("zones", unknown.join(", "))
                        .text(language);
                    let said = hint(ui, &words);
                    address(ui, &said, "zones-unknown");
                }
                if let Some(readings) = readings {
                    let words = Message::new(keys::tools::zones::AMBIGUOUS)
                        .with("zone", from)
                        .text(language);
                    hint(ui, &words);
                    for (at, reading) in readings.iter().enumerate() {
                        let mut on = self.later == (at == 1);
                        let id = format!("zones-reading-{at}");
                        if check(ui, &mut on, &reading_words(reading), &id).clicked() {
                            self.later = at == 1;
                        }
                    }
                }
            }
            Some(ZonesAnswer::Skipped {
                from,
                before,
                after,
            }) => {
                let words = Message::new(keys::tools::zones::SKIPPED)
                    .with("zone", from.as_str())
                    .with("before", before.as_str())
                    .with("after", after.as_str())
                    .text(language);
                problem(ui, &words, "zones-problem");
            }
            Some(ZonesAnswer::Unreadable { at }) => {
                let words = Message::new(keys::tools::zones::UNREADABLE)
                    .with("at", *at)
                    .text(language);
                problem(ui, &words, "zones-problem");
            }
            Some(ZonesAnswer::UnknownZone { zone }) => {
                let words = Message::new(keys::tools::zones::UNKNOWN_ZONE)
                    .with("zone", zone.as_str())
                    .text(language);
                problem(ui, &words, "zones-problem");
            }
            Some(ZonesAnswer::OutOfRange) => {
                problem(ui, &tr(keys::tools::zones::OUT_OF_RANGE), "zones-problem");
            }
            None => {}
        }
    }

    fn search_results(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        if self.query.trim().is_empty() {
            return;
        }
        let Some(found) = self.found.value() else {
            return;
        };
        if found.is_empty() {
            let words = Message::new(keys::tools::zones::NO_MATCH)
                .with("query", self.query.as_str())
                .text(drawn.language);
            let said = hint(ui, &words);
            address(ui, &said, "zones-no-match");
            return;
        }
        let mut added = None;
        for entry in found {
            if found_entry(ui, entry).clicked() {
                added = Some(entry.zone.clone());
            }
        }
        if let Some(zone) = added {
            if !self.zones.contains(&zone) {
                self.zones.push(zone);
                drawn.push(Action::SetZones(self.zones.clone()));
            }
            self.query.clear();
        }
    }

    fn times(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let Some(ZonesAnswer::Placed { times, .. }) = self.answer.value() else {
            hint(
                ui,
                &i18n::translate(language, keys::tools::zones::EMPTY, &[]),
            );
            return;
        };
        let mut here = None;
        let mut removed = None;
        for time in times {
            let removable = !time.local && !time.source && self.zones.contains(&time.zone);
            match time_row(ui, time, removable, language) {
                Some(RowAsked::Here) => here = Some(time.zone.clone()),
                Some(RowAsked::Remove) => removed = Some(time.zone.clone()),
                None => {}
            }
        }
        if here.is_some() {
            self.from = here;
            self.later = false;
        }
        if let Some(zone) = removed {
            self.zones.retain(|listed| *listed != zone);
            drawn.push(Action::SetZones(self.zones.clone()));
        }
    }
}

fn found_entry(ui: &mut Ui, entry: &ZoneEntry) -> egui::Response {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let corner = CornerRadius::same(7);
    let response = interact(
        ui,
        Vec2::new(width, 30.0),
        Sense::click(),
        &format!("zones-found-{}", entry.zone),
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
    let offset = format!(
        "UTC{} {}",
        entry.offset,
        entry.abbreviation.as_deref().unwrap_or_default()
    );
    let offset = widgets::text(ui, &offset, 11.5, 400.0, FontFamily::Monospace, p.text_2);
    let at = Pos2::new(
        rect.right() - 10.0 - offset.size().x,
        rect.center().y - offset.size().y / 2.0,
    );
    ui.painter().galley(at, offset, p.text_2);
    response
}

enum RowAsked {
    Here,
    Remove,
}

fn identity(time: &ZoneTime, language: Language) -> String {
    let mut words = format!("{} · UTC{}", time.zone, time.offset);
    if let Some(name) = &time.abbreviation {
        words = format!("{words} · {name}");
    }
    if time.summer_time {
        let summer = i18n::translate(language, keys::tools::zones::SUMMER, &[]);
        words = format!("{words} · {summer}");
    }
    words
}

fn city_words(time: &ZoneTime, language: Language) -> String {
    let tr = |key| i18n::translate(language, key, &[]);
    let mut words = time.city.clone();
    if time.local {
        words = format!("{words}  {}", tr(keys::tools::zones::LOCAL));
    }
    if time.source {
        words = format!("{words}  {}", tr(keys::tools::zones::SOURCE));
    }
    words
}

fn date_words(time: &ZoneTime, language: Language) -> String {
    if time.day_shift == 0 {
        return time.date.clone();
    }
    let key = if time.day_shift > 0 {
        keys::tools::zones::DAY_AHEAD
    } else {
        keys::tools::zones::DAY_BEHIND
    };
    let shift = Message::new(key)
        .with("count", time.day_shift.unsigned_abs())
        .text(language);
    format!("{} {shift}", time.date)
}

fn time_row(ui: &mut Ui, time: &ZoneTime, removable: bool, language: Language) -> Option<RowAsked> {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let mut asked = None;
    let edge = if time.source {
        p.amber_dim
    } else {
        p.line_soft
    };
    let shown = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, edge))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.set_width((ui.available_width() - 250.0).max(120.0));
                    let words = city_words(time, language);
                    crate::tools::ui::paragraph(ui, &words, (13.0, 600.0), p.text_0);
                    hint(ui, &identity(time, language));
                });
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let hour = crate::tools::ui::paragraph(ui, &time.time, (16.0, 600.0), p.text_0);
                    address(ui, &hour, "zones-time");
                    hint(ui, &date_words(time, language));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if removable
                        && Button::new("×", &format!("zones-remove-{}", time.zone))
                            .kind(Kind::Quiet)
                            .show(ui)
                            .clicked()
                    {
                        asked = Some(RowAsked::Remove);
                    }
                    if !time.source
                        && Button::new(
                            &tr(keys::tools::zones::HERE),
                            &format!("zones-type-in-{}", time.zone),
                        )
                        .show(ui)
                        .clicked()
                    {
                        asked = Some(RowAsked::Here);
                    }
                    CopyValue::new(&time.iso, &time.zone, language)
                        .compact()
                        .show(ui);
                });
            });
        });
    let response = shown.response;
    response.widget_info(|| WidgetInfo::labeled(Role::Row, true, &time.zone));
    address(ui, &response, "zones-row");
    asked
}

impl Tool for ZonesTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        self.zones.clone_from(&turn.zones.to_vec());
        if std::mem::take(&mut self.wants_now) {
            self.now = Some(api::tools::time_in_zone(turn.backend, self.from.clone()));
        }
        if let Some(now) = &mut self.now
            && let Some(answer) = now.poll()
        {
            self.now = None;
            if let Ok(now) = answer {
                self.later = false;
                self.text = now;
            }
        }
        let request = (!self.text.trim().is_empty()).then(|| ZonesRequest {
            text: self.text.clone(),
            from: self.from.clone(),
            zones: self.zones.clone(),
            later: self.later,
        });
        self.answer
            .update(turn, request, api::tools::place_in_zones);
        let query = (!self.query.trim().is_empty()).then(|| self.query.clone());
        self.found
            .update(turn, query, api::tools::search_time_zones);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::input, Self::times));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let Some(ZonesAnswer::Placed { times, .. }) = self.answer.value() else {
            return None;
        };
        let asked = self.answer.answered()?;
        if times.is_empty() {
            return None;
        }
        let lines: Vec<String> = times
            .iter()
            .map(|time| {
                let name = time.abbreviation.as_deref().unwrap_or_default();
                let line = format!(
                    "{:24}  {} {}  UTC{}  {name}",
                    time.zone, time.date, time.time, time.offset
                );
                line.trim_end().to_owned()
            })
            .collect();
        Some(ToolResult {
            title: Message::new(keys::tools::zones::NOTE_TITLE)
                .with("time", asked.text.trim())
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
        "2026-03-29 09:00".clone_into(&mut self.text);
        self.later = false;
    }

    fn clear(&mut self) {
        self.text.clear();
        self.later = false;
        self.query.clear();
    }
}
