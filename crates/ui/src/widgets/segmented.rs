//! A choice among a few, all in view: the reference's `segmented-choice`. The segment on is
//! filled and its label goes bright — an amber label on an amber tint is the trade the badges
//! refuse, and this one is read at 11.5 px.

use egui::{
    Color32, CornerRadius, FontFamily, Margin, Response, Role, Sense, Stroke, Ui, Vec2, WidgetInfo,
};

use super::{hovered, interact, text, towards};
use crate::theme::palette;

pub struct Segmented<'a, T> {
    options: &'a [(T, &'a str)],
    author_id: &'a str,
}

impl<'a, T: PartialEq + Copy> Segmented<'a, T> {
    pub fn new(options: &'a [(T, &'a str)], author_id: &'a str) -> Self {
        Self { options, author_id }
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
        egui::Frame::new()
            .fill(p.bg_2)
            .stroke(Stroke::new(1.0, p.line_soft))
            .corner_radius(CornerRadius::same(7))
            .inner_margin(Margin::ZERO)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.horizontal(|ui| {
                    let mut changed = None;
                    for (index, (value, label)) in self.options.iter().enumerate() {
                        let id = format!("{}-{index}", self.author_id);
                        let response = segment(ui, label, &id, *value == *selected);
                        if response.clicked() && *value != *selected {
                            *selected = *value;
                            changed = Some(response.clone());
                        }
                    }
                    changed
                })
                .inner
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

fn segment(ui: &mut Ui, label: &str, author_id: &str, on: bool) -> Response {
    let p = palette(ui.ctx());
    let probe = text(ui, label, 11.5, 400.0, FontFamily::Proportional, p.text_2);
    let corner = CornerRadius::same(6);
    let response = interact(
        ui,
        probe.size() + Vec2::new(20.0, 8.0),
        Sense::click(),
        author_id,
        || WidgetInfo::selected(Role::RadioButton, true, on, label),
        corner,
    );
    let ink = if on {
        p.text_0
    } else {
        towards(p.text_2, p.text_0, hovered(ui, &response))
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
