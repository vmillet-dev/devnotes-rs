//! The keys that can be moved. A field listens for a keystroke rather than accepting text: a
//! modifier alone is a combination still being typed, Tab goes on to the next field, and a first
//! Escape stops the listening without closing the panel, which the next one does.

use egui::{
    CornerRadius, Event, EventFilter, FontFamily, Key, RichText, Role, Sense, Stroke, Ui, Vec2,
};
use i18n::{Key as Text, Language, Message, keys};

use super::general::{group, note, row};
use crate::services::settings::Settings;
use crate::services::shortcuts::{self, Bindings, Rebindable, Refused, Scope};
use crate::state::AppState;
use crate::theme::palette;
use crate::widgets::{self, Button};

const FIELD: Vec2 = Vec2::new(130.0, 26.0);

/// What the page remembers between frames.
#[derive(Default)]
pub(super) struct View {
    /// A refusal belongs to the field that caused it.
    refusal: Option<(&'static str, Refused)>,
    /// The field Escape stopped: the next Escape is the panel's, until it is clicked again.
    stopped: Option<&'static str>,
    /// Set by the drawing, frame after frame: a page left or a panel closed lets go by itself.
    pub(super) listens_for_a_global_key: bool,
}

pub(super) fn page(ui: &mut Ui, state: &AppState, settings: &mut Settings, view: &mut View) {
    use keys::settings::shortcuts as words;
    let language = state.language;
    let tr = |key: Text| i18n::translate(language, key, &[]);
    for (accelerator, actions) in settings.bindings.conflicts() {
        conflict(ui, language, &accelerator, &actions);
    }
    let sections = [
        (
            Scope::Global,
            keys::shortcuts::groups::GLOBAL,
            words::GLOBAL_NOTE,
        ),
        (
            Scope::Canvas,
            keys::shortcuts::groups::CANVAS,
            words::CANVAS_NOTE,
        ),
        (
            Scope::Areas,
            keys::shortcuts::groups::AREAS,
            words::AREAS_NOTE,
        ),
    ];
    for (scope, title, said) in sections {
        group(ui, language, title);
        for action in shortcuts::every().filter(|action| action.scope == scope) {
            ui.push_id(action.id, |ui| {
                line(ui, language, action, settings, view);
                held_elsewhere(ui, state, action, settings);
            });
        }
        note(ui, &tr(said));
    }
    note(ui, &tr(words::FIXED_NOTE));
}

/// A global key another application took first: pressing it would do nothing, and say nothing.
fn held_elsewhere(ui: &mut Ui, state: &AppState, action: Rebindable, settings: &Settings) {
    let accelerator = settings.bindings.of(action);
    let held = state.global_keys.held();
    if action.scope != Scope::Global || !held.iter().any(|key| key == accelerator) {
        return;
    }
    let said = Message::new(keys::settings::shortcuts::HELD_ELSEWHERE)
        .with("accelerator", accelerator)
        .text(state.language);
    let words = RichText::new(said).size(11.5).color(palette(ui.ctx()).red);
    let shown = ui.add(egui::Label::new(words).wrap());
    widgets::address(ui, &shown, &format!("shortcut-held-{}", action.id));
}

fn conflict(ui: &mut Ui, language: Language, accelerator: &str, actions: &[Rebindable]) {
    let names: Vec<String> = actions
        .iter()
        .map(|action| i18n::translate(language, action.label, &[]))
        .collect();
    let said = Message::new(keys::settings::shortcuts::CONFLICT)
        .with("accelerator", accelerator)
        .with("actions", names.join(", "))
        .text(language);
    let words = RichText::new(said).size(12.0).color(palette(ui.ctx()).red);
    let shown = ui.add(egui::Label::new(words).wrap());
    widgets::address(ui, &shown, "shortcut-conflict");
}

fn line(
    ui: &mut Ui,
    language: Language,
    action: Rebindable,
    settings: &mut Settings,
    view: &mut View,
) {
    let label = i18n::translate(language, action.label, &[]);
    let reset = i18n::translate(language, keys::settings::shortcuts::RESET, &[]);
    row(ui, &label, |ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let id = format!("shortcut-reset-{}", action.id);
        let moved = !settings.bindings.is_default(action);
        if Button::new(&reset, &id).enabled(moved).show(ui).clicked() {
            settings.bindings.reset(action);
            view.refusal = None;
        }
        field(ui, &label, action, &mut settings.bindings, view);
    });
    let refused = view.refusal.as_ref().filter(|(id, _)| *id == action.id);
    if let Some((_, refused)) = refused {
        let said = explain(refused, language);
        let words = RichText::new(said).size(11.5).color(palette(ui.ctx()).red);
        let shown = ui.add(egui::Label::new(words).wrap());
        widgets::address(ui, &shown, &format!("shortcut-error-{}", action.id));
    }
}

/// `readonly`, in the reference's terms: it takes the focus, and what it listens for is a key.
fn field(ui: &mut Ui, label: &str, action: Rebindable, bindings: &mut Bindings, view: &mut View) {
    let p = palette(ui.ctx());
    let shown = bindings.of(action).to_owned();
    let corner = CornerRadius::same(6);
    let response = widgets::interact(
        ui,
        FIELD,
        Sense::click(),
        &format!("shortcut-field-{}", action.id),
        || {
            let mut info = egui::WidgetInfo::labeled(Role::TextInput, true, label);
            info.current_text_value = Some(shown.clone());
            info
        },
        corner,
    );
    if response.clicked() {
        response.request_focus();
        view.stopped = None;
    }
    let listening = widgets::takes_keys(ui, &response) && view.stopped != Some(action.id);
    if listening {
        // ⚠️ egui drops the focus on Escape before anything is drawn: the field would never see
        // it, and the panel would close on the first one.
        let keeps = EventFilter {
            tab: false,
            horizontal_arrows: true,
            vertical_arrows: true,
            escape: true,
        };
        ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, keeps));
        view.listens_for_a_global_key |= action.scope == Scope::Global;
        listen(ui, action, bindings, view);
    }
    let edge = if listening { p.amber_edge } else { p.line };
    ui.painter().rect(
        response.rect,
        corner,
        p.bg_1,
        Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    let text = bindings.of(action);
    let words = widgets::text(ui, text, 11.5, 600.0, FontFamily::Monospace, p.amber_text);
    let at = response.rect.center() - words.size() / 2.0;
    ui.painter().galley(at, words, p.amber_text);
}

/// The keys of the frame are the field's, Tab excepted.
fn listen(ui: &Ui, action: Rebindable, bindings: &mut Bindings, view: &mut View) {
    ui.input_mut(|input| {
        input.events.retain(|event| {
            let Event::Key {
                key,
                physical_key,
                pressed: true,
                modifiers,
                ..
            } = event
            else {
                return true;
            };
            match key {
                Key::Tab => return true,
                Key::Escape => view.stopped = Some(action.id),
                _ => {
                    match shortcuts::capture(action, (*key, *physical_key), *modifiers, bindings) {
                        Ok(accelerator) => {
                            bindings.set(action, accelerator);
                            view.refusal = None;
                        }
                        Err(refused) => view.refusal = Some((action.id, refused)),
                    }
                }
            }
            false
        });
    });
}

/// A refusal nobody can read is the same as a key that quietly did nothing.
fn explain(refused: &Refused, language: Language) -> String {
    use keys::settings::shortcuts as words;
    match refused {
        Refused::NeedsModifier => i18n::translate(language, words::NEEDS_MODIFIER, &[]),
        Refused::Reserved => i18n::translate(language, words::RESERVED, &[]),
        Refused::Unknown => i18n::translate(language, words::UNKNOWN_KEY, &[]),
        Refused::Taken { accelerator, by } => Message::new(words::TAKEN)
            .with("accelerator", accelerator.as_str())
            .with("action", i18n::translate(language, by.label, &[]))
            .text(language),
    }
}
