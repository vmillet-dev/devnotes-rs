//! The library rail: every space, its folders nested under it, and each one's panel — pin,
//! rename, recolour, a folder inside, delete (`library-tree` in the reference). A row says where
//! to go; `AppState` turns it into the space and the folder the canvas shows.

mod row;
mod tree;

use std::collections::HashSet;

use egui::{FontFamily, Key, Margin, Modifiers, Pos2, Rect, Ui};
use i18n::keys;

use crate::action::{Action, Actions};
use crate::notes::panel::{self, Draft, FolderEdit, SpaceEdit};
use crate::state::{AppState, Area};
use crate::theme::icons::Icon;
use crate::theme::{palette, spacing};
use crate::widgets::{self, IconButton};
use row::{Lead, Mark, Name};
use tree::Row;

/// What only the rail's drawing sees: what is closed, which panel is open, what is dragged.
#[derive(Default)]
pub struct SidebarView {
    collapsed: HashSet<String>,
    panel: Panel,
    /// The space and folder rows as last drawn: where a dragged folder or card would land.
    rows: Vec<RailRow>,
    /// The folder whose row is being dragged.
    dragging: Option<String>,
    /// The row a dragged folder is over.
    lit: Option<String>,
    /// The row a card dragged on the board is over, said by the board.
    pub card_over: Option<String>,
}

/// A row something can be let go of over: a space's, which is its top, or a folder's.
#[derive(Debug, Clone)]
pub struct RailRow {
    pub rect: Rect,
    pub space_id: String,
    /// `None` for the space's own row.
    pub folder_id: Option<String>,
}

impl RailRow {
    fn id(&self) -> &str {
        self.folder_id.as_deref().unwrap_or(&self.space_id)
    }
}

/// One at a time: two open forms in a rail this narrow read as one.
#[derive(Default)]
enum Panel {
    #[default]
    None,
    NewSpace(Draft),
    NewFolder(Draft),
    Space(SpaceEdit),
    Folder(FolderEdit),
}

impl SidebarView {
    fn toggle(&mut self, id: &str) {
        if !self.collapsed.remove(id) {
            self.collapsed.insert(id.to_owned());
        }
    }

    /// Choosing a row opens it: a row that selects without showing what is inside is half a
    /// tree.
    fn reveal(&mut self, id: &str) {
        self.collapsed.remove(id);
    }

    pub fn rows(&self) -> &[RailRow] {
        &self.rows
    }

    fn lit(&self, id: &str) -> bool {
        self.lit.as_deref() == Some(id) || self.card_over.as_deref() == Some(id)
    }
}

pub fn show(ui: &mut Ui, state: &AppState, view: &mut SidebarView, actions: &mut Actions) {
    let s = spacing(ui.ctx());
    let margin = Margin {
        left: 10,
        right: 10,
        top: to_margin(s.card),
        bottom: to_margin(s.canvas),
    };
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        ui.set_min_height(ui.available_height());
        ui.spacing_mut().item_spacing.y = 6.0;
        crate::titlebar::area_switch::rail(ui, Area::Notes, state.language, actions);
        head(ui, view, state.language);
        let language = state.language;
        if let Panel::NewSpace(draft) = &mut view.panel {
            let words = panel::Words::new(
                language,
                keys::notes::NEW_SPACE_LABEL,
                keys::notes::NEW_SPACE_PLACEHOLDER,
                keys::notes::NEW_SPACE_SUBMIT,
            );
            let ids = ("space-create-input", "space-create-submit");
            match panel::name_form(ui, draft, &words, ids) {
                panel::Typed::Name(name) => {
                    push(actions, crate::notes::Action::CreateSpace(name));
                    view.panel = Panel::None;
                }
                panel::Typed::Dropped => view.panel = Panel::None,
                panel::Typed::Typing => {}
            }
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                tree_rows(ui, state, view, actions);
            });
    });
}

fn to_margin(points: f32) -> i8 {
    #[expect(clippy::cast_possible_truncation, reason = "a few points")]
    let margin = points.round() as i8;
    margin
}

fn push(actions: &mut Actions, action: crate::notes::Action) {
    actions.push(Action::Notes(action));
}

/// « SPACES » and the « + » that names a new one.
fn head(ui: &mut Ui, view: &mut SidebarView, language: i18n::Language) {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        let title = i18n::translate(language, keys::sidebar::LIBRARY, &[]).to_uppercase();
        let galley = widgets::text(ui, &title, 10.5, 600.0, FontFamily::Proportional, p.text_2);
        let room = ui.available_width() - widgets::MIN_TARGET - ui.spacing().item_spacing.x;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(room, 24.0), egui::Sense::hover());
        ui.painter().galley(
            Pos2::new(rect.left(), rect.center().y - galley.size().y / 2.0),
            galley,
            p.text_2,
        );
        let label = i18n::translate(language, keys::notes::NEW_SPACE, &[]);
        if IconButton::new(Icon::Plus, &label, "space-create-open")
            .show(ui)
            .clicked()
        {
            view.panel = Panel::NewSpace(Draft::default());
        }
    });
}

fn tree_rows(ui: &mut Ui, state: &AppState, view: &mut SidebarView, actions: &mut Actions) {
    let language = state.language;
    let spaces = &state.spaces;
    let active_space = spaces.active_space();
    let rows = tree::rows(
        spaces.spaces(),
        state.folders.list(),
        active_space,
        &view.collapsed,
    );
    view.rows.clear();
    for row in rows {
        match row {
            Row::All => {
                let name = Name {
                    label: i18n::translate(language, keys::notes::ALL_SPACES, &[]),
                    author_id: "space-option-all",
                    mark: Mark::Nothing,
                    active: active_space.is_none(),
                    lit: false,
                };
                let drawn = row::show(ui, 0, &Lead::Sparkle, &name, None);
                if drawn.name.clicked() {
                    view.panel = Panel::None;
                    push(actions, crate::notes::Action::ChooseSpace(None));
                }
            }
            Row::Space { space, open } => {
                space_row(ui, state, view, actions, space, open);
            }
            Row::Folder {
                folder,
                depth,
                nested,
                open,
            } => {
                let at = Place {
                    depth,
                    nested,
                    open,
                };
                folder_row(ui, state, view, actions, folder, at);
            }
            Row::NewFolder { space } => new_folder(ui, view, actions, &space.id, language),
            Row::NoFolders => {
                let p = palette(ui.ctx());
                ui.horizontal(|ui| {
                    ui.add_space(row::INDENT + 8.0);
                    let words = i18n::translate(language, keys::sidebar::NO_FOLDERS, &[]);
                    let galley =
                        widgets::text(ui, &words, 11.0, 400.0, FontFamily::Proportional, p.text_2);
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(galley.size().x, galley.size().y + 8.0),
                        egui::Sense::hover(),
                    );
                    ui.painter()
                        .galley(rect.min + egui::vec2(0.0, 4.0), galley, p.text_2);
                });
            }
        }
    }
    drop_folder(ui, view, actions);
}

/// Where a folder's row sits in the tree.
struct Place {
    depth: usize,
    nested: bool,
    open: bool,
}

fn folder_row(
    ui: &mut Ui,
    state: &AppState,
    view: &mut SidebarView,
    actions: &mut Actions,
    folder: &engine::folders::model::Folder,
    Place {
        depth,
        nested,
        open,
    }: Place,
) {
    let language = state.language;
    let active_folder = state.active_folder();
    let twisty = row::Twisty {
        open,
        label: i18n::translate(
            language,
            if open {
                keys::sidebar::COLLAPSE_FOLDER
            } else {
                keys::sidebar::EXPAND_FOLDER
            },
            &[("name", folder.name.as_str().into())],
        ),
        author_id: "folder-twisty",
    };
    let lead = if nested {
        Lead::Twisty(twisty)
    } else {
        Lead::Blank
    };
    let name = Name {
        label: folder.name.clone(),
        author_id: "folder-option",
        mark: Mark::Swatch(folder.colour),
        active: active_folder == Some(folder.id.as_str()),
        lit: view.lit(&folder.id),
    };
    let menu = i18n::translate(
        language,
        keys::folders::OPTIONS_LABEL,
        &[("name", folder.name.as_str().into())],
    );
    let drawn = row::show(ui, depth + 1, &lead, &name, Some((&menu, "folder-edit")));
    view.rows.push(RailRow {
        rect: drawn.name.rect,
        space_id: folder.space_id.clone(),
        folder_id: Some(folder.id.clone()),
    });
    if drawn.name.drag_started() {
        view.dragging = Some(folder.id.clone());
    }
    nudge(ui, &drawn.name, &folder.id, actions);
    if drawn.twisty.is_some_and(|twisty| twisty.clicked()) {
        view.toggle(&folder.id);
    }
    if drawn.name.clicked() {
        view.panel = Panel::None;
        push(
            actions,
            crate::notes::Action::ChooseFolder(folder.id.clone()),
        );
    }
    if drawn.menu.is_some_and(|menu| menu.clicked()) {
        view.panel = match &view.panel {
            Panel::Folder(edit) if edit.id == folder.id => Panel::None,
            _ => Panel::Folder(FolderEdit::open(folder)),
        };
    }
    if let Panel::Folder(edit) = &mut view.panel
        && edit.id == folder.id
    {
        let made_inside = edit.nesting.is_some();
        let closed = panel::folder(ui, edit, folder, language, actions);
        if closed {
            if made_inside {
                view.reveal(&folder.id);
            }
            view.panel = Panel::None;
        }
    }
}

fn space_row(
    ui: &mut Ui,
    state: &AppState,
    view: &mut SidebarView,
    actions: &mut Actions,
    space: &engine::spaces::model::Space,
    open: bool,
) {
    let language = state.language;
    let named = [("name", space.name.as_str().into())];
    let twisty = row::Twisty {
        open,
        label: i18n::translate(
            language,
            if open {
                keys::sidebar::COLLAPSE
            } else {
                keys::sidebar::EXPAND
            },
            &named,
        ),
        author_id: "space-twisty",
    };
    let name = Name {
        label: space.name.clone(),
        author_id: "space-option",
        mark: if space.pinned {
            Mark::Pin
        } else {
            Mark::Nothing
        },
        active: state.spaces.active_space() == Some(space.id.as_str())
            && state.active_folder().is_none(),
        lit: view.lit(&space.id),
    };
    let menu = i18n::translate(language, keys::notes::SPACE_OPTIONS_LABEL, &named);
    let drawn = row::show(
        ui,
        0,
        &Lead::Twisty(twisty),
        &name,
        Some((&menu, "space-edit")),
    );
    view.rows.push(RailRow {
        rect: drawn.name.rect,
        space_id: space.id.clone(),
        folder_id: None,
    });
    if drawn.twisty.is_some_and(|twisty| twisty.clicked()) {
        view.toggle(&space.id);
    }
    if drawn.name.clicked() {
        view.panel = Panel::None;
        view.reveal(&space.id);
        push(
            actions,
            crate::notes::Action::ChooseSpace(Some(space.id.clone())),
        );
    }
    if drawn.menu.is_some_and(|menu| menu.clicked()) {
        view.panel = match &view.panel {
            Panel::Space(edit) if edit.id == space.id => Panel::None,
            _ => Panel::Space(SpaceEdit::open(space)),
        };
    }
    if let Panel::Space(edit) = &mut view.panel
        && edit.id == space.id
    {
        let targets: Vec<_> = state
            .spaces
            .spaces()
            .iter()
            .filter(|other| other.id != space.id)
            .collect();
        let contents = state.spaces.contents(&space.id);
        if panel::space(ui, edit, space, &targets, contents, language, actions) {
            view.panel = Panel::None;
        }
    }
}

/// `Alt+←` up a level, `Alt+→` into the folder above it among its siblings: the drag's twin.
/// There is no order of siblings to change, so no `Alt+↑/↓`.
fn nudge(ui: &Ui, name: &egui::Response, id: &str, actions: &mut Actions) {
    if !widgets::takes_keys(ui, name) {
        return;
    }
    for (key, outwards) in [(Key::ArrowLeft, true), (Key::ArrowRight, false)] {
        if ui.input_mut(|input| input.consume_key(Modifiers::ALT, key)) {
            let id = id.to_owned();
            push(actions, crate::notes::Action::NudgeFolder { id, outwards });
        }
    }
}

/// A folder's row let go of over another row: under that folder, or at the top of that space.
/// Whether it may go there is the state's to say.
fn drop_folder(ui: &Ui, view: &mut SidebarView, actions: &mut Actions) {
    let Some(id) = view.dragging.clone() else {
        return;
    };
    let (down, at) = ui.input(|input| (input.pointer.any_down(), input.pointer.latest_pos()));
    let over = at
        .and_then(|at| view.rows.iter().find(|row| row.rect.contains(at)))
        .filter(|row| row.folder_id.as_deref() != Some(id.as_str()))
        .cloned();
    view.lit = over.as_ref().map(|row| row.id().to_owned());
    if down {
        return;
    }
    view.dragging = None;
    view.lit = None;
    if let Some(row) = over {
        push(
            actions,
            crate::notes::Action::MoveFolder {
                id,
                parent: row.folder_id,
                space: row.space_id,
            },
        );
    }
}

/// « + New folder », or its field once asked for.
fn new_folder(
    ui: &mut Ui,
    view: &mut SidebarView,
    actions: &mut Actions,
    space_id: &str,
    language: i18n::Language,
) {
    if let Panel::NewFolder(draft) = &mut view.panel {
        let words = panel::Words::new(
            language,
            keys::folders::NEW_FOLDER_LABEL,
            keys::folders::NEW_FOLDER_PLACEHOLDER,
            keys::folders::NEW_FOLDER_SUBMIT,
        );
        let ids = ("folder-create-input", "folder-create-submit");
        let typed = ui
            .horizontal(|ui| {
                ui.add_space(row::INDENT);
                ui.vertical(|ui| panel::name_form(ui, draft, &words, ids))
                    .inner
            })
            .inner;
        match typed {
            panel::Typed::Name(name) => {
                push(
                    actions,
                    crate::notes::Action::CreateFolder {
                        space_id: space_id.to_owned(),
                        parent_id: None,
                        name,
                    },
                );
                view.panel = Panel::None;
            }
            panel::Typed::Dropped => view.panel = Panel::None,
            panel::Typed::Typing => {}
        }
        return;
    }
    let name = Name {
        label: i18n::translate(language, keys::folders::NEW_FOLDER, &[]),
        author_id: "folder-create-open",
        mark: Mark::Plus,
        active: false,
        lit: false,
    };
    if row::show(ui, 1, &Lead::None, &name, None).name.clicked() {
        view.panel = Panel::NewFolder(Draft::default());
    }
}

#[cfg(test)]
mod tests;
