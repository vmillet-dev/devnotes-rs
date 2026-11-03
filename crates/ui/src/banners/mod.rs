//! The two bands under the title bar: what failed (red, announced at once) and what was done
//! (green, announced politely). One of each at a time; each stays until it is dismissed.

use egui::accesskit::Live;
use egui::{CornerRadius, Margin, Rect, Role, Sense, Stroke, Ui, Vec2, WidgetInfo};
use i18n::{Language, Message, keys};

use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, tooltip};

/// The error banner: `true` when it was dismissed.
pub(crate) fn error_banner(ui: &mut Ui, notice: &Message, language: Language) -> bool {
    let hue = palette(ui.ctx()).red;
    band(
        ui,
        notice,
        language,
        Band {
            hue,
            live: Live::Assertive,
            author_id: "error-banner",
            warns: true,
        },
    )
}

/// The status toast: `true` when it was dismissed.
pub(crate) fn status_toast(ui: &mut Ui, status: &Message, language: Language) -> bool {
    let hue = palette(ui.ctx()).green;
    band(
        ui,
        status,
        language,
        Band {
            hue,
            live: Live::Polite,
            author_id: "status-toast",
            warns: false,
        },
    )
}

#[derive(Clone, Copy)]
struct Band {
    hue: egui::Color32,
    live: Live,
    author_id: &'static str,
    warns: bool,
}

fn band(ui: &mut Ui, message: &Message, language: Language, band: Band) -> bool {
    let p = palette(ui.ctx());
    let said = message.text(language);
    let vertical = if band.warns { 10 } else { 8 };
    let shown = egui::Frame::new()
        .fill(band.hue.gamma_multiply(0.12))
        .inner_margin(Margin::symmetric(14, vertical))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                if band.warns {
                    let (glyph, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                    icons::paint(ui, glyph, Icon::TriangleAlert, band.hue);
                }
                let dismiss_room = 34.0;
                let text = egui::RichText::new(&said).size(12.5).color(p.text_0);
                let words = ui.add_sized(
                    Vec2::new(ui.available_width() - dismiss_room, 18.0),
                    egui::Label::new(text).wrap(),
                );
                let label = i18n::translate(language, keys::errors::DISMISS, &[]);
                let close = widgets::interact(
                    ui,
                    Vec2::splat(24.0),
                    Sense::click(),
                    if band.warns {
                        "error-dismiss"
                    } else {
                        "status-dismiss"
                    },
                    || WidgetInfo::labeled(Role::Button, true, &label),
                    CornerRadius::same(5),
                );
                let close = tooltip(close, &label);
                if close.hovered() {
                    ui.painter().rect_filled(
                        close.rect,
                        CornerRadius::same(5),
                        band.hue.gamma_multiply(0.18),
                    );
                }
                let ink = if close.hovered() { p.text_0 } else { p.text_1 };
                icons::paint(
                    ui,
                    Rect::from_center_size(close.rect.center(), Vec2::splat(12.0)),
                    Icon::X,
                    ink,
                );
                (words, close.clicked())
            })
            .inner
        });
    let rect = shown.response.rect;
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, band.hue.gamma_multiply(0.35)),
    );
    let (words, dismissed) = shown.inner;
    widgets::address(ui, &words, band.author_id);
    ui.ctx()
        .accesskit_node_builder(words.id, |node| node.set_live(band.live));
    dismissed
}
