//! The question before the window goes with requests not saved in it. Drawn by the window, since
//! any area can be the one on screen.

use egui::{Align, FontFamily, Layout, Sense, Ui};
use i18n::keys;

use super::{Action, View, modal, save_request};
use crate::action::{Action as Asked, Actions};
use crate::state::http::leave::Asked as Ask;
use crate::state::{AppState, Area};
use crate::theme::palette;
use crate::widgets::{self, Button, Kind};

const WIDTH: f32 = 420.0;

pub(crate) fn show(ui: &Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let http = &state.http;
    if !http.leave().is_asking() {
        return;
    }
    // « Save all » reaches a request never saved: it is asked where it goes.
    if http.tabs().asking_place().is_some() {
        if state.settings.area != Area::Http {
            save_request::show(ui, state, &mut view.save_form, actions);
        }
        return;
    }
    let shown = modal(ui, "http-leave-dialog", |ui| body(ui, state));
    let mut answer = |asked| actions.push(Asked::Http(Action::Leave(asked)));
    match shown.inner {
        Some(asked) => answer(asked),
        None if shown.should_close() => answer(Ask::Cancel),
        None => {}
    }
}

fn body(ui: &mut Ui, state: &AppState) -> Option<Ask> {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let names: Vec<&str> = state
        .http
        .modified()
        .map(|tab| tab.draft.name.as_str())
        .collect();
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = 10.0;
    let count = u32::try_from(names.len()).unwrap_or(u32::MAX);
    let title = i18n::translate(
        language,
        keys::http::leave::TITLE,
        &[("count", count.into())],
    );
    let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
    let (rect, heading) = ui.allocate_exact_size(words.size(), Sense::hover());
    ui.painter().galley(rect.min, words, p.text_0);
    heading.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, &title));
    widgets::address(ui, &heading, "http-leave-dialog");
    let hint = egui::RichText::new(tr(keys::http::leave::hint::QUIT))
        .size(12.5)
        .color(p.text_2);
    ui.add(egui::Label::new(hint).wrap());
    for name in names {
        let words = egui::RichText::new(name).size(12.5).color(p.text_0);
        let said = ui.add(egui::Label::new(words).truncate());
        widgets::address(ui, &said, "http-leave-name");
    }
    let connected = state.http.connected();
    if connected > 0 {
        let words = i18n::translate(
            language,
            keys::http::leave::CONNECTED,
            &[("count", connected.into())],
        );
        let words = egui::RichText::new(words).size(12.5).color(p.text_2);
        let said = ui.add(egui::Label::new(words).wrap());
        widgets::address(ui, &said, "http-leave-connected");
    }
    let mut chosen = None;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if Button::new(&tr(keys::http::leave::SAVE_ALL), "http-leave-save")
            .kind(Kind::Filled)
            .enabled(!state.http.leave().is_saving())
            .show(ui)
            .clicked()
        {
            chosen = Some(Ask::SaveAll);
        }
        if Button::new(&tr(keys::http::leave::discard::QUIT), "http-leave-discard")
            .kind(Kind::Destructive)
            .show(ui)
            .clicked()
        {
            chosen = Some(Ask::Discard);
        }
        if Button::new(&tr(keys::http::rail::CANCEL), "http-leave-cancel")
            .show(ui)
            .clicked()
        {
            chosen = Some(Ask::Cancel);
        }
    });
    chosen
}
