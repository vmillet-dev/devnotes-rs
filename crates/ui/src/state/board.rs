//! Which of the two views a space is looked at in, what the board draws, and where its gestures
//! left things until that is written. The board reads the whole space and marks what matches: it
//! dims, never narrows. It is unavailable on « all spaces », which have no zones, and the switch
//! falls back to the list view.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, Utc};
use engine::error::AppError;
use engine::folders::board::{
    BoardArrangement, BoardFrame, BoardNote, BoardPoint, BoardQuery, BoardScope, BoardView,
    BoardZone, CardPlacement, PreviewLines, ZonePlacement,
};
use engine::folders::model::Folder;
use i18n::{Message, keys};

use super::notes_query::Criteria;
use super::{QueryCell, Revision};
use crate::api;
use crate::api::undo::Reversible;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::services::preferences::Preferences;

/// Three cards dragged in a row are one write, and a quit right after a drop still beats it; in
/// egui's seconds.
pub const LAYOUT_SAVE_DEBOUNCE: f64 = 0.4;

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
    /// The library's card height, which sizes a first layout.
    lines: PreviewLines,
    /// The local day, not the instant: what dims by date moves at midnight, not every frame.
    day: NaiveDate,
    revision: Revision,
}

/// What a gesture moved and no view has come back with yet, laid over the view so that nothing
/// snaps back while it is written. ⚠️ Let go of when a view carries it, never when the write
/// returns: the view still drawn then is the one read before the gesture. A failed write keeps
/// it too — dropped, every card would snap back with nothing saying why.
#[derive(Default)]
struct Staged {
    /// The board the gestures were made on: a place means nothing on another one.
    root_id: Option<String>,
    frames: HashMap<String, BoardFrame>,
    places: HashMap<String, BoardPoint>,
    /// When it is written, in egui's seconds.
    due: Option<f64>,
}

/// A zone being made: drawn, then named, then placed where it was drawn.
struct Making {
    root_id: Option<String>,
    frame: BoardFrame,
    name: String,
    pending: Pending<Folder>,
}

#[derive(Default)]
pub struct BoardState {
    cell: QueryCell<Params, BoardView>,
    staged: Staged,
    /// Which zone a drop put a card in, `None` the ground, before any view says so: without it
    /// a card snaps back into or out of its zone for a round trip. ⚠️ Dropped when the filing is
    /// refused, where a refused place is kept: it would be a lie about which folder a note is in.
    filing: HashMap<String, Option<String>>,
    filings: Vec<(String, Pending<Reversible>)>,
    /// A band drawn and waiting for its name.
    drawn: Option<BoardFrame>,
    making: Option<Making>,
    saving: Vec<Pending<()>>,
    arranging: Option<(Option<String>, Pending<BoardArrangement>)>,
    /// ⚠️ Counted, not flagged: the ground pans home after each arrangement and back after each
    /// undo of one, and two running pan twice. An arrangement landing at the top left while the
    /// ground is panned elsewhere shows empty ground under a banner announcing success.
    arrangements: u64,
    restorations: u64,
}

impl BoardState {
    /// Once a frame while the board is shown; `true` when the view or its error changed.
    pub fn update(
        &mut self,
        backend: &Backend,
        now: DateTime<Utc>,
        (place, criteria, lines): (Place, Criteria, PreviewLines),
        revision: Revision,
    ) -> bool {
        self.cell.want(Params {
            place,
            criteria,
            lines,
            day: now.with_timezone(&Local).date_naive(),
            revision,
        });
        let changed = self
            .cell
            .update(|params| api::board::view(backend, query(params, now)));
        if changed && let Some(view) = self.cell.value() {
            release(&mut self.staged, view);
            let held = membership(view);
            self.filing
                .retain(|id, folder| held.get(id.as_str()) != Some(&folder.as_deref()));
        }
        changed
    }

    /// A card let go of: in the zone under it, or on the ground, which files it in the board's
    /// root. Through the batch command the selection bar uses too, and offered back by nothing:
    /// a drop is undone by the opposite drop.
    pub fn drop_card(
        &mut self,
        backend: &Backend,
        place: &Place,
        (note_id, folder): (String, Option<String>),
        (at, now): (BoardPoint, f64),
        lines: PreviewLines,
    ) {
        if folder.is_none() {
            self.move_card(place, note_id.clone(), at, now);
        }
        let into = folder.clone().or_else(|| place.root_id.clone());
        self.filing.insert(note_id.clone(), folder);
        let pending = api::folders::file(backend, vec![note_id.clone()], into, lines);
        self.filings.push((note_id, pending));
    }

    /// Where the card sits once the drops have had their say: a zone, or `None` for the ground.
    pub fn sitting_in<'a>(&'a self, note_id: &str, in_view: Option<&'a str>) -> Option<&'a str> {
        self.filing
            .get(note_id)
            .map_or(in_view, |folder| folder.as_deref())
    }

    /// A band drawn on the ground, held until it is named.
    pub fn propose_zone(&mut self, frame: BoardFrame) {
        self.drawn = Some(frame);
    }

    pub fn drawn_zone(&self) -> Option<BoardFrame> {
        self.drawn
    }

    pub fn dismiss_zone(&mut self) {
        self.drawn = None;
    }

    /// The band drawn last becomes a folder in the board's root, placed where it was drawn.
    pub fn name_zone(&mut self, backend: &Backend, place: &Place, name: &str) {
        let (Some(frame), name) = (self.drawn.take(), name.trim()) else {
            return;
        };
        if name.is_empty() {
            return;
        }
        let pending = api::folders::create(
            backend,
            place.space_id.clone(),
            place.root_id.clone(),
            name.to_owned(),
        );
        self.making = Some(Making {
            root_id: place.root_id.clone(),
            frame,
            name: name.to_owned(),
            pending,
        });
    }

    /// Once a frame, shown or not: the staged layout written once its time comes, the answers
    /// adopted. An arrangement done is answered, for the undo bar.
    pub fn write(
        &mut self,
        ctx: &egui::Context,
        backend: &Backend,
        revision: &mut Revision,
        errors: &mut ErrorNotifier,
    ) -> Option<Reversible> {
        if let Some(due) = self.staged.due {
            let time = ctx.input(|input| input.time);
            if time >= due {
                self.staged.due = None;
                self.saving.push(save(backend, &self.staged));
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(due - time));
            }
        }
        self.adopt_drops(revision, errors);
        self.adopt_made(backend, revision, errors);
        self.saving.retain_mut(|pending| match pending.poll() {
            None => true,
            Some(Ok(())) => {
                revision.bump();
                false
            }
            Some(Err(error)) => {
                errors.report(keys::errors::BOARD_SAVE_FAILED, &error);
                false
            }
        });
        let (root_id, pending) = self.arranging.as_mut()?;
        let answer = pending.poll()?;
        let root_id = root_id.clone();
        self.arranging = None;
        match answer {
            Ok(done) => {
                // Every place staged has just been overwritten, and would draw the cards back
                // where the gesture left them.
                self.staged = Staged::default();
                self.arrangements += 1;
                revision.bump();
                Some(Reversible::Arrangement {
                    root_id,
                    previous: done.previous,
                    moved: done.moved,
                })
            }
            Err(error) => {
                errors.report(keys::errors::BOARD_ARRANGE_FAILED, &error);
                None
            }
        }
    }

    fn adopt_drops(&mut self, revision: &mut Revision, errors: &mut ErrorNotifier) {
        let filing = &mut self.filing;
        self.filings
            .retain_mut(|(note_id, pending)| match pending.poll() {
                None => true,
                Some(Ok(_)) => {
                    revision.bump();
                    false
                }
                Some(Err(error)) => {
                    filing.remove(note_id.as_str());
                    errors.report(keys::errors::FILE_FAILED, &error);
                    false
                }
            });
    }

    /// The folder made, its frame written straight through: a frame staged for a folder the
    /// next view has only just heard of would be let go of.
    fn adopt_made(
        &mut self,
        backend: &Backend,
        revision: &mut Revision,
        errors: &mut ErrorNotifier,
    ) {
        let Some(making) = &mut self.making else {
            return;
        };
        let Some(answer) = making.pending.poll() else {
            return;
        };
        let Making {
            root_id,
            frame,
            name,
            ..
        } = self.making.take().expect("polled just above");
        match answer {
            Ok(folder) => {
                let zone = ZonePlacement {
                    folder_id: folder.id,
                    frame,
                };
                self.saving.push(api::board::save_layout(
                    backend,
                    root_id,
                    vec![zone],
                    Vec::new(),
                ));
                revision.bump();
            }
            Err(error) => {
                let failed =
                    Message::new(keys::errors::FOLDER_CREATE_FAILED).with("name", name.as_str());
                errors.report_message(failed, &error);
            }
        }
    }

    /// Where a zone ended up: staged, and written behind the debounce with whatever else moved.
    pub fn move_zone(&mut self, place: &Place, folder_id: String, frame: BoardFrame, now: f64) {
        self.stage(place, now).frames.insert(folder_id, frame);
    }

    /// Only ever a loose card: a filed one flows inside its zone and has no place of its own.
    pub fn move_card(&mut self, place: &Place, note_id: String, at: BoardPoint, now: f64) {
        self.stage(place, now).places.insert(note_id, at);
    }

    fn stage(&mut self, place: &Place, now: f64) -> &mut Staged {
        if self.staged.root_id != place.root_id {
            self.staged = Staged {
                root_id: place.root_id.clone(),
                ..Staged::default()
            };
        }
        self.staged.due = Some(now + LAYOUT_SAVE_DEBOUNCE);
        &mut self.staged
    }

    /// The board on screen put back in order, as far as `scope` allows.
    pub fn arrange(
        &mut self,
        backend: &Backend,
        place: &Place,
        scope: BoardScope,
        lines: PreviewLines,
    ) {
        let pending = api::board::arrange(
            backend,
            place.space_id.clone(),
            place.root_id.clone(),
            scope,
            lines,
        );
        self.arranging = Some((place.root_id.clone(), pending));
    }

    /// An arrangement taken back: the ground pans back to where it was.
    pub fn restored(&mut self) {
        self.staged = Staged::default();
        self.restorations += 1;
    }

    /// How many arrangements were made, and taken back: the ground pans after each.
    pub fn pans(&self) -> (u64, u64) {
        (self.arrangements, self.restorations)
    }

    /// The zone's frame as it is now: a move staged lies over the view's.
    pub fn frame(&self, zone: &BoardZone) -> BoardFrame {
        self.staged
            .frames
            .get(&zone.folder.id)
            .copied()
            .unwrap_or(zone.frame)
    }

    /// Where a loose card sits now, a move staged over the view's.
    pub fn place(&self, entry: &BoardNote) -> Option<BoardPoint> {
        self.staged
            .places
            .get(&entry.note.id)
            .copied()
            .or(entry.position)
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

/// A place the view agrees with is dropped; one it says nothing about is kept — a card dropped a
/// moment ago is exactly what the overlay is for.
fn release(staged: &mut Staged, view: &BoardView) {
    let frames: HashMap<&str, BoardFrame> = view
        .zones
        .iter()
        .map(|zone| (zone.folder.id.as_str(), zone.frame))
        .collect();
    staged
        .frames
        .retain(|id, frame| frames.get(id.as_str()) != Some(frame));
    let places: HashMap<&str, BoardPoint> = view
        .loose
        .iter()
        .filter_map(|entry| Some((entry.note.id.as_str(), entry.position?)))
        .collect();
    staged
        .places
        .retain(|id, at| places.get(id.as_str()) != Some(at));
}

/// Every card of the view, by the zone it is drawn in, `None` for the ground.
fn membership(view: &BoardView) -> HashMap<&str, Option<&str>> {
    view.zones
        .iter()
        .flat_map(|zone| {
            zone.notes
                .iter()
                .map(|entry| (entry.note.id.as_str(), Some(zone.folder.id.as_str())))
        })
        .chain(
            view.loose
                .iter()
                .map(|entry| (entry.note.id.as_str(), None)),
        )
        .collect()
}

fn save(backend: &Backend, staged: &Staged) -> Pending<()> {
    let zones = staged
        .frames
        .iter()
        .map(|(folder_id, frame)| ZonePlacement {
            folder_id: folder_id.clone(),
            frame: *frame,
        })
        .collect();
    let cards = staged
        .places
        .iter()
        .map(|(note_id, position)| CardPlacement {
            note_id: note_id.clone(),
            position: *position,
        })
        .collect();
    api::board::save_layout(backend, staged.root_id.clone(), zones, cards)
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
        preview_lines: params.lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    /// The board of `space`, read until the core answered.
    fn read(state: &mut BoardState, backend: &Backend, place: &Place, revision: Revision) {
        for _ in 0..2 {
            state.update(
                backend,
                Utc::now(),
                (place.clone(), Criteria::default(), PreviewLines::default()),
                revision,
            );
            backend.settle();
        }
    }

    fn space_of_one_zone() -> (Backend, tempfile::TempDir, Place, String) {
        let (backend, profile) = testing::unlocked();
        let note = testing::note(&backend, "psql prod");
        let draft = engine::folders::model::FolderDraft {
            space_id: note.space_id.clone(),
            name: "Perf".into(),
            parent_id: None,
        };
        let folder = engine::db::run_here(backend.engine(), move |engine| {
            engine::folders::create_folder(engine, draft)
        })
        .unwrap();
        let place = Place {
            space_id: note.space_id.clone(),
            root_id: None,
        };
        (backend, profile, place, folder.id)
    }

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
                (place.clone(), criteria.clone(), PreviewLines::default()),
                Revision::default(),
            );
            backend.settle();
        }

        let lit: Vec<bool> = state.cards().map(|entry| entry.matches).collect();
        assert!(lit.len() >= 2, "every note of the space is drawn");
        assert_eq!(lit.iter().filter(|matches| **matches).count(), 1);
        assert_eq!(state.matched(), Some(1));
    }

    #[test]
    fn moves_are_written_once_the_gestures_rest_and_held_until_a_view_carries_them() {
        let (backend, _profile, place, folder) = space_of_one_zone();
        let ctx = egui::Context::default();
        let mut errors = ErrorNotifier::default();
        let mut revision = Revision::default();
        let mut state = BoardState::default();
        read(&mut state, &backend, &place, revision);
        let card = state.view().unwrap().loose[0].note.id.clone();
        let moved = BoardFrame {
            x: 400,
            y: 300,
            width: 300,
            height: 260,
        };
        let at = BoardPoint { x: 40, y: 600 };
        let mut frame = |state: &mut BoardState, revision: &mut Revision, time: f64| {
            testing::frame(&ctx, time, |_| {});
            state.write(&ctx, &backend, revision, &mut errors);
            backend.settle();
            state.write(&ctx, &backend, revision, &mut errors);
        };

        state.move_zone(&place, folder.clone(), moved, 0.0);
        frame(&mut state, &mut revision, 0.1);
        state.move_card(&place, card.clone(), at, 0.2);
        frame(&mut state, &mut revision, 0.5);
        assert_eq!(
            revision,
            Revision::default(),
            "nothing written while the hand moves"
        );
        assert_eq!(state.frame(&state.view().unwrap().zones[0]), moved);

        frame(&mut state, &mut revision, 0.7);
        assert_ne!(revision, Revision::default(), "both written at once");
        read(&mut state, &backend, &place, revision);

        let view = state.view().unwrap();
        assert_eq!(view.zones[0].frame, moved);
        assert_eq!(view.loose[0].position, Some(at));
        assert!(state.staged.frames.is_empty() && state.staged.places.is_empty());
    }

    #[test]
    fn an_arrangement_answers_what_it_replaced_and_pans_home() {
        let (backend, _profile, place, folder) = space_of_one_zone();
        let ctx = egui::Context::default();
        let mut errors = ErrorNotifier::default();
        let mut revision = Revision::default();
        let mut state = BoardState::default();
        read(&mut state, &backend, &place, revision);
        let before = state.view().unwrap().zones[0].frame;
        let far = BoardFrame { x: 2000, ..before };
        engine::db::run_here(backend.engine(), {
            let folder = folder.clone();
            move |engine| {
                let zone = ZonePlacement {
                    folder_id: folder,
                    frame: far,
                };
                engine::folders::save_board_layout(engine, None, vec![zone], Vec::new())
            }
        })
        .unwrap();

        state.arrange(
            &backend,
            &place,
            BoardScope::Everything,
            PreviewLines::default(),
        );
        backend.settle();
        let done = state.write(&ctx, &backend, &mut revision, &mut errors);

        let Some(Reversible::Arrangement {
            previous, moved, ..
        }) = done
        else {
            panic!("an arrangement answered");
        };
        assert_eq!(moved, 1);
        assert_eq!(previous.zones[0].frame, far);
        assert_eq!(state.pans(), (1, 0));
        read(&mut state, &backend, &place, revision);
        assert_eq!(state.view().unwrap().zones[0].frame, before);
    }
}
