//! Which of the two views a space is looked at in, and what the board draws. The board reads the
//! whole space and marks what matches: it dims, never narrows. It is unavailable on « all
//! spaces », which have no zones, and the switch falls back to the list view.

use chrono::{DateTime, Local, NaiveDate, Utc};
use engine::error::AppError;
use engine::folders::board::{BoardNote, BoardQuery, BoardView};

use super::notes_query::Criteria;
use super::{QueryCell, Revision};
use crate::api;
use crate::backend::Backend;
use crate::gate::IndexFill;
use crate::services::preferences::Preferences;

/// One key per space, not one map: arranging one space must not move the switch on the others.
fn view_key(space_id: &str) -> String {
    format!("devnotes.notes.view.{space_id}")
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    List,
    Board,
}

impl Mode {
    /// What the space was left on; anything but the reference's `board` is the list.
    pub fn read(preferences: &Preferences, space_id: &str) -> Self {
        match preferences.get(&view_key(space_id)) {
            Some("board") => Self::Board,
            _ => Self::List,
        }
    }

    pub fn write(self, preferences: &mut Preferences, space_id: &str) {
        let name = match self {
            Self::List => "list",
            Self::Board => "board",
        };
        preferences.set(&view_key(space_id), name);
    }
}

/// The board on screen: a space's, or an open folder's when it holds folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub space_id: String,
    /// The folder whose sub-folders are the zones; `None` for the space's own board.
    pub root_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct Params {
    place: Place,
    criteria: Criteria,
    /// The local day, not the instant: what dims by date moves at midnight, not every frame.
    day: NaiveDate,
    revision: Revision,
    index: IndexFill,
}

#[derive(Default)]
pub struct BoardState {
    cell: QueryCell<Params, BoardView>,
}

impl BoardState {
    /// Once a frame while the board is shown; `true` when the view or its error changed.
    pub fn update(
        &mut self,
        backend: &Backend,
        now: DateTime<Utc>,
        (place, criteria): (Place, Criteria),
        (revision, index): (Revision, IndexFill),
    ) -> bool {
        self.cell.want(Params {
            place,
            criteria,
            day: now.with_timezone(&Local).date_naive(),
            revision,
            index,
        });
        self.cell
            .update(|params| api::board::view(backend, query(params, now)))
    }

    pub fn view(&self) -> Option<&BoardView> {
        self.cell.value()
    }

    pub fn error(&self) -> Option<&AppError> {
        self.cell.error()
    }

    /// Every card the board draws, dimmed ones included, zone by zone and then the ground: the
    /// order the arrows walk and a `Shift`-click ranges over.
    pub fn cards(&self) -> impl Iterator<Item = &BoardNote> {
        self.view().into_iter().flat_map(|view| {
            view.zones
                .iter()
                .flat_map(|zone| &zone.notes)
                .chain(&view.loose)
        })
    }

    /// How many notes the criteria leave lit; `None` while nothing dims anything.
    pub fn matched(&self) -> Option<u32> {
        self.view()
            .filter(|view| view.is_filtering)
            .map(|view| view.matched)
    }
}

fn query(params: Params, now: DateTime<Utc>) -> BoardQuery {
    let Criteria {
        search,
        filter,
        facets,
    } = params.criteria;
    BoardQuery {
        space_id: params.place.space_id,
        root_id: params.place.root_id,
        search,
        filter,
        tags: facets.tags.into_iter().collect(),
        languages: facets.languages.into_iter().collect(),
        kinds: facets.kinds.into_iter().collect(),
        priorities: facets.priorities.into_iter().collect(),
        now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    #[test]
    fn the_view_is_remembered_space_by_space() {
        let mut preferences = Preferences::default();

        Mode::Board.write(&mut preferences, "a");

        assert_eq!(Mode::read(&preferences, "a"), Mode::Board);
        assert_eq!(Mode::read(&preferences, "b"), Mode::List);
        preferences.set("devnotes.notes.view.b", "grid");
        assert_eq!(Mode::read(&preferences, "b"), Mode::List);
    }

    #[test]
    fn a_search_dims_on_the_board_and_drops_nothing() {
        let (backend, _profile) = testing::unlocked();
        let kept = testing::note(&backend, "psql prod");
        testing::note(&backend, "curl health");
        let place = Place {
            space_id: kept.space_id.clone(),
            root_id: None,
        };
        let criteria = Criteria {
            search: "psql".into(),
            ..Criteria::default()
        };
        let mut state = BoardState::default();
        for _ in 0..2 {
            state.update(
                &backend,
                Utc::now(),
                (place.clone(), criteria.clone()),
                (Revision::default(), IndexFill::Ready),
            );
            backend.settle();
        }

        let lit: Vec<bool> = state.cards().map(|entry| entry.matches).collect();
        assert!(lit.len() >= 2, "every note of the space is drawn");
        assert_eq!(lit.iter().filter(|matches| **matches).count(), 1);
        assert_eq!(state.matched(), Some(1));
    }
}
