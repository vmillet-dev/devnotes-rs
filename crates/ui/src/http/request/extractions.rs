//! The rules that keep a value of the answer in a variable after each send. A rule is made from
//! the answer's explorer; here it is read, and removed.

use egui::{Ui, Vec2};
use engine::http::model::Extraction;
use i18n::keys;

use crate::state::http::tabs::Tab;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, IconButton};

/// The rules left, when one was removed.
pub(super) fn show(ui: &mut Ui, tab: &Tab, language: i18n::Language) -> Option<Vec<Extraction>> {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let rules = &tab.draft.document.extractions;
    let head = format!(
        "{} · {}",
        tr(keys::http::extract::LIST_TITLE),
        tr(keys::http::extract::LIST_HINT)
    );
    let head = egui::RichText::new(head).size(12.0).color(p.text_2);
    let shown = ui.add(egui::Label::new(head));
    widgets::address(ui, &shown, "http-extractions");
    let mut removed = None;
    for (at, rule) in rules.iter().enumerate() {
        if row(ui, rule, language) {
            removed = Some(at);
        }
    }
    removed.map(|at| {
        let mut rest = rules.clone();
        rest.remove(at);
        rest
    })
}

/// `body.token → {{token}}`, and the cross that removes it; `true` on a click.
fn row(ui: &mut Ui, rule: &Extraction, language: i18n::Language) -> bool {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
        let mono =
            |words: String, ink| egui::RichText::new(words).monospace().size(12.5).color(ink);
        let path = ui.add(egui::Label::new(mono(rule.path.clone(), p.text_1)));
        widgets::address(ui, &path, "http-extraction");
        let (arrow, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
        icons::paint(ui, arrow, Icon::ArrowRight, p.text_2);
        let written = format!("{{{{{}}}}}", rule.variable);
        ui.add(egui::Label::new(mono(written, p.amber_text)));
        if rule.secret {
            let words = i18n::translate(language, keys::http::extract::SECRET_SHORT, &[]);
            let words = egui::RichText::new(words).size(11.0).color(p.text_2);
            ui.add(egui::Label::new(words));
        }
        let label = i18n::translate(
            language,
            keys::http::extract::REMOVE,
            &[("variable", rule.variable.as_str().into())],
        );
        IconButton::new(Icon::X, &label, "http-extraction-remove")
            .show(ui)
            .clicked()
    })
    .inner
}
