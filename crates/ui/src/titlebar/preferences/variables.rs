//! The corpus's own variables. They are data and not a preference, but they are edited under the
//! same footer: a panel where one page commits on blur and the others wait for a button would be
//! worse than either rule alone. The rows live in the view until a button sends them.

use egui::{RichText, Ui};
use i18n::{Key, Message, keys};

use crate::api;
use crate::state::AppState;
use crate::state::security::{Variable, duplicates};
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, IconButton, TextField};

pub(super) fn page(ui: &mut Ui, state: &AppState, rows: &mut Option<Vec<Variable>>) {
    use keys::settings::variables;
    let language = state.language;
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let p = palette(ui.ctx());
    if rows.is_none() {
        rows.clone_from(&state.security.variables().map(<[Variable]>::to_vec));
    }
    let intro = RichText::new(tr(variables::INTRO))
        .size(12.0)
        .color(p.text_2);
    ui.add(egui::Label::new(intro).wrap());
    ui.add_space(10.0);
    let Some(rows) = rows else {
        return;
    };
    if rows.is_empty() {
        let none = RichText::new(tr(variables::EMPTY))
            .size(12.0)
            .color(p.text_2);
        ui.label(none);
    }
    let mut removed = None;
    let twice = duplicates(rows);
    for (at, row) in rows.iter_mut().enumerate() {
        let rejected = (!row.name.is_empty() && !api::notes::is_field_name(&row.name))
            || twice.contains(&row.name);
        ui.push_id(at, |ui| {
            if line(ui, state, row, rejected) {
                removed = Some(at);
            }
        });
        ui.add_space(6.0);
    }
    if let Some(at) = removed {
        rows.remove(at);
    }
    if !twice.is_empty() {
        let said = Message::new(variables::DUPLICATE)
            .with("names", twice.join(", "))
            .text(language);
        ui.label(RichText::new(said).size(11.5).color(p.red));
    }
    ui.add_space(4.0);
    if Button::new(&tr(variables::ADD), "variable-add")
        .icon(Icon::Plus)
        .show(ui)
        .clicked()
    {
        rows.push(Variable::default());
    }
    ui.add_space(8.0);
    let rule = RichText::new(tr(variables::NAME_RULE))
        .size(11.5)
        .color(p.text_2);
    ui.add(egui::Label::new(rule).wrap());
}

/// `{{ name }}`, its value, and the cross that removes it: `true` when it was pressed.
fn line(ui: &mut Ui, state: &AppState, row: &mut Variable, rejected: bool) -> bool {
    use keys::settings::variables;
    let language = state.language;
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let p = palette(ui.ctx());
    let shown = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let brace = |text: &str| RichText::new(text).size(12.5).monospace().color(p.text_2);
        ui.label(brace("{{"));
        let (name, placeholder) = (tr(variables::NAME), tr(variables::NAME_PLACEHOLDER));
        ui.scope(|ui| {
            ui.set_width(150.0);
            TextField::new(&mut row.name, &name, "variable-name")
                .hint(&placeholder)
                .compact()
                .invalid(rejected)
                .show(ui);
        });
        ui.label(brace("}}"));
        let (value, placeholder) = (tr(variables::VALUE), tr(variables::VALUE_PLACEHOLDER));
        ui.scope(|ui| {
            ui.set_width((ui.available_width() - 32.0).max(80.0));
            TextField::new(&mut row.value, &value, "variable-value")
                .hint(&placeholder)
                .compact()
                .show(ui);
        });
        let label = Message::new(variables::REMOVE)
            .with(
                "name",
                if row.name.is_empty() {
                    "—"
                } else {
                    &row.name
                },
            )
            .text(language);
        IconButton::new(Icon::X, &label, "variable-remove")
            .show(ui)
            .clicked()
    });
    widgets::address(ui, &shown.response, "variable-row");
    shown.inner
}
