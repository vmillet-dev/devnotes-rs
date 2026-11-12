//! The libraries, and the four things one can do with them. Opening one is a full teardown: the
//! panel goes with the rest, a panel left over the gate would be asking about a library nobody
//! has opened yet.

use egui::{Align, FontFamily, Id, Key, Layout, Modifiers, RichText, Sense, Stroke, Ui, Vec2};
use engine::libraries::registry::LibraryEntry;
use i18n::{Key as Text, Language, Message, keys};

use super::Action;
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::theme::{palette, spacing};
use crate::widgets::{self, Button, Kind, TextField};

const WIDTH: f32 = 560.0;

/// What the panel asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    Close,
    Create(String),
    Open(String),
    Rename { id: String, name: String },
    Delete(String),
    KeepIt,
    ConfirmDelete,
}

#[derive(Default)]
pub(super) struct View {
    new_name: String,
    renaming: Option<String>,
    renamed_to: String,
}

pub(crate) fn id() -> Id {
    Id::unique("libraries-dialog")
}

pub(super) fn show(ctx: &egui::Context, state: &AppState, view: &mut View, actions: &mut Actions) {
    let shown = widgets::dialog(ctx, &state.dialogs, id(), WIDTH, |ui| {
        body(ui, state, view, actions);
    });
    // Escape takes back a name being typed before it closes anything.
    if shown.is_some_and(|shown| shown.should_close()) && view.renaming.take().is_none() {
        actions.push(Asked::Menu(Action::Libraries(Ask::Close)));
    }
}

fn body(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let language = state.language;
    let tr = |key: Text| i18n::translate(language, key, &[]);
    let p = palette(ui.ctx());
    ui.spacing_mut().item_spacing.y = 6.0;
    let title = widgets::text(
        ui,
        &tr(keys::libraries::TITLE),
        15.0,
        600.0,
        FontFamily::Proportional,
        p.text_0,
    );
    let (rect, heading) = ui.allocate_exact_size(title.size(), Sense::hover());
    ui.painter().galley(rect.min, title, p.text_0);
    widgets::address(ui, &heading, "libraries-dialog");
    ui.add(
        egui::Label::new(
            RichText::new(tr(keys::libraries::LEAD))
                .size(11.5)
                .color(p.text_2),
        )
        .wrap(),
    );
    rule(ui);
    let several = state.libraries.libraries().len() > 1;
    for entry in state.libraries.libraries() {
        ui.push_id(&entry.id, |ui| {
            row(ui, state, entry, several, view, actions);
        });
    }
    rule(ui);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        creating(ui, state, view, actions);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if Button::new(&tr(keys::libraries::CLOSE), "libraries-close")
                .show(ui)
                .clicked()
            {
                actions.push(Asked::Menu(Action::Libraries(Ask::Close)));
            }
        });
    });
}

/// From edge to edge of the panel, under its padding.
fn rule(ui: &mut Ui) {
    let line = palette(ui.ctx()).line_soft;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 9.0), Sense::hover());
    ui.painter().hline(
        rect.x_range().expand(spacing(ui.ctx()).canvas),
        rect.center().y,
        Stroke::new(1.0, line),
    );
}

fn creating(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let language = state.language;
    let tr = |key: Text| i18n::translate(language, key, &[]);
    let placeholder = tr(keys::libraries::NAME_PLACEHOLDER);
    let create = tr(keys::libraries::CREATE);
    ui.scope(|ui| {
        ui.set_width(220.0);
        let field = TextField::new(&mut view.new_name, &create, "library-new-name")
            .hint(&placeholder)
            .compact()
            .show(ui);
        let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
        let name = view.new_name.trim().to_owned();
        if entered && !name.is_empty() {
            actions.push(Asked::Menu(Action::Libraries(Ask::Create(name))));
        }
    });
    let name = view.new_name.trim().to_owned();
    let ready = !name.is_empty() && !state.libraries.is_working();
    if Button::new(&create, "library-create")
        .enabled(ready)
        .show(ui)
        .clicked()
    {
        actions.push(Asked::Menu(Action::Libraries(Ask::Create(name))));
    }
}

fn row(
    ui: &mut Ui,
    state: &AppState,
    entry: &LibraryEntry,
    several: bool,
    view: &mut View,
    actions: &mut Actions,
) {
    let language = state.language;
    let tr = |key: Text| i18n::translate(language, key, &[]);
    let open = state.libraries.is_open(&entry.id);
    let shown = ui.horizontal(|ui| {
        ui.set_min_height(38.0);
        ui.spacing_mut().item_spacing.x = 8.0;
        if view.renaming.as_deref() == Some(&entry.id) {
            renaming(ui, entry, language, view, actions);
        } else {
            name(ui, entry, open, language);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if several && !open {
                let delete = Button::new(&tr(keys::libraries::DELETE), "library-delete")
                    .kind(Kind::Destructive)
                    .show(ui);
                if delete.clicked() {
                    let asked = Ask::Delete(entry.id.clone());
                    actions.push(Asked::Menu(Action::Libraries(asked)));
                }
            }
            let rename = Button::new(&tr(keys::libraries::RENAME), "library-start-rename").show(ui);
            if rename.clicked() {
                view.renaming = Some(entry.id.clone());
                entry.name.trim().clone_into(&mut view.renamed_to);
            }
            if !open {
                let switch = Button::new(&tr(keys::libraries::SWITCH), "library-switch")
                    .enabled(!state.libraries.is_working())
                    .show(ui);
                if switch.clicked() {
                    let asked = Ask::Open(entry.id.clone());
                    actions.push(Asked::Menu(Action::Libraries(asked)));
                }
            }
        });
    });
    widgets::address(ui, &shown.response, "library-row");
    if state.libraries.pending_deletion() == Some(entry.id.as_str()) {
        confirm(ui, state, entry, actions);
    }
}

fn name(ui: &mut Ui, entry: &LibraryEntry, open: bool, language: Language) {
    let p = palette(ui.ctx());
    let shown = entry.name.trim();
    let unnamed = i18n::translate(language, keys::libraries::UNNAMED, &[]);
    let shown = if shown.is_empty() { &unnamed } else { shown };
    ui.label(RichText::new(shown).size(12.5).color(p.text_1));
    if open {
        let words = i18n::translate(language, keys::libraries::OPEN, &[]);
        let badge = widgets::hue_badge(ui, &words, p.amber_text);
        widgets::address(ui, &badge, "library-open");
    }
}

fn renaming(
    ui: &mut Ui,
    entry: &LibraryEntry,
    language: Language,
    view: &mut View,
    actions: &mut Actions,
) {
    let label = i18n::translate(language, keys::libraries::RENAME, &[]);
    ui.scope(|ui| {
        ui.set_width(220.0);
        let field = TextField::new(&mut view.renamed_to, &label, "library-rename")
            .compact()
            .show(ui);
        if !field.has_focus() && !field.lost_focus() {
            field.request_focus();
        }
        let escaped = ui.input(|input| input.key_pressed(Key::Escape));
        if field.lost_focus() && !escaped {
            let name = view.renamed_to.trim().to_owned();
            view.renaming = None;
            if !name.is_empty() {
                let id = entry.id.clone();
                actions.push(Asked::Menu(Action::Libraries(Ask::Rename { id, name })));
            }
        }
    });
    ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Tab));
}

/// The trigger is replaced by a sentence naming what goes, and the confirm sits elsewhere: it is
/// irreversible and it takes everything at once.
fn confirm(ui: &mut Ui, state: &AppState, entry: &LibraryEntry, actions: &mut Actions) {
    let language = state.language;
    let p = palette(ui.ctx());
    let tr = |key: Text| i18n::translate(language, key, &[]);
    let said = Message::new(keys::libraries::CONFIRM_DELETE)
        .with("name", entry.name.as_str())
        .text(language);
    let words = ui.add(egui::Label::new(RichText::new(said).size(12.0).color(p.red)).wrap());
    widgets::address(ui, &words, "library-confirm");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if Button::new(&tr(keys::libraries::CANCEL), "library-cancel-delete")
            .show(ui)
            .clicked()
        {
            actions.push(Asked::Menu(Action::Libraries(Ask::KeepIt)));
        }
        if Button::new(
            &tr(keys::libraries::CONFIRM_DELETE_ACTION),
            "library-confirm-delete",
        )
        .kind(Kind::Destructive)
        .enabled(!state.libraries.is_working())
        .show(ui)
        .clicked()
        {
            actions.push(Asked::Menu(Action::Libraries(Ask::ConfirmDelete)));
        }
    });
}
