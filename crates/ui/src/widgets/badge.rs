//! What labels a thing rather than acts: a chip, a priority's dot, a kind's or a language's
//! badge, a key cap, the spinner.

use egui::{
    Color32, CornerRadius, FontFamily, Response, Role, Sense, Stroke, StrokeKind, Ui, Vec2,
    WidgetInfo,
};
use engine::notes::priority::Priority;

use super::{hovered, interact, text, towards};
use crate::theme::{motion, palette};

/// A pill a rail toggles: amber edge and label when selected.
pub struct Chip<'a> {
    label: &'a str,
    author_id: &'a str,
    selected: bool,
}

impl<'a> Chip<'a> {
    pub fn new(label: &'a str, author_id: &'a str, selected: bool) -> Self {
        Self {
            label,
            author_id,
            selected,
        }
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let p = palette(ui.ctx());
        let probe = text(
            ui,
            self.label,
            12.0,
            400.0,
            FontFamily::Proportional,
            p.text_1,
        );
        let corner = CornerRadius::same(u8::MAX);
        let (label, selected) = (self.label, self.selected);
        let response = interact(
            ui,
            probe.size() + Vec2::new(20.0, 6.0),
            Sense::click(),
            self.author_id,
            || WidgetInfo::selected(Role::CheckBox, true, selected, label),
            corner,
        );
        let lit = if self.selected {
            1.0
        } else {
            hovered(ui, &response)
        };
        let edge = towards(p.line_soft, p.amber_dim, lit);
        let ink = if self.selected {
            p.amber_text
        } else {
            p.text_1
        };
        ui.painter().rect(
            response.rect,
            corner,
            Color32::TRANSPARENT,
            Stroke::new(1.0, edge),
            StrokeKind::Inside,
        );
        let galley = text(ui, self.label, 12.0, 400.0, FontFamily::Proportional, ink);
        let at = response.rect.center() - galley.size() / 2.0;
        ui.painter().galley(at, galley, ink);
        response
    }
}

/// A priority's 8 px dot, in the hue its pill carries; « none » keeps the unlit grey.
pub fn priority_dot(ui: &mut Ui, priority: Priority) -> Response {
    let p = palette(ui.ctx());
    let colour = match priority {
        Priority::None => p.line_no,
        Priority::Low => p.text_2,
        Priority::Medium => p.blue,
        Priority::High => p.amber_text,
        Priority::Urgent => p.red,
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, colour);
    response
}

/// A kind's or a language's badge: the hue is the information, so the label keeps it and the
/// fill goes neutral, with a hairline in the hue that gives the badge its shape on a light page.
pub fn hue_badge(ui: &mut Ui, label: &str, hue: Color32) -> Response {
    let p = palette(ui.ctx());
    let words = label.to_uppercase();
    let galley = text(ui, &words, 9.5, 700.0, FontFamily::Monospace, hue);
    let (rect, response) =
        ui.allocate_exact_size(galley.size() + Vec2::new(12.0, 4.0), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, label));
    let corner = CornerRadius::same(4);
    ui.painter().rect(
        rect,
        corner,
        p.bg_3,
        Stroke::new(1.0, hue.gamma_multiply(0.4)),
        StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, hue);
    response
}

/// A key cap, as the shortcuts sheet and the search field's hint draw it.
pub struct Kbd<'a>(pub &'a str);

impl Kbd<'_> {
    pub fn show(self, ui: &mut Ui) -> Response {
        let p = palette(ui.ctx());
        let galley = text(ui, self.0, 10.5, 400.0, FontFamily::Monospace, p.text_1);
        let (rect, response) =
            ui.allocate_exact_size(galley.size() + Vec2::new(12.0, 2.0), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, self.0));
        ui.painter().rect(
            rect,
            CornerRadius::same(4),
            p.bg_3,
            Stroke::new(1.0, p.line),
            StrokeKind::Inside,
        );
        ui.painter()
            .galley(rect.center() - galley.size() / 2.0, galley, p.text_1);
        response
    }
}

/// What is waited for: a turning arc, still under reduced motion.
pub fn spinner(ui: &mut Ui, size: f32, label: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::ProgressIndicator, true, label));
    let stroke = Stroke::new(2.0, palette(ui.ctx()).text_2);
    motion::spinner(ui, rect.center(), size / 2.0 - 1.0, stroke);
    response
}
