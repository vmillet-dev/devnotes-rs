//! « Generate code »: the target and the switches chosen here, the code written by Rust, and what
//! was left as a variable said under it.

use egui::{Align, CornerRadius, FontFamily, Layout, Role, Sense, Ui, Vec2, WidgetInfo, pos2};
use engine::http::codegen::{CodeNotice, CodeTarget, GeneratedCode};
use i18n::keys;

use super::response::body::TextLines;
use super::{Action, modal};
use crate::action::{Action as Asked, Actions};
use crate::api;
use crate::state::AppState;
use crate::state::http::code::{Asked as Ask, CodeDialog, Switch};
use crate::state::http::kept::NoteAsked;
use crate::state::http::tabs::Tab;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, Button, Checkbox, Kind, TextField, interact};

const WIDTH: f32 = 880.0;
const LIST: f32 = 210.0;
const GAP: f32 = 14.0;
const HEIGHT: f32 = 380.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    Shell,
    Javascript,
    Other,
}

/// A target as the list shows it: its name and its library are proper names, never translated.
/// A `match` and not a table: a target added in Rust breaks this build until it is named.
fn entry(target: CodeTarget) -> (&'static str, Option<&'static str>, Group) {
    match target {
        CodeTarget::Curl => ("cURL", None, Group::Shell),
        CodeTarget::Httpie => ("HTTPie", None, Group::Shell),
        CodeTarget::Wget => ("wget", None, Group::Shell),
        CodeTarget::Powershell => ("PowerShell", Some("Invoke-RestMethod"), Group::Shell),
        CodeTarget::Fetch => ("JavaScript", Some("fetch"), Group::Javascript),
        CodeTarget::Axios => ("JavaScript", Some("axios"), Group::Javascript),
        CodeTarget::Undici => ("Node.js", Some("undici"), Group::Javascript),
        CodeTarget::Angular => ("TypeScript", Some("Angular HttpClient"), Group::Javascript),
        CodeTarget::Requests => ("Python", Some("requests"), Group::Other),
        CodeTarget::Httpx => ("Python", Some("httpx"), Group::Other),
        CodeTarget::Go => ("Go", Some("net/http"), Group::Other),
        CodeTarget::Reqwest => ("Rust", Some("reqwest"), Group::Other),
        CodeTarget::Java => ("Java", Some("HttpClient"), Group::Other),
        CodeTarget::Okhttp => ("Kotlin", Some("OkHttp"), Group::Other),
        CodeTarget::Csharp => ("C#", Some("HttpClient"), Group::Other),
        CodeTarget::Guzzle => ("PHP", Some("Guzzle"), Group::Other),
        CodeTarget::Ruby => ("Ruby", Some("Net::HTTP"), Group::Other),
        CodeTarget::Swift => ("Swift", Some("URLSession"), Group::Other),
        CodeTarget::Dart => ("Dart", Some("http"), Group::Other),
        CodeTarget::Elixir => ("Elixir", Some("Req"), Group::Other),
    }
}

fn written(target: CodeTarget) -> String {
    let (name, library, _) = entry(target);
    library.map_or_else(|| name.to_owned(), |library| format!("{name} · {library}"))
}

/// What only the dialog's drawing sees.
#[derive(Default)]
pub(crate) struct CodeView {
    filter: String,
    /// The code in view, cut into lines once per answer.
    shown: Option<(String, TextLines)>,
}

fn ask(actions: &mut Actions, asked: Ask) {
    actions.push(Asked::Http(Action::Code(asked)));
}

pub(super) fn show(ui: &Ui, state: &AppState, view: &mut CodeView, actions: &mut Actions) {
    let dialog = state.http.code();
    let Some(tab) = dialog.key().and_then(|key| state.http.tabs().tab(key)) else {
        return;
    };
    let shown = modal(ui, "http-code-dialog", |ui| {
        body(ui, state, (dialog, tab), view, actions);
    });
    if shown.should_close() {
        ask(actions, Ask::Close);
    }
}

fn body(
    ui: &mut Ui,
    state: &AppState,
    (dialog, tab): (&CodeDialog, &Tab),
    view: &mut CodeView,
    actions: &mut Actions,
) {
    let language = state.language;
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.92));
    ui.spacing_mut().item_spacing.y = 10.0;
    head(ui, state, tab);
    let room = ui.available_width() - LIST - GAP;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        ui.allocate_ui(Vec2::new(LIST, HEIGHT), |ui| {
            ui.set_width(LIST);
            ui.set_height(HEIGHT);
            ui.vertical(|ui| targets(ui, dialog, view, language, actions));
        });
        ui.allocate_ui(Vec2::new(room, HEIGHT), |ui| {
            ui.set_width(room);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                switches(ui, dialog, language, actions);
                output(ui, dialog, view, language);
            });
        });
    });
    foot(ui, (dialog, tab), language, actions);
}

fn head(ui: &mut Ui, state: &AppState, tab: &Tab) {
    let p = palette(ui.ctx());
    let language = state.language;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let title = i18n::translate(language, keys::http::code::TITLE, &[]);
        let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
        let (rect, heading) = ui.allocate_exact_size(words.size(), Sense::hover());
        ui.painter().galley(rect.min, words, p.text_0);
        heading.widget_info(|| WidgetInfo::labeled(Role::Heading, true, &title));
        widgets::address(ui, &heading, "http-code-dialog");
        let name = egui::RichText::new(&tab.draft.name)
            .size(12.5)
            .color(p.text_1);
        ui.add(egui::Label::new(name));
        let environment = state.http.environments().active().map_or_else(
            || i18n::translate(language, keys::http::environments::NONE, &[]),
            |active| active.name.clone(),
        );
        let environment = egui::RichText::new(environment).size(12.5).color(p.text_2);
        let said = ui.add(egui::Label::new(environment));
        widgets::address(ui, &said, "http-code-environment");
    });
}

fn targets(
    ui: &mut Ui,
    dialog: &CodeDialog,
    view: &mut CodeView,
    language: i18n::Language,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let (label, hint) = (
        tr(keys::http::code::FILTER),
        tr(keys::http::code::FILTER_HINT),
    );
    TextField::new(&mut view.filter, &label, "http-code-filter")
        .hint(&hint)
        .compact()
        .show(ui);
    let wanted = api::notes::fold(view.filter.trim());
    let matches = |target: CodeTarget| {
        wanted.is_empty() || api::notes::fold(&written(target)).contains(&wanted)
    };
    let mut any = false;
    egui::ScrollArea::vertical()
        .id_salt("http-code-targets")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for (group, key) in [
                (Group::Shell, keys::http::code::groups::SHELL),
                (Group::Javascript, keys::http::code::groups::JAVASCRIPT),
                (Group::Other, keys::http::code::groups::OTHER),
            ] {
                let shown: Vec<CodeTarget> = CodeTarget::ALL
                    .into_iter()
                    .filter(|target| entry(*target).2 == group && matches(*target))
                    .collect();
                if shown.is_empty() {
                    continue;
                }
                any = true;
                let title = tr(key).to_uppercase();
                let title = egui::RichText::new(title)
                    .size(10.5)
                    .strong()
                    .color(p.text_2);
                ui.add(egui::Label::new(title));
                for target in shown {
                    if target_row(ui, target, target == dialog.target()) {
                        ask(actions, Ask::Target(target));
                    }
                }
            }
            if !any {
                let words = egui::RichText::new(tr(keys::http::code::NO_TARGET))
                    .size(12.0)
                    .color(p.text_2);
                ui.add(egui::Label::new(words));
            }
        });
}

fn target_row(ui: &mut Ui, target: CodeTarget, on: bool) -> bool {
    let p = palette(ui.ctx());
    let words = written(target);
    let size = Vec2::new(ui.available_width(), 26.0);
    let response = interact(
        ui,
        size,
        Sense::click(),
        "http-code-target",
        || WidgetInfo::selected(Role::ListBoxOption, true, on, &words),
        CornerRadius::same(6),
    );
    let rect = response.rect;
    if on {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), p.bg_3);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), p.bg_2);
    }
    let ink = if on { p.text_0 } else { p.text_1 };
    let galley = widgets::text(ui, &words, 12.5, 400.0, FontFamily::Proportional, ink);
    let at = pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0);
    ui.painter_at(rect).galley(at, galley, ink);
    response.clicked()
}

/// A box and what it says; `true` on a click.
fn switch(ui: &mut Ui, (on, enabled): (bool, bool), label: &str, author_id: &str) -> bool {
    let p = palette(ui.ctx());
    let mut checked = on;
    let changed = Checkbox::new(&mut checked, label, author_id)
        .enabled(enabled)
        .show(ui)
        .changed();
    let ink = if enabled { p.text_1 } else { p.text_2 };
    ui.add(egui::Label::new(
        egui::RichText::new(label).size(12.5).color(ink),
    ));
    changed
}

fn switches(ui: &mut Ui, dialog: &CodeDialog, language: i18n::Language, actions: &mut Actions) {
    let tr = |key| i18n::translate(language, key, &[]);
    let options = dialog.options();
    let shell = entry(dialog.target()).2 == Group::Shell;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let on = (options.resolve_variables, true);
        if switch(ui, on, &tr(keys::http::code::RESOLVE), "http-code-resolve") {
            ask(actions, Ask::Toggle(Switch::Resolve));
        }
        let on = (options.include_secrets, true);
        if switch(ui, on, &tr(keys::http::code::SECRETS), "http-code-secrets") {
            ask(actions, Ask::ToggleSecrets);
        }
        let on = (options.multiline, shell);
        if switch(
            ui,
            on,
            &tr(keys::http::code::MULTILINE),
            "http-code-multiline",
        ) {
            ask(actions, Ask::Toggle(Switch::Multiline));
        }
        let on = (options.follow_redirects, true);
        if switch(ui, on, &tr(keys::http::code::FOLLOW), "http-code-follow") {
            ask(actions, Ask::Toggle(Switch::Follow));
        }
    });
    if dialog.asking_secrets() {
        secrets_question(ui, language, actions);
    }
}

fn secrets_question(ui: &mut Ui, language: i18n::Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let words = egui::RichText::new(tr(keys::http::code::ASK_SECRETS))
            .size(12.5)
            .color(p.amber_text);
        let said = ui.add(egui::Label::new(words).wrap());
        widgets::address(ui, &said, "http-code-secrets-ask");
        if Button::new(&tr(keys::http::code::INCLUDE), "http-code-secrets-yes")
            .kind(Kind::Destructive)
            .show(ui)
            .clicked()
        {
            ask(actions, Ask::IncludeSecrets);
        }
        if Button::new(&tr(keys::http::rail::CANCEL), "http-code-secrets-cancel")
            .show(ui)
            .clicked()
        {
            ask(actions, Ask::CancelSecrets);
        }
    });
}

fn output(ui: &mut Ui, dialog: &CodeDialog, view: &mut CodeView, language: i18n::Language) {
    let Some(generated) = dialog.code() else {
        return;
    };
    let width = ui.available_width();
    let same = view
        .shown
        .as_ref()
        .is_some_and(|(code, _)| *code == generated.code);
    if !same {
        let syntax = crate::notes::language::syntax(generated.language);
        let lines = TextLines::new(ui, &generated.code, syntax);
        view.shown = Some((generated.code.clone(), lines));
    }
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, HEIGHT - 110.0), Sense::hover());
    let p = palette(ui.ctx());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(8), p.bg_0);
    if let Some((_, lines)) = &mut view.shown {
        let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(6.0)));
        lines.show(&mut inside, "http-code-output", true);
    }
    notes(ui, dialog, generated, language);
}

/// The names a note lists, as the request writes them.
fn named(names: &[String]) -> String {
    let names: Vec<String> = names.iter().map(|name| format!("{{{{{name}}}}}")).collect();
    names.join(" ")
}

fn note(ui: &mut Ui, words: String, ink: egui::Color32, author_id: &str) {
    let words = egui::RichText::new(words).size(12.0).color(ink);
    let said = ui.add(egui::Label::new(words).wrap());
    widgets::address(ui, &said, author_id);
}

fn count(names: &[String]) -> u32 {
    u32::try_from(names.len()).unwrap_or(u32::MAX)
}

fn notes(ui: &mut Ui, dialog: &CodeDialog, generated: &GeneratedCode, language: i18n::Language) {
    let p = palette(ui.ctx());
    if dialog.options().include_secrets {
        let words = i18n::translate(language, keys::http::code::SECRETS_CLEAR, &[]);
        note(ui, words, p.amber_text, "http-code-note");
    } else if !generated.secrets.is_empty() {
        let left = i18n::translate(
            language,
            keys::http::code::SECRETS_LEFT,
            &[("count", count(&generated.secrets).into())],
        );
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
            icons::paint(ui, rect, Icon::Lock, p.text_2);
            let words = format!("{left} {}", named(&generated.secrets));
            note(ui, words, p.text_2, "http-code-note");
        });
    }
    if !generated.unresolved.is_empty() {
        let left = i18n::translate(
            language,
            keys::http::code::UNRESOLVED,
            &[("count", count(&generated.unresolved).into())],
        );
        let words = format!("{left} {}", named(&generated.unresolved));
        note(ui, words, p.amber_text, "http-code-unresolved");
    }
    if let Some(CodeNotice::MultipartUnsupported) = generated.notice {
        let (name, _, _) = entry(dialog.target());
        let words = i18n::translate(
            language,
            keys::http::code::notices::MULTIPART_UNSUPPORTED,
            &[("target", name.into())],
        );
        note(ui, words, p.amber_text, "http-code-notice");
    }
}

fn foot(
    ui: &mut Ui,
    (dialog, tab): (&CodeDialog, &Tab),
    language: i18n::Language,
    actions: &mut Actions,
) {
    let tr = |key| i18n::translate(language, key, &[]);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if Button::new(&tr(keys::http::code::SAVE), "http-code-save")
            .enabled(dialog.code().is_some())
            .show(ui)
            .clicked()
            && let Some(generated) = dialog.code()
        {
            let name = &tab.draft.name;
            let title = i18n::translate(
                language,
                keys::http::code::NOTE_TITLE,
                &[
                    ("name", name.as_str().into()),
                    ("target", entry(dialog.target()).0.into()),
                ],
            );
            let asked = NoteAsked {
                title,
                language: generated.language,
                content: generated.code.clone(),
                what: keys::http::code::NOTE_WHAT,
                tags: "http".to_owned(),
                source: format!("{} / {name}", tr(keys::areas::HTTP)),
            };
            actions.push(Asked::Http(Action::AskNote(Box::new(asked))));
        }
        if Button::new(&tr(keys::http::code::COPY), "http-code-copy")
            .kind(Kind::Filled)
            .enabled(dialog.code().is_some())
            .show(ui)
            .clicked()
            && let Some(generated) = dialog.code()
        {
            actions.push(Asked::Http(Action::CopyCode(generated.code.clone())));
        }
    });
}
