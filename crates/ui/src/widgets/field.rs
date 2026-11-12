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
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is its own display choice"
)]
pub struct TextField<'a> {
    value: &'a mut String,
    name: &'a str,
    author_id: &'a str,
    hint: &'a str,
    invalid: bool,
    password: bool,
    compact: bool,
    mono: bool,
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
            compact: false,
            mono: false,
        }
    }

    /// What is typed is code: an expression, a name a program reads.
    #[must_use]
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }

    /// A rail's: 12.5 px on 5 × 7, a 6 px corner.
    #[must_use]
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
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
        let (size, corner, margin) = if self.compact {
            (12.5, 6, Margin::symmetric(7, 5))
        } else {
            (14.0, 8, Margin::symmetric(11, 9))
        };
        let family = if self.mono {
            FontFamily::Monospace
        } else {
            FontFamily::Proportional
        };
        let frame = egui::Frame::new()
            .fill(p.bg_1)
            .stroke(Stroke::new(1.0, edge))
            .corner_radius(CornerRadius::same(corner))
            .inner_margin(margin);
        frame
            .show(ui, |ui| {
                let response = ui.add(
                    TextEdit::singleline(self.value)
                        .frame(egui::Frame::new())
                        .margin(Margin::ZERO)
                        .password(self.password)
                        .hint_text(
                            egui::RichText::new(self.hint).font(FontId::new(size, family.clone())),
                        )
                        .text_color(p.text_0)
                        .font(FontId::new(size, family))
                        .desired_width(f32::INFINITY),
                );
                name(ui, &response, self.name);
                address(ui, &response, self.author_id);
                response
            })
            .inner
    }
}

/// The reference's `setting-switch`: a track and its knob, amber when on. `on` is changed by a
/// click.
pub fn switch(ui: &mut Ui, label: &str, author_id: &str, on: &mut bool, enabled: bool) -> Response {
    let p = palette(ui.ctx());
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let mut response = interact(
        ui,
        Vec2::new(34.0, 24.0),
        sense,
        author_id,
        || WidgetInfo::selected(Role::CheckBox, enabled, *on, label),
        CornerRadius::same(9),
    );
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let track = Rect::from_center_size(response.rect.center(), Vec2::new(34.0, 18.0));
    let (fill, knob) = if *on {
        (p.amber_fill.gamma_multiply(0.35), p.amber_text)
    } else {
        (p.bg_3, p.text_2)
    };
    let (fill, knob) = (super::dimmed(fill, enabled), super::dimmed(knob, enabled));
    ui.painter().rect(
        track,
        CornerRadius::same(9),
        fill,
        Stroke::new(1.0, p.line_soft),
        StrokeKind::Inside,
    );
    let x = if *on {
        track.right() - 9.0
    } else {
        track.left() + 9.0
    };
    ui.painter()
        .circle_filled(egui::pos2(x, track.center().y), 6.0, knob);
    response
}

/// The search of an area: the magnifier, the field, the cross that empties it, and the key that
/// reaches it — or, once something is searched, how much was found.
pub struct SearchField<'a> {
    value: &'a mut String,
    name: &'a str,
    author_id: &'a str,
    hint: &'a str,
    key: Option<&'a str>,
    matched: Option<&'a str>,
    matched_id: &'a str,
    clear: Option<(&'a str, &'a str)>,
    width: Option<f32>,
}

/// What the search's controls did this frame.
pub struct Searched {
    pub field: Response,
    /// The cross: the typed text, and nothing else.
    pub cleared: bool,
}

impl<'a> SearchField<'a> {
    pub fn new(value: &'a mut String, name: &'a str, author_id: &'a str) -> Self {
        Self {
            value,
            name,
            author_id,
            hint: "",
            key: None,
            matched: None,
            matched_id: "search-matched",
            clear: None,
            width: None,
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

    /// Takes the key's place: the key is needed before a search, the count after.
    #[must_use]
    pub fn matched(mut self, matched: Option<&'a str>) -> Self {
        self.matched = matched;
        self
    }

    /// The count's own address, where two searches are on screen at once.
    #[must_use]
    pub fn matched_id(mut self, author_id: &'a str) -> Self {
        self.matched_id = author_id;
        self
    }

    /// The cross shown while something is typed: its name and its `author_id`.
    #[must_use]
    pub fn clear(mut self, label: &'a str, author_id: &'a str) -> Self {
        self.clear = Some((label, author_id));
        self
    }

    /// The box's width, all of what is left without it.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Searched {
        let p = palette(ui.ctx());
        let id = ui.next_auto_id();
        let focused = ui.memory(|memory| memory.has_focus(id.with("field")));
        let edge = if focused { p.amber_edge } else { p.line };
        let width = self.width.unwrap_or_else(|| ui.available_width());
        egui::Frame::new()
            .fill(p.bg_1)
            .stroke(Stroke::new(1.0, edge))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(10, 4))
            .show(ui, |ui| {
                ui.set_width(width - 22.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let (glyph, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                    icons::paint(ui, glyph, Icon::Search, p.text_2);
                    let trailing = self.trailing_width(ui);
                    let response = ui.add(
                        TextEdit::singleline(self.value)
                            .id(id.with("field"))
                            .frame(egui::Frame::new())
                            .margin(Margin::symmetric(0, 4))
                            .hint_text(self.hint)
                            .text_color(p.text_0)
                            .font(FontId::new(13.0, FontFamily::Proportional))
                            .desired_width((ui.available_width() - trailing).max(24.0)),
                    );
                    name(ui, &response, self.name);
                    address(ui, &response, self.author_id);
                    let mut cleared = false;
                    if let Some((label, author_id)) = self.clear
                        && !self.value.is_empty()
                    {
                        cleared = super::IconButton::new(Icon::X, label, author_id)
                            .show(ui)
                            .clicked();
                    }
                    if let Some(matched) = self.matched {
                        let galley = super::text(
                            ui,
                            matched,
                            11.0,
                            400.0,
                            FontFamily::Proportional,
                            p.text_2,
                        );
                        let (rect, count) = ui.allocate_exact_size(galley.size(), Sense::hover());
                        count.widget_info(|| WidgetInfo::labeled(Role::Label, true, matched));
                        address(ui, &count, self.matched_id);
                        ui.painter().galley(rect.min, galley, p.text_2);
                    } else if let Some(key) = self.key {
                        super::Kbd(key).show(ui);
                    }
                    Searched {
                        field: response,
                        cleared,
                    }
                })
                .inner
            })
            .inner
    }

    /// Room for what follows the field: the cross, then the count or the key.
    fn trailing_width(&self, ui: &Ui) -> f32 {
        let gap = ui.spacing().item_spacing.x;
        let cross = if self.clear.is_some() && !self.value.is_empty() {
            super::MIN_TARGET + gap
        } else {
            0.0
        };
        let words = self.matched.or(self.key).unwrap_or_default();
        let family = if self.matched.is_some() {
            FontFamily::Proportional
        } else {
            FontFamily::Monospace
        };
        let after = if words.is_empty() {
            0.0
        } else {
            super::text(ui, words, 11.0, 400.0, family, Color32::WHITE)
                .size()
                .x
                + gap
                + gap
        };
        cross + after
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
    enabled: bool,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, label: &'a str, author_id: &'a str) -> Self {
        Self {
            checked,
            label,
            author_id,
            enabled: true,
        }
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let (label, checked, enabled) = (self.label, *self.checked, self.enabled);
        let mut response = interact(
            ui,
            Vec2::splat(super::MIN_TARGET),
            if enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
            self.author_id,
            || WidgetInfo::selected(Role::CheckBox, enabled, checked, label),
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
