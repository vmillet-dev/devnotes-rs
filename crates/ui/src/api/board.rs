//! The board of a space or of a folder: what it draws, where its zones and loose cards were
//! left, and putting it back in order.

use engine::folders::{
    self,
    board::{
        BoardArrangement, BoardQuery, BoardScope, BoardView, CardPlacement, PreviewLines,
        ZonePlacement,
    },
};

use crate::backend::{Backend, Pending};

/// The whole space, every note marked whether it matches: the board dims, it never narrows.
pub fn view(backend: &Backend, query: BoardQuery) -> Pending<BoardView> {
    backend.db(move |engine| folders::board_view(engine, query))
}

/// Every gesture staged since the last write, in one transaction: an interrupted batch would
/// leave half a board.
pub fn save_layout(
    backend: &Backend,
    root_id: Option<String>,
    zones: Vec<ZonePlacement>,
    cards: Vec<CardPlacement>,
) -> Pending<()> {
    backend.db(move |engine| folders::save_board_layout(engine, root_id, zones, cards))
}

/// Answers the layout it replaced, the only moment it still exists: the undo puts it back.
pub fn arrange(
    backend: &Backend,
    space_id: String,
    root_id: Option<String>,
    scope: BoardScope,
    lines: PreviewLines,
) -> Pending<BoardArrangement> {
    backend.db(move |engine| folders::arrange_board(engine, space_id, root_id, scope, lines))
}
