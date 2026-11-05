//! What a write answered that can put it back, and the call that does.

use engine::folders::{self, board::BoardLayout, model::NoteFiling};
use engine::notes::{
    self,
    model::{NotePlacement, NotePriority, NoteTag},
};

use crate::backend::{Backend, Pending};

/// Each variant carries what the core answered, never what the interface guessed: rebuilt from
/// the selection, an undo would take off a tag the note already carried.
#[derive(Debug, Clone)]
pub enum Reversible {
    Deletion {
        ids: Vec<String>,
        count: u32,
    },
    Move(Vec<NotePlacement>),
    Tag(Vec<NoteTag>),
    Untag(Vec<NoteTag>),
    Pin {
        ids: Vec<String>,
        pinned: bool,
    },
    Priority(Vec<NotePriority>),
    File(Vec<NoteFiling>),
    /// A board tidied up, and the layout it replaced, on the board it was made on.
    Arrangement {
        root_id: Option<String>,
        previous: BoardLayout,
        moved: u32,
    },
    /// A change of kind, and the body the note had before it.
    Convert {
        id: String,
        into: engine::notes::kind::NoteKind,
        previous: engine::notes::convert::NoteBody,
    },
}

impl Reversible {
    /// How many notes it touched: none is nothing to offer.
    pub fn count(&self) -> usize {
        match self {
            Self::Deletion { count, .. } => *count as usize,
            Self::Move(previous) => previous.len(),
            Self::Tag(pairs) | Self::Untag(pairs) => pairs.len(),
            Self::Pin { ids, .. } => ids.len(),
            Self::Priority(previous) => previous.len(),
            Self::File(previous) => previous.len(),
            Self::Arrangement { moved, .. } => *moved as usize,
            Self::Convert { .. } => 1,
        }
    }
}

pub fn revert(backend: &Backend, action: Reversible) -> Pending<u32> {
    backend.db(move |engine| match action {
        Reversible::Deletion { ids, .. } => notes::restore_notes(engine, ids),
        Reversible::Move(previous) => notes::move_notes_back(engine, previous),
        Reversible::Tag(added) => notes::untag_notes(engine, added),
        Reversible::Untag(removed) => notes::restore_tags(engine, removed),
        Reversible::Pin { ids, pinned } => {
            notes::set_pinned(engine, ids, !pinned).map(|ids| count(ids.len()))
        }
        Reversible::Priority(previous) => notes::restore_priorities(engine, previous),
        Reversible::File(previous) => folders::file_notes_back(engine, previous),
        Reversible::Arrangement {
            root_id, previous, ..
        } => {
            let count = previous.zones.len() + previous.cards.len();
            folders::save_board_layout(engine, root_id, previous.zones, previous.cards)
                .map(|()| self::count(count))
        }
        Reversible::Convert { id, previous, .. } => {
            let patch = notes::model::NotePatch {
                kind: Some(previous.kind),
                language: Some(previous.language),
                content: Some(previous.content),
                items: Some(previous.items),
                ..notes::model::NotePatch::default()
            };
            notes::update_note(engine, id, patch).map(|_| 1)
        }
    })
}

fn count(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}
