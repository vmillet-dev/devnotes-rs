//! An action and, joined to it, the arrow that opens its other choices: one rounded rectangle cut
//! in two by a hairline.

use egui::{
    Color32, CornerRadius, FontFamily, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui, Vec2,
    WidgetInfo,
};

use super::{interact, text};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::theme::tokens::{font, radius};

const HEIGHT: f32 = 32.0;
/// On each side of the label.
const SIDE: f32 = 13.0;
const ARROW: f32 = 28.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitKind {
    /// Solid amber: the one primary action of an area.
    Primary,
    /// A surface and its outline, amber under the pointer.
    Neutral,
}

pub struct SplitButton<'a> {
    main: (&'a str, &'a str),
    more: (&'a str, &'a str),
    kind: SplitKind,
}

/// What both halves are painted with.
struct Colours {
    ink: Color32,
    edge: Stroke,
    divider: Color32,
}

impl<'a> SplitButton<'a> {
    /// Each half as its label and its `author_id`.
    pub fn new(main: (&'a str, &'a str), more: (&'a str, &'a str), kind: SplitKind) -> Self {
        Self { main, more, kind }
    }

    /// How wide it is drawn, for a row that lays out what comes before it.
    pub fn width(ui: &Ui, label: &str) -> f32 {
        let family = FontFamily::Proportional;
        let label = text(ui, label, font::READING, 700.0, family, Color32::WHITE);
        label.size().x + 2.0 * SIDE + ARROW
    }

    /// The action's response, then the arrow's. In the row it is in, not one of its own, which
    /// would stand off its line.
    pub fn show(self, ui: &mut Ui) -> (Response, Response) {
        let spacing = std::mem::take(&mut ui.spacing_mut().item_spacing.x);
        let halves = self.halves(ui);
        ui.spacing_mut().item_spacing.x = spacing;
        halves
    }

    fn halves(&self, ui: &mut Ui) -> (Response, Response) {
        let whole = CornerRadius::same(radius::MENU);
        let left = CornerRadius {
            ne: 0,
            se: 0,
            ..whole
        };
        let right = CornerRadius {
            nw: 0,
            sw: 0,
            ..whole
        };
        let width = Self::width(ui, self.main.0) - ARROW;
        let main = half(ui, width, self.main, left);
        let more = half(ui, ARROW, self.more, right);
        for (response, corner) in [(&main, left), (&more, right)] {
            let fill = self.fill(ui, response.hovered());
            ui.painter().rect_filled(response.rect, corner, fill);
        }
        let colours = self.colours(ui, main.hovered() || more.hovered());
        let rect = main.rect.union(more.rect);
        ui.painter()
            .rect_stroke(rect, whole, colours.edge, StrokeKind::Inside);
        ui.painter().vline(
            more.rect.left(),
            rect.y_range(),
            Stroke::new(1.0, colours.divider),
        );
        let family = FontFamily::Proportional;
        let words = text(ui, self.main.0, font::READING, 700.0, family, colours.ink);
        let at = main.rect.center() - words.size() / 2.0;
        ui.painter().galley(at, words, colours.ink);
        let glyph = Rect::from_center_size(more.rect.center(), Vec2::splat(12.0));
        icons::paint(ui, glyph, Icon::ArrowDown, colours.ink);
        (main, more)
    }

    fn fill(&self, ui: &Ui, hovered: bool) -> Color32 {
        let p = palette(ui.ctx());
        match self.kind {
            SplitKind::Primary if hovered => p.amber_fill.gamma_multiply(1.08),
            SplitKind::Primary => p.amber_fill,
            SplitKind::Neutral => p.bg_2,
        }
    }

    fn colours(&self, ui: &Ui, hovered: bool) -> Colours {
        let p = palette(ui.ctx());
        match self.kind {
            SplitKind::Primary => Colours {
                ink: p.amber_ink,
                edge: Stroke::NONE,
                divider: Color32::BLACK.gamma_multiply(0.22),
            },
            SplitKind::Neutral => {
                let edge = if hovered { p.amber_dim } else { p.line_soft };
                Colours {
                    ink: p.text_1,
                    edge: Stroke::new(1.0, edge),
                    divider: edge,
                }
            }
        }
    }
}

fn half(
    ui: &mut Ui,
    width: f32,
    (label, author_id): (&str, &str),
    corner: CornerRadius,
) -> Response {
    interact(
        ui,
        Vec2::new(width, HEIGHT),
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::Button, true, label),
        corner,
    )
}
