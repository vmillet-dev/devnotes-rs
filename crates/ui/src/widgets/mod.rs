//! The interface's own components, in place of the reference's `shared/` (plan § 3.7): painted
//! by hand after its CSS, reading the theme and nothing else — no global state. Every one goes
//! through [`interact`], which gives it its response, its AccessKit node named and addressed by
//! the `data-testid` its Angular twin had, and its focus ring.

mod badge;
mod button;
mod dialog;
mod field;
mod layout;
mod list;
mod menu;
mod release_notes;
mod segmented;

pub use badge::{Chip, Kbd, hue_badge, priority_dot, spinner};
pub use button::{Button, IconButton, Kind, RailToggle, SplitButton};
pub use dialog::{DialogStack, Layer, dialog};
pub use field::{Checkbox, SearchField, TextField};
pub use layout::{rail_frame, split_handle, tooltip};
pub use list::{code_viewer, virtual_list};
pub use menu::{ChoiceMenu, FILTER_FROM, MenuItem, Naming, menu};
pub use release_notes::release_notes;
pub use segmented::Segmented;

use std::sync::Arc;

use egui::{
    Color32, CornerRadius, FontFamily, FontId, Galley, Response, RichText, Sense, Ui, Vec2,
    WidgetInfo,
};

use crate::theme::{focus, motion};

/// WCAG 2.2 AA (2.5.8): a control is never smaller than this, whatever its glyph.
pub const MIN_TARGET: f32 = 24.0;

/// Room for one control of at least `size`, its response and its node: role and name from
/// `info`, `author_id` as its address, and the focus ring after the keyboard moved.
pub fn interact(
    ui: &mut Ui,
    size: Vec2,
    sense: Sense,
    author_id: &str,
    info: impl Fn() -> WidgetInfo,
    corner: CornerRadius,
) -> Response {
    let size = if sense.senses_click() {
        size.max(Vec2::splat(MIN_TARGET))
    } else {
        size
    };
    let (_, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(info);
    address(ui, &response, author_id);
    focus::ring(ui, &response, corner);
    response
}

/// Whether the keys of the frame are `response`'s to take: the focus egui keeps, as its own
/// fields read it. ⚠️ Not `Response::has_focus`, which also asks the window's focus — a style
/// question, the caret's — and a window without it under xvfb then takes no key at all.
pub fn takes_keys(ui: &Ui, response: &Response) -> bool {
    ui.memory(|memory| memory.has_focus(response.id))
}

/// The node's `author_id`: what kittest and the e2e driver find a control by.
pub fn address(ui: &Ui, response: &Response, author_id: &str) {
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_author_id(author_id.to_string());
    });
}

/// Text at a size and a CSS weight, the variable font's `wght` axis set (ADR S2).
pub(crate) fn text(
    ui: &Ui,
    words: &str,
    size: f32,
    weight: f32,
    family: FontFamily,
    colour: Color32,
) -> Arc<Galley> {
    let words = RichText::new(words)
        .font(FontId::new(size, family))
        .variation(*b"wght", weight)
        .color(colour);
    egui::WidgetText::from(words).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Body,
    )
}

/// How far a control is under the pointer, eased over `motion::FAST`.
fn hovered(ui: &Ui, response: &Response) -> f32 {
    ui.ctx()
        .animate_bool_with_time(response.id.with("hover"), response.hovered(), motion::FAST)
}

/// A colour on its way to another, `t` from 0 to 1.
fn towards(from: Color32, to: Color32, t: f32) -> Color32 {
    from.lerp_to_gamma(to, t)
}

/// A disabled control is drawn at half strength.
fn dimmed(colour: Color32, enabled: bool) -> Color32 {
    if enabled {
        colour
    } else {
        colour.gamma_multiply(0.5)
    }
}

#[cfg(test)]
mod tests;
