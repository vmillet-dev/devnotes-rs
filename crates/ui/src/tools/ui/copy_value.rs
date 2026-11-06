//! « Copy », beside a result, and what it says for a moment once done.

use egui::{
    CornerRadius, FontFamily, Id, Rect, Response, Role, Sense, Stroke, Ui, Vec2, WidgetInfo,
};
use i18n::{Key, Language, Message, keys};

use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, hovered, interact, towards};

/// How long « Copied » shows, in seconds.
const FEEDBACK: f64 = 2.0;

pub struct CopyValue<'a> {
    value: &'a str,
    /// What is copied, for a screen reader: « Copy camelCase ».
    what: &'a str,
    language: Language,
    compact: bool,
    label: Key,
}

impl<'a> CopyValue<'a> {
    pub fn new(value: &'a str, what: &'a str, language: Language) -> Self {
        Self {
            value,
            what,
            language,
            compact: false,
            label: keys::tools::COPY,
        }
    }

    /// The icon alone, at the end of a row: its word is the tooltip's.
    #[must_use]
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    #[must_use]
    pub fn label(mut self, label: Key) -> Self {
        self.label = label;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let id = ui.next_auto_id().with("copied");
        let time = ui.input(|input| input.time);
        let copied = copied_until(ui, id).is_some_and(|until| until > time);
        let enabled = !self.value.is_empty();
        let key = if copied {
            keys::tools::COPIED
        } else {
            self.label
        };
        let word = i18n::translate(self.language, key, &[]);
        let response = if self.compact {
            self.icon(ui, copied, enabled, &word)
        } else {
            self.framed(ui, copied, enabled, &word)
        };
        if enabled && response.clicked() {
            ui.ctx().copy_text(self.value.to_owned());
            ui.data_mut(|data| data.insert_temp(id, time + FEEDBACK));
            let wait = std::time::Duration::from_secs_f64(FEEDBACK);
            ui.ctx().request_repaint_after(wait);
        }
        response
    }

    fn accessible_name(&self) -> String {
        Message::new(keys::tools::COPY_WHAT)
            .with("what", self.what)
            .text(self.language)
    }

    fn icon(&self, ui: &mut Ui, copied: bool, enabled: bool, word: &str) -> Response {
        let p = palette(ui.ctx());
        let name = self.accessible_name();
        let corner = CornerRadius::same(7);
        let response = interact(
            ui,
            Vec2::splat(28.0),
            sense(enabled),
            "copy-value",
            || WidgetInfo::labeled(Role::Button, enabled, &name),
            corner,
        );
        let hover = hovered(ui, &response);
        if hover > 0.0 {
            ui.painter().rect(
                response.rect,
                corner,
                p.bg_2.gamma_multiply(hover),
                Stroke::new(1.0, p.line.gamma_multiply(hover)),
                egui::StrokeKind::Inside,
            );
        }
        let ink = if copied {
            p.green
        } else {
            towards(p.text_2, p.text_0, hover)
        };
        let glyph = Rect::from_center_size(response.rect.center(), Vec2::splat(12.0));
        icons::paint(ui, glyph, mark(copied), ink);
        widgets::tooltip(response, word)
    }

    fn framed(&self, ui: &mut Ui, copied: bool, enabled: bool, word: &str) -> Response {
        let p = palette(ui.ctx());
        let name = self.accessible_name();
        let galley = widgets::text(ui, word, 11.5, 400.0, FontFamily::Proportional, p.text_1);
        let corner = CornerRadius::same(7);
        let response = interact(
            ui,
            Vec2::new(galley.size().x + 12.0 + 6.0 + 20.0, 28.0),
            sense(enabled),
            "copy-value",
            || WidgetInfo::labeled(Role::Button, enabled, &name),
            corner,
        );
        let hover = hovered(ui, &response);
        let rect = response.rect;
        ui.painter().rect(
            rect,
            corner,
            p.bg_2,
            Stroke::new(1.0, towards(p.line, p.amber_dim, hover)),
            egui::StrokeKind::Inside,
        );
        let ink = if copied {
            p.amber_text
        } else {
            towards(p.text_1, p.text_0, hover)
        };
        let glyph = Rect::from_center_size(
            egui::pos2(rect.left() + 16.0, rect.center().y),
            Vec2::splat(12.0),
        );
        icons::paint(ui, glyph, mark(copied), ink);
        let at = egui::pos2(rect.left() + 28.0, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(at, galley, ink);
        response
    }
}

fn copied_until(ui: &Ui, id: Id) -> Option<f64> {
    ui.data(|data| data.get_temp(id))
}

fn sense(enabled: bool) -> Sense {
    if enabled {
        Sense::click()
    } else {
        Sense::hover()
    }
}

fn mark(copied: bool) -> Icon {
    if copied { Icon::Check } else { Icon::Copy }
}
