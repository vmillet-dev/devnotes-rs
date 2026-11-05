//! A split button floating over the ground's corner: aligning the loose cards keeps one click,
//! reorganising everything takes two. The loose cards are what goes to pieces and cost nothing to
//! redo; a zone placed and sized by hand is the only manual work a board holds, so the click that
//! repairs the cheap half must not destroy the expensive one.

use egui::{
    CornerRadius, FontFamily, Popup, PopupCloseBehavior, Pos2, RectAlign, Role, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};
use engine::folders::board::BoardScope;
use i18n::keys;

use crate::action::{Action, Actions};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, interact};

fn ask(actions: &mut Actions, scope: BoardScope) {
    actions.push(Action::Notes(crate::notes::Action::Tidy(scope)));
}

/// `corner` is where its bottom right sits. The counts, not a warning: a corpus-wide action names
/// what it will touch.
pub(super) fn show(
    ui: &mut Ui,
    corner: Pos2,
    (zones, cards): (usize, usize),
    language: i18n::Language,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let counts = [
        ("zones", u32::try_from(zones).unwrap_or(u32::MAX).into()),
        ("cards", u32::try_from(cards).unwrap_or(u32::MAX).into()),
    ];
    let align = i18n::translate(language, keys::board::ALIGN, &[]);
    let hint = i18n::translate(language, keys::board::ALIGN_HINT, &counts);
    let more = i18n::translate(language, keys::board::TIDY_MORE_LABEL, &[]);
    let words = widgets::text(ui, &align, 12.0, 400.0, FontFamily::Proportional, p.text_1);
    let height = 30.0;
    let left_width = 13.0 + 14.0 + 6.0 + words.size().x + 13.0;
    let right_width = 10.0 + 14.0 + 10.0;
    let pill = egui::Rect::from_min_max(corner - vec2(left_width + right_width, height), corner);
    let mut pill_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(pill)
            .id_salt("board-tidy")
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    pill_ui.spacing_mut().item_spacing.x = 0.0;
    let round = CornerRadius::same(15);
    ui.painter().add(p.shadow_dropdown.as_shape(pill, round));
    ui.painter().rect(
        pill,
        round,
        p.bg_1,
        Stroke::new(1.0, p.line),
        StrokeKind::Inside,
    );
    let tidy = half(
        &mut pill_ui,
        left_width,
        "board-tidy",
        &align,
        CornerRadius {
            nw: 15,
            sw: 15,
            ne: 0,
            se: 0,
        },
    );
    let tidy = widgets::tooltip(tidy, &hint);
    let ink = if tidy.hovered() { p.text_0 } else { p.text_1 };
    let icon = egui::Rect::from_min_size(
        pos2(pill.left() + 13.0, pill.center().y - 7.0),
        vec2(14.0, 14.0),
    );
    icons::paint(ui, icon, Icon::Table, ink);
    ui.painter().galley(
        pos2(icon.right() + 6.0, pill.center().y - words.size().y / 2.0),
        words,
        ink,
    );
    if tidy.clicked() {
        ask(actions, BoardScope::LooseCards);
    }
    let caret = half(
        &mut pill_ui,
        right_width,
        "board-tidy-more",
        &more,
        CornerRadius {
            nw: 0,
            sw: 0,
            ne: 15,
            se: 15,
        },
    );
    ui.painter().vline(
        caret.rect.left(),
        caret.rect.y_range().shrink(6.0),
        Stroke::new(1.0, p.line_soft),
    );
    let ink = if caret.hovered() { p.text_0 } else { p.text_1 };
    let chevron = egui::Rect::from_center_size(caret.rect.center(), vec2(12.0, 12.0));
    icons::paint(ui, chevron, Icon::ChevronDown, ink);
    menu(ui, &caret, &counts, language, actions);
}
/// A notch further than the corner click, and saying what it touches: it is the only thing on
/// the board that overwrites a frame set by hand.
fn menu(
    ui: &Ui,
    caret: &egui::Response,
    counts: &[(&str, i18n::Value); 2],
    language: i18n::Language,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let everything = i18n::translate(language, keys::board::REORGANISE, counts);
    let note = i18n::translate(language, keys::board::REORGANISE_HINT, &[]);
    Popup::menu(caret)
        .frame(widgets::menu_frame(ui.ctx()))
        .align(RectAlign::TOP_END)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(240.0);
            if widgets::menu_entry(
                ui,
                &everything,
                "board-tidy-everything",
                false,
                false,
                false,
            )
            .clicked()
            {
                ask(actions, BoardScope::Everything);
                ui.close();
            }
            let mut job = egui::text::LayoutJob::single_section(
                note,
                egui::TextFormat::simple(
                    egui::FontId::new(11.0, FontFamily::Proportional),
                    p.text_2,
                ),
            );
            job.wrap.max_width = 240.0 - 18.0;
            let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
            let (rect, _) = ui.allocate_exact_size(galley.size() + vec2(18.0, 6.0), Sense::hover());
            ui.painter()
                .galley(rect.min + vec2(9.0, 2.0), galley, p.text_2);
        });
}
/// One half of the pill: a button of its own, rounded on its outer side.
fn half(
    ui: &mut Ui,
    width: f32,
    author_id: &str,
    label: &str,
    corner: CornerRadius,
) -> egui::Response {
    let p = palette(ui.ctx());
    let response = interact(
        ui,
        vec2(width, ui.available_height()),
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::Button, true, label),
        corner,
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(response.rect.shrink(1.0), corner, p.bg_2);
    }
    response
}
