//! The gate as the reference drew it: one panel in the middle of the window — `surface`, 420 px,
//! 28 px inside, 14 px between — naming its library, then asking what it needs.

use egui::{Align, Layout, Margin, RichText, Stroke, Ui, Vec2};
use engine::vault::{MINIMUM_LENGTH, VaultState};
use i18n::{Key, Language, Message, keys};

use super::passphrase::PassphraseField;
use super::{Action, Gate, GateView};
use crate::theme::palette;
use crate::widgets::{self, Button, ChoiceMenu, Kind};

/// The whole window while the library is not open.
pub fn show(
    ui: &mut Ui,
    gate: &Gate,
    view: &mut GateView,
    language: Language,
    actions: &mut Vec<Action>,
) {
    let p = palette(ui.ctx());
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let height_id = ui.scope_id().with("gate-height");
    let height: f32 = ui.data(|data| data.get_temp(height_id)).unwrap_or(0.0);
    let width = 420.0_f32.min(ui.available_width() - 2.0 * crate::theme::spacing(ui.ctx()).canvas);
    ui.add_space(
        ((ui.available_height() - height) / 2.0).max(crate::theme::spacing(ui.ctx()).canvas),
    );
    ui.vertical_centered(|ui| {
        let shown = egui::Frame::new()
            .fill(p.bg_2)
            .stroke(Stroke::new(1.0, p.line_soft))
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(Margin::same(28))
            .show(ui, |ui| {
                ui.set_width(width - 56.0);
                ui.spacing_mut().item_spacing.y = 14.0;
                ui.with_layout(Layout::top_down(Align::Min), |ui| {
                    library(ui, gate, view, language, actions);
                    if gate.vault() == Some(VaultState::KeyMissing) {
                        set_aside(
                            ui,
                            gate,
                            &tr,
                            keys::vault::KEY_MISSING_TITLE,
                            keys::vault::KEY_MISSING_LEAD,
                            keys::vault::KEY_MISSING_NOTE,
                            actions,
                        );
                    } else if gate.damaged {
                        set_aside(
                            ui,
                            gate,
                            &tr,
                            keys::vault::DAMAGED_TITLE,
                            keys::vault::DAMAGED_LEAD,
                            keys::vault::DAMAGED_NOTE,
                            actions,
                        );
                    } else if gate.confirming_archive {
                        archive(ui, gate, &tr, actions);
                    } else {
                        form(ui, gate, view, language, actions);
                    }
                });
            });
        ui.data_mut(|data| data.insert_temp(height_id, shown.response.rect.height()));
    });
}

/// Which library the panel is about, above what it asks: several can be asked for here.
fn library(
    ui: &mut Ui,
    gate: &Gate,
    view: &mut GateView,
    language: Language,
    actions: &mut Vec<Action>,
) {
    let Some(registry) = gate.registry() else {
        return;
    };
    let p = palette(ui.ctx());
    let unnamed = i18n::translate(language, keys::libraries::UNNAMED, &[]);
    let names: Vec<String> = registry
        .libraries
        .iter()
        .map(|entry| {
            if entry.name.trim().is_empty() {
                unnamed.clone()
            } else {
                entry.name.trim().to_string()
            }
        })
        .collect();
    let open = registry
        .open
        .as_ref()
        .and_then(|id| registry.libraries.iter().position(|entry| &entry.id == id));
    let Some(open) = open else {
        return;
    };
    let row = ui.horizontal(|ui| {
        ui.label(small(
            &i18n::translate(language, keys::vault::LIBRARY, &[]),
            p.text_2,
        ));
        if names.len() > 1 {
            let options: Vec<(usize, &str)> =
                names.iter().map(String::as_str).enumerate().collect();
            let mut chosen = open;
            let label = i18n::translate(language, keys::vault::SWITCH_LIBRARY, &[]);
            let response =
                ChoiceMenu::new(&label, "vault-library-switch", &options).show(ui, &mut chosen);
            if response.changed() {
                view.switch_to = Some(chosen);
                actions.push(Action::SwitchLibrary);
            }
        } else {
            ui.label(
                RichText::new(&names[open])
                    .size(11.2)
                    .variation(*b"wght", 600.0)
                    .color(p.text_0),
            );
        }
    });
    widgets::address(ui, &row.response, "vault-library");
    let line = ui.max_rect().x_range();
    ui.painter().hline(
        line,
        row.response.rect.bottom() + 6.0,
        Stroke::new(1.0, p.line_soft),
    );
}

fn form(
    ui: &mut Ui,
    gate: &Gate,
    view: &mut GateView,
    language: Language,
    actions: &mut Vec<Action>,
) {
    let p = palette(ui.ctx());
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let creating = gate.is_creating();
    title(
        ui,
        &tr(if creating {
            keys::vault::CREATE_TITLE
        } else {
            keys::vault::UNLOCK_TITLE
        }),
    );
    lead(
        ui,
        &tr(if creating {
            keys::vault::CREATE_LEAD
        } else {
            keys::vault::UNLOCK_LEAD
        }),
        p.text_2,
    );
    if creating {
        lead(ui, &tr(keys::vault::NO_RECOVERY), p.amber_text);
    }

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        let name = tr(keys::vault::PASSPHRASE);
        ui.horizontal(|ui| {
            ui.label(small(&name, p.text_2));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let reveal = Button::new(&tr(keys::vault::REVEAL), "vault-reveal")
                    .kind(Kind::Quiet)
                    .armed(view.revealed)
                    .show(ui);
                if reveal.clicked() {
                    actions.push(Action::ToggleReveal);
                }
            });
        });
        let invalid = gate.refused || gate.too_short(view);
        let (field, typed) = PassphraseField::new(&mut view.passphrase, &name, "vault-passphrase")
            .revealed(view.revealed)
            .invalid(invalid)
            .enabled(!gate.is_working())
            .show(ui);
        if ui.memory(egui::Memory::focused).is_none() && !gate.is_working() {
            field.request_focus();
        }
        if typed.submitted {
            actions.push(Action::Submit);
        }
        if creating {
            let hint = i18n::translate(
                language,
                keys::vault::TOO_SHORT,
                &[("length", MINIMUM_LENGTH.into())],
            );
            let colour = if gate.too_short(view) {
                p.red
            } else {
                p.text_2
            };
            let shown = ui.label(small(&hint, colour));
            widgets::address(ui, &shown, "vault-minimum");
        }
    });

    if creating {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let name = tr(keys::vault::CONFIRMATION);
            ui.label(small(&name, p.text_2));
            let (_, typed) =
                PassphraseField::new(&mut view.confirmation, &name, "vault-confirmation")
                    .revealed(view.revealed)
                    .enabled(!gate.is_working())
                    .show(ui);
            if typed.submitted {
                actions.push(Action::Submit);
            }
        });
    }

    submit(ui, gate, view, language, actions);
}

/// The problem of the moment, the button, and the way out for a phrase forgotten.
fn submit(
    ui: &mut Ui,
    gate: &Gate,
    view: &GateView,
    language: Language,
    actions: &mut Vec<Action>,
) {
    let p = palette(ui.ctx());
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let creating = gate.is_creating();
    // One line, whichever problem is current, its room kept so the button never moves.
    let problem = if gate.refused {
        Some(Message::new(keys::errors::WRONG_PASSPHRASE))
    } else if gate.mismatched(view) {
        Some(Message::new(keys::vault::MISMATCHED))
    } else {
        None
    };
    let said = problem
        .map(|message| message.text(language))
        .unwrap_or_default();
    let shown = ui.add_sized(
        Vec2::new(ui.available_width(), 17.0),
        egui::Label::new(RichText::new(said).size(11.9).color(p.red)),
    );
    widgets::address(ui, &shown, "vault-problem");
    ui.ctx().accesskit_node_builder(shown.id, |node| {
        node.set_live(egui::accesskit::Live::Polite);
    });

    let action = match (gate.is_working(), creating) {
        (true, true) => keys::vault::CREATING,
        (true, false) => keys::vault::WORKING,
        (false, true) => keys::vault::CREATE_ACTION,
        (false, false) => keys::vault::UNLOCK_ACTION,
    };
    let submit = Button::new(&tr(action), "vault-submit")
        .kind(Kind::Filled)
        .large()
        .enabled(gate.can_submit(view))
        .show(ui);
    if submit.clicked() {
        actions.push(Action::Submit);
    }

    // Only on a library that exists: a fresh one has no phrase to have forgotten.
    if !creating {
        ui.vertical_centered(|ui| {
            if Button::new(&tr(keys::vault::FORGOTTEN_LINK), "vault-forgotten")
                .kind(Kind::Quiet)
                .show(ui)
                .clicked()
            {
                actions.push(Action::AskToArchive);
            }
        });
    }
}

/// Instead of the form, never beside it: retyping cannot help a file that is damaged or keyless.
fn set_aside(
    ui: &mut Ui,
    gate: &Gate,
    tr: &dyn Fn(Key) -> String,
    heading: Key,
    said: Key,
    note: Key,
    actions: &mut Vec<Action>,
) {
    let p = palette(ui.ctx());
    let panel = if heading == keys::vault::DAMAGED_TITLE {
        "vault-damaged"
    } else {
        "vault-key-missing"
    };
    let response = title(ui, &tr(heading));
    widgets::address(ui, &response, panel);
    lead(ui, &tr(said), p.text_2);
    lead(ui, &tr(note), p.amber_text);
    let label = if gate.is_working() {
        keys::vault::WORKING
    } else {
        keys::vault::DAMAGED_ACTION
    };
    if Button::new(&tr(label), "vault-set-aside")
        .kind(Kind::Filled)
        .large()
        .enabled(!gate.is_working())
        .show(ui)
        .clicked()
    {
        actions.push(Action::SetAside);
    }
}

/// Reads as giving up, not as a shortcut past a field: the notes leave sealed.
fn archive(ui: &mut Ui, gate: &Gate, tr: &dyn Fn(Key) -> String, actions: &mut Vec<Action>) {
    let p = palette(ui.ctx());
    let response = title(ui, &tr(keys::vault::FORGOTTEN_TITLE));
    widgets::address(ui, &response, "vault-archive");
    lead(ui, &tr(keys::vault::FORGOTTEN_LEAD), p.text_2);
    lead(ui, &tr(keys::vault::FORGOTTEN_NOTE), p.amber_text);
    ui.horizontal(|ui| {
        if Button::new(&tr(keys::vault::FORGOTTEN_BACK), "vault-keep-trying")
            .show(ui)
            .clicked()
        {
            actions.push(Action::KeepTrying);
        }
        let label = if gate.is_working() {
            keys::vault::WORKING
        } else {
            keys::vault::FORGOTTEN_ACTION
        };
        // Red text and a hairline, never the amber fill: this one loses the notes.
        if Button::new(&tr(label), "vault-archive-confirm")
            .kind(Kind::Destructive)
            .enabled(!gate.is_working())
            .show(ui)
            .clicked()
        {
            actions.push(Action::Archive);
        }
    });
}

fn title(ui: &mut Ui, words: &str) -> egui::Response {
    let p = palette(ui.ctx());
    ui.label(
        RichText::new(words)
            .size(16.1)
            .variation(*b"wght", 700.0)
            .color(p.text_0),
    )
}

fn lead(ui: &mut Ui, words: &str, colour: egui::Color32) {
    ui.label(RichText::new(words).size(11.9).color(colour));
}

fn small(words: &str, colour: egui::Color32) -> RichText {
    RichText::new(words).size(11.2).color(colour)
}
