//! The request's `{{variables}}` under its URL, each in the colour of where it resolves from.
//! Pointed at or focused, a chip opens its card, as the reference's: its name, where it comes
//! from, its value — a secret masked until asked —, what it hides, or that nothing sets it.

use crate::theme::tokens::{font, gap, radius};
use egui::{CornerRadius, FontFamily, Id, Popup, RectAlign, Response, Role, Sense, Ui, Vec2};
use engine::http::environments::VariableKind;
use engine::http::variables::{ScopedVariable, UsedVariable, VariableScope};
use i18n::keys;

use super::Action;
use super::url;
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::theme::palette;
use crate::widgets::{self, Button, interact};

/// A secret's value, until « Show » is pressed.
const MASK: &str = "••••••••";
const CARD_WIDTH: f32 = 320.0;

pub(super) fn show(ui: &mut Ui, state: &AppState, used: &[UsedVariable], actions: &mut Actions) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(gap::SNUG);
        for known in used {
            chip(ui, state, known, actions);
        }
    });
}

fn chip(ui: &mut Ui, state: &AppState, known: &UsedVariable, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let scope = known.resolved.as_ref().map(|found| found.scope);
    let hue = url::hue(p, scope);
    let written = format!("{{{{{}}}}}", known.name);
    let galley = widgets::text(ui, &written, font::LABEL, 400.0, FontFamily::Monospace, hue);
    let response = interact(
        ui,
        galley.size() + Vec2::new(16.0, 8.0),
        Sense::click(),
        "http-used-variable",
        || egui::WidgetInfo::labeled(Role::Label, true, &written),
        CornerRadius::same(radius::CONTROL),
    );
    let rect = response.rect;
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(radius::CONTROL),
        hue.gamma_multiply(0.16),
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, hue);
    card(ui, &response, state, known, actions);
    if response.clicked() && known.resolved.is_none() {
        actions.push(Asked::Http(Action::Define(known.name.clone())));
    }
}

/// Open while the chip is pointed at or focused, and while the pointer is where the card was
/// drawn, so its button can be reached.
fn card(ui: &Ui, chip: &Response, state: &AppState, known: &UsedVariable, actions: &mut Actions) {
    let card_id = chip.id.with("card");
    let drawn: Option<egui::Rect> = ui.data(|data| data.get_temp(card_id));
    let pointer = ui.ctx().pointer_hover_pos();
    let kept = drawn
        .zip(pointer)
        .is_some_and(|(rect, at)| rect.contains(at));
    let open = chip.hovered() || chip.has_focus() || kept;
    let shown = Popup::from_response(chip)
        .id(card_id)
        .open(open)
        .align(RectAlign::BOTTOM_START)
        .gap(0.0)
        .frame(widgets::menu_frame(ui.ctx()).inner_margin(gap::margin(gap::GROUPS, gap::INSET)))
        .show(|ui| {
            ui.set_max_width(CARD_WIDTH);
            ui.spacing_mut().item_spacing = Vec2::new(gap::SNUG, gap::SNUG);
            match &known.resolved {
                Some(found) => resolved(ui, state, found, card_id),
                None => unset(ui, state, &known.name, actions),
            }
        });
    ui.data_mut(|data| {
        if let Some(shown) = shown {
            data.insert_temp(card_id, shown.response.rect);
        } else {
            // Left, a secret shown is masked again.
            data.remove::<egui::Rect>(card_id);
            data.remove::<bool>(card_id.with("revealed"));
        }
    });
}

fn resolved(ui: &mut Ui, state: &AppState, found: &ScopedVariable, card_id: Id) {
    let p = palette(ui.ctx());
    let language = state.language;
    let secret = found.kind == VariableKind::Secret;
    let mut from = from_words(language, found);
    if secret {
        from = format!(
            "{from} · {}",
            i18n::translate(language, keys::http::variables::used::SECRET, &[])
        );
    }
    head(ui, &found.name, &from, p.text_2);
    let revealed_id = card_id.with("revealed");
    let revealed: bool = ui.data(|data| data.get_temp(revealed_id)).unwrap_or(false);
    ui.horizontal(|ui| {
        let (words, family) = if found.scope == VariableScope::Dynamic {
            let made = i18n::translate(language, keys::http::variables::used::MADE, &[]);
            (made, FontFamily::Proportional)
        } else if secret && !revealed {
            (MASK.to_owned(), FontFamily::Monospace)
        } else {
            (found.value.clone(), FontFamily::Monospace)
        };
        let value = egui::RichText::new(&words)
            .family(family)
            .size(font::LABEL)
            .color(p.text_0);
        let shown = ui.add(egui::Label::new(value).wrap());
        widgets::address(ui, &shown, "http-used-variable-value");
        if secret {
            let key = if revealed {
                keys::http::variables::used::HIDE
            } else {
                keys::http::variables::used::REVEAL
            };
            let label = i18n::translate(language, key, &[]);
            if Button::new(&label, "http-used-variable-reveal")
                .show(ui)
                .clicked()
            {
                ui.data_mut(|data| data.insert_temp(revealed_id, !revealed));
            }
        }
    });
    if !found.shadows.is_empty() {
        let hidden: Vec<String> = found
            .shadows
            .iter()
            .filter_map(|scope| {
                i18n::Key::find(&format!("http.variables.used.scopes.{}", scope.as_str()))
            })
            .map(|key| i18n::translate(language, key, &[]))
            .collect();
        let words = format!(
            "{} {}",
            i18n::translate(language, keys::http::variables::used::SHADOWS, &[]),
            hidden.join(", ")
        );
        ui.label(egui::RichText::new(words).size(font::META).color(p.text_2));
    }
}

fn unset(ui: &mut Ui, state: &AppState, name: &str, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let none = i18n::translate(state.language, keys::http::variables::used::NONE, &[]);
    head(ui, name, &none, p.red);
    let define = define_words(state);
    if Button::new(&define, "http-used-variable-define")
        .show(ui)
        .clicked()
    {
        actions.push(Asked::Http(Action::Define(name.to_owned())));
    }
}

/// The name in the code face, and where it comes from beside it.
fn head(ui: &mut Ui, name: &str, from: &str, ink: egui::Color32) {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        let name = egui::RichText::new(name)
            .monospace()
            .size(font::LABEL)
            .color(p.text_0);
        ui.label(name);
        let from = ui.label(egui::RichText::new(from).size(font::META).color(ink));
        widgets::address(ui, &from, "http-used-variable-from");
    });
}

/// « collection API Paiements », « globales »…
fn from_words(language: i18n::Language, found: &ScopedVariable) -> String {
    let key = i18n::Key::find(&format!(
        "http.variables.used.from.{}",
        found.scope.as_str()
    ));
    key.map_or_else(String::new, |key| {
        i18n::translate(
            language,
            key,
            &[("name", found.from.clone().unwrap_or_default().into())],
        )
    })
}

/// « Set in Staging », or in the globals with no environment.
fn define_words(state: &AppState) -> String {
    match state.http.environments().active() {
        Some(active) => i18n::translate(
            state.language,
            keys::http::variables::used::DEFINE,
            &[("name", active.name.as_str().into())],
        ),
        None => i18n::translate(
            state.language,
            keys::http::variables::used::DEFINE_GLOBALS,
            &[],
        ),
    }
}
