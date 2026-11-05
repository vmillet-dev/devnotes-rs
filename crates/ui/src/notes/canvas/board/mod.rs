//! The second view of a space: its folders as zones from stored frames, the cards filed there
//! flowing inside them, the loose ones sitting freely on a dotted ground that pans and never
//! zooms. Only what crosses the viewport is drawn (plan § 3.6).

mod zone;

use egui::{
    CornerRadius, FontFamily, Pos2, Rect, Role, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2,
    WidgetInfo, pos2, vec2,
};
use engine::folders::board::{
    BoardFrame, BoardNote, BoardPoint, BoardView, CARD_HEIGHT, CARD_WIDTH,
};
use i18n::{Key, keys};

use super::selection_bar::{self, Shown};
use super::{CanvasView, card, padded, state_text};
use crate::action::{Action, Actions};
use crate::notes::panel::FolderEdit;
use crate::state::AppState;
use crate::theme::{palette, spacing};
use crate::widgets::{self, Button, address};

/// The lattice the ground is dotted with, and that every gesture lands on.
pub(super) const GRID_PX: f32 = 20.0;

/// Where a card the view never placed is drawn: `folders::board` places every one it reads.
const UNPLACED: BoardPoint = BoardPoint { x: 16, y: 16 };

/// What only the board's drawing sees.
#[derive(Default)]
pub(super) struct BoardScreen {
    /// The zone whose ⋯ is open, and what its panel holds.
    menu: Option<FolderEdit>,
}

pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    canvas_view: &mut CanvasView,
    actions: &mut Actions,
) {
    let language = state.language;
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let canvas = spacing(ui.ctx()).canvas;
    let board = &state.board;
    if board.error().is_some() {
        padded(ui, canvas, |ui| {
            state_text(ui, &tr(keys::errors::BOARD_LOAD_FAILED), "canvas-error");
            if Button::new(&tr(keys::errors::RETRY), "canvas-retry")
                .show(ui)
                .clicked()
            {
                actions.push(Action::Notes(crate::notes::Action::Reload));
            }
        });
        return;
    }
    let Some(view) = board.view() else {
        padded(ui, canvas, |ui| {
            state_text(ui, &tr(keys::notes::LOADING), "canvas-loading");
        });
        return;
    };
    let ids: Vec<&str> = board.cards().map(|entry| entry.note.id.as_str()).collect();
    let context = card::Context::of(state, &ids);
    // Said again by whichever card has it this frame, as on the list.
    canvas_view.anchor = canvas_view.focused.take();
    let mut screen = std::mem::take(&mut canvas_view.board);
    ground(ui, view, &context, &mut screen, (canvas_view, actions));
    canvas_view.board = screen;
    let shown: Vec<_> = board
        .cards()
        .map(|entry| Shown {
            id: &entry.note.id,
            pinned: entry.note.pinned,
            tags: &entry.note.tags,
        })
        .collect();
    selection_bar::show(ui, state, &shown, canvas_view, actions);
}

/// The dotted ground, in its frame: the only thing that scrolls, both ways.
fn ground(
    ui: &mut Ui,
    view: &BoardView,
    context: &card::Context<'_>,
    screen: &mut BoardScreen,
    (canvas_view, actions): (&mut CanvasView, &mut Actions),
) {
    let p = palette(ui.ctx());
    let region = ui.max_rect();
    let frame = Rect::from_min_max(region.min + vec2(24.0, 16.0), region.max - vec2(24.0, 24.0));
    ui.painter().rect(
        frame,
        CornerRadius::same(10),
        p.bg_0,
        Stroke::new(1.0, p.line_soft),
        StrokeKind::Inside,
    );
    let inner = frame.shrink(1.0);
    let ground = widgets::region(ui, inner, "board-ground", Sense::hover());
    ground.widget_info(|| WidgetInfo::labeled(Role::Region, true, "board"));
    address(ui, &ground, "board");
    let mut ground_ui = ui.new_child(UiBuilder::new().max_rect(inner).id_salt("board"));
    ground_ui.set_clip_rect(inner);
    egui::ScrollArea::both()
        .id_salt("board")
        .auto_shrink([false, false])
        .show_viewport(&mut ground_ui, |ui, viewport| {
            // At least the visible ground, whatever the core sized it: past the content's edge
            // the dotted ground still takes the pointer.
            let size = vec2(px(view.width), px(view.height)).max(viewport.size());
            let surface = Rect::from_min_size(ui.min_rect().min, size);
            let drawn = ui.allocate_rect(surface, Sense::hover());
            drawn.widget_info(|| WidgetInfo::labeled(Role::Canvas, true, ""));
            address(ui, &drawn, "board-surface");
            dots(ui, surface.min);
            let origin = surface.min;
            reveal(ui, view, origin, canvas_view.request.as_deref());
            let clip = ui.clip_rect();
            for shown in &view.zones {
                let rect = frame_rect(origin, shown.frame);
                if rect.intersects(clip) {
                    zone::show(ui, rect, shown, context, screen, (canvas_view, actions));
                }
            }
            for entry in &view.loose {
                let rect = card_rect(origin, entry.position.unwrap_or(UNPLACED));
                if rect.intersects(clip) {
                    loose(ui, rect, entry, context, (canvas_view, actions));
                }
            }
            if view.zones.is_empty() && view.loose.is_empty() {
                empty(ui, ui.clip_rect(), context.language);
            }
        });
}

/// A loose card: it sits where it was left.
fn loose(
    ui: &mut Ui,
    rect: Rect,
    entry: &BoardNote,
    context: &card::Context<'_>,
    (canvas_view, actions): (&mut CanvasView, &mut Actions),
) {
    let mut card_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .id_salt(("board-card", &entry.note.id)),
    );
    let seat = card_ui.allocate_rect(rect, Sense::hover());
    let title = entry.note.title.clone();
    seat.widget_info(|| WidgetInfo::labeled(Role::Group, true, &title));
    address(&card_ui, &seat, "board-loose-card");
    card::show(
        &mut card_ui,
        rect,
        &entry.note,
        context,
        (canvas_view, actions),
    );
}

/// A card asked to take the focus may sit out of view, so not be drawn: the ground scrolls to
/// it, a zone's own body then to the card.
fn reveal(ui: &Ui, view: &BoardView, origin: Pos2, request: Option<&str>) {
    let Some(id) = request else { return };
    let zone = view
        .zones
        .iter()
        .find(|zone| zone.notes.iter().any(|entry| entry.note.id == id))
        .map(|zone| frame_rect(origin, zone.frame));
    let card = view
        .loose
        .iter()
        .find(|entry| entry.note.id == id)
        .map(|entry| card_rect(origin, entry.position.unwrap_or(UNPLACED)));
    if let Some(rect) = zone.or(card) {
        ui.scroll_to_rect(rect, None);
    }
}

/// One dot every [`GRID_PX`], only where the ground is seen.
fn dots(ui: &Ui, origin: Pos2) {
    let p = palette(ui.ctx());
    let clip = ui.clip_rect();
    let first = |from: f32, to: f32| ((to - from) / GRID_PX).ceil() * GRID_PX + from;
    let painter = ui.painter();
    let mut y = first(origin.y, clip.top());
    while y <= clip.bottom() {
        let mut x = first(origin.x, clip.left());
        while x <= clip.right() {
            painter.rect_filled(
                Rect::from_center_size(pos2(x, y), Vec2::splat(2.0)),
                CornerRadius::same(1),
                p.bg_2,
            );
            x += GRID_PX;
        }
        y += GRID_PX;
    }
}

fn empty(ui: &mut Ui, seen: Rect, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, keys::board::EMPTY, &[]);
    let galley = widgets::text(ui, &words, 12.5, 400.0, FontFamily::Proportional, p.text_2);
    let centre = pos2(seen.center().x, seen.top() + seen.height() * 0.4);
    let rect = Rect::from_center_size(centre, galley.size());
    let response = ui.allocate_rect(rect, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    address(ui, &response, "board-empty");
    ui.painter().galley(rect.min, galley, p.text_2);
}

/// The core's pixels as egui's points: a board is far smaller than where `f32` loses one.
#[expect(clippy::cast_precision_loss, reason = "a board is far under 2²⁴ px")]
pub(super) fn px(value: i32) -> f32 {
    value as f32
}

pub(super) fn frame_rect(origin: Pos2, frame: BoardFrame) -> Rect {
    Rect::from_min_size(
        origin + vec2(px(frame.x), px(frame.y)),
        vec2(px(frame.width), px(frame.height)),
    )
}

fn card_rect(origin: Pos2, at: BoardPoint) -> Rect {
    Rect::from_min_size(
        origin + vec2(px(at.x), px(at.y)),
        vec2(px(CARD_WIDTH), px(CARD_HEIGHT)),
    )
}

#[cfg(test)]
mod tests;
