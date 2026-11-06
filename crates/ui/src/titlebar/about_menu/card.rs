//! Purely informative: the application, its version and who made it.

use egui::{Align, FontFamily, Layout, RichText, Sense, Ui};
use engine::app_info::METADATA;
use i18n::{Key, Message, keys};

use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::theme::palette;
use crate::titlebar::Action;
use crate::widgets::{self, Button};

const WIDTH: f32 = 440.0;
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `true` when it asks to close.
pub(super) fn show(ctx: &egui::Context, state: &AppState, actions: &mut Actions) -> bool {
    let shown = widgets::dialog(ctx, &state.dialogs, super::id(), WIDTH, |ui| {
        body(ui, state, actions)
    });
    shown.is_some_and(|shown| shown.inner || shown.should_close())
}

/// The version the lock file holds: nothing else knows it at compile time.
fn egui_version() -> Option<&'static str> {
    let lock = include_str!("../../../../../Cargo.lock");
    let (_, after) = lock.split_once("name = \"egui\"\nversion = \"")?;
    after.split('"').next()
}

fn body(ui: &mut Ui, state: &AppState, actions: &mut Actions) -> bool {
    let language = state.language;
    let p = palette(ui.ctx());
    let tr = |key: Key| i18n::translate(language, key, &[]);
    ui.spacing_mut().item_spacing.y = 8.0;
    let name = Message::new(keys::about::TITLE)
        .with("app", METADATA.name)
        .text(language);
    ui.horizontal(|ui| {
        let words = widgets::text(ui, &name, 17.0, 700.0, FontFamily::Proportional, p.text_0);
        let (rect, title) = ui.allocate_exact_size(words.size(), Sense::hover());
        ui.painter().galley(rect.min, words, p.text_0);
        widgets::address(ui, &title, "about-dialog");
        let version = ui.label(
            RichText::new(format!("v{VERSION}"))
                .size(11.5)
                .monospace()
                .color(p.text_2),
        );
        widgets::address(ui, &version, "about-version");
    });
    let description = RichText::new(tr(keys::about::DESCRIPTION))
        .size(12.5)
        .color(p.text_1);
    ui.add(egui::Label::new(description).wrap());
    ui.add_space(4.0);
    let developer = format!("{} ({})", METADATA.author, METADATA.author_handle);
    fact(ui, &tr(keys::about::DEVELOPED_BY), |ui| {
        ui.label(RichText::new(developer).size(12.0).color(p.text_1));
    });
    fact(ui, &tr(keys::about::REPOSITORY), |ui| {
        let label = METADATA.repository.trim_start_matches("https://");
        if Button::new(label, "about-repository").show(ui).clicked() {
            actions.push(Asked::Menu(Action::OpenLink(
                METADATA.repository.to_owned(),
            )));
        }
    });
    let stack = format!(
        "egui {} · Rust {}",
        egui_version().unwrap_or("—"),
        METADATA.rust_version
    );
    fact(ui, &tr(keys::about::BUILT_WITH), |ui| {
        ui.label(RichText::new(stack).size(12.0).color(p.text_1));
    });
    ui.add_space(6.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        Button::new(&tr(keys::about::CLOSE), "about-close")
            .show(ui)
            .clicked()
    })
    .inner
}

fn fact(ui: &mut Ui, name: &str, value: impl FnOnce(&mut Ui)) {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(110.0, 18.0), Sense::hover());
        let words = widgets::text(ui, name, 11.5, 400.0, FontFamily::Proportional, p.text_2);
        ui.painter().galley(rect.min, words, p.text_2);
        value(ui);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lock_file_says_which_egui_this_is() {
        let version = egui_version().unwrap();

        assert!(version.starts_with("0."), "{version}");
    }
}
