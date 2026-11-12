//! The second view of a space: its folders as zones from stored frames, the cards filed there
//! flowing inside them, the loose ones sitting freely on a dotted ground that pans and never
//! zooms. Only what crosses the viewport is drawn (plan § 3.6).
//!
//! Every gesture is a pointer drag read here, not egui's drag and drop: a card is its own handle
//! (the distance travelled tells a drag from the click that opens it), a zone moves by its grip
//! and resizes by its corner, the empty ground draws a folder, and the right button anywhere
//! sweeps a selection. ⚠️ A gesture commits on release and nowhere else, as one action.

mod gesture;
mod tidy;
mod zone;

use egui::{
    CornerRadius, FontFamily, PointerButton, Pos2, Rect, Response, Role, Sense, Shape, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};
use engine::folders::board::{BoardFrame, BoardNote, BoardPoint, BoardView, CARD_WIDTH, zone_at};
use i18n::{Key, keys};

use super::selection_bar::{self, Shown};
use super::{CanvasView, card, padded, state_text};
use crate::action::{Action, Actions};
use crate::notes::BoardGesture;
use crate::notes::panel::FolderEdit;
use crate::notes::sidebar::RailRow;
use crate::state::AppState;
use crate::state::board::BoardState;
use crate::theme::{palette, spacing};
use crate::titlebar::about_menu::Panel;
use crate::widgets::{self, Button, address};
use gesture::{Gesture, Kind};

/// Where a card the view never placed is drawn: `folders::board` places every one it reads.
const UNPLACED: BoardPoint = BoardPoint { x: 16, y: 16 };

const NOWHERE: BoardFrame = BoardFrame {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
};

/// A card that matches nothing asked stays in its seat and stops asking for attention.
const DIMMED: f32 = 0.32;
/// The seat a filed card was dragged out of, until the pointer lifts: the drop can still be
/// called off, and a hole opening mid-drag would reflow the zone under the pointer.
const LIFTED: f32 = 0.25;

/// What only the board's drawing sees.
#[derive(Default)]
pub(super) struct BoardScreen {
    /// The zone whose ⋯ is open, and what its panel holds.
    menu: Option<FolderEdit>,
    gesture: Option<Gesture>,
    /// Where the ground is panned to, as last drawn.
    offset: Vec2,
    /// The arrangements and restorations already panned for.
    pans: (u64, u64),
    /// Where the pan was before an arrangement sent it home: its undo puts it back.
    panned_from: Option<Vec2>,
}

/// What a draggable on the board hands its press to, and what a band is measured against.
struct Hand<'a> {
    gesture: &'a mut Option<Gesture>,
    /// The cards drawn this frame, in the surface's pixels. ⚠️ Measured, not computed: a card in
    /// a zone flows, and has no coordinates of its own to test a band against.
    seats: Vec<(String, BoardFrame)>,
    /// The surface's corner on screen.
    origin: Pos2,
    /// The rail's rows a card of this board may be let go of over.
    rail: &'a [RailRow],
}

impl Hand<'_> {
    fn point(&self, at: Pos2) -> BoardPoint {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a board is far under 2³¹ px"
        )]
        let round = |value: f32| value.round() as i32;
        BoardPoint {
            x: round(at.x - self.origin.x),
            y: round(at.y - self.origin.y),
        }
    }

    fn frame(&self, rect: Rect) -> BoardFrame {
        let at = self.point(rect.min);
        #[expect(clippy::cast_possible_truncation, reason = "a card's or a zone's size")]
        let size = |value: f32| value.round() as i32;
        BoardFrame {
            x: at.x,
            y: at.y,
            width: size(rect.width()),
            height: size(rect.height()),
        }
    }

    /// A press that started travelling on `response`: the left button's gesture is `kind`, if it
    /// has one; the right button's is a band, wherever it started.
    fn grab(&mut self, ui: &Ui, response: &Response, kind: Option<Kind>, from: BoardFrame) {
        if self.gesture.is_some() || !response.drag_started() {
            return;
        }
        let Some(press) = ui.input(|input| input.pointer.press_origin()) else {
            return;
        };
        let origin = self.point(press);
        *self.gesture = if response.dragged_by(PointerButton::Secondary) {
            Some(Gesture::new(Kind::SelectBand, origin, NOWHERE))
        } else if response.dragged_by(PointerButton::Primary) {
            kind.map(|kind| Gesture::new(kind, origin, from))
        } else {
            None
        };
    }

    /// The rail row under a card being dragged, if the pointer is over one.
    fn over_rail(&self, ui: &Ui) -> Option<&RailRow> {
        let dragging = self
            .moving()
            .is_some_and(|gesture| matches!(gesture.kind, Kind::Card { .. }));
        let at = ui.input(|input| input.pointer.latest_pos())?;
        dragging
            .then(|| self.rail.iter().find(|row| row.rect.contains(at)))
            .flatten()
    }

    /// The gesture under way, once it has travelled.
    fn moving(&self) -> Option<&Gesture> {
        self.gesture.as_ref().filter(|gesture| gesture.moved)
    }
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
    ground(
        ui,
        (board, view),
        &context,
        &mut screen,
        (canvas_view, actions),
    );
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
    (board, view): (&BoardState, &BoardView),
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
    // A drag on the ground draws a folder, even on a touch screen: it must not pan as well.
    let scroll = pan(
        egui::ScrollArea::both()
            .id_salt("board")
            .auto_shrink([false, false])
            .scroll_source(egui::scroll_area::ScrollSource {
                drag: egui::scroll_area::DragScroll::Never,
                ..egui::scroll_area::ScrollSource::default()
            }),
        screen,
        board.pans(),
    );
    let BoardScreen { menu, gesture, .. } = screen;
    let mut committed = None;
    let rail = rail_rows(&canvas_view.rail, context.library.active_space());
    let output = scroll.show_viewport(&mut ground_ui, |ui, viewport| {
        // At least the visible ground, whatever the core sized it: past the content's edge
        // the dotted ground still takes the pointer.
        let size = vec2(px(view.width), px(view.height)).max(viewport.size());
        let surface = Rect::from_min_size(ui.min_rect().min, size);
        let drawn = ui.allocate_rect(surface, Sense::drag());
        drawn.widget_info(|| WidgetInfo::labeled(Role::Canvas, true, ""));
        address(ui, &drawn, "board-surface");
        dots(ui, surface.min);
        let mut hand = Hand {
            gesture,
            seats: Vec::new(),
            origin: surface.min,
            rail: &rail,
        };
        track(ui, &mut hand);
        hand.grab(ui, &drawn, Some(Kind::DrawZone), NOWHERE);
        reveal(
            ui,
            (board, view),
            (surface.min, context.card_height()),
            canvas_view.request.as_deref(),
        );
        cards_and_zones(
            ui,
            (board, view),
            (context, &mut *menu, &mut hand),
            (&mut *canvas_view, &mut *actions),
        );
        ghost(ui, board, context, &hand);
        bands(ui, &hand);
        if view.zones.is_empty() && view.loose.is_empty() {
            empty(ui, ui.clip_rect(), context.language, actions);
        }
        committed = follow(ui, (board, view), &mut hand);
        canvas_view.over_rail = hand.over_rail(ui).map(|row| {
            row.folder_id
                .clone()
                .unwrap_or_else(|| row.space_id.clone())
        });
    });
    if let Some(done) = committed {
        actions.push(Action::Notes(crate::notes::Action::Board(done)));
    }
    screen.offset = output.state.clamped_offset();
    // Over the ground and not in the header: it acts on this board's geometry alone. Bottom
    // right, where it covers no zone's ⋯.
    let counts = (view.zones.len(), view.loose.len());
    tidy::show(
        ui,
        frame.max - vec2(16.0, 18.0),
        counts,
        context.language,
        actions,
    );
}

/// The zones, then the loose cards over them, each only when it crosses what is seen.
fn cards_and_zones(
    ui: &mut Ui,
    (board, view): (&BoardState, &BoardView),
    (context, menu, hand): (&card::Context<'_>, &mut Option<FolderEdit>, &mut Hand<'_>),
    (canvas_view, actions): (&mut CanvasView, &mut Actions),
) {
    let origin = hand.origin;
    let target = drop_target(board, view, hand.moving());
    let clip = ui.clip_rect();
    for shown in &view.zones {
        let frame = frame_of(board, shown, hand.moving());
        let rect = frame_rect(origin, frame);
        if rect.intersects(clip) {
            let drawn = zone::Drawn {
                rect,
                frame,
                lit: target.as_deref() == Some(shown.folder.id.as_str()),
                notes: cards_in(board, view, Some(&shown.folder.id)).collect(),
            };
            let drawing = (context, &mut *menu, &mut *hand);
            zone::show(
                ui,
                &drawn,
                shown,
                drawing,
                (&mut *canvas_view, &mut *actions),
            );
        }
    }
    for entry in loose_cards(board, view) {
        let rect = card_rect(
            origin,
            place_of(board, entry, hand.moving()),
            context.card_height(),
        );
        if rect.intersects(clip) {
            let seat = (rect, "board-loose-card", false);
            seated(
                ui,
                seat,
                entry,
                (context, &mut *hand),
                (&mut *canvas_view, &mut *actions),
            );
        }
    }
}

/// Before anything is drawn: what follows the pointer is drawn where the pointer is now, not a
/// frame behind it.
fn track(ui: &Ui, hand: &mut Hand<'_>) {
    let (down, at) = ui.input(|input| (input.pointer.any_down(), input.pointer.latest_pos()));
    if let (true, Some(at)) = (down, at) {
        let at = hand.point(at);
        if let Some(gesture) = hand.gesture.as_mut() {
            gesture.follow(at);
        }
    }
}

/// The pointer followed while it is down; on release, what the gesture commits. Nothing else
/// commits, and a drag that never travelled was a click.
fn follow(
    ui: &Ui,
    (board, view): (&BoardState, &BoardView),
    hand: &mut Hand<'_>,
) -> Option<BoardGesture> {
    hand.gesture.as_ref()?;
    let (down, at) = ui.input(|input| (input.pointer.any_down(), input.pointer.latest_pos()));
    let Some(at) = at else {
        // The pointer left the window, or the system took it back: nobody chose that place.
        *hand.gesture = None;
        return None;
    };
    let at = hand.point(at);
    let gesture = hand.gesture.as_mut()?;
    if down {
        gesture.follow(at);
        ui.ctx().request_repaint();
        return None;
    }
    gesture.follow(at);
    let rail = hand.over_rail(ui).map(|row| row.folder_id.clone());
    let gesture = hand.gesture.take()?;
    if !gesture.moved {
        return None;
    }
    if let (Some(folder), Kind::Card { note_id, .. }) = (rail, &gesture.kind) {
        let note_id = note_id.clone();
        return Some(BoardGesture::FileOnRail { note_id, folder });
    }
    let to = gesture.to;
    match gesture.kind {
        Kind::Card { note_id, filed } => {
            let point = BoardPoint { x: to.x, y: to.y };
            let frames: Vec<(String, BoardFrame)> = view
                .zones
                .iter()
                .map(|zone| (zone.folder.id.clone(), board.frame(zone)))
                .collect();
            let folder = zone_at(&frames, point);
            let sits = sitting(board, view, &note_id);
            // Let go of in the zone it flows in: a wobble, not a filing.
            if filed && folder.is_some() && folder == sits {
                return None;
            }
            Some(BoardGesture::DropCard {
                note_id,
                folder,
                at: point,
            })
        }
        Kind::MoveZone(folder_id) | Kind::ResizeZone(folder_id) => Some(BoardGesture::MoveZone {
            folder_id,
            frame: to,
        }),
        Kind::DrawZone => {
            gesture::worth_drawing(to).then(|| BoardGesture::DrawZone(gesture::as_zone(to)))
        }
        Kind::SelectBand => {
            let swept: Vec<String> = hand
                .seats
                .iter()
                .filter(|(_, seat)| gesture::overlaps(to, *seat))
                .map(|(id, _)| id.clone())
                .collect();
            (!swept.is_empty()).then_some(BoardGesture::Band(swept))
        }
    }
}

/// The rows of the rail a card of this board may land in: the space's own row, and its folders.
fn rail_rows(rows: &[RailRow], space: Option<&str>) -> Vec<RailRow> {
    rows.iter()
        .filter(|row| Some(row.space_id.as_str()) == space)
        .cloned()
        .collect()
}

/// The zone a card being dragged would land in, lit before the pointer lifts.
fn drop_target(board: &BoardState, view: &BoardView, moving: Option<&Gesture>) -> Option<String> {
    let gesture = moving.filter(|gesture| matches!(gesture.kind, Kind::Card { .. }))?;
    let frames: Vec<(String, BoardFrame)> = view
        .zones
        .iter()
        .map(|zone| (zone.folder.id.clone(), board.frame(zone)))
        .collect();
    zone_at(
        &frames,
        BoardPoint {
            x: gesture.to.x,
            y: gesture.to.y,
        },
    )
}

/// A zone's frame now: the gesture moving it, else a move staged, else the view's.
fn frame_of(
    board: &BoardState,
    zone: &engine::folders::board::BoardZone,
    moving: Option<&Gesture>,
) -> BoardFrame {
    match moving {
        Some(Gesture {
            kind: Kind::MoveZone(id) | Kind::ResizeZone(id),
            to,
            ..
        }) if *id == zone.folder.id => *to,
        _ => board.frame(zone),
    }
}

fn place_of(board: &BoardState, entry: &BoardNote, moving: Option<&Gesture>) -> BoardPoint {
    match moving {
        Some(Gesture {
            kind: Kind::Card {
                note_id,
                filed: false,
            },
            to,
            ..
        }) if *note_id == entry.note.id => BoardPoint { x: to.x, y: to.y },
        _ => board.place(entry).unwrap_or(UNPLACED),
    }
}

/// Where a card is drawn, a drop staged included: `None` for the ground.
fn sitting(board: &BoardState, view: &BoardView, note_id: &str) -> Option<String> {
    let in_view = view
        .zones
        .iter()
        .find(|zone| zone.notes.iter().any(|entry| entry.note.id == note_id))
        .map(|zone| zone.folder.id.as_str());
    board.sitting_in(note_id, in_view).map(str::to_owned)
}

/// Every card the view carries, sitting where the drops staged have put it.
fn cards_in<'a>(
    board: &'a BoardState,
    view: &'a BoardView,
    zone: Option<&'a str>,
) -> impl Iterator<Item = &'a BoardNote> {
    let in_zones = view.zones.iter().flat_map(|shown| {
        shown
            .notes
            .iter()
            .map(move |entry| (entry, Some(shown.folder.id.as_str())))
    });
    in_zones
        .chain(view.loose.iter().map(|entry| (entry, None)))
        .filter(move |(entry, in_view)| board.sitting_in(&entry.note.id, *in_view) == zone)
        .map(|(entry, _)| entry)
}

fn loose_cards<'a>(
    board: &'a BoardState,
    view: &'a BoardView,
) -> impl Iterator<Item = &'a BoardNote> {
    cards_in(board, view, None)
}

/// Home after an arrangement, back after its undo: the pan is the ground's alone, and a result
/// written off screen reads as an erasure.
fn pan(scroll: egui::ScrollArea, screen: &mut BoardScreen, pans: (u64, u64)) -> egui::ScrollArea {
    let (arranged, restored) = std::mem::replace(&mut screen.pans, pans);
    if pans.0 != arranged {
        screen.panned_from = Some(screen.offset);
        return scroll.scroll_offset(Vec2::ZERO);
    }
    if pans.1 != restored
        && let Some(back) = screen.panned_from.take()
    {
        return scroll.scroll_offset(back);
    }
    scroll
}

/// A card in its seat — a zone's, or the ground's — under an id its note keeps, dimmed in place
/// when it matches nothing asked. The seat says so: disabled is what a screen reader hears of a
/// card faded out. The seat is the card's handle: it senses the drag, the card under it the
/// click, and egui gives each to its own.
fn seated(
    ui: &mut Ui,
    (rect, author_id, filed): (Rect, &str, bool),
    entry: &BoardNote,
    (context, hand): (&card::Context<'_>, &mut Hand<'_>),
    (canvas_view, actions): (&mut CanvasView, &mut Actions),
) {
    let mut card_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .id_salt(("board-card", &entry.note.id)),
    );
    let lifted = hand.moving().is_some_and(|gesture| {
        filed && matches!(&gesture.kind, Kind::Card { note_id, .. } if *note_id == entry.note.id)
    });
    if lifted {
        card_ui.multiply_opacity(LIFTED);
    } else if !entry.matches {
        card_ui.multiply_opacity(DIMMED);
    }
    let seat = card_ui.allocate_rect(rect, Sense::drag());
    let title = entry.note.title.clone();
    seat.widget_info(|| WidgetInfo::labeled(Role::Group, entry.matches, &title));
    address(&card_ui, &seat, author_id);
    let note_id = entry.note.id.clone();
    let from = hand.frame(rect);
    hand.grab(ui, &seat, Some(Kind::Card { note_id, filed }), from);
    hand.seats.push((entry.note.id.clone(), from));
    card::show(
        &mut card_ui,
        rect,
        &entry.note,
        context,
        (canvas_view, actions),
    );
}

/// The card a drag took out of a zone, at the pointer, above everything: a filed card flows and
/// has no place of its own to move, so without it the pointer would carry nothing.
fn ghost(ui: &mut Ui, board: &BoardState, context: &card::Context<'_>, hand: &Hand<'_>) {
    let Some(Gesture {
        kind: Kind::Card {
            note_id,
            filed: true,
        },
        to,
        ..
    }) = hand.moving()
    else {
        return;
    };
    let Some(entry) = board.cards().find(|entry| entry.note.id == *note_id) else {
        return;
    };
    let rect = card_rect(
        hand.origin,
        BoardPoint { x: to.x, y: to.y },
        context.card_height(),
    );
    let mut ghost_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .id_salt(("board-ghost", note_id))
            .layer_id(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::unique("board-ghost"),
            )),
    );
    ghost_ui.set_clip_rect(ui.clip_rect());
    let p = palette(ui.ctx());
    ghost_ui
        .painter()
        .add(p.shadow_overlay.as_shape(rect, CornerRadius::same(10)));
    let mut idle = CanvasView::default();
    card::show(
        &mut ghost_ui,
        rect,
        &entry.note,
        context,
        (&mut idle, &mut Actions::default()),
    );
}

/// The band being drawn: a folder's, dashed in the accent; a selection's, filled and plain — the
/// left button draws a region that will exist, the right one a sweep that will not.
fn bands(ui: &Ui, hand: &Hand<'_>) {
    let Some(gesture) = hand.moving() else { return };
    let p = palette(ui.ctx());
    let rect = frame_rect(hand.origin, gesture.to);
    match gesture.kind {
        Kind::DrawZone => {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(10),
                p.amber_fill.gamma_multiply(0.08),
            );
            ui.painter().extend(Shape::dashed_line(
                &[
                    rect.left_top(),
                    rect.right_top(),
                    rect.right_bottom(),
                    rect.left_bottom(),
                    rect.left_top(),
                ],
                Stroke::new(1.0, p.amber_edge),
                4.0,
                3.0,
            ));
        }
        Kind::SelectBand => {
            ui.painter().rect(
                rect,
                CornerRadius::same(4),
                p.amber_fill.gamma_multiply(0.12),
                Stroke::new(1.0, p.amber_dim),
                StrokeKind::Inside,
            );
        }
        _ => {}
    }
}

/// A card asked to take the focus may sit out of view, so not be drawn: the ground scrolls to
/// it, a zone's own body then to the card.
fn reveal(
    ui: &Ui,
    (board, view): (&BoardState, &BoardView),
    (origin, card_height): (Pos2, f32),
    request: Option<&str>,
) {
    let Some(id) = request else { return };
    let zone = view
        .zones
        .iter()
        .find(|zone| zone.notes.iter().any(|entry| entry.note.id == id))
        .map(|zone| frame_rect(origin, board.frame(zone)));
    let card = view
        .loose
        .iter()
        .find(|entry| entry.note.id == id)
        .map(|entry| {
            let at = board.place(entry).unwrap_or(UNPLACED);
            card_rect(origin, at, card_height)
        });
    if let Some(rect) = zone.or(card) {
        ui.scroll_to_rect(rect, None);
    }
}

/// One dot every `GRID_PX`, only where the ground is seen.
fn dots(ui: &Ui, origin: Pos2) {
    let p = palette(ui.ctx());
    let clip = ui.clip_rect();
    let step = px(gesture::GRID_PX);
    let first = |from: f32, to: f32| ((to - from) / step).ceil() * step + from;
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
            x += step;
        }
        y += step;
    }
}

fn empty(ui: &mut Ui, seen: Rect, language: i18n::Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, keys::board::EMPTY, &[]);
    let galley = widgets::text(ui, &words, 12.5, 400.0, FontFamily::Proportional, p.text_2);
    let centre = pos2(seen.center().x, seen.top() + seen.height() * 0.4);
    let rect = Rect::from_center_size(centre, galley.size());
    let response = ui.allocate_rect(rect, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    address(ui, &response, "board-empty");
    ui.painter().galley(rect.min, galley, p.text_2);
    // The chapter about folders and the board, not the top of the guide.
    let label = i18n::translate(language, keys::getting_started::OPEN_CHAPTER, &[]);
    let under = Rect::from_center_size(
        rect.center_bottom() + vec2(0.0, 24.0),
        vec2(widgets::Button::width(ui, &label), 28.0),
    );
    let mut room = ui.new_child(egui::UiBuilder::new().max_rect(under));
    if widgets::Button::new(&label, "board-guide")
        .show(&mut room)
        .clicked()
    {
        let guide = Panel::Guide(crate::services::guide::FOLDERS);
        actions.push(crate::action::Action::Menu(
            crate::titlebar::Action::OpenHelp(guide),
        ));
    }
}

/// The core's pixels as egui's points: a board is far smaller than where `f32` loses one.
#[expect(clippy::cast_precision_loss, reason = "a board is far under 2²⁴ px")]
fn px(value: i32) -> f32 {
    value as f32
}

fn frame_rect(origin: Pos2, frame: BoardFrame) -> Rect {
    Rect::from_min_size(
        origin + vec2(px(frame.x), px(frame.y)),
        vec2(px(frame.width), px(frame.height)),
    )
}

fn card_rect(origin: Pos2, at: BoardPoint, height: f32) -> Rect {
    Rect::from_min_size(
        origin + vec2(px(at.x), px(at.y)),
        vec2(px(CARD_WIDTH), height),
    )
}

#[cfg(test)]
mod tests;
