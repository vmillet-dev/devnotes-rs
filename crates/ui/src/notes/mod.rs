//! The notes area: the canvas of cards, the header over it and the rail beside it, as they are
//! ported (plan § 7 « Phase 3 »).

pub mod calc;
pub mod canvas;
mod editor;
mod header;
pub mod language;
mod panel;
pub mod sidebar;
pub mod ui;
mod undo_bar;

use egui::{Rect, Role, Sense, Ui, UiBuilder, WidgetInfo, vec2};
use engine::notes::kind::NoteKind;
use engine::notes::priority::Priority;
use i18n::{Key, keys};

use engine::folders::model::FolderColour;
use engine::notes::checklist::ChecklistItem;
use engine::notes::view::NoteFilter;

use crate::action::{Action as Asked, Actions, AppAction};
use crate::state::AppState;
use crate::state::drawing_pictures::Stamp;
use crate::widgets::address;

/// What the area's controls ask, applied at the end of the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// The view, asked again after it failed to load.
    Reload,
    /// A todo list's items, ticked from its card: the whole list.
    SetItems {
        id: String,
        items: Vec<ChecklistItem>,
    },
    /// A drawing's picture, for the card that just showed it.
    Thumbnail(Stamp),
    /// `None`: every space.
    ChooseSpace(Option<String>),
    /// In its own space, which is then the one shown.
    ChooseFolder(String),
    /// What deleting it would move and take, for its panel.
    CountSpace(String),
    CreateSpace(String),
    RenameSpace {
        id: String,
        name: String,
    },
    PinSpace {
        id: String,
        pinned: bool,
    },
    /// Its notes go to `refuge` first.
    DeleteSpace {
        id: String,
        refuge: String,
    },
    CreateFolder {
        space_id: String,
        parent_id: Option<String>,
        name: String,
    },
    RenameFolder {
        id: String,
        name: String,
    },
    RecolourFolder {
        id: String,
        colour: FolderColour,
    },
    DeleteFolder(String),
    /// What the field shows; asked of the core once the keys rest.
    Search(String),
    ClearSearch,
    SetFilter(NoteFilter),
    Arrange(crate::state::notes_query::Arrangement),
    /// `Ctrl+K`: the caret into the search.
    FocusSearch,
    /// Into the stand-in editor, read whole by id.
    OpenNote(String),
    /// The fields left in an opening; `None` leaves a field as it is.
    SaveNote {
        session: u64,
        title: Option<String>,
        body: Option<crate::state::editor::Body>,
    },
    CloseNote,
    /// A draft of that kind, or of the preferred one: written once worth keeping.
    NewNote(Option<engine::notes::kind::NoteKind>),
    Pin {
        id: String,
        pinned: bool,
    },
    Prioritise {
        id: String,
        priority: engine::notes::priority::Priority,
    },
    /// `None`: out of every folder.
    File {
        id: String,
        folder: Option<String>,
    },
    MoveTo {
        id: String,
        space: String,
    },
    /// Into the trash.
    Delete(String),
    /// The copy is opened in the editor, under the title the interface words.
    Duplicate {
        id: String,
        title: String,
    },
    /// Read whole, then onto the clipboard.
    Copy(String),
    /// A key of the canvas's table, on the card with the focus.
    Canvas(canvas::shortcuts::CanvasKey),
    /// Ticked or unticked: `Ctrl`-click, the card's box, `X`.
    Check(String),
    /// `Shift`-click: from the card that had the focus to this one.
    CheckRange {
        id: String,
        anchor: Option<String>,
    },
    ClearSelection,
    /// On every ticked note still on screen.
    Batch(Batch),
    /// The last write put back, from the undo bar.
    Undo,
    DismissUndo,
}

/// What the selection bar does to every ticked note at once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Batch {
    MoveTo(String),
    /// `None` takes them out of their folders.
    File(Option<String>),
    Prioritise(engine::notes::priority::Priority),
    Tag(String),
    Untag(String),
    Pin(bool),
    Delete,
}

/// The rail's edge, which widens it.
const EDGE: f32 = 8.0;

/// What only the area's drawing sees (D13).
#[derive(Default)]
pub struct View {
    pub sidebar: sidebar::SidebarView,
    pub canvas: canvas::CanvasView,
    /// While its edge moves; the setting is written once the pointer rests.
    rail_width: Option<f32>,
    /// Asked by `Ctrl+K`: the search takes the focus on the next frame.
    pub(crate) focus_search: bool,
    header: header::HeaderView,
    editor: Option<editor::Draft>,
}

/// The rail on the left, as wide as the setting says, and the canvas beside it.
pub fn show(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let (region, response) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Main, true, "area-notes"));
    address(ui, &response, "area-notes");
    let (min, max) = crate::services::settings::RAIL_WIDTH;
    let mut width = view
        .rail_width
        .unwrap_or_else(|| f32::from(state.settings.rail_width))
        .clamp(f32::from(min), f32::from(max))
        .round();
    // Cut by hand: a horizontal layout hands its children the height of one line.
    let mut rest = region;
    if state.settings.rail_shown {
        let rail = Rect::from_min_size(region.min, vec2(width, region.height()));
        let edge = Rect::from_min_size(rail.right_top(), vec2(EDGE, region.height()));
        rest = Rect::from_min_max(edge.right_top(), region.max);
        let mut rail_ui = ui.new_child(UiBuilder::new().max_rect(rail));
        rail_ui.set_clip_rect(rail);
        sidebar::show(&mut rail_ui, state, &mut view.sidebar, actions);
        let mut edge_ui = ui.new_child(UiBuilder::new().max_rect(edge));
        width += crate::widgets::split_handle(&mut edge_ui, "library-rail-edge");
    }
    let mut header_ui = ui.new_child(UiBuilder::new().max_rect(rest));
    header::show(
        &mut header_ui,
        state,
        (&mut view.header, &mut view.focus_search),
        actions,
    );
    let below = Rect::from_min_max(
        egui::pos2(rest.left(), header_ui.min_rect().bottom()),
        rest.max,
    );
    let mut canvas_ui = ui.new_child(UiBuilder::new().max_rect(below));
    canvas::show(&mut canvas_ui, state, &mut view.canvas, actions);
    editor::show(ui, state, &mut view.editor, actions);
    undo_bar::show(ui, state, actions);
    view.rail_width = Some(width.clamp(f32::from(min), f32::from(max)));
    let width = view.rail_width.unwrap_or(width);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "within RAIL_WIDTH"
    )]
    let rested = width.round() as u16;
    if rested != state.settings.rail_width && !ui.input(|input| input.pointer.any_down()) {
        actions.push(Asked::App(AppAction::SetRailWidth(rested)));
    }
}

/// « Snippets », the heading of a kind's section.
fn kind_plural(kind: NoteKind) -> Key {
    match kind {
        NoteKind::Snippet => keys::notes::kind_plural::SNIPPET,
        NoteKind::Note => keys::notes::kind_plural::NOTE,
        NoteKind::Checklist => keys::notes::kind_plural::CHECKLIST,
        NoteKind::Calc => keys::notes::kind_plural::CALC,
        NoteKind::Drawing => keys::notes::kind_plural::DRAWING,
    }
}

/// The heading of a priority's section: « No priority » for the notes without one.
fn priority_section(priority: Priority) -> Key {
    match priority {
        Priority::None => keys::sections::NO_PRIORITY,
        Priority::Low => keys::notes::priority::LOW,
        Priority::Medium => keys::notes::priority::MEDIUM,
        Priority::High => keys::notes::priority::HIGH,
        Priority::Urgent => keys::notes::priority::URGENT,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_kind_and_priority_heads_a_section_of_its_own() {
        for locale in [i18n::Language::En, i18n::Language::Fr] {
            let mut headings = HashSet::new();
            let keys = NoteKind::ALL
                .map(kind_plural)
                .into_iter()
                .chain(Priority::ALL.map(priority_section));
            for key in keys {
                let heading = i18n::translate(locale, key, &[]);
                assert!(headings.insert(heading.clone()), "{heading}");
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod bench;
