//! The application's bar: the window keeps the system's decorations, so this is a menu bar
//! inside it — the areas on the left (the File and About menus join them with their phase), the
//! name in the middle, the theme on the right. 38 px on `bg-0`, a `line-soft` underline.

use egui::{
    CornerRadius, FontFamily, Margin, Rect, Role, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
};
use engine::app_info::METADATA;
use i18n::{Key, Language, Message, keys};

use crate::action::{Action, Actions, AppAction};
use crate::state::Area;
use crate::theme::icons::{self, Icon};
use crate::theme::tokens::TITLEBAR_HEIGHT;
use crate::theme::{Appearance, palette};
use crate::widgets::{self, tooltip};

/// In the switch's order, which is also the order of `Ctrl+1/2/3`.
const AREAS: [(Area, Icon, Key, &str); 3] = [
    (Area::Notes, Icon::FileText, keys::areas::NOTES, "notes"),
    (Area::Tools, Icon::Wrench, keys::areas::TOOLS, "tools"),
    (Area::Http, Icon::ArrowLeftRight, keys::areas::HTTP, "http"),
];

pub(crate) fn show(
    ui: &mut Ui,
    area: Area,
    unlocked: bool,
    language: Language,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let bar = ui.max_rect();
    ui.painter().rect_filled(bar, CornerRadius::ZERO, p.bg_0);
    ui.painter().hline(
        bar.x_range(),
        bar.bottom() - 0.5,
        Stroke::new(1.0, p.line_soft),
    );

    // Centred on the window, not on what the two groups leave between them.
    let title = widgets::text(
        ui,
        METADATA.name,
        12.0,
        400.0,
        FontFamily::Proportional,
        p.text_2,
    );
    let at = bar.center() - title.size() / 2.0;
    ui.painter().galley(at, title, p.text_2);

    let inner = bar - Margin::symmetric(12, 0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.horizontal_centered(|ui| {
            // Every entry of the menus acts on the library: none while it is locked.
            if unlocked {
                area_switch(ui, area, language, actions);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                theme_toggle(ui, language, actions);
            });
        });
    });
}

/// The areas in the bar: icons alone, their names for the screen reader and the tooltip.
fn area_switch(ui: &mut Ui, current: Area, language: Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let group = egui::Frame::new()
        .fill(p.bg_2)
        .stroke(Stroke::new(1.0, p.line_soft))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.horizontal(|ui| {
                for (rank, (area, icon, name, id)) in AREAS.into_iter().enumerate() {
                    let name = i18n::translate(language, name, &[]);
                    let on = area == current;
                    let response = widgets::interact(
                        ui,
                        Vec2::new(26.0, 24.0),
                        Sense::click(),
                        &format!("area-option-{id}"),
                        || WidgetInfo::selected(Role::Tab, true, on, &name),
                        CornerRadius::same(6),
                    );
                    let hint = Message::new(keys::areas::HINT)
                        .with("area", name.as_str())
                        .with("key", format!("Ctrl+{}", rank + 1))
                        .text(language);
                    let response = tooltip(response, &hint);
                    if on {
                        ui.painter()
                            .rect_filled(response.rect, CornerRadius::same(6), p.bg_3);
                    }
                    let ink = if on {
                        p.amber_text
                    } else if response.hovered() {
                        p.text_0
                    } else {
                        p.text_2
                    };
                    let glyph = Rect::from_center_size(response.rect.center(), Vec2::splat(14.0));
                    icons::paint(ui, glyph, icon, ink);
                    if response.clicked() && !on {
                        actions.push(Action::App(AppAction::ShowArea(area)));
                    }
                }
            });
        });
    widgets::address(ui, &group.response, "area-switch");
}

/// One gesture, two themes: the icon and the label both name what it switches **to**.
fn theme_toggle(ui: &mut Ui, language: Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let to = match ui.ctx().theme() {
        egui::Theme::Dark => Appearance::Light,
        egui::Theme::Light => Appearance::Dark,
    };
    let (icon, label) = match to {
        Appearance::Dark => (Icon::Moon, keys::settings::theme::TO_DARK),
        _ => (Icon::Sun, keys::settings::theme::TO_LIGHT),
    };
    let label = i18n::translate(language, label, &[]);
    let corner = CornerRadius::same(6);
    let response = widgets::interact(
        ui,
        Vec2::new(27.0, 24.0),
        Sense::click(),
        "theme-toggle",
        || WidgetInfo::labeled(Role::Button, true, &label),
        corner,
    );
    let response = tooltip(response, &label);
    ui.painter().rect(
        response.rect.shrink2(Vec2::new(0.0, 2.5)),
        corner,
        egui::Color32::TRANSPARENT,
        Stroke::new(1.0, p.line_soft),
        StrokeKind::Inside,
    );
    let ink = if response.hovered() {
        p.text_1
    } else {
        p.text_2
    };
    let glyph = Rect::from_center_size(response.rect.center(), Vec2::splat(13.0));
    icons::paint(ui, glyph, icon, ink);
    if response.clicked() {
        actions.push(Action::App(AppAction::SetAppearance(to)));
    }
}

/// The bar's height, for the panel that holds it.
pub(crate) const HEIGHT: f32 = TITLEBAR_HEIGHT;
