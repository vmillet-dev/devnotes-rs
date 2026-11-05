//! What was sent, by day, beside the entry chosen, read like an answer.

use chrono::{Days, Local, NaiveDate};
use egui::{
    Align, CornerRadius, FontFamily, Key, Layout, Modifiers, Rect, Role, Sense, Stroke, Ui,
    UiBuilder, Vec2, WidgetInfo, pos2,
};
use engine::error::AppError;
use engine::http::history::{HistoryDay, HistoryEntry, HistoryItem};
use i18n::keys;

use super::{Action, badge_hue, response};
use crate::action::{Action as Asked, Actions, AppAction};
use crate::notes::ui::folder_hue;
use crate::services::settings::HISTORY_LIST_WIDTH;
use crate::state::AppState;
use crate::state::http::history::Asked as Ask;
use crate::state::http::tree::Badge;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, IconButton, Kind, RailWidth, interact, virtual_list};

const HEAD: f32 = 56.0;
const ROW: f32 = 48.0;

/// What only the page's drawing sees.
#[derive(Default)]
pub(crate) struct PageView {
    list_width: RailWidth,
    response: response::ResponseView,
}

fn ask(actions: &mut Actions, asked: Ask) {
    actions.push(Asked::Http(Action::History(asked)));
}

enum Row<'a> {
    Day(&'a HistoryDay),
    Item(&'a HistoryItem),
}

fn rows(days: &[HistoryDay]) -> Vec<Row<'_>> {
    let mut rows = Vec::new();
    for day in days {
        rows.push(Row::Day(day));
        rows.extend(day.items.iter().map(Row::Item));
    }
    rows
}

pub(super) fn show(
    ui: &mut Ui,
    region: Rect,
    state: &AppState,
    view: &mut PageView,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    ui.painter().rect_filled(region, CornerRadius::ZERO, p.bg_0);
    let head_rect = Rect::from_min_size(region.min, Vec2::new(region.width(), HEAD));
    let mut top = ui.new_child(UiBuilder::new().max_rect(head_rect.shrink2(Vec2::new(20.0, 0.0))));
    head(&mut top, state, actions);
    ui.painter().hline(
        region.x_range(),
        head_rect.bottom() - 0.5,
        Stroke::new(1.0, p.line_soft),
    );
    let body = Rect::from_min_max(pos2(region.left(), head_rect.bottom()), region.max);
    let setting = (state.settings.history_list_width, HISTORY_LIST_WIDTH);
    let rest = view
        .list_width
        .show(ui, body, setting, "http-history-edge", |ui| {
            list(ui, state, actions);
        });
    if let Some(width) = view
        .list_width
        .rested(ui, state.settings.history_list_width)
    {
        actions.push(Asked::App(AppAction::SetHistoryWidth(width)));
    }
    let mut detail_ui = ui.new_child(UiBuilder::new().max_rect(rest));
    detail(&mut detail_ui, state, &mut view.response, actions);
}

fn head(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let history = state.http.history();
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let toggle = tr(keys::sidebar::TOGGLE);
        if widgets::SquareButton::new(Icon::Sidebar, &toggle, "http-rail-toggle")
            .pressed(state.settings.rail_shown)
            .show(ui)
            .clicked()
        {
            actions.push(Asked::App(AppAction::ToggleRail));
        }
        let title = tr(keys::http::history::TITLE);
        let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
        let (rect, heading) = ui.allocate_exact_size(words.size(), Sense::hover());
        ui.painter().galley(rect.min, words, p.text_0);
        heading.widget_info(|| WidgetInfo::labeled(Role::Heading, true, &title));
        widgets::address(ui, &heading, "http-history");
        let kept = egui::RichText::new(tr(keys::http::history::KEPT))
            .size(12.0)
            .color(p.text_2);
        ui.add(egui::Label::new(kept));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let word = tr(keys::http::history::CLEAR);
            let Some(count) = history.confirming() else {
                if Button::new(&word, "http-history-clear")
                    .enabled(!history.days().is_empty())
                    .show(ui)
                    .clicked()
                {
                    ask(actions, Ask::AskClear);
                }
                return;
            };
            if Button::new(&tr(keys::http::rail::CANCEL), "http-history-clear-cancel")
                .show(ui)
                .clicked()
            {
                ask(actions, Ask::CancelClear);
            }
            if Button::new(&word, "http-history-clear-confirm")
                .kind(Kind::Destructive)
                .show(ui)
                .clicked()
            {
                ask(actions, Ask::Clear);
            }
            let question = i18n::translate(
                language,
                keys::http::history::CONFIRM,
                &[("count", count.into())],
            );
            let question = egui::RichText::new(question).size(12.5).color(p.text_1);
            let said = ui.add(egui::Label::new(question));
            widgets::address(ui, &said, "http-history-clear-count");
        });
    });
}

/// Today, yesterday, or the day as Rust cut it.
fn day_label(day: &str, today: NaiveDate, language: i18n::Language) -> String {
    let yesterday = today.checked_sub_days(Days::new(1));
    match NaiveDate::parse_from_str(day, "%Y-%m-%d") {
        Ok(date) if date == today => i18n::translate(language, keys::http::history::TODAY, &[]),
        Ok(date) if Some(date) == yesterday => {
            i18n::translate(language, keys::http::history::YESTERDAY, &[])
        }
        _ => day.to_owned(),
    }
}

fn list(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let history = state.http.history();
    let today = state.clock.now().with_timezone(&Local).date_naive();
    let rows = rows(history.days());
    ui.spacing_mut().item_spacing.y = 0.0;
    let bottom = if history.removed().is_some() {
        44.0
    } else {
        0.0
    };
    let room = Vec2::new(ui.available_width(), ui.available_height() - bottom);
    ui.allocate_ui(room, |ui| {
        ui.set_min_height(room.y);
        if rows.is_empty() && history.is_loaded() {
            let words = i18n::translate(language, keys::http::history::EMPTY, &[]);
            let words = egui::RichText::new(words).size(12.5).color(p.text_2);
            let said = ui.add(egui::Label::new(words).wrap());
            widgets::address(ui, &said, "http-history-empty");
            return;
        }
        virtual_list(
            ui,
            "http-history-list",
            ROW,
            rows.len(),
            |ui, at| match rows[at] {
                Row::Day(day) => day_row(ui, &day_label(&day.day, today, language)),
                Row::Item(item) => item_row(ui, state, item, actions),
            },
        );
    });
    if let Some(removed) = history.removed() {
        undo(ui, removed, language, actions);
    }
}

fn day_row(ui: &mut Ui, label: &str) {
    let p = palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW), Sense::hover());
    let mut over = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(Vec2::new(12.0, 8.0))));
    over.with_layout(Layout::left_to_right(Align::Max), |ui| {
        let words = egui::RichText::new(label)
            .size(12.0)
            .strong()
            .color(p.text_2);
        let said = ui.add(egui::Label::new(words));
        widgets::address(ui, &said, "http-history-day");
    });
}

fn status_hue(ui: &Ui, status: Option<u16>) -> egui::Color32 {
    let p = palette(ui.ctx());
    match status.map(|status| status / 100) {
        Some(2) => p.green,
        Some(3) => p.blue,
        Some(4) => p.amber_text,
        _ => p.red,
    }
}

fn item_row(ui: &mut Ui, state: &AppState, item: &HistoryItem, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let on = state.http.history().selected() == Some(item.id.as_str());
    let size = Vec2::new(ui.available_width(), ROW);
    let name = &item.summary.name;
    let response = interact(
        ui,
        size,
        Sense::click(),
        "http-history-item",
        || WidgetInfo::selected(Role::Button, true, on, name),
        CornerRadius::same(8),
    );
    let rect = response.rect;
    if on {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), p.bg_2);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), p.bg_1);
    }
    let method = item.method.as_str();
    let hue = badge_hue(ui, Badge::Method(item.method));
    let badge = widgets::text(ui, method, 11.0, 700.0, FontFamily::Monospace, hue);
    ui.painter()
        .galley(pos2(rect.left() + 10.0, rect.top() + 9.0), badge, hue);
    let right = rect.right() - 36.0;
    let status = item.status.map_or_else(
        || i18n::translate(language, keys::http::history::NO_ANSWER, &[]),
        |status| status.to_string(),
    );
    let ink = status_hue(ui, item.status);
    let galley = widgets::text(ui, &status, 12.0, 600.0, FontFamily::Monospace, ink);
    let at = pos2(right - galley.size().x, rect.top() + 8.0);
    let mut over = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(at, galley.size())));
    let said = over.add(egui::Label::new(galley).selectable(false));
    widgets::address(ui, &said, "http-history-status");
    let time = item
        .sent_at
        .with_timezone(&Local)
        .format("%H:%M:%S")
        .to_string();
    let galley = widgets::text(ui, &time, 11.0, 400.0, FontFamily::Proportional, p.text_2);
    let at = pos2(right - galley.size().x, rect.top() + 27.0);
    ui.painter().galley(at, galley, p.text_2);
    let text_left = rect.left() + 64.0;
    let clip = Rect::from_min_max(
        pos2(text_left, rect.top()),
        pos2(right - 64.0, rect.bottom()),
    );
    let name_ink = widgets::text(ui, name, 13.0, 500.0, FontFamily::Proportional, p.text_0);
    let width = name_ink.size().x;
    ui.painter_at(clip)
        .galley(pos2(text_left, rect.top() + 7.0), name_ink, p.text_0);
    if let Some(environment) = &item.summary.environment {
        let hue = folder_hue(ui, environment.colour);
        let dot = pos2(text_left + width + 12.0, rect.top() + 15.0);
        ui.painter_at(clip).circle_filled(dot, 3.5, hue);
        let words = widgets::text(
            ui,
            &environment.name,
            11.0,
            400.0,
            FontFamily::Proportional,
            p.text_2,
        );
        let at = pos2(dot.x + 8.0, rect.top() + 9.0);
        let size = words.size();
        let mut over =
            ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(at, size).intersect(clip)));
        let said = over.add(egui::Label::new(words).selectable(false).truncate());
        widgets::address(ui, &said, "http-history-environment");
    }
    url(ui, clip, item);
    if response.clicked() {
        response.request_focus();
        ask(actions, Ask::Select(item.id.clone()));
    }
    if response.has_focus() && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Delete))
    {
        ask(actions, Ask::Delete(item.id.clone()));
    }
    remove(ui, rect, item, language, actions);
}

/// Cut in what repeats, not at its end: the host gives way, the path and the query stay.
fn url(ui: &Ui, clip: Rect, item: &HistoryItem) {
    let p = palette(ui.ctx());
    let tail = widgets::text(
        ui,
        &item.url_tail,
        11.5,
        400.0,
        FontFamily::Monospace,
        p.text_1,
    );
    let head = widgets::text(
        ui,
        &item.url_head,
        11.5,
        400.0,
        FontFamily::Monospace,
        p.text_2,
    );
    let top = clip.top() + 27.0;
    let room = clip.width();
    let tail_width = tail.size().x.min(room);
    let head_room = (room - tail_width).max(0.0);
    let head_clip = Rect::from_min_size(clip.min, Vec2::new(head_room, clip.height()));
    ui.painter_at(head_clip)
        .galley(pos2(clip.left(), top), head.clone(), p.text_2);
    let at = clip.left() + head.size().x.min(head_room);
    ui.painter_at(clip).galley(pos2(at, top), tail, p.text_1);
}

fn remove(
    ui: &mut Ui,
    rect: Rect,
    item: &HistoryItem,
    language: i18n::Language,
    actions: &mut Actions,
) {
    let label = i18n::translate(
        language,
        keys::http::history::DELETE,
        &[("name", item.summary.name.as_str().into())],
    );
    let at = Rect::from_center_size(
        pos2(rect.right() - 18.0, rect.center().y),
        Vec2::splat(widgets::MIN_TARGET),
    );
    let mut over = ui.new_child(UiBuilder::new().max_rect(at));
    if IconButton::new(Icon::X, &label, "http-history-delete")
        .show(&mut over)
        .clicked()
    {
        ask(actions, Ask::Delete(item.id.clone()));
    }
}

fn undo(ui: &mut Ui, removed: &HistoryEntry, language: i18n::Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let words = i18n::translate(
            language,
            keys::http::history::DELETED,
            &[("name", removed.item.summary.name.as_str().into())],
        );
        let words = egui::RichText::new(words).size(12.0).color(p.text_1);
        let said = ui.add(egui::Label::new(words).truncate());
        widgets::address(ui, &said, "http-history-undo");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if IconButton::new(
                Icon::X,
                &tr(keys::undo::DISMISS),
                "http-history-undo-dismiss",
            )
            .show(ui)
            .clicked()
            {
                ask(actions, Ask::DismissUndo);
            }
            if Button::new(&tr(keys::undo::RESTORE), "http-history-undo-restore")
                .show(ui)
                .clicked()
            {
                ask(actions, Ask::Undo);
            }
        });
    });
}

fn detail(ui: &mut Ui, state: &AppState, view: &mut response::ResponseView, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let history = state.http.history();
    let Some(entry) = history.entry() else {
        let words = i18n::translate(language, keys::http::history::CHOOSE, &[]);
        let words = egui::RichText::new(words).size(12.5).color(p.text_2);
        egui::Frame::new()
            .inner_margin(egui::Margin::same(20))
            .show(ui, |ui| ui.add(egui::Label::new(words)));
        return;
    };
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(20, 12))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.spacing_mut().item_spacing.y = 10.0;
            entry_head(ui, state, entry, actions);
            if let Some(response) = &entry.record.response {
                let id = entry.item.id.as_str();
                response::recorded(ui, state, (id, response), view, actions);
            } else {
                let words = egui::RichText::new(failure(&entry.item, language))
                    .size(13.0)
                    .color(p.red);
                let said = ui.add(egui::Label::new(words).wrap());
                widgets::address(ui, &said, "http-history-failure");
            }
        });
}

/// The sentence the send itself showed: the code, and the values it stated.
fn failure(item: &HistoryItem, language: i18n::Language) -> String {
    let known = item.summary.failure.and_then(|code| {
        let error = AppError {
            code,
            params: item.summary.failure_params.clone(),
            detail: String::new(),
        };
        crate::services::errors::message_of(&error)
    });
    known
        .unwrap_or_else(|| i18n::Message::new(keys::http::send::FAILED))
        .text(language)
}

fn entry_head(ui: &mut Ui, state: &AppState, entry: &HistoryEntry, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let method = entry.item.method;
        let words = egui::RichText::new(method.as_str())
            .monospace()
            .size(12.0)
            .strong()
            .color(badge_hue(ui, Badge::Method(method)));
        ui.add(egui::Label::new(words));
        let url = egui::RichText::new(&entry.record.exchange.url)
            .monospace()
            .size(12.5)
            .color(p.text_0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if let Some(name) = state.http.history().replacing() {
                if Button::new(&tr(keys::http::rail::CANCEL), "http-history-replace-cancel")
                    .show(ui)
                    .clicked()
                {
                    ask(actions, Ask::CancelReplace);
                }
                let word = tr(keys::http::history::REPLACE_CONFIRM);
                if Button::new(&word, "http-history-replace-confirm")
                    .kind(Kind::Destructive)
                    .show(ui)
                    .clicked()
                {
                    ask(actions, Ask::Reopen(true));
                }
                let question = i18n::translate(
                    language,
                    keys::http::history::REPLACE,
                    &[("name", name.into())],
                );
                let question = egui::RichText::new(question).size(12.5).color(p.text_1);
                let said = ui.add(egui::Label::new(question));
                widgets::address(ui, &said, "http-history-replace");
            } else {
                let words = state.http.reopen_into(&entry.item).map_or_else(
                    || tr(keys::http::history::REOPEN_DRAFT),
                    |name| {
                        i18n::translate(
                            language,
                            keys::http::history::REOPEN_INTO,
                            &[("name", name.into())],
                        )
                    },
                );
                if Button::new(&words, "http-history-reopen")
                    .show(ui)
                    .clicked()
                {
                    ask(actions, Ask::Reopen(false));
                }
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let said = ui.add(egui::Label::new(url).truncate());
                widgets::address(ui, &said, "http-history-url");
            });
        });
    });
}
