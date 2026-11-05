//! The collections, their folders and requests, in the library rail's frame (`http-rail` in the
//! reference). Only the rows in view are drawn: a library holds thousands of requests.

mod row;

use egui::{FontFamily, Margin, Pos2, Ui};
use engine::http::model::HttpItemKind;
use i18n::keys;

use super::Action;
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::theme::{palette, spacing};
use crate::widgets;

/// What only the rail's drawing sees.
#[derive(Default)]
pub(super) struct RailView;

pub(super) fn show(ui: &mut Ui, state: &AppState, _view: &mut RailView, actions: &mut Actions) {
    let s = spacing(ui.ctx());
    // The tree is wider than the head by what the rail's padding can spare: the rows are what
    // run out of room.
    let margin = Margin {
        left: 4,
        right: 4,
        top: to_margin(s.card),
        bottom: to_margin(s.canvas),
    };
    let http = &state.http;
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        ui.set_min_height(ui.available_height());
        ui.spacing_mut().item_spacing.y = 6.0;
        head(ui, state.language);
        if http.loaded() && http.rows().is_empty() {
            empty(ui, state.language);
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        let rows = http.rows();
        widgets::virtual_list(ui, "http-rail-tree", row::HEIGHT, rows.len(), |ui, at| {
            let row = &rows[at];
            let drawn = row::show(ui, row, false, state.language);
            let toggled = drawn.twisty.is_some_and(|twisty| twisty.clicked())
                || (drawn.name.clicked() && row.kind != HttpItemKind::Request);
            if toggled {
                actions.push(Asked::Http(Action::Toggle(row.id.clone())));
            }
        });
    });
}

fn to_margin(points: f32) -> i8 {
    #[expect(clippy::cast_possible_truncation, reason = "a few points")]
    let margin = points.round() as i8;
    margin
}

/// « COLLECTIONS ».
fn head(ui: &mut Ui, language: i18n::Language) {
    let p = palette(ui.ctx());
    let title = i18n::translate(language, keys::http::rail::TITLE, &[]).to_uppercase();
    let galley = widgets::text(ui, &title, 10.5, 600.0, FontFamily::Proportional, p.text_2);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), widgets::MIN_TARGET),
        egui::Sense::hover(),
    );
    ui.painter().galley(
        Pos2::new(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0),
        galley,
        p.text_2,
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, &title));
    widgets::address(ui, &response, "http-rail-title");
}

fn empty(ui: &mut Ui, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, keys::http::rail::EMPTY, &[]);
    let label = egui::Label::new(egui::RichText::new(&words).size(11.5).color(p.text_2)).wrap();
    let response = egui::Frame::new()
        .inner_margin(Margin::symmetric(10, 4))
        .show(ui, |ui| ui.add(label))
        .inner;
    widgets::address(ui, &response, "http-rail-empty");
}
