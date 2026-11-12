//! A choice among a few, all in view: the reference's `segmented-choice`. The segment on is
//! filled and its label goes bright — an amber label on an amber tint is the trade the badges
//! refuse, and this one is read at 11.5 px.

use egui::{
    Color32, CornerRadius, FontFamily, Response, Role, Sense, Stroke, Ui, Vec2, WidgetInfo,
};

use super::{hovered, interact, text, towards};
use crate::theme::palette;

pub struct Segmented<'a, T> {
    options: &'a [(T, &'a str)],
    author_id: &'a str,
    taken: Option<T>,
}

impl<'a, T: PartialEq + Copy> Segmented<'a, T> {
    pub fn new(options: &'a [(T, &'a str)], author_id: &'a str) -> Self {
        Self {
            options,
            author_id,
            taken: None,
        }
    }

    /// The segment that cannot be chosen: dimmed, and a click on it does nothing.
    #[must_use]
    pub fn taken(mut self, value: T) -> Self {
        self.taken = Some(value);
        self
    }

    /// How wide it will be drawn: what a row lays out before it.
    pub fn width(&self, ui: &Ui) -> f32 {
        let p = palette(ui.ctx());
        self.options
            .iter()
            .map(|(_, label)| {
                text(ui, label, 11.5, 400.0, FontFamily::Proportional, p.text_2)
                    .size()
                    .x
                    .max(super::MIN_TARGET - 20.0)
                    + 20.0
            })
            .sum::<f32>()
            + 2.0
    }

    /// `selected` is changed by a click on another segment; the response says so.
    pub fn show(self, ui: &mut Ui, selected: &mut T) -> Response {
        let p = palette(ui.ctx());
        let width = self.width(ui);
        // ⚠️ Its place taken at once: a frame sits at the top of its row and a child centred on
        // a height of nothing runs below it, so the control landed lower than its neighbours.
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(width, super::MIN_TARGET + 2.0), Sense::hover());
        ui.painter().rect(
            rect,
            CornerRadius::same(7),
            p.bg_2,
            Stroke::new(1.0, p.line_soft),
            egui::StrokeKind::Inside,
        );
        // In its order at the end of a row too: `horizontal` follows the parent's way.
        let reading = egui::Layout::left_to_right(egui::Align::Center);
        let inside = egui::UiBuilder::new()
            .max_rect(rect.shrink(1.0))
            .layout(reading);
        ui.scope_builder(inside, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let mut changed = None;
            for (index, (value, label)) in self.options.iter().enumerate() {
                let id = format!("{}-{index}", self.author_id);
                let open = self.taken != Some(*value);
                let response = segment(ui, label, &id, (*value == *selected, open));
                if response.clicked() && *value != *selected {
                    *selected = *value;
                    changed = Some(response.clone());
                }
            }
            changed
        })
        .inner
        .map_or_else(
            || ui.allocate_response(Vec2::ZERO, Sense::hover()),
            |mut response| {
                response.mark_changed();
                response
            },
        )
    }
}

fn segment(ui: &mut Ui, label: &str, author_id: &str, (on, open): (bool, bool)) -> Response {
    let p = palette(ui.ctx());
    let probe = text(ui, label, 11.5, 400.0, FontFamily::Proportional, p.text_2);
    let corner = CornerRadius::same(6);
    let response = interact(
        ui,
        probe.size() + Vec2::new(20.0, 8.0),
        if open { Sense::click() } else { Sense::hover() },
        author_id,
        || WidgetInfo::selected(Role::RadioButton, open, on, label),
        corner,
    );
    let ink = if on {
        p.text_0
    } else if open {
        towards(p.text_2, p.text_0, hovered(ui, &response))
    } else {
        p.text_2.gamma_multiply(0.5)
    };
    if on {
        ui.painter().rect_filled(response.rect, corner, p.bg_3);
    }
    let galley = text(ui, label, 11.5, 400.0, FontFamily::Proportional, ink);
    ui.painter().galley(
        response.rect.center() - galley.size() / 2.0,
        galley,
        Color32::TRANSPARENT,
    );
    response
}
