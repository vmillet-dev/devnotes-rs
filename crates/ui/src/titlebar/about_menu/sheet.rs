//! Every key, by where it is live. A row that can be moved shows the key really bound, never the
//! one that shipped.

use egui::{Align, FontFamily, Layout, RichText, Sense, Ui, Vec2};
use i18n::{Key, keys};

use super::framed;
use crate::action::Actions;
use crate::services::guide::{self, SheetRow};
use crate::state::AppState;
use crate::theme::palette;
use crate::widgets::{self, Button, Kbd};

const SIZE: Vec2 = Vec2::new(640.0, 560.0);

pub(super) fn show(ctx: &egui::Context, state: &AppState, _actions: &mut Actions) -> bool {
    let language = state.language;
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let mut close = false;
    let shown = widgets::framed_dialog(ctx, &state.dialogs, super::id(), SIZE, |ui| {
        let (title, subtitle) = (tr(keys::shortcuts::TITLE), tr(keys::shortcuts::SUBTITLE));
        framed(
            ui,
            (&title, &subtitle),
            |ui| body(ui, state),
            |ui| {
                let p = palette(ui.ctx());
                let hint = RichText::new(tr(keys::shortcuts::EDIT_HINT))
                    .size(11.5)
                    .color(p.text_2);
                ui.label(hint);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    close = Button::new(&tr(keys::shortcuts::CLOSE), "sheet-close")
                        .show(ui)
                        .clicked();
                });
            },
        );
    });
    close || shown.is_some_and(|shown| shown.should_close())
}

fn body(ui: &mut Ui, state: &AppState) {
    let language = state.language;
    let p = palette(ui.ctx());
    for group in guide::sheet() {
        let title = i18n::translate(language, group.title(), &[]).to_uppercase();
        let words = widgets::text(ui, &title, 11.0, 600.0, FontFamily::Proportional, p.text_2);
        let (rect, heading) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), words.size().y + 8.0),
            Sense::hover(),
        );
        ui.painter().galley(rect.min, words, p.text_2);
        widgets::address(ui, &heading, "shortcut-group");
        for row in &group.rows {
            line(ui, state, row);
        }
        ui.add_space(12.0);
    }
}

fn line(ui: &mut Ui, state: &AppState, row: &SheetRow) {
    let p = palette(ui.ctx());
    let label = i18n::translate(state.language, row.label(), &[]);
    let caps = row.caps(&state.settings.bindings);
    ui.horizontal(|ui| {
        ui.set_min_height(24.0);
        let room = (ui.available_width() - 220.0).max(120.0);
        ui.allocate_ui_with_layout(
            Vec2::new(room, 22.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.add(
                    egui::Label::new(RichText::new(label).size(12.5).color(p.text_1)).truncate(),
                );
            },
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (at, cap) in caps.iter().rev().enumerate() {
                if at > 0 {
                    ui.label(RichText::new("+").size(11.0).color(p.text_2));
                }
                Kbd(cap).show(ui);
            }
        });
    });
}
