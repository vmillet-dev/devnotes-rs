//! A collection or a folder run: how — iterations, a delay, a data file, three switches —, in
//! which order, and the report of the last run with its console. A page of the area, as the
//! history and the environments are.

mod order;
mod results;

use egui::{
    Align, CornerRadius, FontFamily, Layout, Rect, Role, Sense, Stroke, Ui, UiBuilder, Vec2,
    WidgetInfo, pos2,
};
use i18n::keys;

use super::{Action, CONSOLE_SHARE, console, response};
use crate::action::{Action as Asked, Actions, AppAction};
use crate::notes::ui::folder_hue;
use crate::state::AppState;
use crate::state::http::runner::{Asked as Ask, Phase};
use crate::state::http::tree::Badge;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, Checkbox, Kind, TextField};

const HEAD: f32 = 56.0;
const SIDE: f32 = 300.0;

/// What only the page's drawing sees.
#[derive(Default)]
pub(crate) struct RunnerView {
    /// What is typed, until the field is left.
    iterations: Option<String>,
    delay: Option<String>,
    order: order::OrderView,
    response: response::ResponseView,
    console: console::ConsoleView,
}

fn ask(actions: &mut Actions, asked: Ask) {
    actions.push(Asked::Http(Action::Runner(asked)));
}

fn badge_of(
    kind: engine::http::model::RequestKind,
    method: engine::http::model::HttpMethod,
) -> Badge {
    use engine::http::model::RequestKind;
    match kind {
        RequestKind::Http => Badge::Method(method),
        RequestKind::Graphql => Badge::Graphql,
        RequestKind::Websocket => Badge::Websocket,
    }
}

fn environment_name(state: &AppState) -> String {
    state.http.environments().active().map_or_else(
        || i18n::translate(state.language, keys::http::runner::NO_ENVIRONMENT, &[]),
        |active| active.name.clone(),
    )
}

pub(super) fn show(
    ui: &mut Ui,
    region: Rect,
    state: &AppState,
    view: &mut RunnerView,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    ui.painter().rect_filled(region, CornerRadius::ZERO, p.bg_0);
    let head_rect = Rect::from_min_size(region.min, Vec2::new(region.width(), HEAD));
    let mut top = ui.new_child(UiBuilder::new().max_rect(head_rect.shrink2(Vec2::new(20.0, 0.0))));
    head(&mut top, state, actions);
    let soft = Stroke::new(1.0, p.line_soft);
    ui.painter()
        .hline(region.x_range(), head_rect.bottom() - 0.5, soft);
    let side_width = SIDE.min(region.width() * 0.5);
    let side_rect = Rect::from_min_max(
        pos2(region.left(), head_rect.bottom()),
        pos2(region.left() + side_width, region.bottom()),
    );
    let mut side_ui = ui.new_child(
        UiBuilder::new()
            .id_salt("http-runner-side")
            .max_rect(side_rect.shrink2(Vec2::new(16.0, 12.0))),
    );
    side(&mut side_ui, state, view, actions);
    ui.painter()
        .vline(side_rect.right(), side_rect.y_range(), soft);
    let rest = Rect::from_min_max(side_rect.right_top(), region.max);
    report(ui, rest, state, view, actions);
}

fn head(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let runner = state.http.runner();
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let toggle = tr(keys::sidebar::TOGGLE);
        if widgets::SquareButton::new(Icon::Sidebar, &toggle, "http-rail-toggle")
            .pressed(state.settings.rail_shown)
            .show(ui)
            .clicked()
        {
            actions.push(Asked::App(AppAction::ToggleRail));
        }
        let crumb = egui::RichText::new(tr(keys::http::runner::CRUMB))
            .size(12.5)
            .color(p.text_2);
        ui.add(egui::Label::new(crumb));
        let title = runner.target().map_or("", |target| target.name.as_str());
        let words = widgets::text(ui, title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
        let (rect, heading) = ui.allocate_exact_size(words.size(), Sense::hover());
        ui.painter().galley(rect.min, words, p.text_0);
        heading.widget_info(|| WidgetInfo::labeled(Role::Heading, true, title));
        widgets::address(ui, &heading, "http-runner");
        environment(ui, state);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            head_actions(ui, state, actions);
        });
    });
}

fn environment(ui: &mut Ui, state: &AppState) {
    let p = palette(ui.ctx());
    let active = state.http.environments().active();
    let hue = active.map_or(p.text_2, |active| folder_hue(ui, active.colour));
    let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
    ui.painter().circle_filled(dot.center(), 4.0, hue);
    let words = egui::RichText::new(environment_name(state))
        .size(12.5)
        .color(p.text_1);
    let said = ui.add(egui::Label::new(words));
    widgets::address(ui, &said, "http-runner-environment");
}

fn head_actions(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let runner = state.http.runner();
    if !runner.is_running() {
        let key = if runner.results().is_empty() {
            keys::http::runner::START
        } else {
            keys::http::runner::AGAIN
        };
        if Button::new(&tr(key), "http-runner-start")
            .kind(Kind::Filled)
            .enabled(runner.enabled_count() > 0)
            .show(ui)
            .clicked()
        {
            ask(actions, Ask::Start);
        }
        return;
    }
    if Button::new(&tr(keys::http::runner::CANCEL), "http-runner-cancel")
        .show(ui)
        .clicked()
    {
        ask(actions, Ask::Cancel);
    }
    if let Some(progress) = runner.progress() {
        let words = i18n::translate(
            language,
            keys::http::runner::PROGRESS,
            &[
                ("iteration", (progress.iteration + 1).into()),
                ("iterations", runner.runs().into()),
                ("index", (progress.index + 1).into()),
                ("count", runner.enabled_count().into()),
                ("name", progress.name.as_str().into()),
            ],
        );
        let words = egui::RichText::new(words).size(12.5).color(p.text_1);
        let said = ui.add(egui::Label::new(words).truncate());
        widgets::address(ui, &said, "http-runner-progress");
        widgets::spinner(ui, 14.0, &tr(keys::http::runner::CANCEL));
    }
}

/// Parsed as it is typed: a run started from the field takes what the field says.
fn number(
    ui: &mut Ui,
    (label, author_id): (&str, &str),
    (value, enabled): (u32, bool),
    typed: &mut Option<String>,
) -> Option<u32> {
    let p = palette(ui.ctx());
    let words = egui::RichText::new(label).size(12.0).color(p.text_2);
    ui.add(egui::Label::new(words));
    let mut text = typed.clone().unwrap_or_else(|| value.to_string());
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            TextField::new(&mut text, label, author_id)
                .compact()
                .show(ui)
        })
        .inner;
    let changed = response.changed();
    if changed {
        *typed = Some(text.clone());
    }
    if response.lost_focus() {
        *typed = None;
    }
    changed.then(|| text.trim().parse().ok()).flatten()
}

fn side(ui: &mut Ui, state: &AppState, view: &mut RunnerView, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let runner = state.http.runner();
    let running = runner.is_running();
    ui.spacing_mut().item_spacing.y = 8.0;
    ui.columns(2, |columns| {
        let iterations = (tr(keys::http::runner::ITERATIONS), "http-runner-iterations");
        let free = !running && runner.data().is_none();
        if let Some(count) = number(
            &mut columns[0],
            (&iterations.0, iterations.1),
            (runner.runs(), free),
            &mut view.iterations,
        ) {
            ask(actions, Ask::Iterations(count));
        }
        let delay = tr(keys::http::runner::DELAY);
        if let Some(millis) = number(
            &mut columns[1],
            (&delay, "http-runner-delay"),
            (runner.delay_ms(), !running),
            &mut view.delay,
        ) {
            ask(actions, Ask::Delay(millis));
        }
    });
    data(ui, state, actions);
    let switches = [
        (
            runner.stop_on_failure(),
            keys::http::runner::STOP_ON_FAILURE,
            "http-runner-stop",
            Ask::StopOnFailure as fn(bool) -> Ask,
        ),
        (
            runner.keep_variables(),
            keys::http::runner::KEEP_VARIABLES,
            "http-runner-keep-variables",
            Ask::KeepVariables,
        ),
        (
            runner.keep_responses(),
            keys::http::runner::KEEP_RESPONSES,
            "http-runner-keep-responses",
            Ask::KeepResponses,
        ),
    ];
    for (on, key, author_id, asked) in switches {
        let mut on = on;
        let label = tr(key);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let shown = Checkbox::new(&mut on, &label, author_id)
                .enabled(!running)
                .show(ui);
            if shown.changed() {
                ask(actions, asked(on));
            }
            let words = egui::RichText::new(&label).size(12.5).color(p.text_1);
            ui.add(egui::Label::new(words));
        });
    }
    let line = ui.available_rect_before_wrap();
    ui.painter()
        .hline(line.x_range(), line.top(), Stroke::new(1.0, p.line_soft));
    ui.add_space(8.0);
    order::show(ui, state, &mut view.order, actions);
}

fn data(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let runner = state.http.runner();
    let running = runner.is_running();
    let Some(file) = runner.data() else {
        if Button::new(
            &tr(keys::http::runner::DATA_CHOOSE),
            "http-runner-data-choose",
        )
        .enabled(!running)
        .show(ui)
        .clicked()
        {
            ask(actions, Ask::ChooseData);
        }
        return;
    };
    let count = |rows: usize| u32::try_from(rows).unwrap_or(u32::MAX);
    let rows = i18n::translate(
        language,
        keys::http::runner::DATA_ROWS,
        &[("count", count(file.data.rows.len()).into())],
    );
    let columns = i18n::translate(
        language,
        keys::http::runner::DATA_COLUMNS,
        &[("count", count(file.data.columns.len()).into())],
    );
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let words = format!("{} · {rows} · {columns}", file.name);
        let words = egui::RichText::new(words).size(12.5).color(p.text_1);
        let width = (ui.available_width() - widgets::MIN_TARGET - 8.0).max(0.0);
        let said = ui.add_sized(
            Vec2::new(width, widgets::MIN_TARGET),
            egui::Label::new(words).truncate(),
        );
        widgets::address(ui, &said, "http-runner-data");
        let clear = tr(keys::http::runner::DATA_CLEAR);
        if widgets::IconButton::new(Icon::X, &clear, "http-runner-data-clear")
            .show(ui)
            .clicked()
            && !running
        {
            ask(actions, Ask::ClearData);
        }
    });
}

fn report(ui: &mut Ui, rect: Rect, state: &AppState, view: &mut RunnerView, actions: &mut Actions) {
    let runner = state.http.runner();
    let mut above = rect;
    if let Some(key) = runner.console_key() {
        let held = if state.http.console().is_open() {
            (rect.height() * CONSOLE_SHARE).max(console::FOLDED)
        } else {
            console::FOLDED
        };
        above.max.y = rect.bottom() - held;
        let pane = Rect::from_min_max(pos2(rect.left(), above.bottom()), rect.max);
        let mut lines = ui.new_child(
            UiBuilder::new()
                .id_salt("http-runner-console")
                .max_rect(pane),
        );
        let of = console::Of {
            key: &key,
            name: "",
        };
        console::show(&mut lines, state, (of, &mut view.console), actions);
    }
    let mut body = ui.new_child(
        UiBuilder::new()
            .id_salt("http-runner-report")
            .max_rect(above.shrink2(Vec2::new(20.0, 12.0))),
    );
    body.set_clip_rect(above);
    body.spacing_mut().item_spacing.y = 10.0;
    if runner.phase() == Phase::Asking {
        asking(&mut body, state, actions);
    }
    if !runner.results().is_empty() {
        results::show(&mut body, state, &mut view.response, actions);
    } else if !runner.is_running() && runner.phase() != Phase::Asking {
        empty(&mut body, state, actions);
    }
}

/// Under a protected environment, what could change something waits for this yes.
fn asking(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let language = state.language;
    let runner = state.http.runner();
    let changing: Vec<String> = runner
        .changing()
        .map(|row| format!("{} {}", badge_of(row.kind, row.method).short(), row.name))
        .collect();
    let question = i18n::translate(
        language,
        keys::http::runner::PROTECTED,
        &[
            ("name", environment_name(state).into()),
            (
                "count",
                u32::try_from(changing.len()).unwrap_or(u32::MAX).into(),
            ),
            ("total", runner.enabled_count().into()),
            ("requests", changing.join(", ").into()),
        ],
    );
    let go = i18n::translate(language, keys::http::runner::START_ANYWAY, &[]);
    let cancel = i18n::translate(language, keys::http::rail::CANCEL, &[]);
    let ids = (
        "http-runner-protected",
        "http-runner-protected-go",
        "http-runner-protected-cancel",
    );
    match super::request::protected_question(ui, (&question, &go, &cancel), ids) {
        Some(true) => ask(actions, Ask::StartAnyway),
        Some(false) => ask(actions, Ask::StopAsking),
        None => {}
    }
}

fn empty(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let runner = state.http.runner();
    let requests = i18n::translate(
        language,
        keys::http::runner::REQUESTS,
        &[("count", runner.enabled_count().into())],
    );
    let iterations = i18n::translate(
        language,
        keys::http::runner::ITERATIONS_COUNT,
        &[("count", runner.runs().into())],
    );
    let of_file = runner
        .data()
        .map_or_else(String::new, |file| format!(" ({})", file.name));
    let ready = format!(
        "{requests} · {iterations}{of_file} · {}",
        environment_name(state)
    );
    ui.vertical_centered(|ui| {
        ui.add_space((ui.available_height() / 2.0 - 40.0).max(0.0));
        let words = egui::RichText::new(ready).size(13.0).color(p.text_1);
        let said = ui.add(egui::Label::new(words));
        widgets::address(ui, &said, "http-runner-ready");
        let word = i18n::translate(language, keys::http::runner::START, &[]);
        if Button::new(&word, "http-runner-start-empty")
            .kind(Kind::Filled)
            .enabled(runner.enabled_count() > 0)
            .show(ui)
            .clicked()
        {
            ask(actions, Ask::Start);
        }
    });
}
