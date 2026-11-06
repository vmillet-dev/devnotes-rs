//! A request's body by its kind. What the body implies, and whether it reads, are Rust's to say;
//! a file is a path, read when the request is sent.

use egui::{
    Align, CornerRadius, FontFamily, FontId, Layout, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2,
};
use engine::http::BodyAnswer;
use engine::http::model::RequestBody;
use engine::json::parse::JsonError;
use i18n::keys;

use super::{Asked, edit, parts, table};
use crate::action::Actions;
use crate::http::Action;
use crate::state::AppState;
use crate::state::http::Rewritten;
use crate::state::http::bodies::{BodyKind, FileFor};
use crate::state::http::tabs::{DraftEdit, Tab};
use crate::theme::palette;
use crate::widgets::editor::{Editor, EditorView};
use crate::widgets::json_explorer;
use crate::widgets::{self, Button, Segmented};

/// Twelve lines, as the reference's boxes.
pub(super) const BOX_HEIGHT: f32 = 240.0;

/// A text of the draft in a box of its own.
pub(super) struct TextBox<'a> {
    pub(super) label: &'a str,
    pub(super) author_id: &'a str,
    pub(super) hint: &'a str,
    pub(super) height: f32,
}

/// The text as edited this frame. What the core rewrites reaches the box through the draft.
pub(super) fn text_box(
    ui: &mut Ui,
    editor: &mut Editor,
    text: &str,
    field: &TextBox<'_>,
    language: i18n::Language,
) -> Option<String> {
    text_box_of(ui, editor, text, field, language).0
}

/// The same, and the box's response for what reads its focus.
pub(super) fn text_box_of(
    ui: &mut Ui,
    editor: &mut Editor,
    text: &str,
    field: &TextBox<'_>,
    language: i18n::Language,
) -> (Option<String>, egui::Response) {
    if editor.text() != text {
        editor.replace_all(text);
    }
    let p = palette(ui.ctx());
    let size = Vec2::new(ui.available_width(), field.height);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        p.bg_1,
        Stroke::new(1.0, p.line),
        StrokeKind::Inside,
    );
    // A warning above comes and goes: the box keeps its id, and the caret, through it.
    let mut inside = ui.new_child(
        UiBuilder::new()
            .scope_id(ui.scope_id().with(field.author_id))
            .max_rect(rect.shrink(1.0)),
    );
    let output = EditorView::new(editor, field.label, field.author_id)
        .hint(field.hint)
        .language(language)
        .gutter(false)
        .wrap(true)
        .font(FontId::new(12.5, FontFamily::Monospace))
        .show(&mut inside);
    (output.changed.map(|_| editor.text()), output.response)
}

pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    tab: &Tab,
    editor: &mut Editor,
    actions: &mut Actions,
) {
    let language = state.language;
    let said = state.http.body_answer();
    head(ui, tab, said, language, actions);
    if said.is_some_and(|said| said.unusual) {
        unusual(ui, tab, language, actions);
    }
    match &tab.draft.document.body {
        RequestBody::None => hint(ui, keys::http::body::NONE_HINT, language),
        RequestBody::Json { text } => {
            typed(ui, tab, (editor, text), language, actions);
            status(ui, tab, (text, said), language, actions);
        }
        RequestBody::Text { text } => typed(ui, tab, (editor, text), language, actions),
        RequestBody::Form { fields } => {
            let label = i18n::translate(language, keys::http::body::kinds::FORM, &[]);
            let table = table::Table {
                kind: "form",
                label: &label,
                headers: false,
            };
            if let Some(fields) = table::show(ui, &table, fields, language) {
                edit(actions, tab, DraftEdit::Body(RequestBody::Form { fields }));
            }
        }
        RequestBody::Multipart { parts } => match parts::show(ui, parts, language) {
            Some(parts::Changed::Parts(parts)) => {
                let body = RequestBody::Multipart { parts };
                edit(actions, tab, DraftEdit::Body(body));
            }
            Some(parts::Changed::Pick(at)) => pick(actions, tab, FileFor::Part(at)),
            None => {}
        },
        RequestBody::Binary { path } => file(ui, tab, path, language, actions),
    }
}

fn pick(actions: &mut Actions, tab: &Tab, target: FileFor) {
    actions.push(Asked::Http(Action::PickFile {
        key: tab.key.clone(),
        target,
    }));
}

fn choose(actions: &mut Actions, tab: &Tab, kind: BodyKind) {
    actions.push(Asked::Http(Action::ChooseBody {
        key: tab.key.clone(),
        kind,
    }));
}

fn kind_name(kind: BodyKind) -> i18n::Key {
    use keys::http::body::kinds;
    match kind {
        BodyKind::None => kinds::NONE,
        BodyKind::Json => kinds::JSON,
        BodyKind::Text => kinds::TEXT,
        BodyKind::Form => kinds::FORM,
        BodyKind::Multipart => kinds::MULTIPART,
        BodyKind::Binary => kinds::BINARY,
    }
}

fn head(
    ui: &mut Ui,
    tab: &Tab,
    said: Option<&BodyAnswer>,
    language: i18n::Language,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let names = BodyKind::ALL.map(|kind| i18n::translate(language, kind_name(kind), &[]));
    let options: Vec<(BodyKind, &str)> = BodyKind::ALL
        .iter()
        .zip(&names)
        .map(|(kind, name)| (*kind, name.as_str()))
        .collect();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        let mut kind = BodyKind::of(&tab.draft.document.body);
        if Segmented::new(&options, "http-body-kind")
            .show(ui, &mut kind)
            .changed()
        {
            choose(actions, tab, kind);
        }
        if let Some(sent_as) = said.and_then(|said| said.content_type.as_deref()) {
            let words = i18n::translate(
                language,
                keys::http::body::SENT_AS,
                &[("type", sent_as.into())],
            );
            let words = egui::RichText::new(words)
                .monospace()
                .size(12.0)
                .color(p.text_2);
            let shown = ui.add(egui::Label::new(words));
            widgets::address(ui, &shown, "http-body-type");
        }
    });
}

/// A body on a method that seldom carries one: sent all the same, and one click from none.
fn unusual(ui: &mut Ui, tab: &Tab, language: i18n::Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let words = i18n::translate(
            language,
            keys::http::body::UNUSUAL,
            &[("method", tab.draft.method.as_str().into())],
        );
        let words = egui::RichText::new(words).size(12.5).color(p.amber_text);
        let shown = ui.add(egui::Label::new(words));
        widgets::address(ui, &shown, "http-body-unusual");
        let to_none = i18n::translate(language, keys::http::body::TO_NONE, &[]);
        if Button::new(&to_none, "http-body-to-none")
            .show(ui)
            .clicked()
        {
            choose(actions, tab, BodyKind::None);
        }
    });
}

fn typed(
    ui: &mut Ui,
    tab: &Tab,
    (editor, text): (&mut Editor, &str),
    language: i18n::Language,
    actions: &mut Actions,
) {
    let kind = BodyKind::of(&tab.draft.document.body);
    let label = i18n::translate(language, kind_name(kind), &[]);
    let field = TextBox {
        label: &label,
        author_id: "http-body-text",
        hint: "",
        height: BOX_HEIGHT,
    };
    let Some(text) = text_box(ui, editor, text, &field, language) else {
        return;
    };
    let body = if kind == BodyKind::Json {
        RequestBody::Json { text }
    } else {
        RequestBody::Text { text }
    };
    edit(actions, tab, DraftEdit::Body(body));
}

/// Why a JSON text does not read, in the explorer's words.
pub(super) fn invalid(problem: &JsonError, language: i18n::Language) -> String {
    format!(
        "{} {}",
        i18n::translate(
            language,
            keys::json::INVALID_AT,
            &[
                ("line", problem.line.into()),
                ("column", problem.column.into()),
                ("format", "JSON".into()),
            ],
        ),
        i18n::translate(language, json_explorer::reason(problem.reason), &[]),
    )
}

/// Whether the JSON reads, and « Format » once there is something that does.
fn status(
    ui: &mut Ui,
    tab: &Tab,
    (text, said): (&str, Option<&BodyAnswer>),
    language: i18n::Language,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let problem = said.and_then(|said| said.problem.as_ref());
    let blank = text.trim().is_empty();
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let format = i18n::translate(language, keys::http::body::FORMAT, &[]);
        if Button::new(&format, "http-body-format")
            .enabled(problem.is_none() && !blank)
            .show(ui)
            .clicked()
        {
            actions.push(Asked::Http(Action::Rewrite {
                key: tab.key.clone(),
                part: Rewritten::Body,
            }));
        }
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            if let Some(problem) = problem {
                let words = egui::RichText::new(invalid(problem, language))
                    .size(12.0)
                    .color(p.red);
                let shown = ui.add(egui::Label::new(words).wrap());
                widgets::address(ui, &shown, "http-body-problem");
            } else if !blank {
                let words =
                    i18n::translate(language, keys::json::VALID, &[("format", "JSON".into())]);
                let words = egui::RichText::new(words).size(12.0).color(p.green);
                let shown = ui.add(egui::Label::new(words));
                widgets::address(ui, &shown, "http-body-valid");
            }
        });
    });
}

fn file(ui: &mut Ui, tab: &Tab, path: &str, language: i18n::Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        if Button::new(&tr(keys::http::body::CHOOSE_FILE), "http-body-choose")
            .show(ui)
            .clicked()
        {
            pick(actions, tab, FileFor::Body);
        }
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            let words = if path.is_empty() {
                tr(keys::http::body::NO_FILE)
            } else {
                path.to_owned()
            };
            let words = egui::RichText::new(words)
                .monospace()
                .size(12.5)
                .color(p.text_1);
            let shown = ui.add(egui::Label::new(words).truncate());
            widgets::address(ui, &shown, "http-body-path");
        });
    });
    hint(ui, keys::http::body::FILE_HINT, language);
}

fn hint(ui: &mut Ui, key: i18n::Key, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = egui::RichText::new(i18n::translate(language, key, &[]))
        .size(12.0)
        .color(p.text_2);
    ui.add(egui::Label::new(words).wrap());
}
