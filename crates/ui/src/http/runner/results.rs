//! A run's report: what failed, how many tests passed, each iteration's requests. Each line one
//! height, so that a long run stays a list drawn in view alone; a kept answer reads under it.

use std::fmt::Write as _;

use egui::{
    Color32, CornerRadius, FontFamily, Margin, Rect, Role, Sense, Ui, UiBuilder, Vec2, WidgetInfo,
    pos2,
};
use engine::error::AppError;
use engine::http::runner::{RunError, RunResult};
use engine::http::scripts::model::TestResult;
use i18n::keys;

use super::{Ask, ask, badge_of};
use crate::action::Actions;
use crate::http::{badge_hue, response};
use crate::state::AppState;
use crate::state::http::runner::{Filter, Runner};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, Button, Kind, interact, virtual_list};

const ROW: f32 = 30.0;
const BAR: f32 = 6.0;
const INDENT: f32 = 26.0;
/// What a kept answer takes of the report.
const ANSWER_SHARE: f32 = 0.5;

enum Line<'a> {
    Iteration(u32),
    Result(usize, &'a RunResult),
    Bad(String),
    Test(&'a TestResult),
    Code(&'a str),
}

fn error_words(error: &RunError, language: i18n::Language) -> String {
    let error = AppError {
        code: error.code,
        params: error.params.clone(),
        detail: String::new(),
    };
    crate::services::errors::message_of(&error)
        .unwrap_or_else(|| i18n::Message::new(keys::http::send::FAILED))
        .text(language)
}

fn lines(runner: &Runner, language: i18n::Language) -> Vec<Line<'_>> {
    let several = runner.runs() > 1;
    let mut lines = Vec::new();
    let mut iteration = None;
    for (at, result) in runner.results().iter().enumerate() {
        if !runner.filter().holds(result) {
            continue;
        }
        if several && iteration != Some(result.iteration) {
            iteration = Some(result.iteration);
            lines.push(Line::Iteration(result.iteration));
        }
        lines.push(Line::Result(at, result));
        if !runner.is_open(at) {
            continue;
        }
        if let Some(error) = &result.error {
            lines.push(Line::Bad(error_words(error, language)));
        }
        if let Some(stop) = &result.stop {
            lines.push(Line::Bad(stop.message.clone()));
        }
        for test in &result.tests {
            lines.push(Line::Test(test));
            if let Some(code) = test.code.as_deref().filter(|_| !test.passed) {
                lines.push(Line::Code(code));
            }
        }
    }
    lines
}

pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    view: &mut response::ResponseView,
    actions: &mut Actions,
) {
    let language = state.language;
    let runner = state.http.runner();
    summary(ui, runner, language);
    filters(ui, runner, language, actions);
    let room = ui.available_rect_before_wrap();
    let answer = runner.answer();
    let list = if answer.is_some() {
        let bottom = room.top() + room.height() * ANSWER_SHARE;
        Rect::from_min_max(room.min, pos2(room.right(), bottom))
    } else {
        room
    };
    let lines = lines(runner, language);
    let mut inside = ui.new_child(UiBuilder::new().id_salt("http-runner-lines").max_rect(list));
    inside.spacing_mut().item_spacing.y = 0.0;
    virtual_list(
        &mut inside,
        "http-runner-lines",
        ROW,
        lines.len(),
        |ui, at| {
            ui.push_id(at, |ui| line(ui, &lines[at], state, actions));
        },
    );
    if let Some((at, response)) = answer {
        let rect = Rect::from_min_max(pos2(room.left(), list.bottom() + 8.0), room.max);
        let mut below = ui.new_child(
            UiBuilder::new()
                .id_salt("http-runner-answer")
                .max_rect(rect),
        );
        let id = format!("run-{at}");
        response::recorded(&mut below, state, (&id, response), view, actions);
    }
}

fn pill(ui: &mut Ui, words: &str, hue: Color32, author_id: &str) {
    egui::Frame::new()
        .fill(hue.gamma_multiply(0.15))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            let words = egui::RichText::new(words).size(12.0).strong().color(hue);
            let said = ui.add(egui::Label::new(words));
            widgets::address(ui, &said, author_id);
        });
}

fn summary(ui: &mut Ui, runner: &Runner, language: i18n::Language) {
    let p = palette(ui.ctx());
    let said = runner.summary();
    let cancelled = runner.ended().is_some_and(|ended| ended.cancelled);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        if cancelled {
            let words = i18n::translate(
                language,
                keys::http::runner::CANCELLED,
                &[
                    ("done", said.requests.into()),
                    ("count", runner.planned().into()),
                ],
            );
            pill(ui, &words, p.text_1, "http-runner-cancelled");
        }
        if said.failures > 0 {
            let words = i18n::translate(
                language,
                keys::http::runner::FAILURES,
                &[("count", said.failures.into())],
            );
            pill(ui, &words, p.red, "http-runner-failures");
        } else if !cancelled {
            let words = i18n::translate(language, keys::http::runner::ALL_PASSED, &[]);
            pill(ui, &words, p.green, "http-runner-passed");
        }
        let requests = i18n::translate(
            language,
            keys::http::runner::REQUESTS,
            &[("count", said.requests.into())],
        );
        let ratio = i18n::translate(
            language,
            keys::http::tests::RATIO,
            &[("passed", said.passed.into()), ("total", said.tests.into())],
        );
        let mut figures = format!("{requests} · {ratio}");
        if let Some(ended) = runner.ended() {
            let seconds = i18n::format::at_most(f64::from(ended.millis) / 1000.0, 1, language);
            let _ = write!(figures, " · {seconds} s");
        }
        let figures = egui::RichText::new(figures).size(12.5).color(p.text_1);
        let shown = ui.add(egui::Label::new(figures));
        widgets::address(ui, &shown, "http-runner-figures");
    });
    if said.tests == 0 {
        return;
    }
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), BAR), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(3), p.red.gamma_multiply(0.5));
    let share = f64::from(said.passed) / f64::from(said.tests);
    #[expect(clippy::cast_possible_truncation, reason = "a share of a width")]
    let right = rect.left() + rect.width() * share as f32;
    let done = Rect::from_min_max(rect.min, pos2(right, rect.bottom()));
    ui.painter()
        .rect_filled(done, CornerRadius::same(3), p.green);
}

fn filters(ui: &mut Ui, runner: &Runner, language: i18n::Language, actions: &mut Actions) {
    let results = runner.results();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for filter in Filter::ALL {
            let (key, id) = match filter {
                Filter::All => (keys::http::runner::filters::ALL, "http-runner-filter-all"),
                Filter::Passed => (
                    keys::http::runner::filters::PASSED,
                    "http-runner-filter-passed",
                ),
                Filter::Failed => (
                    keys::http::runner::filters::FAILED,
                    "http-runner-filter-failed",
                ),
            };
            let count = results.iter().filter(|it| filter.holds(it)).count();
            let words = format!("{} {count}", i18n::translate(language, key, &[]));
            let on = runner.filter() == filter;
            let pressed = Button::new(&words, id).kind(Kind::Quiet).armed(on).show(ui);
            ui.ctx()
                .accesskit_node_builder(pressed.id, |node| node.set_selected(on));
            if pressed.clicked() {
                ask(actions, Ask::Filter(filter));
            }
        }
    });
}

fn line(ui: &mut Ui, line: &Line<'_>, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    match line {
        Line::Iteration(iteration) => iteration_line(ui, *iteration, state),
        Line::Result(at, result) => result_line(ui, (*at, result), state, actions),
        Line::Bad(words) => text_line(ui, words, p.red, "http-runner-why"),
        Line::Test(test) => test_line(ui, test, language),
        Line::Code(code) => {
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW), Sense::hover());
            let at = pos2(rect.left() + INDENT + 18.0, rect.center().y);
            let galley = widgets::text(ui, code, 12.0, 400.0, FontFamily::Monospace, p.text_2);
            let at = pos2(at.x, at.y - galley.size().y / 2.0);
            ui.painter_at(rect).galley(at, galley, p.text_2);
        }
    }
}

fn text_line(ui: &mut Ui, words: &str, ink: Color32, author_id: &str) {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, words));
    widgets::address(ui, &response, author_id);
    let galley = widgets::text(ui, words, 12.5, 400.0, FontFamily::Proportional, ink);
    let at = pos2(
        rect.left() + INDENT,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter_at(rect).galley(at, galley, ink);
    widgets::tooltip(response, words);
}

fn iteration_line(ui: &mut Ui, iteration: u32, state: &AppState) {
    let p = palette(ui.ctx());
    let runner = state.http.runner();
    let words = i18n::translate(
        state.language,
        keys::http::runner::ITERATION,
        &[
            ("iteration", (iteration + 1).into()),
            ("count", runner.runs().into()),
        ],
    );
    let row = runner
        .data()
        .and_then(|file| file.data.rows.get(iteration as usize));
    let values: Vec<String> = row
        .into_iter()
        .flatten()
        .map(|(key, value)| format!("{key} = {value}"))
        .collect();
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Heading, true, &words));
    widgets::address(ui, &response, "http-runner-iteration");
    let galley = widgets::text(ui, &words, 12.0, 600.0, FontFamily::Proportional, p.text_2);
    let left = rect.left() + galley.size().x + 12.0;
    let middle = rect.center().y;
    ui.painter().galley(
        pos2(rect.left(), middle - galley.size().y / 2.0),
        galley,
        p.text_2,
    );
    let row = widgets::text(
        ui,
        &values.join(" · "),
        12.0,
        400.0,
        FontFamily::Monospace,
        p.text_2,
    );
    let at = pos2(left, middle - row.size().y / 2.0);
    ui.painter_at(rect).galley(at, row, p.text_2);
}

fn status_words(result: &RunResult, language: i18n::Language) -> String {
    if result.skipped {
        return i18n::translate(language, keys::http::runner::SKIPPED, &[]);
    }
    match result.status {
        Some(status) => format!("{status} {}", result.reason),
        None => i18n::translate(language, keys::http::runner::NO_ANSWER, &[]),
    }
}

fn result_line(
    ui: &mut Ui,
    (at, result): (usize, &RunResult),
    state: &AppState,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    let failed = result.failed();
    let mut name = result.path.iter().fold(String::new(), |mut name, folder| {
        name.push_str(folder);
        name.push_str(" › ");
        name
    });
    name.push_str(&result.name);
    let author_id = if result.skipped {
        "http-runner-result-skipped"
    } else if failed {
        "http-runner-result-failed"
    } else {
        "http-runner-result-passed"
    };
    let response = interact(
        ui,
        Vec2::new(ui.available_width(), ROW),
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::Button, true, &name),
        CornerRadius::same(5),
    );
    let open = state.http.runner().is_open(at);
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_expanded(open));
    if response.clicked() {
        ask(actions, Ask::Fold(at));
    }
    let rect = response.rect;
    if failed {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), p.red.gamma_multiply(0.08));
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), p.bg_1);
    }
    let glyph =
        Rect::from_center_size(pos2(rect.left() + 12.0, rect.center().y), Vec2::splat(14.0));
    mark(ui, glyph, result.skipped, !failed);
    let badge = badge_of(result.kind, result.method);
    let hue = badge_hue(ui, badge);
    let mut left = rect.left() + INDENT;
    let galley = widgets::text(ui, badge.short(), 11.0, 600.0, FontFamily::Monospace, hue);
    let middle = rect.center().y;
    ui.painter()
        .galley(pos2(left, middle - galley.size().y / 2.0), galley, hue);
    left += 38.0;
    let figures = figures_of(result, language);
    let right = figures_at(ui, rect, &figures);
    let galley = widgets::text(ui, &name, 12.5, 400.0, FontFamily::Proportional, p.text_0);
    let clip = Rect::from_min_max(pos2(left, rect.top()), pos2(right - 12.0, rect.bottom()));
    let at = pos2(left, middle - galley.size().y / 2.0);
    ui.painter_at(clip).galley(at, galley, p.text_0);
}

/// The status, the time and the tests passed, as they read at the end of a row.
fn figures_of(result: &RunResult, language: i18n::Language) -> String {
    let mut figures = status_words(result, language);
    if result.status.is_some() {
        let _ = write!(figures, "   {} ms", result.millis);
    }
    let ran = result.tests.iter().filter(|test| !test.skipped);
    let (passed, total) = ran.fold((0, 0), |(passed, total), test| {
        (passed + u32::from(test.passed), total + 1)
    });
    if total > 0 {
        let _ = write!(figures, "   {passed}/{total}");
    }
    figures
}

fn figures_at(ui: &Ui, rect: Rect, figures: &str) -> f32 {
    let p = palette(ui.ctx());
    let galley = widgets::text(ui, figures, 12.0, 400.0, FontFamily::Monospace, p.text_2);
    let left = rect.right() - galley.size().x - 8.0;
    let at = pos2(left, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, p.text_2);
    left
}

fn mark(ui: &Ui, rect: Rect, skipped: bool, passed: bool) {
    let p = palette(ui.ctx());
    if skipped {
        let dash = Rect::from_center_size(rect.center(), Vec2::new(8.0, 1.5));
        ui.painter().rect_filled(dash, CornerRadius::ZERO, p.text_2);
    } else if passed {
        icons::paint(ui, rect, Icon::Check, p.green);
    } else {
        icons::paint(ui, rect, Icon::X, p.red);
    }
}

fn test_line(ui: &mut Ui, test: &TestResult, language: i18n::Language) {
    let p = palette(ui.ctx());
    let bad = !test.passed && !test.skipped;
    let mut words = test.name.clone();
    if bad && (test.expected.is_some() || test.actual.is_some()) {
        let dash = "—";
        let versus = i18n::translate(
            language,
            keys::http::runner::VERSUS,
            &[
                ("expected", test.expected.as_deref().unwrap_or(dash).into()),
                ("actual", test.actual.as_deref().unwrap_or(dash).into()),
            ],
        );
        let _ = write!(words, " · {versus}");
    } else if let Some(message) = test.message.as_deref().filter(|_| bad) {
        let _ = write!(words, " · {message}");
    }
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    widgets::address(ui, &response, "http-runner-test");
    let glyph = Rect::from_center_size(
        pos2(rect.left() + INDENT + 7.0, rect.center().y),
        Vec2::splat(12.0),
    );
    mark(ui, glyph, test.skipped, test.passed);
    let ink = if bad { p.red } else { p.text_1 };
    let galley = widgets::text(ui, &words, 12.5, 400.0, FontFamily::Proportional, ink);
    let at = pos2(
        rect.left() + INDENT + 18.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter_at(rect).galley(at, galley, ink);
    widgets::tooltip(response, &words);
}
