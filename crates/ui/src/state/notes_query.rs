//! Which notes are shown. It filters, sorts and groups nothing: `query_notes` does, and the
//! canvas draws the view it answers.

use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, Offset, TimeZone, Utc};
use engine::error::AppError;
use engine::notes::view::{Grouping, NoteFilter, NoteOrder, NotesQuery, NotesView};

use super::{QueryCell, Revision};
use crate::api;
use crate::backend::Backend;
use crate::gate::IndexFill;

/// What the view depends on, compared by value: the instant is not among them, only the local
/// day, so the sections move at midnight and not on every frame.
#[derive(Debug, Clone, PartialEq)]
struct Params {
    space_id: Option<String>,
    folder_id: Option<String>,
    day: NaiveDate,
    revision: Revision,
    /// The index answers what it holds so far: each slice it adds is asked again.
    index: IndexFill,
}

#[derive(Default)]
pub struct NotesQueryState {
    cell: QueryCell<Params, NotesView>,
}

impl NotesQueryState {
    /// Once a frame, with the library open. `true` when the view or its error changed.
    /// `place` is the space and the folder shown, `None` for every one.
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        backend: &Backend,
        now: DateTime<Utc>,
        place: (Option<&str>, Option<&str>),
        (revision, index): (Revision, IndexFill),
    ) -> bool {
        let local = now.with_timezone(&Local);
        ctx.request_repaint_after(until_midnight(&local));
        self.cell.want(Params {
            space_id: place.0.map(str::to_owned),
            folder_id: place.1.map(str::to_owned),
            day: local.date_naive(),
            revision,
            index,
        });
        self.cell
            .update(|params| api::notes::query(backend, query(&params, now)))
    }

    pub fn view(&self) -> Option<&NotesView> {
        self.cell.value()
    }

    pub fn error(&self) -> Option<&AppError> {
        self.cell.error()
    }

    pub fn is_loading(&self) -> bool {
        self.cell.is_loading()
    }
}

fn query(params: &Params, now: DateTime<Utc>) -> NotesQuery {
    NotesQuery {
        space_id: params.space_id.clone(),
        folder_id: params.folder_id.clone(),
        search: String::new(),
        filter: NoteFilter::All,
        tags: Vec::new(),
        languages: Vec::new(),
        kinds: Vec::new(),
        priorities: Vec::new(),
        order: NoteOrder::default(),
        grouping: Grouping::Date,
        ranked: false,
        now,
        tz_offset_minutes: tz_offset_minutes(now),
        pinned_first: true,
    }
}

/// `Date#getTimezoneOffset()`'s sign, which the core reads: −120 for UTC+2.
fn tz_offset_minutes(now: DateTime<Utc>) -> i32 {
    -Local
        .offset_from_utc_datetime(&now.naive_utc())
        .fix()
        .local_minus_utc()
        / 60
}

/// « Today » becomes « this week » at the next local midnight, when nothing else may repaint.
fn until_midnight(now: &DateTime<Local>) -> Duration {
    let midnight = now
        .date_naive()
        .succ_opt()
        .and_then(|day| day.and_hms_opt(0, 0, 0))
        .and_then(|midnight| midnight.and_local_timezone(Local).earliest());
    midnight
        .and_then(|midnight| (midnight - *now).to_std().ok())
        .unwrap_or(Duration::from_hours(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    fn settled(state: &mut NotesQueryState, backend: &Backend, revision: Revision) {
        let ctx = egui::Context::default();
        let now = Utc::now();
        state.update(
            &ctx,
            backend,
            now,
            (None, None),
            (revision, IndexFill::Ready),
        );
        backend.settle();
        state.update(
            &ctx,
            backend,
            now,
            (None, None),
            (revision, IndexFill::Ready),
        );
    }

    #[test]
    fn a_write_adopted_asks_for_the_view_again() {
        let (backend, _profile) = testing::unlocked();
        let mut state = NotesQueryState::default();
        let mut revision = Revision::default();
        settled(&mut state, &backend, revision);
        let before = state.view().unwrap().cards().count();

        testing::note(&backend, "psql");
        revision.bump();
        settled(&mut state, &backend, revision);

        assert_eq!(state.view().unwrap().cards().count(), before + 1);
        assert!(state.error().is_none());
    }

    #[test]
    fn midnight_is_never_further_than_a_day() {
        let now = Local::now();

        let wait = until_midnight(&now);

        assert!(wait > Duration::ZERO && wait <= Duration::from_hours(25));
    }
}
