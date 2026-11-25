//! An action and, joined to it, the arrow that opens its other choices: one rounded rectangle cut
//! in two by a hairline.

use egui::{
    Align, Color32, CornerRadius, FontFamily, FontId, Layout, Popup, Pos2, Rect, Response, Role,
    Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo,
};

use super::{interact, text};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::theme::tokens::{font, gap, radius};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitKind {
    /// Solid amber: the one primary action of an area.
    Primary,
    /// A surface and its outline, amber under the pointer.
    Neutral,
    /// A tint of amber and its hairline, in a toolbar: neutral once its action cannot be done.
    Tint,
}

pub struct SplitButton<'a> {
    main: (&'a str, &'a str),
    more: (&'a str, &'a str),
    kind: SplitKind,
    icon: Option<Icon>,
    keys: Option<&'a str>,
    named: bool,
    enabled: bool,
}

/// What a kind is drawn at.
struct Measures {
    height: f32,
    /// On each side of the label.
    side: f32,
    arrow: f32,
    corner: u8,
    /// The label's size and weight.
    label: (f32, f32),
    glyph: Icon,
}

/// What a half is painted with.
#[derive(Clone, Copy)]
struct Colours {
    fill: Color32,
    edge: Color32,
    ink: Color32,
}

/// Between the icon, the label and the keys.
const BETWEEN: f32 = gap::SNUG;
const ICON: f32 = 14.0;

impl<'a> SplitButton<'a> {
    /// Each half as its label and its `author_id`.
    pub fn new(main: (&'a str, &'a str), more: (&'a str, &'a str), kind: SplitKind) -> Self {
        Self {
            main,
            more,
            kind,
            icon: None,
            keys: None,
            named: true,
            enabled: true,
        }
    }

    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The keys that do the same, small after the label.
    #[must_use]
    pub fn keys(mut self, keys: &'a str) -> Self {
        self.keys = Some(keys);
        self
    }

    /// Its icon alone, where the words do not fit: they stay its name.
    #[must_use]
    pub fn icon_only(mut self) -> Self {
        self.named = false;
        self
    }

    /// Whether the action can be done: the arrow answers all the same.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// How wide it is drawn, for a row that lays out what comes before it.
    pub fn width(&self, ui: &Ui) -> f32 {
        let measures = self.measures();
        self.content_width(ui) + 2.0 * measures.side + measures.arrow
    }

    /// The action's response, then the arrow's: side by side whichever way the row goes.
    pub fn show(self, ui: &mut Ui) -> (Response, Response) {
        let measures = self.measures();
        let size = Vec2::new(self.width(ui), measures.height);
        let (whole, _) = ui.allocate_exact_size(size, Sense::hover());
        let (left, right) = whole.split_left_right_at_x(whole.right() - measures.arrow);
        let round = CornerRadius::same(measures.corner);
        let corners = [
            CornerRadius {
                ne: 0,
                se: 0,
                ..round
            },
            CornerRadius {
                nw: 0,
                sw: 0,
                ..round
            },
        ];
        let main = half(ui, left, self.main, (self.enabled, corners[0]));
        let more = half(ui, right, self.more, (true, corners[1]));
        let opened = Popup::is_id_open(ui.ctx(), Popup::default_response_id(&more));
        let acting = self.colours(ui, (self.enabled, main.hovered(), false));
        let opening = self.colours(ui, (self.enabled, more.hovered(), opened));
        let painter = ui.painter();
        painter.rect_filled(left, corners[0], acting.fill);
        painter.rect_filled(right, corners[1], opening.fill);
        if self.kind == SplitKind::Primary {
            let cut = Color32::BLACK.gamma_multiply(0.22);
            painter.vline(right.left(), right.y_range(), Stroke::new(1.0, cut));
        } else {
            let lit = main.hovered() || more.hovered() || opened;
            let edge = if lit { opening.edge } else { acting.edge };
            let edge = Stroke::new(1.0, edge);
            painter.rect_stroke(whole, round, edge, StrokeKind::Inside);
            painter.vline(right.left(), right.y_range(), edge);
        }
        self.content(ui, left, acting.ink);
        let glyph = Rect::from_center_size(right.center(), Vec2::splat(12.0));
        icons::paint(ui, glyph, measures.glyph, opening.ink);
        (main, more)
    }

    fn measures(&self) -> Measures {
        match self.kind {
            SplitKind::Primary | SplitKind::Neutral => Measures {
                height: 32.0,
                side: 13.0,
                arrow: 28.0,
                corner: radius::MENU,
                label: (font::READING, 700.0),
                glyph: Icon::ArrowDown,
            },
            SplitKind::Tint => Measures {
                height: 28.0,
                side: 11.0,
                arrow: 26.0,
                corner: radius::CONTROL,
                label: (font::LABEL, if self.enabled { 600.0 } else { 400.0 }),
                glyph: Icon::ChevronDown,
            },
        }
    }

    fn content_width(&self, ui: &Ui) -> f32 {
        let (size, weight) = self.measures().label;
        let family = FontFamily::Proportional;
        let icon = self.icon.map(|_| ICON);
        let label = self
            .named
            .then(|| text(ui, self.main.0, size, weight, family, Color32::WHITE));
        let keys = self.shown_keys().map(|keys| keys_text(ui, keys));
        let parts = [
            icon,
            label.map(|galley| galley.size().x),
            keys.map(|galley| galley.size().x),
        ];
        let parts: Vec<f32> = parts.into_iter().flatten().collect();
        let between = parts.len().saturating_sub(1);
        #[expect(clippy::cast_precision_loss, reason = "three parts at most")]
        let between = between as f32 * BETWEEN;
        parts.iter().sum::<f32>() + between
    }

    fn shown_keys(&self) -> Option<&'a str> {
        self.keys.filter(|_| self.named)
    }

    fn content(&self, ui: &Ui, rect: Rect, ink: Color32) {
        let (size, weight) = self.measures().label;
        let mut left = rect.center().x - self.content_width(ui) / 2.0;
        let middle = rect.center().y;
        if let Some(icon) = self.icon {
            let glyph =
                Rect::from_min_size(Pos2::new(left, middle - ICON / 2.0), Vec2::splat(ICON));
            icons::paint(ui, glyph, icon, ink);
            left += ICON + BETWEEN;
        }
        if self.named {
            let family = FontFamily::Proportional;
            let words = text(ui, self.main.0, size, weight, family, ink);
            let at = Pos2::new(left, middle - words.size().y / 2.0);
            left += words.size().x + BETWEEN;
            ui.painter().galley(at, words, ink);
        }
        if let Some(keys) = self.shown_keys() {
            let words = keys_text(ui, keys);
            let at = Pos2::new(left, middle - words.size().y / 2.0);
            let ink = palette(ui.ctx()).text_2;
            ui.painter().galley(at, words, ink);
        }
    }

    fn colours(&self, ui: &Ui, (enabled, hovered, opened): (bool, bool, bool)) -> Colours {
        let p = palette(ui.ctx());
        match self.kind {
            SplitKind::Primary => Colours {
                fill: p
                    .amber_fill
                    .gamma_multiply(if hovered { 1.08 } else { 1.0 }),
                edge: Color32::TRANSPARENT,
                ink: p.amber_ink,
            },
            SplitKind::Neutral => Colours {
                fill: p.bg_2,
                edge: if hovered { p.amber_dim } else { p.line_soft },
                ink: p.text_1,
            },
            SplitKind::Tint if hovered || opened => Colours {
                fill: p.amber_fill.gamma_multiply(0.2),
                edge: p.amber_edge,
                ink: if hovered { p.text_0 } else { p.amber_text },
            },
            SplitKind::Tint if enabled => Colours {
                fill: p.amber_fill.gamma_multiply(0.12),
                edge: p.amber_dim,
                ink: p.amber_text,
            },
            SplitKind::Tint => Colours {
                fill: Color32::TRANSPARENT,
                edge: p.line_soft,
                ink: p.text_2,
            },
        }
    }
}

fn keys_text(ui: &Ui, keys: &str) -> std::sync::Arc<egui::Galley> {
    let font = FontId::monospace(font::BADGE);
    ui.painter()
        .layout_no_wrap(keys.to_owned(), font, palette(ui.ctx()).text_2)
}

fn half(
    ui: &mut Ui,
    rect: Rect,
    (label, author_id): (&str, &str),
    (enabled, corner): (bool, CornerRadius),
) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let mut place = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Min)),
    );
    let info = || WidgetInfo::labeled(Role::Button, enabled, label);
    interact(&mut place, rect.size(), sense, author_id, info, corner)
}
