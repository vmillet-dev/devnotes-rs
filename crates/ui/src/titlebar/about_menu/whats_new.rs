//! The changelog shipped with the application: the core cuts it into releases and entries, so
//! there is no Markdown to render here.

use egui::{Align, Layout, RichText, Ui, Vec2};
use engine::app_info::METADATA;
use engine::changelog::model::ChangelogRelease;
use i18n::{Key, Language, keys};

use super::framed;
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::theme::palette;
use crate::titlebar::Action;
use crate::widgets::{self, Button};

const SIZE: Vec2 = Vec2::new(640.0, 560.0);
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(super) fn show(ctx: &egui::Context, state: &AppState, actions: &mut Actions) -> bool {
    let language = state.language;
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let mut close = false;
    let shown = widgets::framed_dialog(ctx, &state.dialogs, super::id(), SIZE, |ui| {
        let (title, subtitle) = (tr(keys::whats_new::TITLE), tr(keys::whats_new::SUBTITLE));
        framed(
            ui,
            (&title, &subtitle),
            |ui| body(ui, state),
            |ui| footer(ui, language, actions, &mut close),
        );
    });
    close || shown.is_some_and(|shown| shown.should_close())
}

fn body(ui: &mut Ui, state: &AppState) {
    let language = state.language;
    let said = |ui: &mut Ui, key| {
        let words = RichText::new(i18n::translate(language, key, &[]))
            .size(12.5)
            .color(palette(ui.ctx()).text_2);
        ui.label(words);
    };
    if state.help.is_loading() {
        said(ui, keys::whats_new::LOADING);
    } else if state.help.has_failed() {
        said(ui, keys::whats_new::UNAVAILABLE);
    } else if state.help.releases().is_empty() {
        said(ui, keys::whats_new::EMPTY);
    }
    for release in state.help.releases() {
        ui.push_id(&release.version, |ui| self::release(ui, release, language));
        ui.add_space(14.0);
    }
}

fn release(ui: &mut Ui, release: &ChangelogRelease, language: Language) {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let version = ui.label(
            RichText::new(&release.version)
                .size(13.0)
                .variation(*b"wght", 600.0)
                .color(p.text_0),
        );
        widgets::address(ui, &version, "news-version");
        if let Some(date) = &release.date {
            ui.label(RichText::new(date).size(11.5).monospace().color(p.text_2));
        }
        if release.version == VERSION {
            let installed = i18n::translate(language, keys::whats_new::INSTALLED, &[]);
            let badge = widgets::hue_badge(ui, &installed, p.amber_text, None);
            widgets::address(ui, &badge, "news-installed");
        }
    });
    widgets::release_notes(ui, release);
}

fn footer(ui: &mut Ui, language: Language, actions: &mut Actions, close: &mut bool) {
    let url = format!("{}/releases", METADATA.repository);
    let label = url.trim_start_matches("https://");
    if Button::new(label, "news-releases").show(ui).clicked() {
        actions.push(Asked::Menu(Action::OpenLink(url.clone())));
    }
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let words = i18n::translate(language, keys::whats_new::CLOSE, &[]);
        *close = Button::new(&words, "news-close").show(ui).clicked();
    });
}
