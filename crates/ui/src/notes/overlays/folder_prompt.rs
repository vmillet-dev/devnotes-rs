//! The name the palette's « New folder » asks for — a folder at the top of the space shown — or
//! a band drawn on the board, which becomes a folder where it was drawn. Its uniqueness is the
//! core's to check, which alone sees the database.

use egui::{CornerRadius, FontFamily, Id, Margin, Sense, Stroke, Ui};
use i18n::keys;

use crate::action::{Action, Actions};
use crate::notes::Action as Asked;
use crate::state::AppState;
use crate::theme::{palette, spacing};
use crate::widgets::{self, Button, Kind, TextField};

const WIDTH: f32 = 380.0;

pub(in crate::notes) fn show(ui: &Ui, state: &AppState, name: &mut String, actions: &mut Actions) {
    let zone = state.board.drawn_zone().is_some();
    let Some(space) = state
        .spaces
        .active_space()
        .filter(|_| state.spaces.naming() || zone)
    else {
        name.clear();
        return;
    };
    let p = palette(ui.ctx());
    #[expect(clippy::cast_possible_truncation, reason = "a few points")]
    let padding = spacing(ui.ctx()).canvas.round() as i8;
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(14))
        .shadow(p.shadow_overlay)
        .inner_margin(Margin::same(padding));
    let shown = egui::Modal::new(Id::unique("folder-name-prompt"))
        .backdrop_color(p.backdrop.gamma_multiply(0.7))
        .frame(frame)
        .show(ui.ctx(), |ui| prompt(ui, state, name));
    let named = shown.inner.filter(|()| !name.trim().is_empty());
    if let Some(()) = named {
        let name = name.trim().to_owned();
        actions.push(Action::Notes(if zone {
            Asked::NameZone(name)
        } else {
            Asked::CreateFolder {
                space_id: space.to_owned(),
                parent_id: None,
                name,
            }
        }));
    }
    if named.is_some() || shown.should_close() || shown.inner.is_some() {
        name.clear();
        actions.push(Action::Notes(Asked::DismissFolderName));
    }
}

/// `Some` once the name is submitted, empty or not; the cancel button closes as Escape does.
fn prompt(ui: &mut Ui, state: &AppState, name: &mut String) -> Option<()> {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let p = palette(ui.ctx());
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = 12.0;
    let title = tr(keys::board::NAME_THE_ZONE);
    let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
    let (rect, _) = ui.allocate_exact_size(words.size(), Sense::hover());
    ui.painter().galley(rect.min, words, p.text_0);
    let hint = ui.add(
        egui::Label::new(
            egui::RichText::new(tr(keys::board::NAME_THE_ZONE_HINT))
                .size(11.5)
                .color(p.text_2),
        )
        .wrap(),
    );
    widgets::address(ui, &hint, "folder-name-hint");
    let (label, placeholder) = (
        tr(keys::folders::NEW_FOLDER_LABEL),
        tr(keys::folders::NEW_FOLDER_PLACEHOLDER),
    );
    let field = TextField::new(name, &label, "folder-name-input")
        .hint(&placeholder)
        .show(ui);
    if !field.has_focus() && name.is_empty() {
        field.request_focus();
    }
    let entered = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    let mut done = None;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if Button::new(&tr(keys::folders::NEW_FOLDER_SUBMIT), "folder-name-submit")
            .kind(Kind::Filled)
            .show(ui)
            .clicked()
            || entered
        {
            done = Some(());
        }
        if Button::new(&tr(keys::placeholders::CANCEL), "folder-name-cancel")
            .show(ui)
            .clicked()
        {
            name.clear();
            done = Some(());
        }
    });
    done
}
