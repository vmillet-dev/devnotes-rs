//! The HTTP area: its rail of collections beside the open requests (plan § 7 « Phase 6 »). The
//! rail is the library's, and so is its width.

mod rail;

use egui::{Rect, Role, Sense, Ui, UiBuilder, WidgetInfo};
use i18n::keys;

use crate::action::{Action as Asked, Actions, AppAction};
use crate::state::AppState;
use crate::theme::palette;
use crate::widgets::{self, address};

/// What the area's controls ask, applied at the end of the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// A collection or a folder folded or unfolded.
    Toggle(String),
    Write(crate::state::http::Write),
    /// `Alt` and an arrow on a row: moved, then shown and said.
    KeyMove {
        item: engine::http::model::HttpItem,
        name: String,
        key: crate::state::http::moves::KeyMove,
    },
    /// The question, once Rust has counted what goes with it.
    AskDelete(engine::http::model::HttpItem),
    CancelDelete,
}

/// What only the area's drawing sees (D13).
#[derive(Default)]
pub struct View {
    rail_width: widgets::RailWidth,
    rail: rail::RailView,
}

pub fn show(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let (region, response) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Main, true, "area-http"));
    address(ui, &response, "area-http");
    let mut rest = region;
    if state.settings.rail_shown {
        let setting = (
            state.settings.rail_width,
            crate::services::settings::RAIL_WIDTH,
        );
        rest = view
            .rail_width
            .show(ui, region, setting, "http-rail-edge", |ui| {
                rail::show(ui, state, &mut view.rail, actions);
            });
    }
    let mut workspace = ui.new_child(UiBuilder::new().max_rect(rest));
    empty_workspace(&mut workspace, rest, state.language);
    if let Some(width) = view.rail_width.rested(ui, state.settings.rail_width) {
        actions.push(Asked::App(AppAction::SetRailWidth(width)));
    }
}

/// The first screen of the area says what it can do.
fn empty_workspace(ui: &mut Ui, region: Rect, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, keys::http::workspace::EMPTY, &[]);
    let galley = widgets::text(
        ui,
        &words,
        13.0,
        400.0,
        egui::FontFamily::Proportional,
        p.text_2,
    );
    let at = region.center() - galley.size() / 2.0;
    let rect = Rect::from_min_size(at, galley.size());
    let response = ui.allocate_rect(rect, Sense::hover());
    ui.painter().galley(at, galley, p.text_2);
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    address(ui, &response, "http-workspace-empty");
}

#[cfg(test)]
pub(crate) mod tests;
