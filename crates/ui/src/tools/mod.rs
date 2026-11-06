//! The Tools area: its rail beside the home or the tool open (plan § 7 « Phase 7 »). A tool is a
//! session of its own, in memory alone and gone with the library: one left and found again is as
//! it was.

pub mod catalogue;
mod frame;
mod home;
pub mod live;
mod rail;
pub mod ui;

use std::collections::HashMap;

use egui::{CornerRadius, Rect, Response, Role, Sense, Ui, UiBuilder, Vec2, WidgetInfo};
use i18n::{Language, keys};

use crate::action::{Action as Asked, Actions, AppAction};
use crate::http::save_as_note;
use crate::state::AppState;
use crate::state::tools::Filter;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, hovered, interact, towards};
use catalogue::ToolEntry;
use live::Turn;
use ui::note_picker::{self, PickedNote, Picker};

/// What the area's controls ask, applied at the end of the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// The whole catalogue, one category of it, or the favourites.
    Home(Option<Filter>),
    Open(&'static str),
    ToggleFavourite(&'static str),
    /// `Ctrl+K`: the home and its search, from an open tool too.
    SearchAsked,
    SampleLoaded,
    /// The zones the clock tool lists, kept with the application.
    SetZones(Vec<String>),
}

/// All the frame knows of a tool, by its own session. ⚠️ What it keeps of a key or a password
/// dies with the tool, never with the session.
pub trait Tool {
    /// Once a frame on screen: what its inputs ask now, and the answers that came.
    fn update(&mut self, _turn: &mut Turn<'_>) {}

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>);

    /// Whether the frame offers « Save as note »: a reference has nothing to keep.
    fn keeps_result(&self) -> bool {
        false
    }

    /// ⚠️ An output, never an input: an HMAC key or a password typed into a tool cannot reach a
    /// note, because nothing here carries one.
    fn result(&self, _language: Language) -> Option<ToolResult> {
        None
    }

    /// What a note gave the input `slot`, copied in once.
    fn imported(&mut self, _slot: &str, _note: &PickedNote) {}

    /// Its own buttons in the frame's header: « Swap A and B ».
    fn actions(&self) -> Vec<ToolAction> {
        Vec::new()
    }

    fn run_action(&mut self, _id: &str, _ctx: &egui::Context) {}

    /// Whether the frame offers « Example ».
    fn has_sample(&self) -> bool {
        false
    }

    /// Fills the inputs with test data, as typing would. Never a real secret.
    fn sample(&mut self, _language: Language) {}

    fn clear(&mut self);

    /// Off the screen: what it holds of a key, a token or a password goes.
    fn left(&mut self) {}
}

/// What « Save as note » keeps: the tool decides, so the dialog asks nothing it could infer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    /// Proposed, and the dialog lets it be changed.
    pub title: String,
    pub language: engine::notes::language::Language,
    pub content: String,
    /// Said in the dialog, whose button then reads « Save anyway »: a password.
    pub warning: Option<i18n::Key>,
}

/// A button of the tool's own in the frame's header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolAction {
    pub id: &'static str,
    pub label: i18n::Key,
    pub enabled: bool,
}

/// What a tool has at hand while it is drawn.
pub struct Drawn<'a> {
    pub entry: &'static ToolEntry,
    pub language: Language,
    pub now: chrono::DateTime<chrono::Utc>,
    pub actions: &'a mut Actions,
    /// The input « Import from a note » was pressed for: its `from_note` name, and its slot.
    pub importing: Option<(&'static str, &'static str)>,
}

impl Drawn<'_> {
    pub fn push(&mut self, action: Action) {
        self.actions.push(Asked::Tools(action));
    }
}

/// What only the area's drawing sees (D13), the tools' sessions among it.
#[derive(Default)]
pub struct View {
    rail_width: widgets::RailWidth,
    home: home::HomeView,
    sessions: HashMap<&'static str, Box<dyn Tool>>,
    picker: Option<Picker>,
    note_form: Option<save_as_note::NoteForm>,
}

impl View {
    /// The home's search takes the focus on the next frame.
    pub fn want_search(&mut self) {
        self.home.want_search();
    }

    /// The tool on screen, its session made the first time it is opened.
    fn session(&mut self, entry: &'static ToolEntry) -> &mut dyn Tool {
        self.sessions
            .entry(entry.id)
            .or_insert_with(entry.make)
            .as_mut()
    }

    pub fn update(&mut self, entry: &'static ToolEntry, turn: &mut Turn<'_>) {
        self.session(entry).update(turn);
        let Some(picker) = &mut self.picker else {
            return;
        };
        if let Some(picked) = picker.update(turn) {
            let slot = picker.slot;
            self.picker = None;
            turn.status.report(said_imported(&picked));
            self.session(entry).imported(slot, &picked);
        }
    }

    pub fn left(&mut self, entry: &'static ToolEntry) {
        if let Some(session) = self.sessions.get_mut(entry.id) {
            session.left();
        }
    }

    /// A dialog of the area is open: its keys stand aside.
    pub fn is_picking(&self) -> bool {
        self.picker.is_some()
    }

    /// Tests only: a session of the test's own in the place of `id`'s.
    #[cfg(test)]
    pub fn put(&mut self, id: &'static str, tool: Box<dyn Tool>) {
        self.sessions.insert(id, tool);
    }
}

pub fn show(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let region = widgets::area_room(ui, "area-tools");
    let rail = (state.settings.rail_shown, state.settings.rail_width);
    let rest = view
        .rail_width
        .beside(ui, region, rail, "tools-rail-edge", |ui| {
            rail::show(ui, state, actions);
        });
    let mut page = ui.new_child(UiBuilder::new().max_rect(rest));
    page.set_clip_rect(rest);
    match state.tools.open_tool() {
        Some(entry) => frame::show(&mut page, rest, state, (entry, view), actions),
        None => home::show(&mut page, rest, state, &mut view.home, actions),
    }
    if let Some(picker) = &mut view.picker {
        note_picker::show(ui, state, picker);
        if picker.is_closed() {
            view.picker = None;
        }
    }
    save_as_note::show(ui, state, &mut view.note_form, actions);
    if let Some(width) = view.rail_width.rested(ui, state.settings.rail_width) {
        actions.push(Asked::App(AppAction::SetRailWidth(width)));
    }
}

fn said_imported(note: &PickedNote) -> i18n::Message {
    if note.title.is_empty() {
        i18n::Message::new(keys::tools::from_note::DONE_UNTITLED)
    } else {
        i18n::Message::new(keys::tools::from_note::DONE).with("title", note.title.as_str())
    }
}

/// How high the bar over the home and over a tool stands.
const BAR: f32 = 57.0;

/// The bar over the page: its hairline, the button of the rail, and what follows it.
fn bar(
    ui: &mut Ui,
    rest: Rect,
    margin: f32,
    state: &AppState,
    actions: &mut Actions,
) -> (Ui, Rect) {
    let p = palette(ui.ctx());
    let (bar, body) = rest.split_top_bottom_at_y(rest.top() + BAR);
    ui.painter()
        .hline(bar.x_range(), bar.bottom() - 0.5, (1.0, p.line_soft));
    let inner = bar.shrink2(Vec2::new(margin, 0.0));
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    row.spacing_mut().item_spacing.x = 12.0;
    let toggle = i18n::translate(state.language, keys::sidebar::TOGGLE, &[]);
    if widgets::SquareButton::new(Icon::Sidebar, &toggle, "tools-rail-toggle")
        .pressed(state.settings.rail_shown)
        .show(&mut row)
        .clicked()
    {
        actions.push(Asked::App(AppAction::ToggleRail));
    }
    (row, body)
}

/// The star of a tool: muted until it is on, the amber of what is chosen and a full shape.
fn star(ui: &mut Ui, on: bool, label: &str, author_id: &str, hint: &str) -> Response {
    let corner = CornerRadius::same(6);
    let response = interact(
        ui,
        Vec2::splat(28.0),
        Sense::click(),
        author_id,
        || WidgetInfo::selected(Role::Button, true, on, label),
        corner,
    );
    let p = palette(ui.ctx());
    let ink = if on {
        p.amber_text
    } else {
        towards(p.text_2, p.text_0, hovered(ui, &response))
    };
    let glyph = Rect::from_center_size(response.rect.center(), Vec2::splat(15.0));
    let icon = if on { Icon::StarFilled } else { Icon::Star };
    icons::paint(ui, glyph, icon, ink);
    widgets::tooltip(response, hint)
}

fn star_hint(language: Language, on: bool) -> String {
    let key = if on {
        keys::tools::favourites::REMOVE
    } else {
        keys::tools::favourites::ADD
    };
    i18n::translate(language, key, &[])
}

#[cfg(test)]
mod tests;
