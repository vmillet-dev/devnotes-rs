//! A tab's console, under its answer: what left and came back, what the scripts said, what failed,
//! in rows drawn only in view. Folded, it says how much it holds.

use chrono::Local;
use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Color32, CornerRadius, FontFamily, FontId, Layout, Margin, Rect, Role, Sense, Stroke,
    Ui, Vec2, WidgetInfo, pos2,
};
use engine::http::console::{ConsoleEvent, ConsoleLine, ConsoleOrigin, secret_segments};
use i18n::keys;

use super::Action;
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::state::http::console::Filter;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::theme::tokens::Palette;
use crate::widgets::{self, Button, interact, virtual_list};

/// Folded, what the console takes.
pub(super) const FOLDED: f32 = 30.0;
const ROW: f32 = 24.0;
const SIZE: f32 = 12.0;

/// What only the console's drawing sees.
#[derive(Default)]
pub(crate) struct ConsoleView {
    filter: Filter,
}

fn filter_name(filter: Filter) -> i18n::Key {
    use keys::http::console::filters;
    match filter {
        Filter::All => filters::ALL,
        Filter::Traffic => filters::TRAFFIC,
        Filter::Logs => filters::LOGS,
        Filter::Errors => filters::ERRORS,
    }
}

fn counted(state: &AppState, key: &str, filter: Filter) -> u32 {
    let lines = state.http.console().lines_of(key);
    let held = lines.filter(|event| filter.holds(&event.line)).count();
    u32::try_from(held).unwrap_or(u32::MAX)
}

/// Whose console it is: a tab's, or a run's under a key of its own.
#[derive(Clone, Copy)]
pub(super) struct Of<'a> {
    pub(super) key: &'a str,
    pub(super) name: &'a str,
}

pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    (of, view): (Of<'_>, &mut ConsoleView),
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let pane = ui.max_rect();
    ui.painter().rect_filled(pane, CornerRadius::ZERO, p.bg_1);
    ui.painter()
        .hline(pane.x_range(), pane.top(), Stroke::new(1.0, p.line_soft));
    if state.http.console().is_open() {
        unfolded(ui, state, (of, view), actions);
    } else {
        folded(ui, state, of, actions);
    }
}

fn folded(ui: &mut Ui, state: &AppState, of: Of<'_>, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let pane = ui.max_rect();
    let label = tr(keys::http::console::UNFOLD);
    let response = interact(
        ui,
        pane.size(),
        Sense::click(),
        "http-console-unfold",
        || WidgetInfo::labeled(Role::Button, true, &label),
        CornerRadius::ZERO,
    );
    let traffic = i18n::translate(
        language,
        keys::http::console::TRAFFIC,
        &[("count", counted(state, of.key, Filter::Traffic).into())],
    );
    let logs = i18n::translate(
        language,
        keys::http::console::LOGS,
        &[("count", counted(state, of.key, Filter::Logs).into())],
    );
    let errors = counted(state, of.key, Filter::Errors);
    let mut job = LayoutJob::default();
    let font = FontId::new(SIZE, FontFamily::Proportional);
    let title = tr(keys::http::console::TITLE);
    let mut heading = TextFormat::simple(font.clone(), p.text_0);
    heading.coords.push(*b"wght", 600.0);
    job.append(&title, 0.0, heading);
    let summary = format!("   {traffic} · {logs}");
    job.append(&summary, 0.0, TextFormat::simple(font.clone(), p.text_2));
    if errors > 0 {
        let said = i18n::translate(
            language,
            keys::http::console::ERRORS,
            &[("count", errors.into())],
        );
        job.append(&format!("   {said}"), 0.0, TextFormat::simple(font, p.red));
    }
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let at = pos2(pane.left() + 20.0, pane.center().y - galley.size().y / 2.0);
    let mut over =
        ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(at, galley.size())));
    let said = over.add(egui::Label::new(galley).selectable(false));
    widgets::address(ui, &said, "http-console-summary");
    let glyph = Rect::from_center_size(
        pos2(pane.right() - 20.0, pane.center().y),
        egui::Vec2::splat(12.0),
    );
    crate::theme::icons::paint(ui, glyph, Icon::ChevronUp, p.text_2);
    if response.clicked() {
        actions.push(Asked::Http(Action::ToggleConsole));
    }
}

fn unfolded(
    ui: &mut Ui,
    state: &AppState,
    (of, view): (Of<'_>, &mut ConsoleView),
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    egui::Frame::new()
        .inner_margin(Margin::symmetric(20, 8))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.spacing_mut().item_spacing.y = 6.0;
            head(ui, state, (of, view), actions);
            let rows = rows_of(state, of.key, view.filter);
            if rows.is_empty() {
                let words = i18n::translate(language, keys::http::console::EMPTY, &[]);
                let words = egui::RichText::new(words).size(12.5).color(p.text_2);
                ui.add(egui::Label::new(words));
                return;
            }
            ui.spacing_mut().item_spacing.y = 0.0;
            virtual_list(ui, "http-console-lines", ROW, rows.len(), |ui, at| {
                row(ui, rows[at], of, language);
            });
        });
}

/// Each event, and under a request the row of its headers.
fn rows_of<'a>(state: &'a AppState, key: &str, filter: Filter) -> Vec<(&'a ConsoleEvent, bool)> {
    let mut rows = Vec::new();
    for event in state.http.console().lines_of(key) {
        if !filter.holds(&event.line) {
            continue;
        }
        rows.push((event, false));
        if matches!(&event.line, ConsoleLine::Out { headers, .. } if !headers.is_empty()) {
            rows.push((event, true));
        }
    }
    rows
}

fn head(
    ui: &mut Ui,
    state: &AppState,
    (of, view): (Of<'_>, &mut ConsoleView),
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let title = egui::RichText::new(tr(keys::http::console::TITLE))
            .size(12.5)
            .strong()
            .color(p.text_1);
        let said = ui.add(egui::Label::new(title));
        widgets::address(ui, &said, "http-console");
        for filter in Filter::ALL {
            let words = format!(
                "{} {}",
                tr(filter_name(filter)),
                counted(state, of.key, filter)
            );
            let id = format!("http-console-filter-{filter:?}").to_lowercase();
            if Button::new(&words, &id)
                .kind(widgets::Kind::Quiet)
                .armed(view.filter == filter)
                .show(ui)
                .clicked()
            {
                view.filter = filter;
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let fold = tr(keys::http::console::FOLD);
            if widgets::IconButton::new(Icon::ChevronDown, &fold, "http-console-fold")
                .show(ui)
                .clicked()
            {
                actions.push(Asked::Http(Action::ToggleConsole));
            }
            if Button::new(&tr(keys::http::console::CLEAR), "http-console-clear")
                .show(ui)
                .clicked()
            {
                actions.push(Asked::Http(Action::ClearConsole(of.key.to_owned())));
            }
            let masked = egui::RichText::new(tr(keys::http::console::MASKED))
                .size(11.5)
                .color(p.text_2);
            ui.add(egui::Label::new(masked));
        });
    });
}

/// The line as drawn: its words, a secret written as its `{{name}}` set apart.
fn job_of(words: &str, ink: Color32, p: &Palette) -> LayoutJob {
    let font = FontId::new(SIZE, FontFamily::Monospace);
    let mut job = LayoutJob::default();
    for (part, secret) in secret_segments(words) {
        let hue = if secret { p.amber_text } else { ink };
        job.append(&part, 0.0, TextFormat::simple(font.clone(), hue));
    }
    job
}

fn sentence(line: &ConsoleLine, language: i18n::Language) -> String {
    use keys::http::console as said;
    match line {
        ConsoleLine::Out { method, url, .. } => format!("{method} {url}"),
        ConsoleLine::In {
            status,
            reason,
            millis,
            size,
            ..
        } => i18n::translate(
            language,
            said::IN,
            &[
                ("status", u32::from(*status).into()),
                ("reason", reason.as_str().into()),
                ("millis", (*millis).into()),
                ("size", (*size).into()),
            ],
        ),
        ConsoleLine::Failed { code, params, .. } => {
            let error = engine::error::AppError {
                code: *code,
                params: params.clone(),
                detail: String::new(),
            };
            crate::services::errors::message_of(&error)
                .unwrap_or_else(|| i18n::Message::new(keys::http::send::FAILED))
                .text(language)
        }
        ConsoleLine::Log { text, .. } => text.clone(),
        ConsoleLine::Test {
            name,
            expected,
            actual,
            message,
            ..
        } => {
            if expected.is_some() || actual.is_some() {
                i18n::translate(
                    language,
                    said::TEST,
                    &[
                        ("name", name.as_str().into()),
                        ("expected", expected.as_deref().unwrap_or("—").into()),
                        ("actual", actual.as_deref().unwrap_or("—").into()),
                    ],
                )
            } else {
                i18n::translate(
                    language,
                    said::TEST_MESSAGE,
                    &[
                        ("name", name.as_str().into()),
                        ("message", message.as_deref().unwrap_or_default().into()),
                    ],
                )
            }
        }
        ConsoleLine::Stopped { stop } => stop.message.clone(),
        ConsoleLine::Missed { path, variable } => i18n::translate(
            language,
            said::MISSED,
            &[
                ("path", path.as_str().into()),
                ("variable", variable.as_str().into()),
            ],
        ),
    }
}

fn by_of(line: &ConsoleLine) -> Option<&ConsoleOrigin> {
    match line {
        ConsoleLine::Out { by, .. }
        | ConsoleLine::In { by, .. }
        | ConsoleLine::Failed { by, .. }
        | ConsoleLine::Log { by, .. }
        | ConsoleLine::Test { by, .. } => by.as_ref(),
        ConsoleLine::Stopped { .. } | ConsoleLine::Missed { .. } => None,
    }
}

/// Who said a line: the request, or a script's kind and holder.
fn origin(event: &ConsoleEvent, of: Of<'_>, language: i18n::Language) -> String {
    use engine::http::scripts::model::ScriptKind;
    use keys::http::console::origins;
    let line = &event.line;
    if let Some(by) = by_of(line) {
        let traffic = matches!(
            line,
            ConsoleLine::Out { .. } | ConsoleLine::In { .. } | ConsoleLine::Failed { .. }
        );
        let key = match by.kind {
            _ if traffic => origins::SENT,
            ScriptKind::Before => origins::BEFORE,
            ScriptKind::After => origins::AFTER,
            ScriptKind::Tests => origins::TESTS,
        };
        return i18n::translate(language, key, &[("name", by.name.as_str().into())]);
    }
    let name = if event.request.is_empty() {
        of.name
    } else {
        event.request.as_str()
    };
    i18n::translate(language, origins::REQUEST, &[("name", name.into())])
}

fn ink_of(line: &ConsoleLine, p: &Palette) -> Color32 {
    match line {
        ConsoleLine::In { status, .. } if *status >= 400 => p.red,
        ConsoleLine::In { .. } | ConsoleLine::Out { .. } | ConsoleLine::Log { .. } => p.text_0,
        _ => p.red,
    }
}

fn mark(ui: &Ui, rect: Rect, line: &ConsoleLine, ink: Color32) {
    let p = palette(ui.ctx());
    let icon = match line {
        ConsoleLine::Out { .. } => Some(Icon::ArrowRight),
        ConsoleLine::In { .. } => Some(Icon::ArrowLeft),
        ConsoleLine::Log { .. } => None,
        _ => Some(Icon::X),
    };
    match icon {
        Some(icon) => icons::paint(ui, rect, icon, ink),
        None => {
            ui.painter().circle_filled(rect.center(), 2.5, p.text_2);
        }
    }
}

fn row(ui: &mut Ui, (event, headers): (&ConsoleEvent, bool), of: Of<'_>, language: i18n::Language) {
    let p = palette(ui.ctx());
    let size = Vec2::new(ui.available_width(), ROW);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let line = &event.line;
    let ink = if headers { p.text_2 } else { ink_of(line, p) };
    let words = if headers {
        let ConsoleLine::Out { headers, .. } = line else {
            return;
        };
        let rows: Vec<String> = headers
            .iter()
            .map(|header| format!("{}: {}", header.key, header.value))
            .collect();
        rows.join("   ")
    } else {
        sentence(line, language)
    };
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    let id = if headers {
        "http-console-sub"
    } else {
        "http-console-line"
    };
    widgets::address(ui, &response, id);
    let middle = rect.center().y;
    let mut left = rect.left();
    if headers {
        left += 88.0;
    } else {
        let time = event
            .at
            .with_timezone(&Local)
            .format("%H:%M:%S%.3f")
            .to_string();
        let galley = widgets::text(ui, &time, 11.0, 400.0, FontFamily::Monospace, p.text_2);
        left += galley.size().x + 8.0;
        ui.painter().galley(
            pos2(rect.left(), middle - galley.size().y / 2.0),
            galley,
            p.text_2,
        );
        let glyph = Rect::from_center_size(pos2(left + 6.0, middle), Vec2::splat(12.0));
        mark(ui, glyph, line, ink);
    }
    left += 20.0;
    let mut right = rect.right();
    if !headers {
        let from = origin(event, of, language);
        let galley = widgets::text(ui, &from, 11.0, 400.0, FontFamily::Proportional, p.text_2);
        right -= galley.size().x + 12.0;
        let at = pos2(
            rect.right() - galley.size().x,
            middle - galley.size().y / 2.0,
        );
        ui.painter().galley(at, galley, p.text_2);
    }
    let job = job_of(&words, ink, p);
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let clip = Rect::from_min_max(pos2(left, rect.top()), pos2(right, rect.bottom()));
    let at = pos2(left, middle - galley.size().y / 2.0);
    ui.painter_at(clip).galley(at, galley, ink);
    widgets::tooltip(response, &words);
}
