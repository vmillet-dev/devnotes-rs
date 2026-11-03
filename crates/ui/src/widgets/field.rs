//! Text fields and the checkbox. A field is egui's own single-line editor, restyled and framed
//! by hand as the reference's `.input`; long text and code have editors of their own (phase 4).

use egui::{
    Color32, CornerRadius, FontFamily, FontId, Margin, Rect, Response, Role, Sense, Stroke,
    StrokeKind, TextEdit, Ui, Vec2, WidgetInfo,
};

use super::{address, interact};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;

/// The reference's `.input`: 9 × 11, an 8 px corner on `bg-1`, red when the value is refused.
pub struct TextField<'a> {
    value: &'a mut String,
    name: &'a str,
    author_id: &'a str,
    hint: &'a str,
    invalid: bool,
    password: bool,
}

impl<'a> TextField<'a> {
    /// `name` is the field's accessible name: what its label says.
    pub fn new(value: &'a mut String, name: &'a str, author_id: &'a str) -> Self {
        Self {
            value,
            name,
            author_id,
            hint: "",
            invalid: false,
            password: false,
        }
    }

    #[must_use]
    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = hint;
        self
    }

    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Masked as it is typed. ⚠️ Not a passphrase field: egui keeps an undo history of what
    /// is typed; the gate has a field of its own.
    #[must_use]
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let p = palette(ui.ctx());
        let edge = if self.invalid { p.red } else { p.line };
        let frame = egui::Frame::new()
            .fill(p.bg_1)
            .stroke(Stroke::new(1.0, edge))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(11, 9));
        frame
            .show(ui, |ui| {
                let response = ui.add(
                    TextEdit::singleline(self.value)
                        .frame(egui::Frame::new())
                        .margin(Margin::ZERO)
                        .password(self.password)
                        .hint_text(self.hint)
                        .text_color(p.text_0)
                        .font(FontId::new(14.0, FontFamily::Proportional))
                        .desired_width(f32::INFINITY),
                );
                name(ui, &response, self.name);
                address(ui, &response, self.author_id);
                response
            })
            .inner
    }
}

/// The search of an area: the magnifier, the field, and the key that reaches it.
pub struct SearchField<'a> {
    value: &'a mut String,
    name: &'a str,
    author_id: &'a str,
    hint: &'a str,
    key: Option<&'a str>,
}

impl<'a> SearchField<'a> {
    pub fn new(value: &'a mut String, name: &'a str, author_id: &'a str) -> Self {
        Self {
            value,
            name,
            author_id,
            hint: "",
            key: None,
        }
    }

    #[must_use]
    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = hint;
        self
    }

    /// The shortcut drawn inside the field, « Ctrl K ».
    #[must_use]
    pub fn key(mut self, key: &'a str) -> Self {
        self.key = Some(key);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let p = palette(ui.ctx());
        egui::Frame::new()
            .fill(p.bg_1)
            .stroke(Stroke::new(1.0, p.line))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (glyph, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                    icons::paint(ui, glyph, Icon::Search, p.text_2);
                    let key_room = self.key.map_or(0.0, |key| {
                        super::text(ui, key, 10.5, 400.0, FontFamily::Monospace, p.text_1)
                            .size()
                            .x
                            + 20.0
                    });
                    let response = ui.add(
                        TextEdit::singleline(self.value)
                            .frame(egui::Frame::new())
                            .margin(Margin::ZERO)
                            .hint_text(self.hint)
                            .text_color(p.text_0)
                            .desired_width(ui.available_width() - key_room),
                    );
                    name(ui, &response, self.name);
                    address(ui, &response, self.author_id);
                    if let Some(key) = self.key {
                        super::Kbd(key).show(ui);
                    }
                    response
                })
                .inner
            })
            .inner
    }
}

/// A field's accessible name, which a hint is not: kittest refuses a field without one.
fn name(ui: &Ui, response: &Response, name: &str) {
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_label(name.to_string()));
}

/// A todo list's tick: a 16 px box in a 24 px target, solid amber when done.
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    label: &'a str,
    author_id: &'a str,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, label: &'a str, author_id: &'a str) -> Self {
        Self {
            checked,
            label,
            author_id,
        }
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let (label, checked) = (self.label, *self.checked);
        let mut response = interact(
            ui,
            Vec2::splat(super::MIN_TARGET),
            Sense::click(),
            self.author_id,
            || WidgetInfo::selected(Role::CheckBox, true, checked, label),
            CornerRadius::same(4),
        );
        if response.clicked() {
            *self.checked = !*self.checked;
            response.mark_changed();
        }
        let p = palette(ui.ctx());
        let tick = Rect::from_center_size(response.rect.center(), Vec2::splat(16.0));
        let (fill, edge) = if *self.checked {
            (p.amber_fill, p.amber_edge)
        } else {
            (Color32::TRANSPARENT, p.line)
        };
        ui.painter().rect(
            tick,
            CornerRadius::same(4),
            fill,
            Stroke::new(1.0, edge),
            StrokeKind::Inside,
        );
        if *self.checked {
            icons::paint(ui, tick.shrink(2.0), Icon::Check, p.amber_ink);
        }
        response
    }
}
