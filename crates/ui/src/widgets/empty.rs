//! The one way a screen says it holds nothing: a sentence in the middle of its room, and under
//! it the actions that would fill it.

use egui::{Align, Layout, Rect, Role, Sense, Ui, UiBuilder, Vec2, WidgetInfo, pos2};

use super::{Button, Kind, address};
use crate::theme::palette;
use crate::theme::tokens::{font, gap};

/// An action under the sentence: its words, its `author_id`, and whether it is the one primary.
pub struct EmptyAction<'a> {
    pub label: &'a str,
    pub author_id: &'a str,
    pub primary: bool,
}

/// Draws in `room` (a scrolling list's has no bottom: the sentence then stands near its top);
/// answers the rank of the action clicked.
pub fn empty_state(
    ui: &mut Ui,
    room: Rect,
    (words, author_id): (&str, &str),
    actions: &[EmptyAction<'_>],
) -> Option<usize> {
    let p = palette(ui.ctx());
    let width = (room.width() - 2.0 * gap::PANEL).max(gap::PANEL);
    let galley = super::wrapped(ui, words, (font::READING, 400.0), p.text_2, width);
    let row: f32 = actions
        .iter()
        .map(|action| Button::width(ui, action.label) + gap::CONTROLS)
        .sum::<f32>()
        - gap::CONTROLS;
    let height = galley.size().y
        + if actions.is_empty() {
            0.0
        } else {
            gap::GROUPS + super::MIN_TARGET + gap::CONTROLS
        };
    let top = if room.height().is_finite() && room.height() < 4_000.0 {
        (room.center().y - height / 2.0).max(room.top())
    } else {
        room.top() + gap::PAGE
    };
    let column_rect = Rect::from_min_max(pos2(room.left(), top), pos2(room.right(), top + height));
    let mut column = ui.new_child(
        UiBuilder::new()
            .max_rect(column_rect)
            .layout(Layout::top_down(Align::Center)),
    );
    column.spacing_mut().item_spacing.y = gap::GROUPS;
    let (rect, said) = column.allocate_exact_size(galley.size(), Sense::hover());
    said.widget_info(|| WidgetInfo::labeled(Role::Label, true, words));
    address(&column, &said, author_id);
    galley.paint(&column, rect.min, p.text_2);
    if actions.is_empty() {
        ui.allocate_rect(column_rect, Sense::hover());
        return None;
    }
    let (row_rect, _) = column.allocate_exact_size(
        Vec2::new(row.max(0.0), super::MIN_TARGET + gap::CONTROLS),
        Sense::hover(),
    );
    let mut buttons = column.new_child(
        UiBuilder::new()
            .max_rect(row_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    buttons.spacing_mut().item_spacing.x = gap::CONTROLS;
    let mut clicked = None;
    for (rank, action) in actions.iter().enumerate() {
        let button = Button::new(action.label, action.author_id);
        let button = if action.primary {
            button.kind(Kind::Filled)
        } else {
            button
        };
        if button.show(&mut buttons).clicked() {
            clicked = Some(rank);
        }
    }
    ui.allocate_rect(column_rect, Sense::hover());
    clicked
}
