//! The notes area: the canvas of cards, the header over it and the rail beside it, as they are
//! ported (plan § 7 « Phase 3 »).

pub mod calc;
pub mod canvas;
pub(crate) mod header;
pub mod language;
mod overlays;
mod panel;
pub mod prettier;
pub mod sidebar;
pub mod ui;

use egui::{Rect, Ui, UiBuilder};
use engine::notes::kind::NoteKind;
use engine::notes::priority::Priority;
use i18n::{Key, keys};

use engine::folders::model::FolderColour;
use engine::notes::checklist::ChecklistItem;
use engine::notes::view::NoteFilter;

use crate::action::{Action as Asked, Actions, AppAction};
use crate::state::AppState;
use crate::state::drawing_pictures::Stamp;

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
    /// A drawing a Note embeds: its large picture, by id.
    Picture(String),
    /// A Note's `mermaid` block, by its source.
    Diagram(String),
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
    /// Under `parent`, or at the top of `space` for `None`, with everything it holds; refused
    /// and said when it cannot go there.
    MoveFolder {
        id: String,
        parent: Option<String>,
        space: String,
    },
    /// Up a level (`outwards`), or into the folder above it among its siblings.
    NudgeFolder {
        id: String,
        outwards: bool,
    },
    /// List or board, for the space shown.
    SetView(crate::state::board::Mode),
    /// The board on screen put back in order, as far as the scope allows.
    Tidy(engine::folders::board::BoardScope),
    /// What a gesture on the board commits on release, or its keyboard twin.
    Board(BoardGesture),
    /// The band drawn last, named: a folder where it was drawn.
    NameZone(String),
    /// What the field shows; asked of the core once the keys rest.
    Search(String),
    ClearSearch,
    SetFilter(NoteFilter),
    /// The search and every facet, from a list they left empty.
    ClearFilters,
    Arrange(crate::state::notes_query::Arrangement),
    /// `Ctrl+K`: the caret into the search.
    FocusSearch,
    /// Into the editor, read whole by id.
    OpenNote(String),
    /// What the editor asks of the note it has open.
    Editor(Editing),
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
    Trash(Trash),
    Facet(crate::state::notes_query::Facet),
    Tags(Tags),
    Fill(Fill),
    /// The folder-name prompt put away.
    DismissFolderName,
}

/// What the editor asks of the note it has open, a draft or a note written.
/// A drawing to write as a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawingExport {
    /// Before its extension.
    pub name: String,
    pub content: String,
    pub palette: std::sync::Arc<drawing::Palette>,
    pub format: drawing::Format,
}

/// What the editor's fields panel asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fields {
    Keep(std::collections::BTreeMap<String, String>),
    Preview {
        session: u64,
        content: String,
        values: std::collections::BTreeMap<String, String>,
    },
    Copy {
        content: String,
        values: std::collections::BTreeMap<String, String>,
    },
    /// The text on screen, tokens and all.
    CopyRaw(String),
    /// `true`: open.
    Fold(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Editing {
    /// A sheet's results shown to so many decimals; `None` reads the value.
    Decimals(Option<u8>),
    /// The fields left in an opening; `None` leaves a field as it is.
    Save {
        session: u64,
        edit: crate::state::editor::Edit,
    },
    /// The drawing left: `missing` takes them only for a note that has none, as one imported.
    Pictures {
        session: u64,
        content: String,
        palette: std::sync::Arc<drawing::Palette>,
        missing: bool,
    },
    Export(DrawingExport),
    Fields(Fields),
    Attachment(crate::state::attachments::Asked),
    /// A snippet's document, explored once the hand paused.
    Explore {
        session: u64,
        query: engine::json::model::JsonQuery,
    },
    /// The snippet's code formatted, as the editor holds it.
    Format {
        session: u64,
        request: text::format::Request,
    },
    /// The library's formatting settings, applied as they are chosen.
    Prettier(prettier::PrettierSettings),
    /// A Note's link, followed with Ctrl+click: `http` and `https` alone.
    OpenLink(String),
    /// A sheet, once the hand paused.
    Evaluate {
        session: u64,
        text: String,
    },
    Pin(bool),
    Prioritise(Priority),
    MoveTo(String),
    /// `None`: out of every folder.
    File(Option<String>),
    /// What is typed is written first: the core converts the text on screen.
    Convert {
        session: u64,
        into: NoteKind,
        edit: crate::state::editor::Edit,
    },
    /// Into the trash; a draft is only let go.
    Delete,
    /// Read as written, then onto the clipboard; a note with fields asks for them.
    Copy,
    Close,
    Fullscreen(bool),
    FoldHeader(bool),
    /// « Présenter »: the draft on screen, cut into slides.
    Present {
        title: String,
        markdown: String,
    },
    StopPresenting,
    Revisions(Revisions),
}

/// What the history under the editor asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revisions {
    Toggle,
    /// A kept body compared with the current text.
    Look(String),
    Back,
    /// Back to the body previewed.
    Restore,
}

/// What the `{{fields}}` form asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fill {
    /// Filled with these, copied, and kept.
    Submit(std::collections::BTreeMap<String, String>),
    /// The text as it is, tokens included.
    Raw,
    Cancel,
}

/// What the tag manager asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tags {
    Open,
    Close,
    Toggle(String),
    /// The ticked tags under one name: a rename for one, a merge for several. Proposed first.
    Rename(String),
    Delete,
    Confirm,
    Cancel,
}

/// What the trash panel asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trash {
    Open,
    Close,
    Search(String),
    Restore(String),
    /// For good: the second click on the row.
    Purge(String),
    /// What the search found, or all of it.
    RestoreAll,
    Empty,
}

/// What a board gesture commits. Membership comes from the drop, and from nothing else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardGesture {
    /// `folder`: the zone under the card, `None` the ground, which files it in the board's root.
    DropCard {
        note_id: String,
        folder: Option<String>,
        at: engine::folders::board::BoardPoint,
    },
    /// Moved or resized: a frame, either way.
    MoveZone {
        folder_id: String,
        frame: engine::folders::board::BoardFrame,
    },
    /// A band on the empty ground, which asks for a name.
    DrawZone(engine::folders::board::BoardFrame),
    /// The cards a right-button sweep went over, ticked.
    Band(Vec<String>),
    /// Every card a zone draws, dimmed ones included, ticked.
    SelectFolder(String),
    /// Let go of over a row of the rail: filed there at any depth, `None` the top of the space.
    /// Offered back: it may land out of sight.
    FileOnRail {
        note_id: String,
        folder: Option<String>,
    },
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

/// What only the area's drawing sees (D13).
#[derive(Default)]
pub struct View {
    pub sidebar: sidebar::SidebarView,
    pub canvas: canvas::CanvasView,
    rail_width: crate::widgets::RailWidth,
    /// Asked by `Ctrl+K`: the search takes the focus on the next frame.
    pub(crate) focus_search: bool,
    header: header::HeaderView,
    editor: Option<overlays::editor::EditorView>,
    presentation: overlays::presentation::PresentationView,
    explorer_layout: overlays::editor::ExplorerLayout,
    trash: overlays::trash_panel::TrashView,
    tag_manager: overlays::tag_manager::TagManagerView,
    fill: overlays::placeholder_form::FillView,
    folder_name: String,
}

/// The `{{fields}}` form, over the palette as over the canvas: drawn by the window, after both.
pub fn fill_form(ui: &Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    overlays::placeholder_form::show(ui, state, &mut view.fill, actions);
}

/// The rail on the left, as wide as the setting says, and the canvas beside it.
pub fn show(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let region = crate::widgets::area_room(ui, "area-notes");
    let rail = (state.settings.rail_shown, state.settings.rail_width);
    let rest = view
        .rail_width
        .beside(ui, region, rail, "library-rail-edge", |ui| {
            sidebar::show(ui, state, &mut view.sidebar, actions);
        });
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
    // A card dragged on the board can be let go of over a row of the rail, which lights.
    view.canvas.rail = if state.settings.rail_shown {
        view.sidebar.rows().to_vec()
    } else {
        Vec::new()
    };
    canvas::show(&mut canvas_ui, state, &mut view.canvas, actions);
    view.sidebar.card_over = view.canvas.over_rail.take();
    overlays::editor::show(
        ui,
        state,
        &mut view.editor,
        &mut view.explorer_layout,
        actions,
    );
    overlays::trash_panel::show(ui, state, &mut view.trash, actions);
    overlays::tag_manager::show(ui, state, &mut view.tag_manager, actions);
    overlays::folder_prompt::show(ui, state, &mut view.folder_name, actions);
    overlays::image_lightbox::show(ui, state, actions);
    overlays::presentation::show(ui, state, &mut view.presentation, actions);
    overlays::undo_bar::show(ui, state, actions);
    if let Some(width) = view.rail_width.rested(ui, state.settings.rail_width) {
        actions.push(Asked::App(AppAction::SetRailWidth(width)));
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
pub(crate) fn priority_section(priority: Priority) -> Key {
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

#[cfg(test)]
mod parity_tests;
