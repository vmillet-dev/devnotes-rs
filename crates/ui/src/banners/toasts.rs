//! What was done and what failed, stacked in the window's bottom right corner. What was done, in
//! green, goes by its cross or by its time, which runs while the pointer is away from them; what
//! failed, in red, stays until its cross. Above every modal, as the undo bar: what a dialog did is
//! said while it is open.

use std::time::Duration;

use crate::theme::tokens::{font, gap, radius};
use egui::accesskit::Live;
use egui::{
    Align2, Color32, Context, CornerRadius, Id, Order, Rect, Role, Sense, Stroke, StrokeKind, Ui,
    Vec2, WidgetInfo, pos2, vec2,
};
use i18n::{Language, keys};

use crate::action::{Action, Actions, AppAction};
use crate::state::AppState;
use crate::theme::icons::{self, Icon};
use crate::theme::{motion, palette};
use crate::widgets::{self, tooltip};

const WIDTH: f32 = 320.0;
/// From the corner.
const FROM_EDGE: f32 = gap::PANEL;
const EDGE: f32 = 3.0;
const GLYPH: f32 = 14.0;
const CLOSE: f32 = 24.0;

/// Told by its icon and its edge, and by how it is said to a screen reader.
#[derive(Clone, Copy)]
struct Kind {
    icon: Icon,
    hue: Color32,
    /// What was done must not interrupt; what failed must.
    live: Live,
    author_id: &'static str,
    dismiss_id: &'static str,
}

/// What failed nearest the corner: the reports come and go over it without moving it.
pub(crate) fn show(ctx: &Context, state: &AppState, actions: &mut Actions) {
    let (status, errors) = (&state.status, &state.errors);
    if status.latest().is_none() && errors.notice().is_none() {
        return;
    }
    let language = state.language;
    let p = palette(ctx);
    let done = Kind {
        icon: Icon::Info,
        hue: p.green,
        live: Live::Polite,
        author_id: "status-toast",
        dismiss_id: "status-dismiss",
    };
    let failed = Kind {
        icon: Icon::TriangleAlert,
        hue: p.red,
        live: Live::Assertive,
        author_id: "error-banner",
        dismiss_id: "error-dismiss",
    };
    let area = Id::unique("status-toasts");
    let pointed = ctx
        .memory(|memory| memory.area_rect(area))
        .zip(ctx.input(|input| input.pointer.hover_pos()))
        .is_some_and(|(rect, at)| rect.contains(at));
    egui::Area::new(area)
        .order(Order::Tooltip)
        .anchor(Align2::RIGHT_BOTTOM, vec2(-FROM_EDGE, -lifted(ctx)))
        .show(ctx, |ui| {
            ui.set_width(WIDTH);
            ui.spacing_mut().item_spacing.y = gap::CONTROLS;
            for (said, message) in status.shown() {
                let closed = toast(ui, &message.text(language), done, language);
                if closed || timed_out(ui, said, pointed) {
                    actions.push(Action::App(AppAction::DismissStatus(said)));
                }
            }
            for (said, message) in errors.shown() {
                if toast(ui, &message.text(language), failed, language) {
                    actions.push(Action::App(AppAction::DismissError(said)));
                }
            }
        });
}

/// From the window's foot: over the undo bar, in a window where it reaches the corner.
fn lifted(ctx: &Context) -> f32 {
    let bar = crate::notes::undo_bar_layer();
    let bar = ctx.memory(|memory| {
        let shown = memory.areas().visible_last_frame(&bar);
        shown.then(|| memory.area_rect(bar.id)).flatten()
    });
    let corner = ctx.content_rect().right() - FROM_EDGE - WIDTH;
    match bar {
        Some(bar) if bar.right() + gap::CONTROLS > corner => {
            FROM_EDGE + bar.height() + gap::CONTROLS
        }
        _ => FROM_EDGE,
    }
}

/// `true` once its time is over, counted from when it was first drawn and started again as long
/// as the pointer is on one of them.
fn timed_out(ui: &Ui, said: u64, pointed: bool) -> bool {
    let now = ui.input(|input| input.time);
    let since = Id::unique("status-toast-since").with(said);
    let started: f64 = ui
        .data(|data| data.get_temp(since))
        .filter(|_| !pointed)
        .unwrap_or(now);
    ui.data_mut(|data| data.insert_temp(since, started));
    let left = f64::from(motion::REPORT) - (now - started);
    if left <= 0.0 {
        return true;
    }
    ui.ctx()
        .request_repaint_after(Duration::from_secs_f64(left));
    false
}

/// `true` when its cross was clicked.
fn toast(ui: &mut Ui, words: &str, kind: Kind, language: Language) -> bool {
    let p = palette(ui.ctx());
    let left = gap::SNUG + EDGE + gap::INSET + GLYPH + gap::INSET;
    let room = WIDTH - left - gap::SNUG - CLOSE - gap::CONTROLS;
    let galley = ui.painter().layout(
        words.to_owned(),
        egui::FontId::proportional(font::BODY),
        p.text_0,
        room,
    );
    let height = (galley.size().y + 2.0 * gap::INSET).max(CLOSE + 2.0 * gap::SNUG);
    let (card, response) = ui.allocate_exact_size(vec2(WIDTH, height), Sense::hover());
    let corner = CornerRadius::same(radius::CARD);
    ui.painter().add(p.shadow_overlay.as_shape(card, corner));
    ui.painter().rect(
        card,
        corner,
        p.bg_1,
        Stroke::new(1.0, p.line),
        StrokeKind::Inside,
    );
    let edge = Rect::from_min_size(
        card.min + vec2(gap::SNUG, gap::SNUG),
        vec2(EDGE, height - 2.0 * gap::SNUG),
    );
    ui.painter()
        .rect_filled(edge, CornerRadius::same(radius::MARK), kind.hue);
    response.widget_info(|| WidgetInfo::labeled(Role::Status, true, words));
    widgets::address(ui, &response, kind.author_id);
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_live(kind.live));
    let top = card.center().y - galley.size().y / 2.0;
    // On the first line, where the words begin.
    let line = galley
        .rows
        .first()
        .map_or(galley.size().y, |row| row.height());
    let glyph = Rect::from_center_size(
        pos2(edge.right() + gap::INSET + GLYPH / 2.0, top + line / 2.0),
        Vec2::splat(GLYPH),
    );
    icons::paint(ui, glyph, kind.icon, kind.hue);
    ui.painter()
        .galley(pos2(card.left() + left, top), galley, p.text_0);
    let cross = Rect::from_min_size(
        pos2(card.right() - gap::SNUG - CLOSE, card.top() + gap::SNUG),
        Vec2::splat(CLOSE),
    );
    let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(cross));
    dismiss(&mut inside, kind.dismiss_id, language)
}

fn dismiss(ui: &mut Ui, author_id: &str, language: Language) -> bool {
    let p = palette(ui.ctx());
    let label = i18n::translate(language, keys::errors::DISMISS, &[]);
    let corner = CornerRadius::same(radius::CONTROL);
    let close = widgets::interact(
        ui,
        Vec2::splat(CLOSE),
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::Button, true, &label),
        corner,
    );
    let close = tooltip(close, &label);
    let ink = if close.hovered() {
        ui.painter().rect_filled(close.rect, corner, p.bg_3);
        p.text_0
    } else {
        p.text_2
    };
    let glyph = Rect::from_center_size(close.rect.center(), Vec2::splat(11.0));
    icons::paint(ui, glyph, Icon::X, ink);
    close.clicked()
}
