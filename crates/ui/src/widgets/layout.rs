//! The width of the library rail, the handle between two panes, and the tooltip.

use egui::{
    CursorIcon, Key, Modifiers, Rect, Response, Role, Sense, Stroke, Ui, UiBuilder, Vec2,
    WidgetInfo,
};

use super::{address, hovered, towards};
use crate::theme::palette;

/// The library rail's width: the setting's, or where its edge is while it moves, with the
/// setting it moved from. Both areas draw the rail, and the setting is written once the pointer
/// rests: a setting that changed since, in the other area, is taken again.
#[derive(Debug, Default)]
pub struct RailWidth(Option<(u16, f32)>);

/// The whole room of an area, named: the three are found by it.
pub fn area_room(ui: &mut Ui, author_id: &str) -> Rect {
    let (region, response) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Main, true, author_id));
    address(ui, &response, author_id);
    region
}

/// The edge between the rail and what is beside it.
const EDGE: f32 = 8.0;

impl RailWidth {
    /// The rail at the left of `region`, `content` drawn in it, its edge after it: the region
    /// left beside them.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        region: Rect,
        (setting, (min, max)): (u16, (u16, u16)),
        edge_id: &str,
        content: impl FnOnce(&mut Ui),
    ) -> Rect {
        let (min, max) = (f32::from(min), f32::from(max));
        let width = self
            .0
            .filter(|(from, _)| *from == setting)
            .map_or(f32::from(setting), |(_, width)| width)
            .clamp(min, max)
            .round();
        let rail = Rect::from_min_size(region.min, Vec2::new(width, region.height()));
        let edge = Rect::from_min_size(rail.right_top(), Vec2::new(EDGE, region.height()));
        let mut rail_ui = ui.new_child(UiBuilder::new().max_rect(rail));
        rail_ui.set_clip_rect(rail);
        content(&mut rail_ui);
        let mut edge_ui = ui.new_child(UiBuilder::new().max_rect(edge));
        let moved = split_handle(&mut edge_ui, edge_id);
        self.0 = Some((setting, (width + moved).clamp(min, max)));
        Rect::from_min_max(edge.right_top(), region.max)
    }

    /// The library rail of an area, when it is out: the region left beside it.
    pub fn beside(
        &mut self,
        ui: &mut Ui,
        region: Rect,
        (shown, setting): (bool, u16),
        edge_id: &str,
        content: impl FnOnce(&mut Ui),
    ) -> Rect {
        if !shown {
            return region;
        }
        let setting = (setting, crate::services::settings::RAIL_WIDTH);
        self.show(ui, region, setting, edge_id, content)
    }

    /// The width to write, once the edge rests somewhere else than `setting`.
    pub fn rested(&self, ui: &Ui, setting: u16) -> Option<u16> {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "within RAIL_WIDTH"
        )]
        let rested = self.0?.1.round() as u16;
        (rested != setting && !ui.input(|input| input.pointer.any_down())).then_some(rested)
    }
}

/// A hairline that moves what is on either side of it: the distance it was moved this frame.
pub fn split_handle(ui: &mut Ui, author_id: &str) -> f32 {
    let height = ui.available_height().max(1.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(8.0, height), Sense::click_and_drag());
    response.widget_info(|| WidgetInfo::labeled(Role::Splitter, true, author_id));
    address(ui, &response, author_id);
    let p = palette(ui.ctx());
    let lit = hovered(ui, &response).max(if response.dragged() { 1.0 } else { 0.0 });
    ui.painter().vline(
        rect.center().x,
        rect.y_range(),
        Stroke::new(1.0, towards(p.line_soft, p.amber_dim, lit)),
    );
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    let mut moved = response.drag_delta().x;
    if super::takes_keys(ui, &response) {
        ui.input_mut(|input| {
            if input.consume_key(Modifiers::NONE, Key::ArrowLeft) {
                moved -= STEP;
            }
            if input.consume_key(Modifiers::NONE, Key::ArrowRight) {
                moved += STEP;
            }
        });
    }
    moved
}

/// What one press of an arrow moves a handle by.
const STEP: f32 = 16.0;

/// A tooltip drawn as the menus are, in place of the system's `title` (E3).
pub fn tooltip(response: Response, words: &str) -> Response {
    response.on_hover_ui(|ui| {
        let p = palette(ui.ctx());
        ui.label(egui::RichText::new(words).size(12.0).color(p.text_1));
    })
}

#[cfg(test)]
mod tests {
    use egui_kittest::kittest::Queryable;

    use super::*;

    #[test]
    fn a_rail_is_widened_within_its_bounds() {
        let mut harness = crate::testing::harness()
            .with_size(egui::vec2(800.0, 400.0))
            .build_ui_state(
                |ui, width: &mut RailWidth| {
                    let region = ui.max_rect();
                    width.show(ui, region, (160, (160, 520)), "rail-edge", |ui| {
                        ui.label("Bibliothèque");
                    });
                },
                RailWidth::default(),
            );
        harness.run();

        harness
            .get_by(|node| node.author_id() == Some("rail-edge"))
            .focus();
        harness.run();
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert_eq!(harness.state().0, Some((160, 176.0)));

        for _ in 0..40 {
            harness.key_press(Key::ArrowRight);
        }
        harness.run();
        assert_eq!(harness.state().0, Some((160, 520.0)));
    }
}
