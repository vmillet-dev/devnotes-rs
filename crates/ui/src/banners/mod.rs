//! What failed, a band under the title bar (red, announced at once, staying until dismissed),
//! and what was done, in the title bar's middle (green, announced politely, gone by itself).
//! Neither covers a control, and the report moves nothing: it takes the name's place.

pub(crate) mod update_prompt;

use egui::accesskit::Live;
use egui::{
    CornerRadius, Margin, Rect, Role, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo,
    pos2,
};
use i18n::{Language, Message, keys};

use crate::theme::icons::{self, Icon};
use crate::theme::{motion, palette};
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

/// The report numbered `said`, in the middle of the title bar `ui` and at most `widest`
/// across: `true` when it was dismissed, by its cross or by its time, which runs while the
/// pointer is away from it.
pub(crate) fn report(
    ui: &mut Ui,
    (said, status): (u64, &Message),
    (language, widest): (Language, f32),
) -> bool {
    const DOT: f32 = 6.0;
    const GAP: f32 = 8.0;
    const CLOSE: f32 = 24.0;
    let p = palette(ui.ctx());
    let words = status.text(language);
    let room = widest - 2.0 * GAP - DOT - GAP - CLOSE - 2.0;
    let galley = widgets::one_line(ui, &words, (12.0, 400.0), p.text_0, room);
    let cut = galley.elided;
    let width = GAP + DOT + GAP + galley.size().x + GAP + CLOSE + 2.0;
    let pill = Rect::from_center_size(ui.max_rect().center(), Vec2::new(width, 28.0));
    let hue = p.green;
    ui.painter().rect(
        pill,
        CornerRadius::same(14),
        hue.gamma_multiply(0.12),
        Stroke::new(1.0, hue.gamma_multiply(0.35)),
        StrokeKind::Inside,
    );
    let middle = pill.center().y;
    ui.painter()
        .circle_filled(pos2(pill.left() + GAP + DOT / 2.0, middle), DOT / 2.0, hue);
    let text = Rect::from_min_size(
        pos2(
            pill.left() + GAP + DOT + GAP,
            middle - galley.size().y / 2.0,
        ),
        galley.size(),
    );
    let mut inside = ui.new_child(UiBuilder::new().max_rect(text));
    let named = inside.allocate_rect(text, Sense::hover());
    named.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    widgets::address(&inside, &named, "status-toast");
    ui.ctx()
        .accesskit_node_builder(named.id, |node| node.set_live(Live::Polite));
    if cut {
        tooltip(named, &words);
    }
    ui.painter().galley(text.min, galley, p.text_0);
    let cross = Rect::from_center_size(
        pos2(pill.right() - 2.0 - CLOSE / 2.0, middle),
        Vec2::splat(CLOSE),
    );
    let mut inside = ui.new_child(UiBuilder::new().max_rect(cross));
    if dismiss(&mut inside, hue, language) {
        return true;
    }
    let now = ui.input(|input| input.time);
    let since = egui::Id::unique("status-report-since");
    let (from, started) = ui
        .data(|data| data.get_temp::<(u64, f64)>(since))
        .unwrap_or((said, now));
    let started = if from == said && !ui.rect_contains_pointer(pill) {
        started
    } else {
        now
    };
    ui.data_mut(|data| data.insert_temp(since, (said, started)));
    let left = f64::from(motion::REPORT) - (now - started);
    if left <= 0.0 {
        return true;
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs_f64(left));
    false
}

/// The report's cross, 24 × 24 in the pill: `true` when clicked.
fn dismiss(ui: &mut Ui, hue: egui::Color32, language: Language) -> bool {
    let p = palette(ui.ctx());
    let label = i18n::translate(language, keys::errors::DISMISS, &[]);
    let close = widgets::interact(
        ui,
        Vec2::splat(24.0),
        Sense::click(),
        "status-dismiss",
        || WidgetInfo::labeled(Role::Button, true, &label),
        CornerRadius::same(12),
    );
    let close = tooltip(close, &label);
    if close.hovered() {
        ui.painter()
            .rect_filled(close.rect, CornerRadius::same(12), hue.gamma_multiply(0.18));
    }
    let ink = if close.hovered() { p.text_0 } else { p.text_1 };
    icons::paint(
        ui,
        Rect::from_center_size(close.rect.center(), Vec2::splat(11.0)),
        Icon::X,
        ink,
    );
    close.clicked()
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
                let words = ui
                    .allocate_ui_with_layout(
                        Vec2::new(ui.available_width() - dismiss_room, 18.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_width(ui.available_width());
                            ui.add(egui::Label::new(text).wrap())
                        },
                    )
                    .inner;
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
