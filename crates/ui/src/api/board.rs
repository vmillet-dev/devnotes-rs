//! The board of a space or of a folder: what it draws.

use engine::folders::{
    self,
    board::{BoardQuery, BoardView},
};

use crate::backend::{Backend, Pending};

/// The whole space, every note marked whether it matches: the board dims, it never narrows.
pub fn view(backend: &Backend, query: BoardQuery) -> Pending<BoardView> {
    backend.db(move |engine| folders::board_view(engine, query))
}
