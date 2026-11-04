//! Which notes are shown. It filters, sorts and groups nothing: `query_notes` does, and the
//! canvas draws the view it answers.

use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, Offset, TimeZone, Utc};
use engine::error::AppError;
use engine::notes::view::{
    Grouping, NoteFilter, NoteOrder, NotesQuery, NotesView, SortDirection, SortKey,
};
use serde_json::{Value, json};

use super::{QueryCell, Revision};
use crate::api;
use crate::backend::Backend;
use crate::gate::IndexFill;
use crate::services::preferences::Preferences;

/// The library's, like the space each was left on.
pub const ARRANGEMENT_KEY: &str = "devnotes.notes.arrangement";

/// How long the search waits after the last key, in egui's seconds: barely needed with the
/// index in memory, kept so a burst of keys asks once.
pub(super) const DEBOUNCE: f64 = 0.150;

pub const SORT_KEYS: [SortKey; 5] = [
    SortKey::Modified,
    SortKey::Created,
    SortKey::Priority,
    SortKey::Format,
    SortKey::Title,
];
pub const DIRECTIONS: [SortDirection; 2] = [SortDirection::Descending, SortDirection::Ascending];
pub const GROUPINGS: [Grouping; 4] = [
    Grouping::Date,
    Grouping::Priority,
    Grouping::Format,
    Grouping::None,
];

/// How the list view is laid out: the library's choice, remembered between launches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arrangement {
    pub order: NoteOrder,
    pub grouping: Grouping,
    pub pinned_first: bool,
}

impl Default for Arrangement {
    fn default() -> Self {
        Self {
            order: natural_order(SortKey::Modified),
            grouping: Grouping::Date,
            pinned_first: true,
        }
    }
}

/// The way a key reads first: the newest or the most pressing on top, words from A to Z.
pub fn natural_order(key: SortKey) -> NoteOrder {
    let direction = match key {
        SortKey::Title | SortKey::Format => SortDirection::Ascending,
        SortKey::Modified | SortKey::Created | SortKey::Priority => SortDirection::Descending,
    };
    NoteOrder { key, direction }
}

impl Arrangement {
    /// The reference's JSON, trusted no further than it reads: what it does not name stays
    /// the default.
    pub fn read(preferences: &Preferences) -> Self {
        let fallback = Self::default();
        let Some(stored) = preferences
            .get(ARRANGEMENT_KEY)
            .and_then(|text| serde_json::from_str::<Value>(text).ok())
        else {
            return fallback;
        };
        Self {
            order: NoteOrder {
                key: pick(&stored["order"]["key"], &SORT_KEYS, key_name)
                    .unwrap_or(fallback.order.key),
                direction: pick(&stored["order"]["direction"], &DIRECTIONS, direction_name)
                    .unwrap_or(fallback.order.direction),
            },
            grouping: pick(&stored["grouping"], &GROUPINGS, grouping_name)
                .unwrap_or(fallback.grouping),
            pinned_first: stored["pinnedFirst"]
                .as_bool()
                .unwrap_or(fallback.pinned_first),
        }
    }

    pub fn write(&self, preferences: &mut Preferences) {
        let stored = json!({
            "order": {
                "key": key_name(self.order.key),
                "direction": direction_name(self.order.direction),
            },
            "grouping": grouping_name(self.grouping),
            "pinnedFirst": self.pinned_first,
        });
        preferences.set(ARRANGEMENT_KEY, &stored.to_string());
    }
}

/// The variant of `all` stored under its name in `value`.
fn pick<T: Copy>(value: &Value, all: &[T], name: fn(T) -> &'static str) -> Option<T> {
    let stored = value.as_str()?;
    all.iter().copied().find(|each| name(*each) == stored)
}

/// The names the reference stored, which the core reads too.
fn key_name(key: SortKey) -> &'static str {
    match key {
        SortKey::Modified => "modified",
        SortKey::Created => "created",
        SortKey::Priority => "priority",
        SortKey::Format => "format",
        SortKey::Title => "title",
    }
}

fn direction_name(direction: SortDirection) -> &'static str {
    match direction {
        SortDirection::Descending => "descending",
        SortDirection::Ascending => "ascending",
    }
}

fn grouping_name(grouping: Grouping) -> &'static str {
    match grouping {
        Grouping::Date => "date",
        Grouping::Priority => "priority",
        Grouping::Format => "format",
        Grouping::None => "none",
    }
}

/// What the view depends on, compared by value: the instant is not among them, only the local
/// day, so the sections move at midnight and not on every frame.
#[derive(Debug, Clone, PartialEq)]
struct Params {
    space_id: Option<String>,
    folder_id: Option<String>,
    search: String,
    filter: NoteFilter,
    arrangement: Arrangement,
    day: NaiveDate,
    revision: Revision,
    /// The index answers what it holds so far: each slice it adds is asked again.
    index: IndexFill,
}

pub struct NotesQueryState {
    cell: QueryCell<Params, NotesView>,
    /// What the field shows, at once.
    typed: String,
    /// What the core is asked: the typed search once it has rested.
    search: String,
    /// When the typed search is asked for, in egui's seconds.
    due: Option<f64>,
    filter: NoteFilter,
    arrangement: Arrangement,
}

impl Default for NotesQueryState {
    fn default() -> Self {
        Self {
            cell: QueryCell::default(),
            typed: String::new(),
            search: String::new(),
            due: None,
            filter: NoteFilter::All,
            arrangement: Arrangement::default(),
        }
    }
}

impl NotesQueryState {
    /// The arrangement the library was left in, from its preferences.
    pub fn restore(&mut self, preferences: &Preferences) {
        self.arrangement = Arrangement::read(preferences);
    }

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
        if let Some(due) = self.due {
            let time = ctx.input(|input| input.time);
            if time >= due {
                self.search = self.typed.trim().to_owned();
                self.due = None;
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(due - time));
            }
        }
        let local = now.with_timezone(&Local);
        ctx.request_repaint_after(until_midnight(&local));
        self.cell.want(Params {
            space_id: place.0.map(str::to_owned),
            folder_id: place.1.map(str::to_owned),
            search: self.search.clone(),
            filter: self.filter,
            arrangement: self.arrangement,
            day: local.date_naive(),
            revision,
            index,
        });
        self.cell
            .update(|params| api::notes::query(backend, query(&params, now)))
    }

    /// What was typed, the moment it is typed; asked of the core once the keys rest.
    pub fn type_search(&mut self, ctx: &egui::Context, typed: String) {
        self.due = Some(ctx.input(|input| input.time) + DEBOUNCE);
        self.typed = typed;
    }

    /// The field and the search at once: a key on its way would put the search back.
    pub fn clear_search(&mut self) {
        self.typed.clear();
        self.search.clear();
        self.due = None;
    }

    pub fn typed(&self) -> &str {
        &self.typed
    }

    pub fn filter(&self) -> NoteFilter {
        self.filter
    }

    pub fn set_filter(&mut self, filter: NoteFilter) {
        self.filter = filter;
    }

    pub fn arrangement(&self) -> Arrangement {
        self.arrangement
    }

    pub fn arrange(&mut self, arrangement: Arrangement) {
        self.arrangement = arrangement;
    }

    pub fn view(&self) -> Option<&NotesView> {
        self.cell.value()
    }

    /// How many notes a filter leaves; `None` while nothing is filtered.
    pub fn matched(&self) -> Option<u32> {
        self.view()
            .filter(|view| view.is_filtering)
            .map(|view| view.matched)
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
        search: params.search.clone(),
        filter: params.filter,
        tags: Vec::new(),
        languages: Vec::new(),
        kinds: Vec::new(),
        priorities: Vec::new(),
        order: params.arrangement.order,
        grouping: params.arrangement.grouping,
        ranked: false,
        now,
        tz_offset_minutes: tz_offset_minutes(now),
        pinned_first: params.arrangement.pinned_first,
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
        let update = |state: &mut NotesQueryState| {
            state.update(
                &ctx,
                backend,
                now,
                (None, None),
                (revision, IndexFill::Ready),
            )
        };
        update(state);
        backend.settle();
        update(state);
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
    fn a_search_waits_for_the_keys_to_rest() {
        let (backend, _profile) = testing::unlocked();
        testing::note(&backend, "psql prod");
        testing::note(&backend, "curl health");
        let mut state = NotesQueryState::default();
        let ctx = egui::Context::default();
        let revision = Revision::default();
        let frame = |state: &mut NotesQueryState, time: f64| {
            testing::frame(&ctx, time, |_| {});
            state.update(
                &ctx,
                &backend,
                Utc::now(),
                (None, None),
                (revision, IndexFill::Ready),
            );
            backend.settle();
            state.update(
                &ctx,
                &backend,
                Utc::now(),
                (None, None),
                (revision, IndexFill::Ready),
            );
        };
        frame(&mut state, 0.0);

        state.type_search(&ctx, "psql ".into());
        frame(&mut state, 0.1);
        assert_eq!(state.matched(), None);
        assert_eq!(state.typed(), "psql ");

        frame(&mut state, 0.2);
        assert_eq!(state.matched(), Some(1));
    }

    #[test]
    fn an_arrangement_survives_its_file_and_a_damaged_one_reads_as_the_default() {
        let mut preferences = Preferences::default();
        let chosen = Arrangement {
            order: NoteOrder {
                key: SortKey::Title,
                direction: SortDirection::Ascending,
            },
            grouping: Grouping::None,
            pinned_first: false,
        };

        chosen.write(&mut preferences);
        assert_eq!(Arrangement::read(&preferences), chosen);

        preferences.set(
            ARRANGEMENT_KEY,
            r#"{"grouping":"kind","pinnedFirst":"yes"}"#,
        );
        assert_eq!(Arrangement::read(&preferences), Arrangement::default());
    }

    #[test]
    fn midnight_is_never_further_than_a_day() {
        let now = Local::now();

        let wait = until_midnight(&now);

        assert!(wait > Duration::ZERO && wait <= Duration::from_hours(25));
    }
}
