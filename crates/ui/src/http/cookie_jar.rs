//! The library's cookie jar: what answers set, by domain, to read and to throw away.

use egui::{Align, FontFamily, Grid, Layout, Sense, Ui, Vec2};
use engine::http::cookies::JarCookie;
use engine::http::cookies::store::{CookieDomain, StoredCookie};
use i18n::keys;

use super::{Action, modal};
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::state::http::cookies::Asked as Ask;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, IconButton, Kind};

const WIDTH: f32 = 720.0;

fn ask(actions: &mut Actions, asked: Ask) {
    actions.push(Asked::Http(Action::Cookies(asked)));
}

pub(super) fn show(ui: &Ui, state: &AppState, actions: &mut Actions) {
    if !state.http.cookies().is_open() {
        return;
    }
    let shown = modal(ui, "http-cookies-dialog", |ui| {
        body(ui, state, actions);
    });
    if shown.should_close() {
        ask(actions, Ask::Close);
    }
}

fn body(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = 10.0;
    head(ui, state, actions);
    let hint = egui::RichText::new(tr(keys::http::cookies::HINT))
        .size(12.5)
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
                    .size(12.5)
                    .color(p.text_2);
                let said = ui.add(egui::Label::new(words));
                widgets::address(ui, &said, "http-cookies-empty");
            }
        });
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if Button::new(&tr(keys::http::cookies::CLOSE), "http-cookies-close")
            .show(ui)
            .clicked()
        {
            ask(actions, Ask::Close);
        }
    });
}

fn head(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal(|ui| {
        let title = tr(keys::http::cookies::TITLE);
        let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
        let (rect, heading) = ui.allocate_exact_size(words.size(), Sense::hover());
        ui.painter().galley(rect.min, words, p.text_0);
        heading.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, &title));
        widgets::address(ui, &heading, "http-cookies-dialog");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let word = tr(keys::http::cookies::CLEAR);
            let Some(count) = state.http.cookies().confirming() else {
                let stored = state
                    .http
                    .cookies()
                    .domains()
                    .is_some_and(|it| !it.is_empty());
                if Button::new(&word, "http-cookies-clear")
                    .enabled(stored)
                    .show(ui)
                    .clicked()
                {
                    ask(actions, Ask::AskClear);
                }
                return;
            };
            if Button::new(&tr(keys::http::rail::CANCEL), "http-cookies-clear-cancel")
                .show(ui)
                .clicked()
            {
                ask(actions, Ask::CancelClear);
            }
            if Button::new(&word, "http-cookies-clear-confirm")
                .kind(Kind::Destructive)
                .show(ui)
                .clicked()
            {
                ask(actions, Ask::Clear);
            }
            let question = i18n::translate(
                language,
                keys::http::cookies::CONFIRM,
                &[("count", count.into())],
            );
            let question = egui::RichText::new(question).size(12.5).color(p.text_1);
            let said = ui.add(egui::Label::new(question));
            widgets::address(ui, &said, "http-cookies-clear-count");
        });
    });
}

fn section(ui: &mut Ui, domain: &CookieDomain, language: i18n::Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let name = egui::RichText::new(&domain.domain)
            .monospace()
            .size(13.0)
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
            egui::RichText::new(count).size(12.0).color(p.text_2),
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
                    .size(11.5)
                    .strong()
                    .color(p.text_2);
                ui.add(egui::Label::new(words));
            }
            ui.end_row();
            for stored in &domain.cookies {
                row(ui, stored, language, actions);
            }
        });
    ui.add_space(6.0);
}

fn mono(ui: &mut Ui, words: &str, author_id: Option<&str>) {
    let p = palette(ui.ctx());
    let words = egui::RichText::new(words)
        .monospace()
        .size(12.0)
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
        .size(12.0)
        .color(p.text_1);
    ui.add(egui::Label::new(expiry));
    let flags = egui::RichText::new(flags(cookie, language))
        .size(11.5)
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
