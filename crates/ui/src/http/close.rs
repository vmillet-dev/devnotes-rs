//! A tab is closed after one question when closing it costs something: what was typed and not
//! saved. Escape and « Cancel » keep it open.

use egui::{FontFamily, Sense, Ui};
use i18n::keys;

use super::Action;
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::theme::palette;
use crate::widgets::{self, Button, Kind};

const WIDTH: f32 = 420.0;

pub(super) fn show(ui: &Ui, state: &AppState, actions: &mut Actions) {
    let Some(tab) = state.http.tabs().closing() else {
        return;
    };
    let key = tab.key.clone();
    let shown = super::modal(ui, "http-close-dialog", |ui| {
        body(ui, &tab.draft.name, state.language)
    });
    let mut close = |action| actions.push(Asked::Http(action));
    match shown.inner {
        Some(Choice::Discard) => close(Action::Close(key)),
        Some(Choice::Save) => close(Action::SaveAndClose(key)),
        Some(Choice::Cancel) => close(Action::CancelClose),
        None if shown.should_close() => close(Action::CancelClose),
        None => {}
    }
}

enum Choice {
    Cancel,
    Discard,
    Save,
}

fn body(ui: &mut Ui, name: &str, language: i18n::Language) -> Option<Choice> {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = 10.0;
    let title = i18n::translate(language, keys::http::close::TITLE, &[("name", name.into())]);
    let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
    let (rect, heading) = ui.allocate_exact_size(words.size(), Sense::hover());
    ui.painter().galley(rect.min, words, p.text_0);
    heading.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, &title));
    widgets::address(ui, &heading, "http-close-dialog");
    let hint = egui::RichText::new(tr(keys::http::close::HINT))
        .size(12.5)
        .color(p.text_2);
    let hint = ui.add(egui::Label::new(hint).wrap());
    widgets::address(ui, &hint, "http-close-modified");
    let mut chosen = None;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if Button::new(&tr(keys::http::editor::SAVE), "http-close-save")
            .kind(Kind::Filled)
            .show(ui)
            .clicked()
        {
            chosen = Some(Choice::Save);
        }
        if Button::new(&tr(keys::http::close::DISCARD), "http-close-discard")
            .kind(Kind::Destructive)
            .show(ui)
            .clicked()
        {
            chosen = Some(Choice::Discard);
        }
        if Button::new(&tr(keys::http::rail::CANCEL), "http-close-cancel")
            .show(ui)
            .clicked()
        {
            chosen = Some(Choice::Cancel);
        }
    });
    chosen
}
