//! A day picked from a month drawn as a grid (plan § 3.9, gap E4): the reference kept the
//! system's `<input type="date">`, which egui has no equivalent of. Arrows move the day,
//! Enter picks it, Delete clears it.

use chrono::{Datelike, Days, Months, NaiveDate};
use egui::{
    Color32, CornerRadius, FontFamily, Key, Modifiers, Popup, PopupCloseBehavior, Response, Role,
    Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
};
use i18n::{Language, keys};

use super::{hovered, interact, menu, text, towards};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;

/// What the picker answered this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picked {
    Day(NaiveDate),
    Cleared,
}

/// The control naming the day, or its hint, and the month it opens on.
pub struct DateField<'a> {
    value: Option<NaiveDate>,
    today: NaiveDate,
    name: &'a str,
    author_id: &'a str,
    language: Language,
}

impl<'a> DateField<'a> {
    pub fn new(
        value: Option<NaiveDate>,
        today: NaiveDate,
        name: &'a str,
        author_id: &'a str,
    ) -> Self {
        Self {
            value,
            today,
            name,
            author_id,
            language: Language::En,
        }
    }

    #[must_use]
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<Picked> {
        let p = palette(ui.ctx());
        let shown = self.value.map_or_else(
            || i18n::translate(self.language, keys::editor::date::EMPTY, &[]),
            |day| i18n::format::date_numeric(day, self.language),
        );
        let ink = if self.value.is_some() {
            p.text_1
        } else {
            p.text_2
        };
        let probe = text(ui, &shown, 11.5, 400.0, FontFamily::Monospace, ink);
        let corner = CornerRadius::same(6);
        let name = self.name;
        let response = interact(
            ui,
            Vec2::new(probe.size().x + 34.0, probe.size().y + 10.0),
            Sense::click(),
            self.author_id,
            || WidgetInfo::labeled(Role::Button, true, name),
            corner,
        );
        let edge = towards(p.line, p.amber_dim, hovered(ui, &response));
        ui.painter().rect(
            response.rect,
            corner,
            p.bg_1,
            Stroke::new(1.0, edge),
            StrokeKind::Inside,
        );
        let at = egui::pos2(
            response.rect.left() + 8.0,
            response.rect.center().y - probe.size().y / 2.0,
        );
        ui.painter().galley(at, probe, ink);
        let glyph = egui::Rect::from_center_size(
            egui::pos2(response.rect.right() - 13.0, response.rect.center().y),
            Vec2::splat(12.0),
        );
        icons::paint(ui, glyph, Icon::Calendar, p.text_2);
        if takes_delete(ui, &response) {
            return Some(Picked::Cleared);
        }
        let cursor_id = response.id.with("cursor");
        if response.clicked() {
            let start = self.value.unwrap_or(self.today);
            ui.data_mut(|data| data.insert_temp(cursor_id, start));
        }
        Popup::menu(&response)
            .frame(menu::frame(ui.ctx()))
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                let cursor = ui
                    .data(|data| data.get_temp(cursor_id))
                    .unwrap_or(self.value.unwrap_or(self.today));
                let (cursor, picked) = month(ui, cursor, &self);
                ui.data_mut(|data| data.insert_temp(cursor_id, cursor));
                if picked.is_some() {
                    Popup::close_id(ui.ctx(), Popup::default_response_id(&response));
                }
                picked
            })
            .and_then(|shown| shown.inner)
    }
}

/// Delete or Backspace on the closed control: the day goes, the note becomes permanent again.
fn takes_delete(ui: &Ui, response: &Response) -> bool {
    super::takes_keys(ui, response)
        && ui.input_mut(|input| {
            input.consume_key(Modifiers::NONE, Key::Delete)
                || input.consume_key(Modifiers::NONE, Key::Backspace)
        })
}

/// The month around `cursor`, from Monday; the day the arrows moved to, and the one picked.
fn month(ui: &mut Ui, cursor: NaiveDate, field: &DateField<'_>) -> (NaiveDate, Option<Picked>) {
    let p = palette(ui.ctx());
    let language = field.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let mut cursor = cursor;
    let mut picked = None;
    ui.spacing_mut().item_spacing = Vec2::splat(2.0);
    ui.horizontal(|ui| {
        let previous = tr(keys::editor::date::PREVIOUS);
        if super::IconButton::new(Icon::ArrowLeft, &previous, "date-previous")
            .show(ui)
            .clicked()
        {
            cursor = cursor - Months::new(1);
        }
        let title = i18n::translate(
            language,
            keys::editor::date::MONTH,
            &[
                ("month", month_name(cursor.month(), language).into()),
                ("year", cursor.year().to_string().into()),
            ],
        );
        let galley = text(ui, &title, 12.5, 600.0, FontFamily::Proportional, p.text_0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(150.0, 24.0), Sense::hover());
        ui.painter()
            .galley(rect.center() - galley.size() / 2.0, galley, p.text_0);
        let next = tr(keys::editor::date::NEXT);
        if super::IconButton::new(Icon::ArrowRight, &next, "date-next")
            .show(ui)
            .clicked()
        {
            cursor = cursor + Months::new(1);
        }
    });
    let first = cursor.with_day(1).unwrap_or(cursor);
    let lead = u64::from(first.weekday().num_days_from_monday());
    let start = first - Days::new(lead);
    egui::Grid::new("date-grid")
        .spacing(Vec2::splat(2.0))
        .show(ui, |ui| {
            for weekday in 1..=7 {
                let initial = weekday_initial(weekday, language);
                let galley = text(
                    ui,
                    &initial,
                    10.5,
                    500.0,
                    FontFamily::Proportional,
                    p.text_2,
                );
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                ui.painter()
                    .galley(rect.center() - galley.size() / 2.0, galley, p.text_2);
            }
            ui.end_row();
            for week in 0..6 {
                for weekday in 0..7 {
                    let day = start + Days::new(week * 7 + weekday);
                    if day_cell(ui, day, cursor, field).clicked() {
                        picked = Some(Picked::Day(day));
                    }
                }
                ui.end_row();
            }
        });
    let clear = tr(keys::editor::date::CLEAR);
    if super::Button::new(&clear, "date-clear").show(ui).clicked() {
        picked = Some(Picked::Cleared);
    }
    ui.input_mut(|input| {
        let moves = [
            (Key::ArrowLeft, -1),
            (Key::ArrowRight, 1),
            (Key::ArrowUp, -7),
            (Key::ArrowDown, 7),
        ];
        for (key, by) in moves {
            if input.consume_key(Modifiers::NONE, key) {
                cursor = shifted(cursor, by);
            }
        }
        if input.consume_key(Modifiers::NONE, Key::Enter) {
            picked = Some(Picked::Day(cursor));
        }
    });
    (cursor, picked)
}

fn day_cell(ui: &mut Ui, day: NaiveDate, cursor: NaiveDate, field: &DateField<'_>) -> Response {
    let p = palette(ui.ctx());
    let label = i18n::format::date_long(day, field.language);
    let corner = CornerRadius::same(6);
    let selected = field.value == Some(day);
    let response = interact(
        ui,
        Vec2::splat(28.0),
        Sense::click(),
        "date-day",
        || WidgetInfo::selected(Role::Button, true, selected, &label),
        corner,
    );
    let outside = day.month() != cursor.month();
    let fill = if selected {
        p.amber_fill
    } else if day == cursor {
        p.bg_3
    } else {
        p.bg_3.gamma_multiply(hovered(ui, &response))
    };
    ui.painter().rect_filled(response.rect, corner, fill);
    if day == field.today {
        ui.painter().rect_stroke(
            response.rect,
            corner,
            Stroke::new(1.0, p.amber_dim),
            StrokeKind::Inside,
        );
    }
    let ink = if selected {
        p.amber_text
    } else if outside {
        p.text_2
    } else {
        p.text_1
    };
    let galley = text(
        ui,
        &day.day().to_string(),
        11.5,
        400.0,
        FontFamily::Proportional,
        ink,
    );
    ui.painter().galley(
        response.rect.center() - galley.size() / 2.0,
        galley,
        Color32::TRANSPARENT,
    );
    response
}

fn shifted(day: NaiveDate, by: i64) -> NaiveDate {
    let days = Days::new(by.unsigned_abs());
    if by < 0 { day - days } else { day + days }
}

fn month_name(month: u32, language: Language) -> String {
    i18n::Key::find(&format!("tools.cron.months.{month}"))
        .map(|key| i18n::translate(language, key, &[]))
        .unwrap_or_default()
}

/// « L », « M »… : the first letter of the weekday's name, Monday first.
fn weekday_initial(weekday: u32, language: Language) -> String {
    i18n::Key::find(&format!("tools.dates.weekdays.{weekday}"))
        .map(|key| i18n::translate(language, key, &[]))
        .and_then(|name| name.chars().next())
        .map(String::from)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_month_and_weekday_has_a_name_in_both_languages() {
        for language in [Language::En, Language::Fr] {
            for month in 1..=12 {
                assert!(!month_name(month, language).is_empty(), "{month}");
            }
            for weekday in 1..=7 {
                assert!(!weekday_initial(weekday, language).is_empty(), "{weekday}");
            }
        }
    }

    #[test]
    fn the_arrows_move_by_a_day_and_by_a_week() {
        let day = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        assert_eq!(
            shifted(day, -1),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
        assert_eq!(
            shifted(day, 7),
            NaiveDate::from_ymd_opt(2026, 3, 8).unwrap()
        );
    }
}
