//! A GraphQL request's query and variables, the operation sent, and whether it goes as a `GET`.
//! What does not pair in the query, and which variables have no value, are Rust's to read and are
//! said above each box as it is typed: they warn, and stop nothing.

use egui::{Align, Layout, Ui, Vec2};
use engine::http::graphql::{GraphqlAnswer, GraphqlDocument, QueryFault, QueryProblem};
use i18n::keys;

use super::body::{BOX_HEIGHT, TextBox, invalid, text_box};
use super::{Asked, TextBoxes, edit};
use crate::action::Actions;
use crate::http::Action;
use crate::state::AppState;
use crate::state::http::Rewritten;
use crate::state::http::tabs::{DraftEdit, Tab};
use crate::theme::palette;
use crate::widgets::{self, Button, Checkbox, ChoiceMenu};

const GAP: f32 = 16.0;
/// Two lines kept whether or not something is said: a line that comes and goes with each brace
/// typed would push the text being typed.
const NOTES: f32 = 36.0;

/// One row of the two columns, so that the boxes start on one line whatever is said of each.
fn cells<C>(
    ui: &mut Ui,
    context: &mut C,
    left: impl FnOnce(&mut Ui, &mut C),
    right: impl FnOnce(&mut Ui, &mut C),
) {
    let room = ui.available_width() - GAP;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        cell(ui, room * 0.6, |ui| left(ui, context));
        cell(ui, room * 0.4, |ui| right(ui, context));
    });
}

fn cell(ui: &mut Ui, width: f32, content: impl FnOnce(&mut Ui)) {
    ui.allocate_ui(Vec2::new(width, 0.0), |ui| {
        ui.set_width(width);
        ui.vertical(content);
    });
}

pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    tab: &Tab,
    boxes: &mut TextBoxes,
    actions: &mut Actions,
) {
    let language = state.language;
    let said = state.http.graphql_answer();
    cells(
        ui,
        actions,
        |ui, actions| query_head(ui, tab, said, language, actions),
        |ui, actions| variables_head(ui, tab, said, language, actions),
    );
    cells(
        ui,
        actions,
        |ui, _| {
            ui.set_min_height(NOTES);
            if let Some(problem) = said.and_then(|said| said.query_problem.as_ref()) {
                query_problem(ui, problem, language);
            }
        },
        |ui, actions| {
            ui.set_min_height(NOTES);
            if variables_notes(ui, said, language) {
                rewrite(actions, tab, Rewritten::Missing);
            }
        },
    );
    if let Some(document) = typed(ui, tab, boxes, language) {
        edit(actions, tab, DraftEdit::Graphql(document));
    }
    as_get(ui, tab, language, actions);
}

/// The two boxes, and the document as typed this frame.
fn typed(
    ui: &mut Ui,
    tab: &Tab,
    boxes: &mut TextBoxes,
    language: i18n::Language,
) -> Option<GraphqlDocument> {
    let tr = |key| i18n::translate(language, key, &[]);
    let document = &tab.draft.document.graphql;
    let mut typed = None;
    cells(
        ui,
        &mut typed,
        |ui, typed| {
            let (label, hint) = (
                tr(keys::http::graphql::QUERY),
                tr(keys::http::graphql::QUERY_PLACEHOLDER),
            );
            let field = TextBox {
                label: &label,
                author_id: "http-graphql-query",
                hint: &hint,
                height: BOX_HEIGHT,
            };
            if let Some(query) = text_box(ui, &mut boxes.query, &document.query, &field, language) {
                *typed = Some(GraphqlDocument {
                    query,
                    ..document.clone()
                });
            }
        },
        |ui, typed| {
            let (label, hint) = (
                tr(keys::http::graphql::VARIABLES),
                tr(keys::http::graphql::VARIABLES_PLACEHOLDER),
            );
            let field = TextBox {
                label: &label,
                author_id: "http-graphql-variables",
                hint: &hint,
                height: BOX_HEIGHT,
            };
            let text = &document.variables;
            if let Some(variables) = text_box(ui, &mut boxes.variables, text, &field, language) {
                *typed = Some(GraphqlDocument {
                    variables,
                    ..document.clone()
                });
            }
        },
    );
    typed
}

fn rewrite(actions: &mut Actions, tab: &Tab, part: Rewritten) {
    actions.push(Asked::Http(Action::Rewrite {
        key: tab.key.clone(),
        part,
    }));
}

fn title(ui: &mut Ui, key: i18n::Key, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, key, &[]).to_uppercase();
    let words = egui::RichText::new(words)
        .size(11.5)
        .strong()
        .color(p.text_2);
    ui.add(egui::Label::new(words));
}

/// « Format », at the end of its line.
fn format_button(ui: &mut Ui, language: i18n::Language, author_id: &str, enabled: bool) -> bool {
    let words = i18n::translate(language, keys::http::graphql::FORMAT, &[]);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        Button::new(&words, author_id)
            .enabled(enabled)
            .show(ui)
            .clicked()
    })
    .inner
}

/// « Query », the operation sent when it holds several, « Format ».
fn query_head(
    ui: &mut Ui,
    tab: &Tab,
    said: Option<&GraphqlAnswer>,
    language: i18n::Language,
    actions: &mut Actions,
) {
    let query = &tab.draft.document.graphql.query;
    let enabled = !query.trim().is_empty() && said.is_none_or(|said| said.query_problem.is_none());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        title(ui, keys::http::graphql::QUERY, language);
        let operations = said.map_or(&[][..], |said| said.operations.as_slice());
        if operations.len() > 1 {
            operation(ui, tab, operations, language, actions);
        }
        if format_button(ui, language, "http-graphql-format-query", enabled) {
            rewrite(actions, tab, Rewritten::Query);
        }
    });
}

fn variables_head(
    ui: &mut Ui,
    tab: &Tab,
    said: Option<&GraphqlAnswer>,
    language: i18n::Language,
    actions: &mut Actions,
) {
    let variables = &tab.draft.document.graphql.variables;
    let enabled =
        !variables.trim().is_empty() && said.is_none_or(|said| said.variables_problem.is_none());
    ui.horizontal(|ui| {
        title(ui, keys::http::graphql::VARIABLES, language);
        if format_button(ui, language, "http-graphql-format-variables", enabled) {
            rewrite(actions, tab, Rewritten::Variables);
        }
    });
}

fn operation(
    ui: &mut Ui,
    tab: &Tab,
    operations: &[String],
    language: i18n::Language,
    actions: &mut Actions,
) {
    let document = &tab.draft.document.graphql;
    let options: Vec<(usize, &str)> = operations
        .iter()
        .enumerate()
        .map(|(at, name)| (at, name.as_str()))
        .collect();
    let mut chosen = operations
        .iter()
        .position(|name| Some(name) == document.operation_name.as_ref())
        .unwrap_or(0);
    let label = i18n::translate(language, keys::http::graphql::OPERATION, &[]);
    if ChoiceMenu::new(&label, "http-graphql-operation", &options)
        .show(ui, &mut chosen)
        .changed()
    {
        let document = GraphqlDocument {
            operation_name: Some(operations[chosen].clone()),
            ..document.clone()
        };
        edit(actions, tab, DraftEdit::Graphql(document));
    }
}

fn query_problem(ui: &mut Ui, problem: &QueryProblem, language: i18n::Language) {
    use keys::http::graphql::problems;
    let p = palette(ui.ctx());
    let fault = match problem.fault {
        QueryFault::UnclosedBrace => problems::UNCLOSED_BRACE,
        QueryFault::UnclosedParenthesis => problems::UNCLOSED_PARENTHESIS,
        QueryFault::UnclosedString => problems::UNCLOSED_STRING,
        QueryFault::StrayBrace => problems::STRAY_BRACE,
        QueryFault::StrayParenthesis => problems::STRAY_PARENTHESIS,
    };
    let words = format!(
        "{} {}",
        i18n::translate(
            language,
            keys::http::graphql::PROBLEM_AT,
            &[
                ("line", problem.line.into()),
                ("column", problem.column.into()),
            ],
        ),
        i18n::translate(language, fault, &[]),
    );
    let words = egui::RichText::new(words).size(12.0).color(p.red);
    let shown = ui.add(egui::Label::new(words).wrap());
    widgets::address(ui, &shown, "http-graphql-query-problem");
}

/// What is said of the variables; `true` when « Add » was pressed.
fn variables_notes(ui: &mut Ui, said: Option<&GraphqlAnswer>, language: i18n::Language) -> bool {
    let p = palette(ui.ctx());
    ui.spacing_mut().item_spacing.y = 4.0;
    let hint = i18n::translate(language, keys::http::graphql::VARIABLES_HINT, &[]);
    let hint = egui::RichText::new(hint).size(12.0).color(p.text_2);
    let shown = ui.add(egui::Label::new(hint).wrap());
    widgets::address(ui, &shown, "http-graphql-variables-hint");
    let Some(said) = said else {
        return false;
    };
    if let Some(problem) = &said.variables_problem {
        let words = egui::RichText::new(invalid(problem, language))
            .size(12.0)
            .color(p.red);
        let shown = ui.add(egui::Label::new(words).wrap());
        widgets::address(ui, &shown, "http-graphql-problem");
    }
    let mut added = false;
    if !said.missing.is_empty() {
        let names: Vec<String> = said.missing.iter().map(|name| format!("${name}")).collect();
        let heading = (keys::http::graphql::MISSING, "http-graphql-missing");
        named(ui, heading, &names, language, |ui| {
            let add = i18n::translate(language, keys::http::graphql::ADD, &[]);
            added = Button::new(&add, "http-graphql-add").show(ui).clicked();
        });
    }
    if !said.unknown.is_empty() {
        let heading = (keys::http::graphql::UNKNOWN, "http-graphql-unknown");
        named(ui, heading, &said.unknown, language, |_| {});
    }
    added
}

/// « Without a value: $code [Add] »: the names on the line, what follows after them.
fn named(
    ui: &mut Ui,
    (heading, author_id): (i18n::Key, &str),
    names: &[String],
    language: i18n::Language,
    after: impl FnOnce(&mut Ui),
) {
    let p = palette(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
        let words = egui::RichText::new(i18n::translate(language, heading, &[]))
            .size(12.0)
            .color(p.amber_text);
        let shown = ui.add(egui::Label::new(words));
        widgets::address(ui, &shown, author_id);
        for name in names {
            let words = egui::RichText::new(name)
                .monospace()
                .size(12.0)
                .color(p.text_0);
            let shown = ui.add(egui::Label::new(words));
            widgets::address(ui, &shown, "http-graphql-variable");
        }
        after(ui);
    });
}

fn as_get(ui: &mut Ui, tab: &Tab, language: i18n::Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let document = &tab.draft.document.graphql;
    let label = i18n::translate(language, keys::http::graphql::AS_GET, &[]);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let mut as_get = document.as_get;
        if Checkbox::new(&mut as_get, &label, "http-graphql-get")
            .show(ui)
            .changed()
        {
            let document = GraphqlDocument {
                as_get,
                ..document.clone()
            };
            edit(actions, tab, DraftEdit::Graphql(document));
        }
        let words = egui::RichText::new(&label).size(12.5).color(p.text_1);
        ui.add(egui::Label::new(words));
    });
}
