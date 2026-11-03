//! The frame of a rail that can be widened, the handle between two panes, and the tooltip.

use egui::{CursorIcon, Key, Modifiers, Response, Role, Sense, Stroke, Ui, Vec2, WidgetInfo};

use super::{address, hovered, towards};
use crate::theme::palette;

/// A rail on the left, as wide as `width` within `min..=max`; a drag on its edge, or the arrows
/// when the edge has the focus, change it. The new width is the caller's to keep.
pub fn rail_frame(
    ui: &mut Ui,
    author_id: &str,
    width: &mut f32,
    (min, max): (f32, f32),
    content: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.vertical(|ui| {
            ui.set_width(width.clamp(min, max));
            content(ui);
        });
        let moved = split_handle(ui, author_id);
        *width = (*width + moved).clamp(min, max);
    });
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
    if response.has_focus() {
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
                |ui, width: &mut f32| {
                    rail_frame(ui, "rail-edge", width, (160.0, 520.0), |ui| {
                        ui.label("Bibliothèque");
                    });
                },
                160.0,
            );
        harness.run();

        harness
            .get_by(|node| node.author_id() == Some("rail-edge"))
            .focus();
        harness.run();
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert!((*harness.state() - 176.0).abs() < f32::EPSILON);

        for _ in 0..40 {
            harness.key_press(Key::ArrowRight);
        }
        harness.run();
        assert!((*harness.state() - 520.0).abs() < f32::EPSILON);
    }
}
