//! One result of a tool: its name, what it measures, the value itself and its copy button.

use egui::{CornerRadius, Margin, Response, Role, Stroke, Ui, Vec2, WidgetInfo};
use i18n::Language;

use super::CopyValue;
use super::result_row::{value_galley, value_label};
use crate::theme::palette;
use crate::widgets::{self, address};

/// Held to a height: a long value scrolls in its own block, not the page past the others.
const MAX_HEIGHT: f32 = 10.0 * 12.0 * 1.55;

#[derive(Clone, Copy)]
pub struct OutputRow<'a> {
    name: &'a str,
    value: &'a str,
    copied: Option<&'a str>,
    meta: &'a str,
    matched: bool,
}

impl<'a> OutputRow<'a> {
    pub fn new(name: &'a str, value: &'a str) -> Self {
        Self {
            name,
            value,
            copied: None,
            meta: "",
            matched: false,
        }
    }

    /// What Copy gives when it is not what is shown: the exact value behind a rounded one.
    #[must_use]
    pub fn copied(mut self, copied: &'a str) -> Self {
        self.copied = Some(copied);
        self
    }

    /// « 256 bits »: what the value is, beside its name.
    #[must_use]
    pub fn meta(mut self, meta: &'a str) -> Self {
        self.meta = meta;
        self
    }

    /// Outlined, when this is the one that answers what was asked: a signature it matches.
    #[must_use]
    pub fn matched(mut self, matched: bool) -> Self {
        self.matched = matched;
        self
    }

    pub fn show(self, ui: &mut Ui, language: Language) -> Response {
        let p = palette(ui.ctx());
        let edge = if self.matched {
            p.green.gamma_multiply(0.5)
        } else {
            p.line_soft
        };
        let ink = if self.matched { p.green } else { p.text_0 };
        let shown = egui::Frame::new()
            .fill(p.bg_1)
            .stroke(Stroke::new(1.0, edge))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    super::paragraph(ui, self.name, (12.5, 600.0), ink);
                    if !self.meta.is_empty() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let family = egui::FontFamily::Monospace;
                            let meta = widgets::text(ui, self.meta, 10.5, 400.0, family, p.text_2);
                            let (rect, _) =
                                ui.allocate_exact_size(meta.size(), egui::Sense::hover());
                            ui.painter().galley(rect.min, meta, p.text_2);
                        });
                    }
                });
                self.line(ui, language);
            });
        let response = shown.response;
        response.widget_info(|| WidgetInfo::labeled(Role::Row, true, self.name));
        address(ui, &response, "output-row");
        response
    }

    fn line(self, ui: &mut Ui, language: Language) {
        let width = ui.available_width();
        let copy = CopyValue::new(self.copied.unwrap_or(self.value), self.name, language);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let room = width - 96.0;
            let galley = value_galley(ui, self.value, room);
            let height = galley.size().y.min(MAX_HEIGHT);
            ui.allocate_ui(Vec2::new(room, height), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(self.name)
                    .max_height(height)
                    .show(ui, |ui| {
                        ui.set_width(room);
                        value_label(ui, galley);
                    });
            });
            copy.show(ui);
        });
    }
}
