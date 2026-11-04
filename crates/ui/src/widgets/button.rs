//! Buttons as the reference drew them: a dialog's actions (`surface` at 6 px, 6 × 12, 12 px),
//! the primary in amber, the destructive red **at rest**, the quiet text button of a field's
//! head, the 32 px rail toggle.

use egui::{
    Color32, CornerRadius, FontFamily, Pos2, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui,
    Vec2, WidgetInfo,
};

use super::{dimmed, hovered, interact, text, towards};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A dialog's action: a surface, amber under the pointer.
    Outline,
    /// The one primary action: solid amber.
    Filled,
    /// Text alone, a fill when pressed: a field's « Show ».
    Quiet,
    /// Red at rest, not only under the pointer: a warning that arrives on hover comes after
    /// the choice. Never a solid fill, which would read as a second accent.
    Destructive,
}

pub struct Button<'a> {
    label: &'a str,
    author_id: &'a str,
    kind: Kind,
    icon: Option<Icon>,
    enabled: bool,
    /// A destructive action waiting for its confirmation, or a quiet toggle that is on.
    armed: bool,
    large: bool,
}

impl<'a> Button<'a> {
    pub fn new(label: &'a str, author_id: &'a str) -> Self {
        Self {
            label,
            author_id,
            kind: Kind::Outline,
            icon: None,
            enabled: true,
            armed: false,
            large: false,
        }
    }

    #[must_use]
    pub fn kind(mut self, kind: Kind) -> Self {
        self.kind = kind;
        self
    }

    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn armed(mut self, armed: bool) -> Self {
        self.armed = armed;
        self
    }

    /// The gate's submit: as wide as its panel, 10 px of padding, 8 px corners, 14 px at 600.
    #[must_use]
    pub fn large(mut self) -> Self {
        self.large = true;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let (size, weight, padding, corner) = match (self.large, self.kind) {
            (true, _) => (14.0, 600.0, Vec2::splat(10.0), 8),
            (false, Kind::Quiet) => (12.0, 400.0, Vec2::new(6.0, 0.0), 6),
            (false, _) => (12.0, 400.0, Vec2::new(12.0, 6.0), 6),
        };
        let probe = text(
            ui,
            self.label,
            size,
            weight,
            FontFamily::Proportional,
            Color32::WHITE,
        );
        let icon_room = if self.icon.is_some() { 16.0 + 6.0 } else { 0.0 };
        let mut wanted = probe.size() + 2.0 * padding + Vec2::new(icon_room, 0.0);
        if self.large {
            wanted.x = ui.available_width();
        }
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let corner = CornerRadius::same(corner);
        let (label, enabled) = (self.label, self.enabled);
        let response = interact(
            ui,
            wanted,
            sense,
            self.author_id,
            || WidgetInfo::labeled(Role::Button, enabled, label),
            corner,
        );
        let hover = if self.enabled {
            hovered(ui, &response)
        } else {
            0.0
        };
        let (fill, edge, ink) = self.colours(ui, hover);
        let rect = response.rect;
        let painter = ui.painter();
        painter.rect(
            rect,
            corner,
            fill,
            Stroke::new(1.0, edge),
            StrokeKind::Inside,
        );
        let galley = text(ui, self.label, size, weight, FontFamily::Proportional, ink);
        let width = galley.size().x + icon_room;
        let mut left = rect.center().x - width / 2.0;
        if let Some(icon) = self.icon {
            let place =
                Rect::from_center_size(Pos2::new(left + 8.0, rect.center().y), Vec2::splat(16.0));
            icons::paint(ui, place, icon, ink);
            left += icon_room;
        }
        painter.galley(
            Pos2::new(left, rect.center().y - galley.size().y / 2.0),
            galley,
            ink,
        );
        response
    }

    /// Fill, edge and text at `hover` (0 at rest, 1 under the pointer).
    fn colours(&self, ui: &Ui, hover: f32) -> (Color32, Color32, Color32) {
        let p = palette(ui.ctx());
        let (fill, edge, ink) = match self.kind {
            Kind::Outline => (
                p.bg_2,
                towards(p.line_soft, p.amber_dim, hover),
                towards(p.text_1, p.amber_text, hover),
            ),
            Kind::Filled => (p.amber_fill, p.amber_edge, p.amber_ink),
            Kind::Quiet if self.armed => (p.bg_3, Color32::TRANSPARENT, p.amber_text),
            Kind::Quiet => (
                Color32::TRANSPARENT,
                Color32::TRANSPARENT,
                towards(p.text_2, p.text_1, hover),
            ),
            Kind::Destructive => {
                let lit = if self.armed { 1.0 } else { hover };
                (
                    towards(p.bg_2, p.bg_3, lit),
                    towards(p.red.gamma_multiply(0.3), p.red, lit),
                    p.red,
                )
            }
        };
        (
            dimmed(fill, self.enabled),
            dimmed(edge, self.enabled),
            dimmed(ink, self.enabled),
        )
    }
}

/// A control that is an icon alone: its label is its accessible name only.
pub struct IconButton<'a> {
    icon: Icon,
    label: &'a str,
    author_id: &'a str,
    pressed: bool,
}

impl<'a> IconButton<'a> {
    pub fn new(icon: Icon, label: &'a str, author_id: &'a str) -> Self {
        Self {
            icon,
            label,
            author_id,
            pressed: false,
        }
    }

    #[must_use]
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let corner = CornerRadius::same(6);
        let (label, pressed) = (self.label, self.pressed);
        let response = interact(
            ui,
            Vec2::splat(super::MIN_TARGET),
            Sense::click(),
            self.author_id,
            || WidgetInfo::selected(Role::Button, true, pressed, label),
            corner,
        );
        let p = palette(ui.ctx());
        let hover = hovered(ui, &response);
        let ink = if self.pressed {
            p.amber_text
        } else {
            towards(p.text_2, p.text_0, hover)
        };
        if self.pressed || hover > 0.0 {
            let fill = if self.pressed {
                p.bg_3
            } else {
                p.bg_3.gamma_multiply(hover)
            };
            ui.painter().rect_filled(response.rect, corner, fill);
        }
        let glyph = Rect::from_center_size(response.rect.center(), Vec2::splat(16.0));
        icons::paint(ui, glyph, self.icon, ink);
        response
    }
}

/// The action and, joined to it, the arrow that opens its other choices.
pub struct SplitButton<'a> {
    label: &'a str,
    more_label: &'a str,
    author_id: &'a str,
}

impl<'a> SplitButton<'a> {
    pub fn new(label: &'a str, more_label: &'a str, author_id: &'a str) -> Self {
        Self {
            label,
            more_label,
            author_id,
        }
    }

    /// The action's response, then the arrow's.
    pub fn show(self, ui: &mut Ui) -> (Response, Response) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let main = Button::new(self.label, self.author_id).show(ui);
            let more_id = format!("{}-more", self.author_id);
            let more = IconButton::new(Icon::ChevronRight, self.more_label, &more_id).show(ui);
            (main, more)
        })
        .inner
    }
}

/// The 32 px square that hides and shows a rail; amber while the rail is out.
pub struct SquareButton<'a> {
    icon: Icon,
    label: &'a str,
    author_id: &'a str,
    open: bool,
}

impl<'a> SquareButton<'a> {
    pub fn new(icon: Icon, label: &'a str, author_id: &'a str) -> Self {
        Self {
            icon,
            label,
            author_id,
            open: false,
        }
    }

    /// Amber while what it shows is shown.
    #[must_use]
    pub fn pressed(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let corner = CornerRadius::same(8);
        let (label, open) = (self.label, self.open);
        let response = interact(
            ui,
            Vec2::splat(32.0),
            Sense::click(),
            self.author_id,
            || WidgetInfo::selected(Role::Button, true, open, label),
            corner,
        );
        let p = palette(ui.ctx());
        let hover = hovered(ui, &response);
        let rest = if self.open { p.amber_text } else { p.text_1 };
        ui.painter().rect(
            response.rect,
            corner,
            p.bg_2,
            Stroke::new(1.0, towards(p.line_soft, p.amber_dim, hover)),
            StrokeKind::Inside,
        );
        let glyph = Rect::from_center_size(response.rect.center(), Vec2::splat(16.0));
        icons::paint(ui, glyph, self.icon, towards(rest, p.text_0, hover));
        response
    }
}
