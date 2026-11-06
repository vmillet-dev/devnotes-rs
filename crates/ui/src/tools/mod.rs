//! The Tools area: its rail beside the home or the tool open (plan § 7 « Phase 7 »). A tool is a
//! session of its own, in memory alone and gone with the library: one left and found again is as
//! it was.

pub mod catalogue;
mod frame;
mod home;
pub mod live;
mod rail;

use std::collections::HashMap;

use egui::{CornerRadius, Rect, Response, Role, Sense, Ui, UiBuilder, Vec2, WidgetInfo};
use i18n::{Language, keys};

use crate::action::{Action as Asked, Actions, AppAction};
use crate::state::AppState;
use crate::state::tools::Filter;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, address, hovered, interact, towards};
use catalogue::ToolEntry;
use live::Turn;

/// What the area's controls ask, applied at the end of the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The whole catalogue, one category of it, or the favourites.
    Home(Option<Filter>),
    Open(&'static str),
    ToggleFavourite(&'static str),
    /// `Ctrl+K`: the home and its search, from an open tool too.
    SearchAsked,
    SampleLoaded,
}

/// All the frame knows of a tool, by its own session. ⚠️ What it keeps of a key or a password
/// dies with the tool, never with the session.
pub trait Tool {
    /// Once a frame on screen: what its inputs ask now, and the answers that came.
    fn update(&mut self, _turn: &mut Turn<'_>) {}

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>);

    /// Whether the frame offers « Example ».
    fn has_sample(&self) -> bool {
        false
    }

    /// Fills the inputs with test data, as typing would. Never a real secret.
    fn sample(&mut self) {}

    fn clear(&mut self);
}

/// What a tool has at hand while it is drawn.
pub struct Drawn<'a> {
    pub language: Language,
    pub actions: &'a mut Actions,
}

/// What only the area's drawing sees (D13), the tools' sessions among it.
#[derive(Default)]
pub struct View {
    rail_width: widgets::RailWidth,
    home: home::HomeView,
    sessions: HashMap<&'static str, Box<dyn Tool>>,
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
    }
}

pub fn show(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let (region, response) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Main, true, "area-tools"));
    address(ui, &response, "area-tools");
    let mut rest = region;
    if state.settings.rail_shown {
        let setting = (
            state.settings.rail_width,
            crate::services::settings::RAIL_WIDTH,
        );
        rest = view
            .rail_width
            .show(ui, region, setting, "tools-rail-edge", |ui| {
                rail::show(ui, state, actions);
            });
    }
    let mut page = ui.new_child(UiBuilder::new().max_rect(rest));
    page.set_clip_rect(rest);
    match state.tools.open_tool() {
        Some(entry) => frame::show(&mut page, rest, state, (entry, view), actions),
        None => home::show(&mut page, rest, state, &mut view.home, actions),
    }
    if let Some(width) = view.rail_width.rested(ui, state.settings.rail_width) {
        actions.push(Asked::App(AppAction::SetRailWidth(width)));
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
