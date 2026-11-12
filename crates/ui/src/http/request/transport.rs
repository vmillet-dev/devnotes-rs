//! How a request travels; each field left unset takes what is above it, shown beside it.

use std::collections::HashMap;

use egui::{Grid, Ui, Vec2};
use engine::http::transport::{Transport, TransportSettings};
use i18n::keys;

use crate::theme::palette;
use crate::widgets::{self, ChoiceMenu, TextField};

const NUMBER: f32 = 120.0;
/// The shortest a timeout may be set to, in milliseconds.
const SHORTEST_TIMEOUT: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Switch {
    Above,
    On,
    Off,
}

impl Switch {
    fn of(value: Option<bool>) -> Self {
        match value {
            None => Self::Above,
            Some(true) => Self::On,
            Some(false) => Self::Off,
        }
    }

    fn value(self) -> Option<bool> {
        match self {
            Self::Above => None,
            Self::On => Some(true),
            Self::Off => Some(false),
        }
    }
}

/// What is being typed in the numbers: a value is settled once its field is left, a timeout
/// typed digit by digit passing through values it would refuse.
pub(crate) type Typed = HashMap<&'static str, String>;

/// What the editor draws.
pub(crate) struct TransportField {
    pub(crate) settings: TransportSettings,
    /// What an unset field takes: the folders and the collection above, or the defaults.
    pub(crate) base: Transport,
    pub(crate) above_is_default: bool,
}

/// A row of the grid: what it is called, and what an unset field takes.
struct Row<'a> {
    label: i18n::Key,
    author_id: &'static str,
    above: &'a str,
}

/// The settings as edited this frame.
pub(crate) fn show(
    ui: &mut Ui,
    field: &TransportField,
    typed: &mut Typed,
    language: i18n::Language,
) -> Option<TransportSettings> {
    let tr = |key| i18n::translate(language, key, &[]);
    let base = field.base;
    let inherit = tr(if field.above_is_default {
        keys::http::transport::BY_DEFAULT
    } else {
        keys::http::transport::INHERITED
    });
    let yes_no = |value: bool| {
        tr(if value {
            keys::http::transport::ON
        } else {
            keys::http::transport::OFF
        })
    };
    let (on, off) = (yes_no(true), yes_no(false));
    let options = [
        (Switch::Above, inherit.as_str()),
        (Switch::On, on.as_str()),
        (Switch::Off, off.as_str()),
    ];
    let mut settings = field.settings;
    let mut changed = false;
    Grid::new("http-transport")
        .num_columns(3)
        .spacing(Vec2::new(16.0, 10.0))
        .show(ui, |ui| {
            let above = format!("{inherit} · {} ms", base.timeout_ms);
            let row = Row {
                label: keys::http::transport::TIMEOUT_MS,
                author_id: "http-transport-timeoutMs",
                above: &above,
            };
            changed |= number(
                ui,
                (&row, language),
                (&mut settings.timeout_ms, base.timeout_ms, SHORTEST_TIMEOUT),
                typed,
            );
            let above = format!("{inherit} · {}", yes_no(base.follow_redirects));
            let row = Row {
                label: keys::http::transport::FOLLOW_REDIRECTS,
                author_id: "http-transport-followRedirects",
                above: &above,
            };
            changed |= switch(
                ui,
                (&row, language),
                &options,
                &mut settings.follow_redirects,
            );
            let above = format!("{inherit} · {}", base.max_redirects);
            let row = Row {
                label: keys::http::transport::MAX_REDIRECTS,
                author_id: "http-transport-maxRedirects",
                above: &above,
            };
            changed |= number(
                ui,
                (&row, language),
                (&mut settings.max_redirects, base.max_redirects, 0),
                typed,
            );
            let above = format!("{inherit} · {}", yes_no(base.verify_tls));
            let row = Row {
                label: keys::http::transport::VERIFY_TLS,
                author_id: "http-transport-verifyTls",
                above: &above,
            };
            changed |= switch(ui, (&row, language), &options, &mut settings.verify_tls);
            let above = format!("{inherit} · {}", yes_no(base.use_cookies));
            let row = Row {
                label: keys::http::transport::USE_COOKIES,
                author_id: "http-transport-useCookies",
                above: &above,
            };
            changed |= switch(ui, (&row, language), &options, &mut settings.use_cookies);
        });
    if !settings.verify_tls.unwrap_or(base.verify_tls) {
        insecure(ui, language);
    }
    changed.then_some(settings)
}

fn label(ui: &mut Ui, row: &Row<'_>, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, row.label, &[]);
    let words = egui::RichText::new(words).size(12.5).color(p.text_1);
    ui.add(egui::Label::new(words));
}

/// What an unset field takes, said beside it; nothing once it is set. Ends the row.
fn from(ui: &mut Ui, row: &Row<'_>, set: bool) {
    let p = palette(ui.ctx());
    let words = if set { "" } else { row.above };
    let words = egui::RichText::new(words).size(12.0).color(p.text_2);
    let shown = ui.add(egui::Label::new(words));
    widgets::address(ui, &shown, "http-transport-from");
    ui.end_row();
}

fn switch(
    ui: &mut Ui,
    (row, language): (&Row<'_>, i18n::Language),
    options: &[(Switch, &str); 3],
    value: &mut Option<bool>,
) -> bool {
    label(ui, row, language);
    let name = i18n::translate(language, row.label, &[]);
    let mut chosen = Switch::of(*value);
    let changed = ChoiceMenu::new(&name, row.author_id, options)
        .show(ui, &mut chosen)
        .changed();
    *value = chosen.value();
    from(ui, row, value.is_some());
    changed
}

fn number(
    ui: &mut Ui,
    (row, language): (&Row<'_>, i18n::Language),
    (value, above, shortest): (&mut Option<u32>, u32, u32),
    typed: &mut Typed,
) -> bool {
    label(ui, row, language);
    let name = i18n::translate(language, row.label, &[]);
    let mut text = typed
        .get(row.author_id)
        .cloned()
        .unwrap_or_else(|| value.map_or_else(String::new, |value| value.to_string()));
    let hint = above.to_string();
    let response = ui
        .vertical(|ui| {
            ui.set_width(NUMBER);
            TextField::new(&mut text, &name, row.author_id)
                .compact()
                .hint(&hint)
                .show(ui)
        })
        .inner;
    if response.changed() {
        typed.insert(row.author_id, text);
    }
    let mut changed = false;
    if response.lost_focus()
        && let Some(text) = typed.remove(row.author_id)
    {
        let settled = text.trim().parse::<u32>().ok().map(|n| n.max(shortest));
        changed = settled != *value;
        *value = settled;
    }
    from(ui, row, value.is_some());
    changed
}

fn insecure(ui: &mut Ui, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, keys::http::transport::INSECURE, &[]);
    let words = egui::RichText::new(words).size(12.5).color(p.amber_text);
    let shown = ui.add(egui::Label::new(words).wrap());
    widgets::address(ui, &shown, "http-transport-insecure");
}

/// How many fields the request sets: the number its tab wears.
pub(super) fn set_fields(settings: &TransportSettings) -> usize {
    [
        settings.timeout_ms.is_some(),
        settings.follow_redirects.is_some(),
        settings.max_redirects.is_some(),
        settings.verify_tls.is_some(),
        settings.use_cookies.is_some(),
    ]
    .iter()
    .filter(|set| **set)
    .count()
}
