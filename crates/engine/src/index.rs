//! The live notes of the open library, decrypted and folded once and kept in memory: a search
//! reads them instead of opening every sealed body again. Nothing of it reaches the disk, and
//! an entry wipes its texts when it goes.
//!
//! Built after the unlock, newest first, a slice at a time on the db queue ([`fill`]), so a
//! command sent meanwhile runs between two slices.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use rayon::prelude::*;
use zeroize::Zeroize;

use crate::db::{self, Library, iso8601, lock};
use crate::error::{AppError, StorageError};
use crate::folders::model::NoteFolder;
use crate::notes::kind::NoteKind;
use crate::notes::model::Note;
use crate::notes::store;
use crate::notes::view::{self, Card, Criteria, Decorations, Facets, Folded, NotesQuery};
use crate::{Engine, EngineEvent};

/// Notes read and opened per job of the queue.
#[cfg(not(test))]
const SLICE: i64 = 2_000;
#[cfg(test)]
const SLICE: i64 = 3;

/// A note as the index holds it. Never changed in place: a write replaces the `Arc`.
#[derive(Debug)]
pub struct IndexedNote {
    pub note: Note,
    pub folded: Folded,
}

impl IndexedNote {
    pub(crate) fn of(note: Note) -> Self {
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

/// What a write touched, for the index to read again (`Engine::write`).
#[derive(Debug, Default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one flag a decoration the index keeps, read again whole"
)]
pub struct Changed {
    /// Notes whose row or side tables changed; one no longer live leaves the index.
    pub notes: Vec<String>,
    pub attachments: bool,
    pub folders: bool,
    pub globals: bool,
    /// Too many notes to name: the next read builds the index again.
    pub rebuild: bool,
}

impl Changed {
    pub fn notes(ids: impl IntoIterator<Item = String>) -> Self {
        Self {
            notes: ids.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn everything() -> Self {
        Self {
            rebuild: true,
            ..Self::default()
        }
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
    /// A [`fill`] is bringing the rest: a list answers with what is in.
    filling: bool,
    attachment_counts: HashMap<String, u32>,
    /// Every folder of the library, by id, as a card's chip names it.
    folders: HashMap<String, NoteFolder>,
    /// The global `{{field}}` values, sealed on disk.
    globals: BTreeMap<String, String>,
}

impl Drop for NoteIndex {
    fn drop(&mut self) {
        wipe_values(&mut self.globals);
        wipe_folders(&mut self.folders);
    }
}

fn wipe_folders(folders: &mut HashMap<String, NoteFolder>) {
    for folder in folders.values_mut() {
        folder.name.zeroize();
        folder.path.zeroize();
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
        self.filling &= !self.complete;
        Ok(!self.complete)
    }

    /// Reads again what `changed` names. An index not started on `library` has nothing to
    /// keep true: the next read builds it whole.
    pub(crate) fn refresh(
        &mut self,
        library: &mut Library,
        changed: &Changed,
    ) -> Result<(), StorageError> {
        if self.library != Some(library.number()) {
            return Ok(());
        }
        if changed.rebuild {
            self.clear();
            return Ok(());
        }
        if changed.attachments {
            self.attachment_counts = crate::attachments::store::counts(library.db())?;
        }
        if changed.folders {
            wipe_folders(&mut self.folders);
            self.folders = crate::folders::store::by_id(library, None)?;
        }
        if changed.globals {
            wipe_values(&mut self.globals);
            self.globals = store::global_placeholder_values(library)?;
        }
        if !changed.notes.is_empty() {
            let fresh = store::live(library, &changed.notes)?;
            for id in &changed.notes {
                self.notes.remove(id);
            }
            for note in fresh {
                self.notes
                    .insert(note.id.clone(), Arc::new(IndexedNote::of(note)));
            }
        }
        Ok(())
    }

    /// Whether this index holds what one built now from `library` would: the tests' check that
    /// no write forgot to say what it changed. One not started on `library` has nothing to be
    /// wrong about. `Err` says the first difference.
    #[doc(hidden)]
    pub fn matches(&self, library: &mut Library) -> Result<(), String> {
        if self.library != Some(library.number()) {
            return Ok(());
        }
        let mut fresh = Self::default();
        fresh.start(library).map_err(|error| error.to_string())?;
        while fresh
            .next_slice(library)
            .map_err(|error| error.to_string())?
        {}

        let mut ids: Vec<&String> = self.notes.keys().collect();
        let mut expected: Vec<&String> = fresh.notes.keys().collect();
        ids.sort();
        expected.sort();
        if ids != expected {
            return Err(format!("notes {ids:?}, expected {expected:?}"));
        }
        for (id, entry) in &fresh.notes {
            if self.notes[id].note != entry.note {
                return Err(format!("note {id} differs"));
            }
        }
        if self.attachment_counts != fresh.attachment_counts {
            return Err("attachment counts differ".to_string());
        }
        if self.folders != fresh.folders {
            return Err("folders differ".to_string());
        }
        if self.globals != fresh.globals {
            return Err("global values differ".to_string());
        }
        Ok(())
    }

    /// The whole index of `library`, whatever was read of it so far: a read waits for no slice.
    pub(crate) fn ready(&mut self, library: &mut Library) -> Result<(), StorageError> {
        self.start(library)?;
        while self.next_slice(library)? {}
        Ok(())
    }

    /// What a list needs before it answers: the first slice while a [`fill`] brings the rest,
    /// everything otherwise. Reading the rest here would keep the first cards for the last note.
    pub(crate) fn enough_to_list(&mut self, library: &mut Library) -> Result<(), StorageError> {
        if !self.filling {
            return self.ready(library);
        }
        if self.notes.is_empty() {
            self.next_slice(library)?;
        }
        Ok(())
    }

    /// What `notes::store::fetch` answers, from memory: the notes the space, the folders, the
    /// quick filter and the rails keep, edited last first, and the facets of the space.
    pub(crate) fn select(
        &self,
        query: &NotesQuery,
        folders: Option<&HashSet<String>>,
    ) -> (Vec<&Arc<IndexedNote>>, Facets) {
        let criteria = Criteria::from(query);
        let mut kept: Vec<&Arc<IndexedNote>> = self
            .notes
            .values()
            .filter(|entry| {
                let note = &entry.note;
                query
                    .space_id
                    .as_ref()
                    .is_none_or(|space| &note.space_id == space)
                    && folders.is_none_or(|ids| {
                        note.folder_id.as_ref().is_some_and(|id| ids.contains(id))
                    })
                    && criteria.passes_rails(note)
            })
            .collect();
        kept.sort_by(|a, b| {
            b.note
                .updated_at
                .cmp(&a.note.updated_at)
                .then_with(|| a.note.id.cmp(&b.note.id))
        });
        (kept, self.facets(query.space_id.as_deref()))
    }

    /// Counted as the SQL counted them, a space or every one. ⚠️ Tags that differ only in case
    /// are one, as under `COLLATE NOCASE`, named by their smallest spelling.
    pub(crate) fn facets(&self, space_id: Option<&str>) -> Facets {
        let mut tags: BTreeMap<String, (String, i64)> = BTreeMap::new();
        let mut languages: BTreeMap<&str, i64> = BTreeMap::new();
        let mut kinds: HashMap<&str, i64> = HashMap::new();
        let mut priorities: HashMap<&str, i64> = HashMap::new();
        let notes = self
            .notes
            .values()
            .map(|entry| &entry.note)
            .filter(|note| space_id.is_none_or(|space| note.space_id == space));
        for note in notes {
            for tag in &note.tags {
                let (spelling, count) = tags
                    .entry(tag.to_ascii_lowercase())
                    .or_insert_with(|| (tag.clone(), 0));
                if tag < spelling {
                    spelling.clone_from(tag);
                }
                *count += 1;
            }
            if note.kind == NoteKind::Snippet {
                *languages.entry(note.language.as_str()).or_default() += 1;
            }
            *kinds.entry(note.kind.as_str()).or_default() += 1;
            *priorities.entry(note.priority.as_str()).or_default() += 1;
        }
        let rows = |counts: HashMap<&str, i64>| -> Vec<(String, i64)> {
            counts
                .into_iter()
                .map(|(value, count)| (value.to_string(), count))
                .collect()
        };
        Facets {
            tags: view::count_tags(tags.into_values().collect()),
            languages: view::count_languages(
                &languages
                    .into_iter()
                    .map(|(value, count)| (value.to_string(), count))
                    .collect::<Vec<_>>(),
            ),
            kinds: view::count_kinds(&rows(kinds)),
            priorities: view::count_priorities(&rows(priorities)),
        }
    }

    /// What a card carries beside its note, for the whole library.
    pub(crate) fn decorations(&self) -> Arc<Decorations> {
        Arc::new(Decorations {
            attachment_counts: self.attachment_counts.clone(),
            folders: self.folders.clone(),
            globals: self.globals.clone(),
        })
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

/// The cards of `entries` that answer `needle`, in their order, each with the quote of a match
/// found elsewhere than in its title. Searched in parallel; nothing is copied.
pub(crate) fn search(entries: &[&Arc<IndexedNote>], needle: &str) -> Vec<Card> {
    let card = |entry: &Arc<IndexedNote>, hit| Card {
        note: Arc::clone(entry),
        hit,
    };
    if needle.is_empty() {
        return entries.iter().map(|entry| card(entry, None)).collect();
    }
    entries
        .par_iter()
        .filter_map(|entry| {
            view::answers(&entry.note, &entry.folded, needle).map(|hit| card(entry, hit))
        })
        .collect()
}

/// Fills the index of the library just unlocked, a slice a job, saying how far it got. Stops
/// when the index is complete, or another library was opened meanwhile.
/// In the job that opens the library, before anything can ask for a list: a [`fill`] follows.
pub fn announce(engine: &Engine) -> Result<(), AppError> {
    let mut library = lock(&engine.db)?;
    let mut index = engine.index()?;
    index.start(&mut library)?;
    index.filling = true;
    Ok(())
}

fn give_up(engine: &Engine) {
    if let Ok(mut index) = engine.index() {
        index.filling = false;
    }
}

pub async fn fill(engine: Arc<Engine>) {
    let total = db::run(Arc::clone(&engine), |engine| {
        let mut library = lock(&engine.db)?;
        let total = store::count_live(&mut library)?;
        engine.index()?.start(&mut library)?;
        Ok(total)
    })
    .await;
    let Ok(total) = total else {
        give_up(&engine);
        return;
    };
    let total = u32::try_from(total).unwrap_or(u32::MAX);
    loop {
        let read = db::run(Arc::clone(&engine), |engine| {
            let mut library = lock(&engine.db)?;
            let mut index = engine.index()?;
            let more = index.next_slice(&mut library)?;
            if !more {
                library.map_less()?;
            }
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
                give_up(&engine);
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
    fn a_list_takes_the_first_slice_while_a_fill_brings_the_rest() {
        let mut library = library(5);
        let mut index = NoteIndex::default();
        index.start(&mut library).unwrap();
        index.filling = true;

        index.enough_to_list(&mut library).unwrap();
        assert_eq!(index.len(), 3);

        index.enough_to_list(&mut library).unwrap();
        assert_eq!(index.len(), 3);
    }

    #[test]
    fn a_list_reads_everything_when_no_fill_is_coming() {
        let mut library = library(5);
        let mut index = NoteIndex::default();

        index.enough_to_list(&mut library).unwrap();

        assert_eq!(index.len(), 5);
    }

    #[test]
    fn the_last_slice_ends_the_fill() {
        let mut library = library(2);
        let mut index = NoteIndex::default();
        index.start(&mut library).unwrap();
        index.filling = true;

        index.next_slice(&mut library).unwrap();

        assert!(!index.filling);
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

#[cfg(test)]
mod oracle;
