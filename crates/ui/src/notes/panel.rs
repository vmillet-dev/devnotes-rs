//! The panel a row's ⋯ opens, under the row: what a space or a folder can be told to do, in one
//! place so the switcher and the rail cannot drift apart. A name is written when its field is
//! left, or on Enter, which also closes the panel; Escape closes it.

use egui::{Color32, CornerRadius, FontFamily, Rect, Response, Role, Sense, Stroke, Ui, Vec2};
use egui::{StrokeKind, WidgetInfo};
use engine::folders::model::{Folder, FolderColour};
use engine::spaces::model::{Space, SpaceContents};
use i18n::{Key, keys};

use crate::action::{Action as Asked, Actions};
use crate::notes::Action;
use crate::notes::ui::folder_hue;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, Button, ChoiceMenu, Kind, TextField, interact};

fn push(actions: &mut Actions, action: Action) {
    actions.push(Asked::Notes(action));
}

/// A name being typed, and whether its field still has to take the focus.
pub(super) struct Draft {
    name: String,
    focus: bool,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            name: String::new(),
            focus: true,
        }
    }
}

/// What a name's form says: its field's name, its placeholder, its button.
pub(super) struct Words {
    field: String,
    hint: String,
    submit: String,
}

impl Words {
    pub(super) fn new(language: i18n::Language, field: Key, hint: Key, submit: Key) -> Self {
        let tr = |key| i18n::translate(language, key, &[]);
        Self {
            field: tr(field),
            hint: tr(hint),
            submit: tr(submit),
        }
    }
}

pub(super) enum Typed {
    Typing,
    Name(String),
    /// Escape: the form goes, nothing is made.
    Dropped,
}

/// The field, then its button under it: the rail is too narrow for both on one line.
pub(super) fn name_form(
    ui: &mut Ui,
    draft: &mut Draft,
    words: &Words,
    (field_id, submit_id): (&str, &str),
) -> Typed {
    let field = TextField::new(&mut draft.name, &words.field, field_id)
        .hint(&words.hint)
        .compact()
        .show(ui);
    if std::mem::take(&mut draft.focus) {
        field.request_focus();
    }
    let entered = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        return Typed::Dropped;
    }
    let clicked = Button::new(&words.submit, submit_id).show(ui).clicked();
    if (entered || clicked) && !draft.name.trim().is_empty() {
        return Typed::Name(draft.name.trim().to_owned());
    }
    Typed::Typing
}

/// A name edited in place: written once its field is left.
enum Renamed {
    Kept,
    Left(String),
    /// Enter: written, and the panel closes.
    Entered(String),
    Escaped,
}

fn rename_field(
    ui: &mut Ui,
    name: &mut String,
    focus: &mut bool,
    label: &str,
    id: &str,
) -> Renamed {
    let field = TextField::new(name, label, id).compact().show(ui);
    if std::mem::take(focus) {
        field.request_focus();
    }
    if !field.lost_focus() {
        return Renamed::Kept;
    }
    let (enter, escape) = ui.input(|input| {
        (
            input.key_pressed(egui::Key::Enter),
            input.key_pressed(egui::Key::Escape),
        )
    });
    if escape {
        Renamed::Escaped
    } else if enter {
        Renamed::Entered(name.clone())
    } else {
        Renamed::Left(name.clone())
    }
}

pub(super) struct SpaceEdit {
    pub(super) id: String,
    name: String,
    focus: bool,
    /// Among the other spaces, in their order.
    refuge: usize,
    /// Two steps: nothing corpus-wide runs before saying what it touches.
    confirming: bool,
}

impl SpaceEdit {
    pub(super) fn open(space: &Space) -> Self {
        Self {
            id: space.id.clone(),
            name: space.name.clone(),
            focus: true,
            refuge: 0,
            confirming: false,
        }
    }
}

/// Pin, rename, delete. `true` once it closes.
pub(super) fn space(
    ui: &mut Ui,
    edit: &mut SpaceEdit,
    space: &Space,
    others: &[&Space],
    contents: Option<&SpaceContents>,
    language: i18n::Language,
    actions: &mut Actions,
) -> bool {
    let tr = |key| i18n::translate(language, key, &[]);
    let mut close = false;
    frame(ui, |ui| {
        title(ui, &space.name);
        let pin = tr(if space.pinned {
            keys::notes::UNPIN_SPACE
        } else {
            keys::notes::PIN_SPACE
        });
        if pin_toggle(ui, &pin, space.pinned).clicked() {
            push(
                actions,
                Action::PinSpace {
                    id: space.id.clone(),
                    pinned: !space.pinned,
                },
            );
        }
        let label = tr(keys::notes::RENAME_SPACE_LABEL);
        let rename = |actions: &mut Actions, name: String| {
            push(
                actions,
                Action::RenameSpace {
                    id: space.id.clone(),
                    name,
                },
            );
        };
        match rename_field(
            ui,
            &mut edit.name,
            &mut edit.focus,
            &label,
            "space-rename-input",
        ) {
            Renamed::Kept => {}
            Renamed::Left(name) => rename(actions, name),
            Renamed::Entered(name) => {
                rename(actions, name);
                close = true;
            }
            Renamed::Escaped => close = true,
        }
        let submit = tr(keys::notes::RENAME_SPACE_SUBMIT);
        if Button::new(&submit, "space-rename-submit")
            .show(ui)
            .clicked()
        {
            rename(actions, edit.name.clone());
            close = true;
        }
        if others.is_empty() {
            note(
                ui,
                &tr(keys::notes::DELETE_SPACE_NEEDS_ANOTHER),
                "space-delete-blocked",
            );
            return;
        }
        close |= deletion(ui, edit, space, others, contents, language, actions);
    });
    close
}

/// The whole width, in the panel rather than the row: the row is the space's own click surface.
fn pin_toggle(ui: &mut Ui, label: &str, on: bool) -> Response {
    let p = palette(ui.ctx());
    let corner = CornerRadius::same(6);
    let response = interact(
        ui,
        Vec2::new(ui.available_width(), 24.0),
        Sense::click(),
        "space-pin",
        || WidgetInfo::selected(Role::Button, true, on, label),
        corner,
    );
    let rect = response.rect;
    let ink = if on {
        p.amber_text
    } else if response.hovered() {
        p.text_1
    } else {
        p.text_2
    };
    if response.hovered() {
        ui.painter().rect_filled(rect, corner, p.bg_2);
    }
    let glyph = Rect::from_center_size(
        egui::pos2(rect.left() + 14.0, rect.center().y),
        Vec2::splat(12.0),
    );
    icons::paint(ui, glyph, Icon::Pin, ink);
    let galley = widgets::text(ui, label, 11.0, 400.0, FontFamily::Proportional, ink);
    ui.painter().galley(
        egui::pos2(rect.left() + 26.0, rect.center().y - galley.size().y / 2.0),
        galley,
        ink,
    );
    response
}

/// The refuge, then the two-step delete and what it will move and take, counted by the core
/// before the first click.
fn deletion(
    ui: &mut Ui,
    edit: &mut SpaceEdit,
    space: &Space,
    others: &[&Space],
    contents: Option<&SpaceContents>,
    language: i18n::Language,
    actions: &mut Actions,
) -> bool {
    let tr = |key| i18n::translate(language, key, &[]);
    push(actions, Action::CountSpace(space.id.clone()));
    let move_to = tr(keys::notes::MOVE_NOTES_TO);
    note(ui, &move_to, "space-move-label");
    edit.refuge = edit.refuge.min(others.len() - 1);
    let options: Vec<(usize, &str)> = others
        .iter()
        .enumerate()
        .map(|(at, other)| (at, other.name.as_str()))
        .collect();
    ChoiceMenu::new(&move_to, "space-move-target", &options).show(ui, &mut edit.refuge);
    let refuge = others[edit.refuge];
    let label = tr(if edit.confirming {
        keys::notes::DELETE_SPACE_CONFIRM
    } else {
        keys::notes::DELETE_SPACE
    });
    let mut close = false;
    let delete = Button::new(&label, "space-delete")
        .kind(Kind::Destructive)
        .armed(edit.confirming)
        .show(ui);
    if delete.clicked() {
        if edit.confirming {
            push(
                actions,
                Action::DeleteSpace {
                    id: space.id.clone(),
                    refuge: refuge.id.clone(),
                },
            );
            close = true;
        } else {
            edit.confirming = true;
        }
    }
    if let Some(contents) = contents {
        note(
            ui,
            &consequences(*contents, &refuge.name, language),
            "space-delete-consequence",
        );
    }
    close
}

/// The notes and where they go, then the folders that go with the space.
fn consequences(contents: SpaceContents, refuge: &str, language: i18n::Language) -> String {
    let notes = if contents.notes == 0 {
        i18n::translate(language, keys::notes::DELETE_SPACE_NO_NOTES, &[])
    } else {
        i18n::translate(
            language,
            keys::notes::DELETE_SPACE_NOTES,
            &[("count", contents.notes.into()), ("refuge", refuge.into())],
        )
    };
    if contents.folders == 0 {
        return notes;
    }
    let folders = i18n::translate(
        language,
        keys::notes::DELETE_SPACE_FOLDERS,
        &[("count", contents.folders.into())],
    );
    format!("{notes} {folders}")
}

pub(super) struct FolderEdit {
    pub(super) id: String,
    name: String,
    focus: bool,
    /// The sub-folder's name, once asked for: the panel still opens on what most come for.
    pub(super) nesting: Option<Draft>,
    confirming: bool,
}

impl FolderEdit {
    pub(super) fn open(folder: &Folder) -> Self {
        Self {
            id: folder.id.clone(),
            name: folder.name.clone(),
            focus: true,
            nesting: None,
            confirming: false,
        }
    }
}

/// Recolour, rename, a folder inside, delete. `true` once it closes.
pub(super) fn folder(
    ui: &mut Ui,
    edit: &mut FolderEdit,
    folder: &Folder,
    language: i18n::Language,
    actions: &mut Actions,
) -> bool {
    let tr = |key| i18n::translate(language, key, &[]);
    let mut close = false;
    frame(ui, |ui| {
        title(ui, &folder.name);
        colours(ui, folder, language, actions);
        let label = tr(keys::folders::RENAME_LABEL);
        let rename = |actions: &mut Actions, name: String| {
            push(
                actions,
                Action::RenameFolder {
                    id: folder.id.clone(),
                    name,
                },
            );
        };
        match rename_field(
            ui,
            &mut edit.name,
            &mut edit.focus,
            &label,
            "folder-rename-input",
        ) {
            Renamed::Kept => {}
            Renamed::Left(name) => rename(actions, name),
            Renamed::Entered(name) => {
                rename(actions, name);
                close = true;
            }
            Renamed::Escaped => close = true,
        }
        let submit = tr(keys::folders::RENAME_SUBMIT);
        if Button::new(&submit, "folder-rename-submit")
            .show(ui)
            .clicked()
        {
            rename(actions, edit.name.clone());
            close = true;
        }
        close |= sub_folder(ui, edit, folder, language, actions);
        let label = tr(if edit.confirming {
            keys::folders::DELETE_CONFIRM
        } else {
            keys::folders::DELETE
        });
        let delete = Button::new(&label, "folder-delete")
            .kind(Kind::Destructive)
            .armed(edit.confirming)
            .show(ui);
        if delete.clicked() {
            if edit.confirming {
                push(actions, Action::DeleteFolder(folder.id.clone()));
                close = true;
            } else {
                edit.confirming = true;
            }
        }
        note(
            ui,
            &tr(keys::folders::DELETE_KEEPS_NOTES),
            "folder-delete-note",
        );
    });
    close
}

fn sub_folder(
    ui: &mut Ui,
    edit: &mut FolderEdit,
    folder: &Folder,
    language: i18n::Language,
    actions: &mut Actions,
) -> bool {
    let Some(draft) = &mut edit.nesting else {
        let open = i18n::translate(language, keys::folders::NEW_SUBFOLDER, &[]);
        if Button::new(&open, "folder-subfolder-open")
            .show(ui)
            .clicked()
        {
            edit.nesting = Some(Draft::default());
        }
        return false;
    };
    let words = Words {
        field: i18n::translate(
            language,
            keys::folders::NEW_SUBFOLDER_LABEL,
            &[("name", folder.name.as_str().into())],
        ),
        hint: i18n::translate(language, keys::folders::NEW_FOLDER_PLACEHOLDER, &[]),
        submit: i18n::translate(language, keys::folders::NEW_FOLDER_SUBMIT, &[]),
    };
    let ids = ("folder-subfolder-input", "folder-subfolder-submit");
    match name_form(ui, draft, &words, ids) {
        Typed::Typing => false,
        Typed::Dropped => {
            edit.nesting = None;
            false
        }
        Typed::Name(name) => {
            push(
                actions,
                Action::CreateFolder {
                    space_id: folder.space_id.clone(),
                    parent_id: Some(folder.id.clone()),
                    name,
                },
            );
            true
        }
    }
}

/// The five hues, the folder's ringed.
fn colours(ui: &mut Ui, folder: &Folder, language: i18n::Language, actions: &mut Actions) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for colour in FolderColour::ALL {
            let on = folder.colour == colour;
            let name = i18n::translate(language, colour_name(colour), &[]);
            let response = interact(
                ui,
                Vec2::splat(18.0),
                Sense::click(),
                "folder-colour",
                || WidgetInfo::selected(Role::RadioButton, true, on, &name),
                CornerRadius::same(4),
            );
            swatch(ui, &response, folder_hue(ui, colour), on);
            if response.clicked() && !on {
                push(
                    actions,
                    Action::RecolourFolder {
                        id: folder.id.clone(),
                        colour,
                    },
                );
            }
        }
    });
}

fn swatch(ui: &Ui, response: &Response, hue: Color32, on: bool) {
    let rect = Rect::from_center_size(response.rect.center(), Vec2::splat(18.0));
    let shown = if on || response.hovered() {
        hue
    } else {
        hue.gamma_multiply(0.45)
    };
    let corner = CornerRadius::same(4);
    ui.painter().rect_filled(rect, corner, shown);
    if on {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            CornerRadius::same(6),
            Stroke::new(1.0, hue),
            StrokeKind::Inside,
        );
    }
}

fn colour_name(colour: FolderColour) -> i18n::Key {
    match colour {
        FolderColour::Blue => keys::folders::colour::BLUE,
        FolderColour::Amber => keys::folders::colour::AMBER,
        FolderColour::Purple => keys::folders::colour::PURPLE,
        FolderColour::Green => keys::folders::colour::GREEN,
        FolderColour::Red => keys::folders::colour::RED,
    }
}

/// Not indented with its row: the rail is narrow enough.
fn frame(ui: &mut Ui, content: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 4,
            right: 4,
            top: 6,
            bottom: 10,
        })
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            content(ui);
        });
}

fn title(ui: &mut Ui, name: &str) {
    let p = palette(ui.ctx());
    let upper = name.to_uppercase();
    let galley = widgets::text(ui, &upper, 11.0, 600.0, FontFamily::Proportional, p.text_2);
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, p.text_2);
}

/// A sentence that only reads, wrapped to the rail.
fn note(ui: &mut Ui, words: &str, author_id: &str) {
    let p = palette(ui.ctx());
    let mut job = egui::text::LayoutJob::single_section(
        words.to_owned(),
        egui::TextFormat::simple(egui::FontId::new(11.0, FontFamily::Proportional), p.text_2),
    );
    job.wrap.max_width = ui.available_width().max(1.0);
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, words));
    widgets::address(ui, &response, author_id);
    ui.painter().galley(rect.min, galley, p.text_2);
}
