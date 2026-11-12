//! The space and folder switchers, while the rail is put away — two places to change space is
//! how a tree and a dropdown drift apart — and the breadcrumb of the folder open, in their
//! place. Each opens a menu; a row's ⋯ turns the menu into the panel the rail opens.

use egui::{
    CornerRadius, FontFamily, Popup, PopupCloseBehavior, Rect, Response, Role, Sense, Stroke,
    StrokeKind, Ui, Vec2, WidgetInfo, pos2,
};
use engine::folders::model::{Folder, Lineage};
use i18n::keys;

use crate::action::{Action, Actions};
use crate::notes::panel::{self, Draft, FolderEdit, SpaceEdit, Typed, Words};
use crate::notes::ui::folder_hue;
use crate::state::AppState;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, MIN_TARGET, interact};

/// What a switcher's menu shows: its list, or what one of its rows opened.
#[derive(Default)]
pub(super) enum Menu {
    #[default]
    List,
    Creating(Draft),
    Space(SpaceEdit),
    Folder(FolderEdit),
}

/// The widest a switcher's name is drawn before its ellipsis.
const NAME_WIDTH: f32 = 180.0;

fn push(actions: &mut Actions, action: crate::notes::Action) {
    actions.push(Action::Notes(action));
}

/// The space shown, or « every space ».
fn space_name(state: &AppState) -> String {
    let spaces = &state.spaces;
    spaces
        .active_space()
        .and_then(|id| spaces.spaces().iter().find(|space| space.id == id))
        .map_or_else(
            || i18n::translate(state.language, keys::notes::ALL_SPACES, &[]),
            |space| space.name.clone(),
        )
}

fn folder_name(state: &AppState) -> String {
    i18n::translate(state.language, keys::folders::ALL_FOLDERS, &[])
}

/// Each switcher's width with its whole name, in the order they are drawn.
pub(super) fn natural_widths(ui: &Ui, state: &AppState) -> [f32; 2] {
    [space_name(state), folder_name(state)].map(|name| switch_width(ui, &name))
}

fn switch_width(ui: &Ui, name: &str) -> f32 {
    let p = palette(ui.ctx());
    let galley = widgets::text(ui, name, 12.5, 400.0, FontFamily::Proportional, p.text_0);
    galley.size().x.min(NAME_WIDTH) + 44.0
}

/// Answers its width: the header narrows it to `widest` before it takes two rows.
pub(super) fn space_switcher(
    ui: &mut Ui,
    state: &AppState,
    menu: &mut Menu,
    widest: f32,
    actions: &mut Actions,
) -> f32 {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let spaces = &state.spaces;
    let active = spaces.active_space();
    let name = space_name(state);
    let button = switch(
        ui,
        &tr(keys::notes::SPACE_SWITCHER_LABEL),
        &name,
        "space-switcher",
        widest,
    );
    let width = button.rect.width();
    let shown = Popup::menu(&button)
        .frame(widgets::menu_frame(ui.ctx()))
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(260.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            match menu {
                Menu::Creating(draft) => {
                    let words = Words::new(
                        language,
                        keys::notes::NEW_SPACE_LABEL,
                        keys::notes::NEW_SPACE_PLACEHOLDER,
                        keys::notes::NEW_SPACE_SUBMIT,
                    );
                    let ids = ("space-create-input", "space-create-submit");
                    if let Typed::Name(name) = panel::name_form(ui, draft, &words, ids) {
                        push(actions, crate::notes::Action::CreateSpace(name));
                        ui.close();
                    }
                }
                Menu::Space(edit) => {
                    let Some(space) = spaces.spaces().iter().find(|space| space.id == edit.id)
                    else {
                        return;
                    };
                    let others: Vec<_> = spaces
                        .spaces()
                        .iter()
                        .filter(|other| other.id != space.id)
                        .collect();
                    let contents = spaces.contents(&space.id);
                    if panel::space(ui, edit, space, &others, contents, language, actions) {
                        ui.close();
                    }
                }
                Menu::List | Menu::Folder(_) => {
                    let all = tr(keys::notes::ALL_SPACES);
                    if option(ui, &all, "space-option-all", None, active.is_none()).clicked() {
                        push(actions, crate::notes::Action::ChooseSpace(None));
                        ui.close();
                    }
                    for space in spaces.spaces() {
                        let options = i18n::translate(
                            language,
                            keys::notes::SPACE_OPTIONS_LABEL,
                            &[("name", space.name.as_str().into())],
                        );
                        let mark = space.pinned.then_some(Mark::Pin);
                        let on = active == Some(space.id.as_str());
                        let (chosen, edit) = row(
                            ui,
                            (&space.name, "space-option"),
                            (&options, "space-edit"),
                            mark,
                            on,
                        );
                        if chosen.clicked() {
                            push(
                                actions,
                                crate::notes::Action::ChooseSpace(Some(space.id.clone())),
                            );
                            ui.close();
                        }
                        if edit.clicked() {
                            *menu = Menu::Space(SpaceEdit::open(space));
                        }
                    }
                    let create = tr(keys::notes::NEW_SPACE);
                    if option(ui, &create, "space-create-open", Some(Mark::Plus), false).clicked() {
                        *menu = Menu::Creating(Draft::default());
                    }
                }
            }
        });
    if shown.is_none() {
        *menu = Menu::List;
    }
    width
}

pub(super) fn folder_switcher(
    ui: &mut Ui,
    state: &AppState,
    menu: &mut Menu,
    widest: f32,
    actions: &mut Actions,
) -> f32 {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let spaces = &state.spaces;
    let space = spaces.active_space();
    // « Every space »: every folder, the choice the space switcher already made.
    let folders: Vec<&Folder> = state
        .folders
        .list()
        .iter()
        .filter(|folder| space.is_none_or(|space| folder.space_id == space))
        .collect();
    let name = folder_name(state);
    let button = switch(
        ui,
        &tr(keys::folders::SWITCHER_LABEL),
        &name,
        "folder-switcher",
        widest,
    );
    let width = button.rect.width();
    let shown = Popup::menu(&button)
        .frame(widgets::menu_frame(ui.ctx()))
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(260.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            match menu {
                Menu::Creating(draft) => {
                    let words = Words::new(
                        language,
                        keys::folders::NEW_FOLDER_LABEL,
                        keys::folders::NEW_FOLDER_PLACEHOLDER,
                        keys::folders::NEW_FOLDER_SUBMIT,
                    );
                    let ids = ("folder-create-input", "folder-create-submit");
                    if let (Typed::Name(name), Some(space)) =
                        (panel::name_form(ui, draft, &words, ids), space)
                    {
                        push(
                            actions,
                            crate::notes::Action::CreateFolder {
                                space_id: space.to_owned(),
                                parent_id: None,
                                name,
                            },
                        );
                        ui.close();
                    }
                }
                Menu::Folder(edit) => {
                    let Some(folder) = folders.iter().find(|folder| folder.id == edit.id) else {
                        return;
                    };
                    if panel::folder(ui, edit, folder, language, actions) {
                        ui.close();
                    }
                }
                Menu::List | Menu::Space(_) => {
                    folder_list(ui, state, (&folders, space), menu, actions);
                }
            }
        });
    if shown.is_none() {
        *menu = Menu::List;
    }
    width
}

/// Every folder of the space shown, each with its ⋯, then the way to make one.
fn folder_list(
    ui: &mut Ui,
    state: &AppState,
    (folders, space): (&[&Folder], Option<&str>),
    menu: &mut Menu,
    actions: &mut Actions,
) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let all = tr(keys::folders::ALL_FOLDERS);
    if option(ui, &all, "folder-option-all", None, true).clicked() {
        ui.close();
    }
    for folder in folders {
        let options = i18n::translate(
            language,
            keys::folders::OPTIONS_LABEL,
            &[("name", folder.name.as_str().into())],
        );
        let mark = Some(Mark::Swatch(folder_hue(ui, folder.colour)));
        let (chosen, edit) = row(
            ui,
            (&folder.name, "folder-option"),
            (&options, "folder-edit"),
            mark,
            false,
        );
        if chosen.clicked() {
            push(
                actions,
                crate::notes::Action::ChooseFolder(folder.id.clone()),
            );
            ui.close();
        }
        if edit.clicked() {
            *menu = Menu::Folder(FolderEdit::open(folder));
        }
    }
    if space.is_some() {
        let create = tr(keys::folders::NEW_FOLDER);
        let open = option(ui, &create, "folder-create-open", Some(Mark::Plus), false);
        if open.clicked() {
            *menu = Menu::Creating(Draft::default());
        }
    } else {
        note(
            ui,
            &tr(keys::folders::NEEDS_ASPACE),
            "folder-create-blocked",
        );
    }
}

/// Where one is, while a folder is open: back to its space, the folders above it, its name, and
/// its ⋯, which opens the folder's panel.
pub(super) fn breadcrumb(
    ui: &mut Ui,
    state: &AppState,
    folder: &Folder,
    edit: &mut Option<FolderEdit>,
    actions: &mut Actions,
) {
    let language = state.language;
    let p = palette(ui.ctx());
    let spaces = &state.spaces;
    let space_name = spaces
        .spaces()
        .iter()
        .find(|space| space.id == folder.space_id)
        .map_or_else(
            || i18n::translate(language, keys::notes::ALL_SPACES, &[]),
            |space| space.name.clone(),
        );
    let back = crumb(
        ui,
        &format!("‹ {space_name}"),
        "folder-breadcrumb-back",
        None,
    );
    if back.clicked() {
        push(
            actions,
            crate::notes::Action::ChooseSpace(Some(folder.space_id.clone())),
        );
    }
    separator(ui, "/");
    let inside: Vec<&Folder> = state
        .folders
        .list()
        .iter()
        .filter(|other| other.space_id == folder.space_id)
        .collect();
    let lineage = Lineage::new(
        inside
            .iter()
            .map(|other| (other.id.clone(), other.parent_id.clone())),
    );
    let path = lineage.path(&folder.id);
    for id in path.iter().take(path.len().saturating_sub(1)) {
        let Some(above) = inside.iter().find(|other| other.id == *id) else {
            continue;
        };
        let hue = folder_hue(ui, above.colour);
        if crumb(ui, &above.name, "folder-breadcrumb-step", Some(hue)).clicked() {
            push(
                actions,
                crate::notes::Action::ChooseFolder(above.id.clone()),
            );
        }
        separator(ui, "›");
    }
    let hue = folder_hue(ui, folder.colour);
    let galley = widgets::text(
        ui,
        &folder.name,
        12.5,
        600.0,
        FontFamily::Proportional,
        p.text_0,
    );
    let (rect, here) = ui.allocate_exact_size(galley.size() + Vec2::new(14.0, 8.0), Sense::hover());
    here.widget_info(|| WidgetInfo::labeled(Role::Label, true, &folder.name));
    widgets::address(ui, &here, "folder-breadcrumb-name");
    let swatch = Rect::from_center_size(pos2(rect.left() + 4.0, rect.center().y), Vec2::splat(8.0));
    ui.painter().rect_filled(swatch, CornerRadius::same(2), hue);
    ui.painter().galley(
        pos2(rect.left() + 14.0, rect.center().y - galley.size().y / 2.0),
        galley,
        p.text_0,
    );
    let options = i18n::translate(
        language,
        keys::folders::OPTIONS_LABEL,
        &[("name", folder.name.as_str().into())],
    );
    let menu =
        widgets::IconButton::new(Icon::Ellipsis, &options, "folder-breadcrumb-menu").show(ui);
    let shown = Popup::menu(&menu)
        .frame(widgets::menu_frame(ui.ctx()))
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(260.0);
            let editing = edit.get_or_insert_with(|| FolderEdit::open(folder));
            if panel::folder(ui, editing, folder, language, actions) {
                ui.close();
            }
        });
    if shown.is_none() {
        *edit = None;
    }
}

/// A switcher's button: its name cut to its room, and the chevron.
fn switch(ui: &mut Ui, label: &str, name: &str, author_id: &str, widest: f32) -> Response {
    let p = palette(ui.ctx());
    let galley = widgets::text(ui, name, 12.5, 400.0, FontFamily::Proportional, p.text_0);
    let width = switch_width(ui, name).min(widest);
    let room = width - 44.0;
    let corner = CornerRadius::same(8);
    let response = interact(
        ui,
        Vec2::new(width, 32.0),
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::Button, true, label),
        corner,
    );
    let lit =
        response.hovered() || Popup::is_id_open(ui.ctx(), Popup::default_response_id(&response));
    let edge = if lit { p.amber_dim } else { p.line_soft };
    let rect = response.rect;
    ui.painter().rect(
        rect,
        corner,
        p.bg_2,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    let left = rect.left() + 12.0;
    let shown = if galley.size().x > room {
        cut(ui, name, room)
    } else {
        galley
    };
    let text_rect = Rect::from_min_size(
        pos2(left, rect.center().y - shown.size().y / 2.0),
        shown.size(),
    );
    // In a child: allocated in the row, the name would bring the row's cursor back.
    let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(text_rect));
    let named = inside.allocate_rect(text_rect, Sense::hover());
    named.widget_info(|| WidgetInfo::labeled(Role::Label, true, name));
    widgets::address(&inside, &named, &format!("{author_id}-name"));
    ui.painter().galley(text_rect.min, shown, p.text_0);
    let chevron = Rect::from_center_size(
        pos2(rect.right() - 16.0, rect.center().y),
        Vec2::splat(12.0),
    );
    icons::paint(ui, chevron, Icon::ArrowDown, p.text_2);
    response
}

fn cut(ui: &Ui, words: &str, width: f32) -> std::sync::Arc<egui::Galley> {
    let p = palette(ui.ctx());
    let mut job = egui::text::LayoutJob::single_section(
        words.to_owned(),
        egui::TextFormat::simple(egui::FontId::new(12.5, FontFamily::Proportional), p.text_0),
    );
    job.wrap = egui::text::TextWrapping {
        max_width: width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

#[derive(Clone, Copy)]
enum Mark {
    Pin,
    Plus,
    Swatch(egui::Color32),
}

/// A choice of a switcher's menu.
fn option(ui: &mut Ui, label: &str, author_id: &str, mark: Option<Mark>, on: bool) -> Response {
    let width = ui.available_width();
    entry(ui, width, label, author_id, mark, on)
}

/// A choice and the ⋯ that opens its panel.
fn row(
    ui: &mut Ui,
    (label, author_id): (&str, &str),
    (options, options_id): (&str, &str),
    mark: Option<Mark>,
    on: bool,
) -> (Response, Response) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let width = ui.available_width() - MIN_TARGET - 2.0;
        let chosen = entry(ui, width, label, author_id, mark, on);
        let edit = widgets::IconButton::new(Icon::Ellipsis, options, options_id).show(ui);
        (chosen, edit)
    })
    .inner
}

fn entry(
    ui: &mut Ui,
    width: f32,
    label: &str,
    author_id: &str,
    mark: Option<Mark>,
    on: bool,
) -> Response {
    let p = palette(ui.ctx());
    let corner = CornerRadius::same(6);
    let response = interact(
        ui,
        Vec2::new(width, 28.0),
        Sense::click(),
        author_id,
        || WidgetInfo::selected(Role::MenuItemRadio, true, on, label),
        corner,
    );
    let rect = response.rect;
    if on {
        ui.painter()
            .rect_filled(rect, corner, p.amber_text.gamma_multiply(0.08));
    } else if response.hovered() {
        ui.painter().rect_filled(rect, corner, p.bg_3);
    }
    let ink = if on { p.amber_text } else { p.text_1 };
    let mut left = rect.left() + 10.0;
    let middle = rect.center().y;
    match mark {
        Some(Mark::Pin) => {
            let glyph = Rect::from_center_size(pos2(left + 6.0, middle), Vec2::splat(12.0));
            icons::paint(ui, glyph, Icon::Pin, p.text_2);
            left += 18.0;
        }
        Some(Mark::Plus) => {
            let glyph = Rect::from_center_size(pos2(left + 6.0, middle), Vec2::splat(12.0));
            icons::paint(ui, glyph, Icon::Plus, p.text_2);
            left += 18.0;
        }
        Some(Mark::Swatch(hue)) => {
            let swatch = Rect::from_center_size(pos2(left + 4.0, middle), Vec2::splat(8.0));
            ui.painter().rect_filled(swatch, CornerRadius::same(2), hue);
            left += 14.0;
        }
        None => {}
    }
    let galley = cut(ui, label, rect.right() - 8.0 - left);
    ui.painter()
        .galley(pos2(left, middle - galley.size().y / 2.0), galley, ink);
    response
}

/// A step of the breadcrumb: a quiet button, its folder's hue before it.
fn crumb(ui: &mut Ui, label: &str, author_id: &str, hue: Option<egui::Color32>) -> Response {
    let p = palette(ui.ctx());
    let galley = widgets::text(ui, label, 12.5, 400.0, FontFamily::Proportional, p.text_1);
    let swatch = if hue.is_some() { 14.0 } else { 0.0 };
    let corner = CornerRadius::same(6);
    let response = interact(
        ui,
        galley.size() + Vec2::new(14.0 + swatch, 10.0),
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::Button, true, label),
        corner,
    );
    let rect = response.rect;
    let ink = if response.hovered() {
        ui.painter().rect_filled(rect, corner, p.bg_3);
        p.text_0
    } else {
        p.text_1
    };
    if let Some(hue) = hue {
        let at =
            Rect::from_center_size(pos2(rect.left() + 11.0, rect.center().y), Vec2::splat(8.0));
        ui.painter().rect_filled(at, CornerRadius::same(2), hue);
    }
    let galley = widgets::text(ui, label, 12.5, 400.0, FontFamily::Proportional, ink);
    ui.painter().galley(
        pos2(
            rect.left() + 7.0 + swatch,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        ink,
    );
    response
}

fn separator(ui: &mut Ui, mark: &str) {
    let p = palette(ui.ctx());
    let galley = widgets::text(ui, mark, 12.5, 400.0, FontFamily::Proportional, p.text_2);
    let (rect, _) = ui.allocate_exact_size(galley.size() + Vec2::new(4.0, 0.0), Sense::hover());
    ui.painter()
        .galley(rect.min + Vec2::new(2.0, 0.0), galley, p.text_2);
}

fn note(ui: &mut Ui, words: &str, author_id: &str) {
    let p = palette(ui.ctx());
    let galley = widgets::text(ui, words, 11.0, 400.0, FontFamily::Proportional, p.text_2);
    let (rect, response) =
        ui.allocate_exact_size(galley.size() + Vec2::new(20.0, 10.0), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, words));
    widgets::address(ui, &response, author_id);
    ui.painter()
        .galley(rect.min + Vec2::new(10.0, 5.0), galley, p.text_2);
}
