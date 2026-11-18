//! The library's cookie jar: what answers set, by domain, to read and to throw away.

use crate::theme::tokens::{font, gap};
use egui::{Align, Grid, Layout, Ui, Vec2};
use engine::http::cookies::JarCookie;
use engine::http::cookies::store::{CookieDomain, StoredCookie};
use i18n::keys;

use super::{Action, modal};
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::state::http::cookies::Asked as Ask;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, IconButton, InPlace, Kind};

const WIDTH: f32 = 720.0;

fn ask(actions: &mut Actions, asked: Ask) {
    actions.push(Asked::Http(Action::Cookies(asked)));
}

pub(super) fn show(ui: &Ui, state: &AppState, actions: &mut Actions) {
    if !state.http.cookies().is_open() {
        return;
    }
    let shown = modal(
        ui,
        (
            "http-cookies-dialog",
            (
                &i18n::translate(state.language, i18n::keys::dialog::CLOSE, &[]),
                "http-cookies-close",
            ),
        ),
        |ui| {
            body(ui, state, actions);
        },
    );
    if shown.should_close() {
        ask(actions, Ask::Close);
    }
}

fn body(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = gap::INSET;
    head(ui, state, actions);
    let hint = egui::RichText::new(tr(keys::http::cookies::HINT))
        .size(font::BODY)
        .color(p.text_2);
    ui.add(egui::Label::new(hint).wrap());
    let height = ui.ctx().content_rect().height() * 0.6;
    egui::ScrollArea::vertical()
        .id_salt("http-cookies")
        .max_height(height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let domains = state.http.cookies().domains().unwrap_or(&[]);
            for domain in domains {
                section(ui, domain, language, actions);
            }
            if state.http.cookies().domains().is_some() && domains.is_empty() {
                let words = egui::RichText::new(tr(keys::http::cookies::EMPTY))
                    .size(font::BODY)
                    .color(p.text_2);
                let said = ui.add(egui::Label::new(words));
                widgets::address(ui, &said, "http-cookies-empty");
            }
        });
    ui.add_space(gap::LINE);
}

fn head(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal(|ui| {
        let title = tr(keys::http::cookies::TITLE);
        crate::widgets::dialog_title(ui, &title, "http-cookies-dialog");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = gap::CONTROLS;
            let cookies = state.http.cookies();
            let question = cookies.confirming().map(|count| {
                i18n::translate(
                    language,
                    keys::http::cookies::CONFIRM,
                    &[("count", count.into())],
                )
            });
            let stored = cookies.domains().is_some_and(|it| !it.is_empty());
            let words = (
                &*tr(keys::http::cookies::CLEAR),
                &*tr(keys::http::rail::CANCEL),
            );
            let pressed =
                widgets::ask_in_place(ui, words, question.as_deref(), stored, "http-cookies-clear");
            match pressed {
                Some(InPlace::Ask) => ask(actions, Ask::AskClear),
                Some(InPlace::Cancel) => ask(actions, Ask::CancelClear),
                Some(InPlace::Confirm) => ask(actions, Ask::Clear),
                None => {}
            }
        });
    });
}

fn section(ui: &mut Ui, domain: &CookieDomain, language: i18n::Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap::INSET;
        let name = egui::RichText::new(&domain.domain)
            .monospace()
            .size(font::READING)
            .color(p.text_0);
        let said = ui.add(egui::Label::new(name));
        widgets::address(ui, &said, "http-cookies-domain");
        let count = u32::try_from(domain.cookies.len()).unwrap_or(u32::MAX);
        let count = i18n::translate(
            language,
            keys::http::cookies::COUNT,
            &[("count", count.into())],
        );
        ui.add(egui::Label::new(
            egui::RichText::new(count).size(font::LABEL).color(p.text_2),
        ));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let word = tr(keys::http::cookies::REMOVE_DOMAIN);
            if Button::new(&word, "http-cookies-domain-remove")
                .kind(Kind::Destructive)
                .show(ui)
                .clicked()
            {
                ask(actions, Ask::RemoveDomain(domain.domain.clone()));
            }
        });
    });
    Grid::new(("http-cookies-of", &domain.domain))
        .num_columns(6)
        .spacing(Vec2::new(14.0, 6.0))
        .show(ui, |ui| {
            for key in [
                keys::http::cookies::NAME,
                keys::http::cookies::VALUE,
                keys::http::cookies::PATH,
                keys::http::cookies::EXPIRES,
                keys::http::cookies::FLAGS,
            ] {
                let words = egui::RichText::new(tr(key))
                    .size(font::CONTROL)
                    .strong()
                    .color(p.text_2);
                ui.add(egui::Label::new(words));
            }
            ui.end_row();
            for stored in &domain.cookies {
                row(ui, stored, language, actions);
            }
        });
    ui.add_space(gap::SNUG);
}

fn mono(ui: &mut Ui, words: &str, author_id: Option<&str>) {
    let p = palette(ui.ctx());
    let words = egui::RichText::new(words)
        .monospace()
        .size(font::LABEL)
        .color(p.text_0);
    let shown = ui.add(egui::Label::new(words).truncate());
    if let Some(author_id) = author_id {
        widgets::address(ui, &shown, author_id);
    }
}

fn row(ui: &mut Ui, stored: &StoredCookie, language: i18n::Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let cookie = &stored.cookie;
    mono(ui, &cookie.name, Some("http-cookie"));
    ui.scope(|ui| {
        ui.set_max_width(220.0);
        mono(ui, &cookie.value, None);
    });
    mono(ui, &cookie.path, None);
    let expiry = egui::RichText::new(expiry(cookie, language))
        .size(font::LABEL)
        .color(p.text_1);
    ui.add(egui::Label::new(expiry));
    let flags = egui::RichText::new(flags(cookie, language))
        .size(font::CONTROL)
        .color(p.text_2);
    ui.add(egui::Label::new(flags));
    let label = i18n::translate(
        language,
        keys::http::cookies::REMOVE_ONE,
        &[("name", cookie.name.as_str().into())],
    );
    if IconButton::new(Icon::X, &label, "http-cookie-remove")
        .show(ui)
        .clicked()
    {
        ask(actions, Ask::Remove(stored.id.clone()));
    }
    ui.end_row();
}

fn expiry(cookie: &JarCookie, language: i18n::Language) -> String {
    cookie.expires.map_or_else(
        || i18n::translate(language, keys::http::cookies::SESSION, &[]),
        |expires| expires.format("%Y-%m-%d %H:%M").to_string(),
    )
}

fn flags(cookie: &JarCookie, language: i18n::Language) -> String {
    let mut flags = Vec::new();
    if cookie.secure {
        flags.push("Secure".to_owned());
    }
    if cookie.http_only {
        flags.push("HttpOnly".to_owned());
    }
    if let Some(same_site) = &cookie.same_site {
        flags.push(format!("SameSite={same_site}"));
    }
    if cookie.host_only {
        flags.push(i18n::translate(
            language,
            keys::http::cookies::HOST_ONLY,
            &[],
        ));
    }
    flags.join(" · ")
}
