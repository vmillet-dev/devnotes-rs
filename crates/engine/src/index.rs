//! The live notes of the open library, decrypted and folded once and kept in memory: a search
//! reads them instead of opening every sealed body again. Nothing of it reaches the disk, and
//! an entry wipes its texts when it goes.
//!
//! Built after the unlock, newest first, a slice at a time on the db queue ([`fill`]), so a
//! command sent meanwhile runs between two slices.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rayon::prelude::*;
use zeroize::Zeroize;

use crate::db::{self, Library, iso8601, lock};
use crate::error::StorageError;
use crate::folders::model::NoteFolder;
use crate::notes::model::Note;
use crate::notes::store;
use crate::notes::view::Folded;
use crate::{Engine, EngineEvent};

/// Notes read and opened per job of the queue.
#[cfg(not(test))]
const SLICE: i64 = 2_000;
#[cfg(test)]
const SLICE: i64 = 3;

/// A note as the index holds it. Never changed in place: a write replaces the `Arc`.
pub struct IndexedNote {
    pub note: Note,
    pub folded: Folded,
}

impl IndexedNote {
    fn of(note: Note) -> Self {
        let folded = Folded::of(&note);
        Self { note, folded }
    }
}

impl Drop for IndexedNote {
    fn drop(&mut self) {
        let Note {
            title,
            content,
            source,
            tags,
            items,
            placeholder_values,
            ..
        } = &mut self.note;
        title.zeroize();
        content.zeroize();
        source.zeroize();
        tags.zeroize();
        for item in items.iter_mut() {
            item.text.zeroize();
        }
        wipe_values(placeholder_values);
        let Folded {
            title,
            tags,
            content,
            items,
            source,
        } = &mut self.folded;
        title.zeroize();
        tags.zeroize();
        content.zeroize();
        items.zeroize();
        source.zeroize();
    }
}

fn wipe_values(values: &mut BTreeMap<String, String>) {
    for (mut name, mut value) in std::mem::take(values) {
        name.zeroize();
        value.zeroize();
    }
}

#[derive(Default)]
pub struct NoteIndex {
    /// The library it holds (`Library::number`); `None` before an unlock and after a switch.
    library: Option<u64>,
    notes: HashMap<String, Arc<IndexedNote>>,
    /// Where the next slice starts: the `updated_at` and id of the last note read. `None` once
    /// every note is in.
    next: Option<(String, String)>,
    complete: bool,
    attachment_counts: HashMap<String, u32>,
    /// Every folder of the library, by id, as a card's chip names it.
    folders: HashMap<String, NoteFolder>,
    /// The global `{{field}}` values, sealed on disk.
    globals: BTreeMap<String, String>,
}

impl Drop for NoteIndex {
    fn drop(&mut self) {
        wipe_values(&mut self.globals);
        for folder in self.folders.values_mut() {
            folder.name.zeroize();
            folder.path.zeroize();
        }
    }
}

impl NoteIndex {
    /// Empties it: another library is opened, or a copy put back. The entries wipe themselves.
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    /// Starts on `library`, with its decorations and no note yet, unless it holds it already.
    pub(crate) fn start(&mut self, library: &mut Library) -> Result<(), StorageError> {
        if self.library == Some(library.number()) {
            return Ok(());
        }
        self.clear();
        self.attachment_counts = crate::attachments::store::counts(library.db())?;
        self.folders = crate::folders::store::by_id(library, None)?;
        self.globals = store::global_placeholder_values(library)?;
        self.library = Some(library.number());
        Ok(())
    }

    /// Reads the next slice; `false` once there is nothing left to read, or when `library` is
    /// not the one this index was started on.
    pub(crate) fn next_slice(&mut self, library: &mut Library) -> Result<bool, StorageError> {
        if self.complete || self.library != Some(library.number()) {
            return Ok(false);
        }
        let notes = store::newest(library, self.next.as_ref(), SLICE)?;
        let read = notes.len();
        if let Some(last) = notes.last() {
            self.next = Some((iso8601::format(last.updated_at), last.id.clone()));
        }
        let entries: Vec<IndexedNote> = notes.into_par_iter().map(IndexedNote::of).collect();
        for entry in entries {
            self.notes.insert(entry.note.id.clone(), Arc::new(entry));
        }
        self.complete = i64::try_from(read).unwrap_or(i64::MAX) < SLICE;
        Ok(!self.complete)
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn get(&self, id: &str) -> Option<&Arc<IndexedNote>> {
        self.notes.get(id)
    }
}

/// Fills the index of the library just unlocked, a slice a job, saying how far it got. Stops
/// when the index is complete, or another library was opened meanwhile.
pub async fn fill(engine: Arc<Engine>) {
    let total = db::run(Arc::clone(&engine), |engine| {
        let mut library = lock(&engine.db)?;
        let total = store::count_live(&mut library)?;
        engine.index()?.start(&mut library)?;
        Ok(total)
    })
    .await;
    let Ok(total) = total else {
        return;
    };
    let total = u32::try_from(total).unwrap_or(u32::MAX);
    loop {
        let read = db::run(Arc::clone(&engine), |engine| {
            let mut library = lock(&engine.db)?;
            let mut index = engine.index()?;
            let more = index.next_slice(&mut library)?;
            let done = u32::try_from(index.len()).unwrap_or(u32::MAX);
            Ok((more, done, index.is_complete()))
        })
        .await;
        match read {
            Ok((true, done, _)) => engine.emit(EngineEvent::IndexProgress { done, total }),
            Ok((false, _, complete)) => {
                if complete {
                    engine.emit(EngineEvent::IndexReady);
                }
                return;
            }
            Err(error) => {
                log::warn!("The notes index stopped filling: {}", error.detail);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use super::*;
    use crate::db::open_in_memory;
    use crate::notes::fixtures::{NOW, at, note};

    /// `count` notes, the one numbered highest edited last.
    fn library(count: u32) -> Library {
        let mut library = open_in_memory().unwrap();
        let space_id = crate::spaces::store::create(&mut library, "Perso")
            .unwrap()
            .id;
        for n in 0..count {
            let note = Note {
                id: format!("n-{n:02}"),
                space_id: space_id.clone(),
                title: format!("Écrite n°{n}"),
                content: format!("Corps {n}"),
                tags: vec!["auth".to_string()],
                updated_at: at(NOW) + TimeDelta::minutes(i64::from(n)),
                ..note()
            };
            store::insert_imported(&mut library, &note).unwrap();
        }
        library
    }

    #[test]
    fn it_fills_a_slice_at_a_time_from_the_note_edited_last() {
        let mut library = library(7);
        let mut index = NoteIndex::default();
        index.start(&mut library).unwrap();

        assert!(index.next_slice(&mut library).unwrap());
        let mut first: Vec<&str> = index.notes.keys().map(String::as_str).collect();
        first.sort_unstable();
        assert_eq!(first, ["n-04", "n-05", "n-06"]);
        assert!(!index.is_complete());

        assert!(index.next_slice(&mut library).unwrap());
        assert!(!index.next_slice(&mut library).unwrap());
        assert!(index.is_complete());
        assert_eq!(index.len(), 7);
        assert!(!index.next_slice(&mut library).unwrap());
    }

    #[test]
    fn an_entry_holds_the_note_as_the_store_reads_it_and_its_folded_text() {
        let mut library = library(4);
        let mut index = NoteIndex::default();
        index.start(&mut library).unwrap();
        while index.next_slice(&mut library).unwrap() {}

        for id in ["n-00", "n-01", "n-02", "n-03"] {
            let entry = index.get(id).unwrap();
            assert_eq!(entry.note, store::get(&mut library, id).unwrap());
            assert_eq!(
                entry.folded.title,
                crate::notes::view::fold(&entry.note.title)
            );
        }
        assert!(
            index
                .get("n-01")
                .unwrap()
                .folded
                .title
                .starts_with("ecrite")
        );
    }

    #[test]
    fn a_slice_read_for_one_library_stops_once_another_is_open() {
        let (mut first, mut second) = (library(5), library(5));
        let mut index = NoteIndex::default();
        index.start(&mut first).unwrap();
        index.next_slice(&mut first).unwrap();

        assert!(!index.next_slice(&mut second).unwrap());
        assert!(!index.is_complete());

        index.start(&mut second).unwrap();
        assert!(index.is_empty(), "started over on the other one");
        assert!(index.next_slice(&mut second).unwrap());
    }

    #[test]
    fn starting_again_on_the_same_library_keeps_what_was_read() {
        let mut library = library(5);
        let mut index = NoteIndex::default();
        index.start(&mut library).unwrap();
        index.next_slice(&mut library).unwrap();

        index.start(&mut library).unwrap();

        assert_eq!(index.len(), 3);
    }

    #[test]
    fn cleared_it_holds_nothing_and_no_library() {
        let mut library = library(2);
        let mut index = NoteIndex::default();
        index.start(&mut library).unwrap();
        index.next_slice(&mut library).unwrap();

        index.clear();

        assert!(index.is_empty());
        assert!(!index.next_slice(&mut library).unwrap());
    }

    /// S4: sorting itself, SQLite read every sealed body before the first row (1.76 s).
    #[test]
    fn a_slice_is_read_in_the_order_of_the_updated_at_index() {
        use diesel::prelude::*;

        #[derive(QueryableByName)]
        struct Step {
            #[diesel(sql_type = diesel::sql_types::Text)]
            detail: String,
        }

        let mut library = library(1);
        let plan: Vec<String> = diesel::sql_query(
            "EXPLAIN QUERY PLAN SELECT * FROM notes WHERE deleted_at IS NULL              AND (updated_at < '9' OR (updated_at = '9' AND id > 'a'))              ORDER BY updated_at DESC, id ASC LIMIT 2000",
        )
        .load::<Step>(library.db())
        .unwrap()
        .into_iter()
        .map(|step| step.detail)
        .collect();

        assert!(
            plan.iter().any(|step| step.contains("notes_updated_at")),
            "{plan:?}"
        );
        assert!(
            !plan
                .iter()
                .any(|step| step == "USE TEMP B-TREE FOR ORDER BY"),
            "{plan:?}"
        );
    }

    #[test]
    fn a_dropped_value_map_is_left_empty() {
        let mut values = BTreeMap::from([("host".to_string(), "prod.example".to_string())]);
        wipe_values(&mut values);
        assert!(values.is_empty());
    }
}
