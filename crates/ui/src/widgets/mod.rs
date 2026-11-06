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

pub mod date;
pub mod editor;
pub mod json_explorer;
pub mod rich;

pub use badge::{Chip, Kbd, hue_badge, priority_dot, spinner};
pub use button::{Button, IconButton, Kind, SplitButton, SquareButton};
pub use dialog::{DialogStack, Layer, dialog, framed_dialog};
pub use field::{Checkbox, SearchField, Searched, TextField};
pub use layout::{RailWidth, area_room, split_handle, tooltip};
pub use list::{code_viewer, virtual_list};
pub use menu::{
    ChoiceMenu, FILTER_FROM, MenuItem, Naming, arrows as menu_arrows, entry as menu_entry,
    frame as menu_frame, menu,
};
pub use release_notes::release_notes;
pub use rich::{Read, rich_reader};
pub use segmented::Segmented;

use std::sync::Arc;

use egui::{
    Color32, CornerRadius, FontFamily, FontId, Galley, Response, RichText, Sense, Ui, Vec2,
    WidgetInfo,
};

use crate::theme::icons::Icon;
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

/// Room at a place already decided — a node on a ground, a pane — under an id that `salt`
/// keeps whatever is drawn before it.
pub fn region(ui: &mut Ui, rect: egui::Rect, salt: impl egui::AsIdSalt, sense: Sense) -> Response {
    ui.new_child(egui::UiBuilder::new().id_salt(salt).max_rect(rect))
        .allocate_rect(rect, sense)
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

/// One line of `words` at a CSS weight, an ellipsis where `width` cuts it: a wrapped row
/// breaks a list.
pub(crate) fn one_line(
    ui: &Ui,
    words: &str,
    (size, weight): (f32, f32),
    ink: Color32,
    width: f32,
) -> Arc<Galley> {
    let words = RichText::new(words)
        .font(FontId::new(size, FontFamily::Proportional))
        .variation(*b"wght", weight)
        .color(ink);
    egui::WidgetText::from(words).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        width.max(1.0),
        egui::TextStyle::Body,
    )
}

/// What an arrow takes in a wrapped text, in non-breaking spaces: it keeps its room, and no line
/// breaks inside it.
const ARROW_CHARS: usize = 4;

/// A text laid out, its arrows waiting to be painted over the room kept for them.
pub struct Wrapped {
    galley: Arc<Galley>,
    /// By the rank of the character their room starts at.
    arrows: Vec<(usize, Icon)>,
}

impl Wrapped {
    pub(crate) fn size(&self) -> Vec2 {
        self.galley.size()
    }

    pub(crate) fn paint(self, ui: &Ui, at: egui::Pos2, ink: Color32) {
        for (rank, icon) in self.arrows {
            let cursor = |rank: usize| egui::text::CCursor::new(rank);
            let from = self.galley.pos_from_cursor(cursor(rank));
            let to = self.galley.pos_from_cursor(cursor(rank + ARROW_CHARS));
            let centre = at + (from.center().to_vec2() + to.center().to_vec2()) / 2.0;
            let side = to.left() - from.left();
            let glyph = egui::Rect::from_center_size(centre, Vec2::splat(side));
            crate::theme::icons::paint(ui, glyph, icon, ink);
        }
        ui.painter().galley(at, self.galley, ink);
    }
}

/// `words` at a CSS weight, wrapped at `width`, their arrows painted as `arrowed` paints them.
pub(crate) fn wrapped(
    ui: &Ui,
    words: &str,
    (size, weight): (f32, f32),
    ink: Color32,
    width: f32,
) -> Wrapped {
    let mut laid = String::with_capacity(words.len());
    let mut rank = 0;
    let mut arrows = Vec::new();
    for glyph in words.chars() {
        if let Some(icon) = arrow_of(glyph) {
            arrows.push((rank, icon));
            laid.extend(std::iter::repeat_n('\u{a0}', ARROW_CHARS));
            rank += ARROW_CHARS;
        } else {
            laid.push(glyph);
            rank += 1;
        }
    }
    let text = RichText::new(laid)
        .font(FontId::new(size, FontFamily::Proportional))
        .variation(*b"wght", weight)
        .color(ink);
    let galley = egui::WidgetText::from(text).into_galley(
        ui,
        Some(egui::TextWrapMode::Wrap),
        width.max(1.0),
        egui::TextStyle::Body,
    );
    Wrapped { galley, arrows }
}

/// The icon an arrow is painted as.
fn arrow_of(glyph: char) -> Option<Icon> {
    Some(match glyph {
        '←' => Icon::ArrowLeft,
        '↑' => Icon::ArrowUp,
        '→' => Icon::ArrowRight,
        '↓' => Icon::ArrowDown,
        '↔' => Icon::MoveHorizontal,
        '↵' => Icon::CornerDownLeft,
        '⌫' => Icon::Delete,
        _ => return None,
    })
}

/// ▾ or ▸, drawn: the glyphs are in none of the embedded faces.
pub(crate) fn triangle(ui: &Ui, centre: egui::Pos2, open: bool, ink: Color32) {
    let points = if open {
        vec![
            centre + egui::vec2(-4.0, -2.0),
            centre + egui::vec2(4.0, -2.0),
            centre + egui::vec2(0.0, 3.0),
        ]
    } else {
        vec![
            centre + egui::vec2(-2.0, -4.0),
            centre + egui::vec2(3.0, 0.0),
            centre + egui::vec2(-2.0, 4.0),
        ]
    };
    ui.painter()
        .add(egui::Shape::convex_polygon(points, ink, egui::Stroke::NONE));
}

/// Words whose arrows (← ↑ → ↓ ↔, ↵ ⌫) are painted as icons: none of the embedded faces has them,
/// the emoji face boxes ↔, and the system's are a different hand. Laid out from `left` on the line through `middle`;
/// answers the width taken, which `at = None` measures without painting.
pub(crate) fn arrowed(
    ui: &Ui,
    at: Option<egui::Pos2>,
    words: &str,
    size: f32,
    ink: Color32,
) -> f32 {
    let mut width = 0.0;
    let mut rest = words;
    loop {
        let found = rest
            .char_indices()
            .find_map(|(index, glyph)| Some((index, glyph.len_utf8(), arrow_of(glyph)?)));
        let piece = found.map_or(rest, |(index, _, _)| &rest[..index]);
        if !piece.is_empty() {
            let galley = text(ui, piece, size, 400.0, FontFamily::Proportional, ink);
            let piece_width = galley.size().x;
            if let Some(at) = at {
                let top = at.y - galley.size().y / 2.0;
                ui.painter()
                    .galley(egui::pos2(at.x + width, top), galley, ink);
            }
            width += piece_width;
        }
        let Some((index, length, icon)) = found else {
            return width;
        };
        if let Some(at) = at {
            let glyph = egui::Rect::from_center_size(
                egui::pos2(at.x + width + size / 2.0, at.y),
                Vec2::splat(size),
            );
            crate::theme::icons::paint(ui, glyph, icon, ink);
        }
        width += size;
        rest = &rest[index + length..];
    }
}

/// How far a control is under the pointer, eased over `motion::FAST`.
pub(crate) fn hovered(ui: &Ui, response: &Response) -> f32 {
    ui.ctx()
        .animate_bool_with_time(response.id.with("hover"), response.hovered(), motion::FAST)
}

/// A colour on its way to another, `t` from 0 to 1.
pub(crate) fn towards(from: Color32, to: Color32, t: f32) -> Color32 {
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
