//! A script that stopped: whose it was, what it said, on which line. Before the send it stands in
//! place of the answer; after it, above the answer that came.

use egui::{CornerRadius, Margin, Stroke, Ui};
use engine::http::scripts::model::{FailureKind, ScriptKind, ScriptLevel, ScriptStop};
use i18n::keys;

use crate::theme::palette;
use crate::widgets::{self, Button};

/// Whose script stopped, and how: the words are the translations' own table.
fn title_key(stop: &ScriptStop) -> i18n::Key {
    use ScriptKind as Kind;
    use ScriptLevel as Level;
    use keys::http::scripts::stop;
    let Some(level) = stop.level else {
        return stop::HOST;
    };
    let script = stop.script.unwrap_or_default();
    if stop.kind == FailureKind::Syntax {
        return match (script, level) {
            (Kind::Before, Level::Request) => stop::invalid::before::REQUEST,
            (Kind::Before, Level::Folder) => stop::invalid::before::FOLDER,
            (Kind::Before, Level::Collection) => stop::invalid::before::COLLECTION,
            (Kind::After, Level::Request) => stop::invalid::after::REQUEST,
            (Kind::After, Level::Folder) => stop::invalid::after::FOLDER,
            (Kind::After, Level::Collection) => stop::invalid::after::COLLECTION,
            (Kind::Tests, Level::Request) => stop::invalid::tests::REQUEST,
            (Kind::Tests, Level::Folder) => stop::invalid::tests::FOLDER,
            (Kind::Tests, Level::Collection) => stop::invalid::tests::COLLECTION,
        };
    }
    match (script, level) {
        (Kind::Before, Level::Request) => stop::before::REQUEST,
        (Kind::Before, Level::Folder) => stop::before::FOLDER,
        (Kind::Before, Level::Collection) => stop::before::COLLECTION,
        (Kind::After, Level::Request) => stop::after::REQUEST,
        (Kind::After, Level::Folder) => stop::after::FOLDER,
        (Kind::After, Level::Collection) => stop::after::COLLECTION,
        (Kind::Tests, Level::Request) => stop::tests::REQUEST,
        (Kind::Tests, Level::Folder) => stop::tests::FOLDER,
        (Kind::Tests, Level::Collection) => stop::tests::COLLECTION,
    }
}

/// What stopped it: the script's own words for a throw or a syntax error, said here otherwise.
fn message(stop: &ScriptStop, language: i18n::Language) -> String {
    use keys::http::scripts::stop::kinds;
    let key = match stop.kind {
        FailureKind::Error | FailureKind::Syntax => return stop.message.clone(),
        FailureKind::Timeout => kinds::TIMEOUT,
        FailureKind::Memory => kinds::MEMORY,
        FailureKind::Host => kinds::HOST,
    };
    i18n::translate(language, key, &[])
}

fn where_words(stop: &ScriptStop, language: i18n::Language) -> Option<String> {
    use keys::http::scripts::stop as words;
    let before = stop.script.unwrap_or_default() == ScriptKind::Before;
    let Some(line) = stop.line else {
        return before.then(|| i18n::translate(language, words::NOT_SENT, &[]));
    };
    let key = if before {
        words::LINE_NOT_SENT
    } else {
        words::LINE
    };
    Some(i18n::translate(language, key, &[("line", line.into())]))
}

/// `true` on « Open the script »; `openable` is off under the history, whose request may have
/// changed since.
pub(super) fn show(
    ui: &mut Ui,
    stop: &ScriptStop,
    openable: bool,
    language: i18n::Language,
) -> bool {
    let p = palette(ui.ctx());
    let title = i18n::translate(
        language,
        title_key(stop),
        &[("name", stop.name.as_deref().unwrap_or_default().into())],
    );
    egui::Frame::new()
        .fill(p.red.gamma_multiply(0.1))
        .stroke(Stroke::new(1.0, p.red.gamma_multiply(0.4)))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let words = egui::RichText::new(title)
                .size(13.0)
                .strong()
                .color(p.text_0);
            let said = ui.add(egui::Label::new(words).wrap());
            widgets::address(ui, &said, "http-script-stop");
            let words = egui::RichText::new(message(stop, language))
                .monospace()
                .size(12.0)
                .color(p.red);
            let said = ui.add(egui::Label::new(words).wrap());
            widgets::address(ui, &said, "http-script-stop-message");
            let mut opened = false;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                if let Some(at) = where_words(stop, language) {
                    let words = egui::RichText::new(at).size(12.0).color(p.text_2);
                    let said = ui.add(egui::Label::new(words));
                    widgets::address(ui, &said, "http-script-stop-where");
                }
                if openable {
                    let words = i18n::translate(language, keys::http::scripts::stop::OPEN, &[]);
                    opened = Button::new(&words, "http-script-open").show(ui).clicked();
                }
            });
            opened
        })
        .inner
}
