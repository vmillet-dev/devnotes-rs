//! The scripts of a request, a folder or a collection — before the send and after the answer, or
//! its tests — in the snippets' editor. A request's says where its own run among those above.

use std::ops::Range;

use egui::{
    Color32, CornerRadius, FontFamily, FontId, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2,
};
use engine::http::model::RequestScripts;
use engine::http::scripts::model::ScriptKind;
use engine::http::settings::{HttpOrigin, InheritedScripts};
use engine::notes::language::Language;
use i18n::keys;

use crate::services::highlight::{self, Colouring};
use crate::theme::palette;
use crate::widgets::editor::{Editor, EditorView};
use crate::widgets::{self, Button, MenuItem, Segmented};

const CODE_HEIGHT: f32 = 240.0;
/// What a script reads, in the order the reference said them.
const NAMES: &str = "req · res · env · pm · require('lodash')…";

/// Five, so the menu stays short; each says the API it shows.
const EXAMPLES: [(&str, i18n::Key, &str); 5] = [
    (
        "http-script-example-read",
        keys::http::scripts::examples::READ_RESPONSE,
        "res.json()",
    ),
    (
        "http-script-example-variable",
        keys::http::scripts::examples::SET_VARIABLE,
        "env.set",
    ),
    (
        "http-script-example-header",
        keys::http::scripts::examples::ADD_HEADER,
        "req.headers",
    ),
    (
        "http-script-example-send",
        keys::http::scripts::examples::SEND_REQUEST,
        "pm.sendRequest",
    ),
    (
        "http-script-example-test",
        keys::http::scripts::examples::TEST_STATUS,
        "test",
    ),
];

fn example_code(author_id: &str, test_name: &str) -> String {
    match author_id {
        "http-script-example-read" => "const body = res.json();\nconsole.log(body);".to_owned(),
        "http-script-example-variable" => "env.set('name', 'value');".to_owned(),
        "http-script-example-header" => {
            "req.headers.set('X-Request-Id', crypto.randomUUID());".to_owned()
        }
        "http-script-example-send" => {
            "const answer = await pm.sendRequest({ url: env.get('baseUrl'), method: 'GET' });\n\
             console.log(answer.code);"
                .to_owned()
        }
        _ => format!("test({test_name:?}, () => {{\n  expect(res.status).toBe(200);\n}});"),
    }
}

/// One document's editor, kept for as long as its owner is shown.
#[derive(Default)]
struct Document {
    editor: Editor,
    colouring: Option<Colouring>,
}

/// What only the editor's drawing sees.
#[derive(Default)]
pub(crate) struct ScriptsView {
    /// What holds them: another one is another history, Ctrl+Z never reaches into it.
    owner: String,
    phase: ScriptKind,
    before: Document,
    after: Document,
    tests: Document,
    /// The line a failure named, from 1, marked until the next edit.
    line: Option<u32>,
}

impl ScriptsView {
    /// « Open the script »: on that phase, the line that stopped marked.
    pub(crate) fn focus(&mut self, phase: ScriptKind, line: Option<u32>) {
        self.phase = phase;
        self.line = line;
    }

    pub(crate) fn phase(&self) -> ScriptKind {
        self.phase
    }

    fn document(&mut self, phase: ScriptKind) -> &mut Document {
        match phase {
            ScriptKind::Before => &mut self.before,
            ScriptKind::After => &mut self.after,
            ScriptKind::Tests => &mut self.tests,
        }
    }
}

fn source(scripts: &RequestScripts, phase: ScriptKind) -> &str {
    match phase {
        ScriptKind::Before => &scripts.before,
        ScriptKind::After => &scripts.after,
        ScriptKind::Tests => &scripts.tests,
    }
}

fn with_source(scripts: &RequestScripts, phase: ScriptKind, text: String) -> RequestScripts {
    let mut scripts = scripts.clone();
    match phase {
        ScriptKind::Before => scripts.before = text,
        ScriptKind::After => scripts.after = text,
        ScriptKind::Tests => scripts.tests = text,
    }
    scripts
}

/// What the editor draws.
pub(crate) struct ScriptsField<'a> {
    pub(crate) scripts: &'a RequestScripts,
    pub(crate) owner: &'a str,
    /// The documents it switches between: the scripts' two, or the tests alone.
    pub(crate) phases: &'a [ScriptKind],
    /// The folders and the collection above a request; `None` for a level's own.
    pub(crate) above: Option<&'a [InheritedScripts]>,
    pub(crate) request_name: &'a str,
}

/// What a frame did.
pub(crate) enum Changed {
    Scripts(RequestScripts),
    /// A level of the order clicked: its settings, on this script.
    Level(HttpOrigin),
}

fn phase_name(phase: ScriptKind) -> i18n::Key {
    match phase {
        ScriptKind::Before => keys::http::scripts::phases::BEFORE,
        ScriptKind::After => keys::http::scripts::phases::AFTER,
        ScriptKind::Tests => keys::http::scripts::phases::TESTS,
    }
}

fn placeholder(phase: ScriptKind) -> i18n::Key {
    match phase {
        ScriptKind::Before => keys::http::scripts::placeholder::BEFORE,
        ScriptKind::After => keys::http::scripts::placeholder::AFTER,
        ScriptKind::Tests => keys::http::scripts::placeholder::TESTS,
    }
}

fn order_words(phase: ScriptKind) -> i18n::Key {
    match phase {
        ScriptKind::Before => keys::http::scripts::orders::BEFORE,
        ScriptKind::After => keys::http::scripts::orders::AFTER,
        ScriptKind::Tests => keys::http::scripts::orders::TESTS,
    }
}

/// `between` is drawn above the code: what a request adds among its scripts.
pub(crate) fn show(
    ui: &mut Ui,
    field: &ScriptsField<'_>,
    view: &mut ScriptsView,
    language: i18n::Language,
    between: impl FnOnce(&mut Ui, ScriptKind),
) -> Option<Changed> {
    if view.owner != field.owner {
        *view = ScriptsView {
            owner: field.owner.to_owned(),
            phase: view.phase,
            line: view.line,
            ..ScriptsView::default()
        };
    }
    if !field.phases.contains(&view.phase) {
        view.phase = field.phases[0];
    }
    let mut changed = toolbar(ui, field, view, language);
    if let Some(above) = field.above
        && let Some(origin) = order(ui, field, above, view.phase, language)
    {
        changed = Some(Changed::Level(origin));
    }
    between(ui, view.phase);
    if let Some(typed) = code(ui, field, view, language) {
        changed = Some(typed);
    }
    let p = palette(ui.ctx());
    let hint = format!(
        "{NAMES} · {}",
        i18n::translate(language, keys::http::scripts::HINT, &[])
    );
    let hint = egui::RichText::new(hint).size(12.0).color(p.text_2);
    ui.add(egui::Label::new(hint).wrap());
    changed
}

fn toolbar(
    ui: &mut Ui,
    field: &ScriptsField<'_>,
    view: &mut ScriptsView,
    language: i18n::Language,
) -> Option<Changed> {
    let tr = |key| i18n::translate(language, key, &[]);
    let names: Vec<String> = field.phases.iter().map(|it| tr(phase_name(*it))).collect();
    let options: Vec<(ScriptKind, &str)> = field
        .phases
        .iter()
        .zip(&names)
        .map(|(phase, name)| (*phase, name.as_str()))
        .collect();
    ui.horizontal(|ui| {
        if options.len() > 1 {
            let mut phase = view.phase;
            if Segmented::new(&options, "http-script-phase")
                .show(ui, &mut phase)
                .changed()
            {
                view.phase = phase;
                view.line = None;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            examples(ui, field, view, language)
        })
        .inner
    })
    .inner
}

/// « Examples »: a block ready to adapt, written at the end of the script.
fn examples(
    ui: &mut Ui,
    field: &ScriptsField<'_>,
    view: &mut ScriptsView,
    language: i18n::Language,
) -> Option<Changed> {
    let tr = |key| i18n::translate(language, key, &[]);
    let anchor = Button::new(
        &tr(keys::http::scripts::examples::OPEN),
        "http-script-examples",
    )
    .show(ui);
    let words: Vec<String> = EXAMPLES
        .iter()
        .map(|(_, key, api)| format!("{} · {api}", tr(*key)))
        .collect();
    let items: Vec<MenuItem<'_>> = EXAMPLES
        .iter()
        .zip(&words)
        .map(|((author_id, _, _), words)| MenuItem::new(words, author_id))
        .collect();
    let chosen = widgets::menu(&anchor, &items)?;
    let name = tr(keys::http::scripts::examples::TEST_NAME);
    let block = example_code(chosen, &name);
    let current = source(field.scripts, view.phase).trim_end();
    let text = if current.is_empty() {
        format!("{block}\n")
    } else {
        format!("{current}\n\n{block}\n")
    };
    view.line = None;
    Some(Changed::Scripts(with_source(
        field.scripts,
        view.phase,
        text,
    )))
}

/// Before the send from the collection down; after the answer, back up. A level above is a click
/// from its settings.
fn order(
    ui: &mut Ui,
    field: &ScriptsField<'_>,
    above: &[InheritedScripts],
    phase: ScriptKind,
    language: i18n::Language,
) -> Option<HttpOrigin> {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let mut levels: Vec<Option<&InheritedScripts>> = above.iter().map(Some).collect();
    levels.push(None);
    if phase != ScriptKind::Before {
        levels.reverse();
    }
    let mut opened = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let words = egui::RichText::new(tr(order_words(phase)))
            .size(12.0)
            .color(p.text_2);
        let said = ui.add(egui::Label::new(words));
        widgets::address(ui, &said, "http-script-order-label");
        for (at, level) in levels.iter().enumerate() {
            if at > 0 {
                ui.add(egui::Label::new(
                    egui::RichText::new("›").size(12.0).color(p.text_2),
                ));
            }
            let Some(level) = level else {
                let here = format!(
                    "{} {}",
                    tr(keys::http::scripts::levels::REQUEST),
                    field.request_name
                );
                let words = egui::RichText::new(here).size(12.0).color(p.text_0);
                let said = ui.add(egui::Label::new(words));
                widgets::address(ui, &said, "http-script-level");
                continue;
            };
            let kind = tr(
                if level.from.kind == engine::http::model::HttpItemKind::Folder {
                    keys::http::scripts::levels::FOLDER
                } else {
                    keys::http::scripts::levels::COLLECTION
                },
            );
            let written = !source(&level.scripts, phase).trim().is_empty();
            let none = if written {
                String::new()
            } else {
                format!(" · {}", tr(keys::http::scripts::NONE))
            };
            let words = format!("{kind} {}{none}", level.from.name);
            let pressed = Button::new(&words, "http-script-level")
                .kind(widgets::Kind::Quiet)
                .show(ui);
            let pressed = widgets::tooltip(pressed, &tr(keys::http::scripts::OPEN_LEVEL));
            if pressed.clicked() {
                opened = Some(level.from.clone());
            }
        }
    });
    opened
}

fn code(
    ui: &mut Ui,
    field: &ScriptsField<'_>,
    view: &mut ScriptsView,
    language: i18n::Language,
) -> Option<Changed> {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let phase = view.phase;
    let marked: Vec<usize> = view
        .line
        .and_then(|line| usize::try_from(line).ok())
        .map(|line| line.saturating_sub(1))
        .into_iter()
        .collect();
    let document = view.document(phase);
    let text = source(field.scripts, phase);
    if document.editor.text() != text {
        document.editor.replace_all(text);
        if let Some(colouring) = &mut document.colouring {
            colouring.edited(0);
        }
    }
    if document.colouring.is_none() && text::highlight::is_ready() {
        document.colouring = crate::notes::language::syntax(Language::Js)
            .and_then(|syntax| Colouring::new(ui.ctx(), syntax));
    }
    let size = Vec2::new(ui.available_width(), CODE_HEIGHT);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        p.bg_1,
        Stroke::new(1.0, p.line),
        StrokeKind::Inside,
    );
    let rope = document.editor.buffer.rope().clone();
    let visible = document.editor.visible_lines();
    let window = document.colouring.as_mut().map(|colouring| {
        colouring.window(&rope, visible);
        &*colouring
    });
    let colours = |line: usize, _: &str| -> Vec<(Range<usize>, Color32)> {
        window
            .and_then(|colouring| colouring.line(line))
            .map(|tokens| {
                tokens
                    .iter()
                    .map(|(range, token)| (range.clone(), highlight::colour(p, *token).0))
                    .collect()
            })
            .unwrap_or_default()
    };
    let (label, hint) = (tr(phase_name(phase)), tr(placeholder(phase)));
    let mut inside = ui.new_child(
        UiBuilder::new()
            .scope_id(ui.scope_id().with(("http-script-code", phase.as_str())))
            .max_rect(rect.shrink(1.0)),
    );
    let output = EditorView::new(&mut document.editor, &label, "http-script-code")
        .hint(&hint)
        .language(language)
        .indent_unit("  ")
        .comment(Some("//"))
        .font(FontId::new(12.5, FontFamily::Monospace))
        .marked_lines(&marked)
        .colours(&colours)
        .show(&mut inside);
    let edited = output.changed?;
    if let Some(colouring) = &mut document.colouring {
        colouring.edited(edited);
    }
    let typed = document.editor.text();
    view.line = None;
    Some(Changed::Scripts(with_source(field.scripts, phase, typed)))
}
