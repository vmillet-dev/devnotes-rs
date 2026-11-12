#![allow(clippy::needless_pass_by_value)]

pub mod calc;
pub mod checklist;
pub mod convert;
pub mod drawing;
pub(crate) mod duplicate;
pub mod kind;
pub mod language;
pub mod markdown;
pub mod model;
pub mod placeholder;
pub mod priority;
pub mod revision;
pub mod store;
pub mod trash;
pub mod view;

use std::collections::{BTreeMap, HashSet};

use chrono::Utc;

use crate::Engine;
use crate::count::saturating_u32 as count;
use crate::db::{Library, lock};
use crate::error::{AppError, StorageError};
use crate::folders;
use crate::index::Changed;
use crate::spaces::model::SpaceDraft;
use language::Language;
use model::{DisplayNote, NoteDraft, NotePatch, SampleNote, TagUsage};
use priority::Priority;
use revision::{DiffLine, Revision};
use store::pictures;
use trash::TrashedNote;
use view::{NotesQuery, NotesView};

/// No command returns the raw list: it would invite re-filtering on the front end.
///
/// Read from the index: no SQLite, no decryption, but for the folders under an opened one.
pub fn query_notes(engine: &Engine, query: NotesQuery) -> Result<NotesView, AppError> {
    let mut connection = lock(&engine.db)?;
    let folders = match query.folder_id.as_deref() {
        Some(id) => Some(folders::store::subtree(connection.db(), id)?),
        None => None,
    }
    .map(|ids| ids.into_iter().collect::<HashSet<_>>());
    let mut index = engine.index()?;
    index.enough_to_list(&mut connection)?;
    drop(connection);

    let (entries, facets) = index.select(&query, folders.as_ref());
    let cards = crate::index::search(&entries, view::Criteria::from(&query).needle());
    let decorations = index.decorations();
    drop(index);

    Ok(view::arrange(cards, facets, &query, decorations))
}

/// The path the index replaced, kept as its oracle: SQL, every body opened, `view::build`.
#[cfg(test)]
pub(crate) fn query_notes_from_database(
    engine: &Engine,
    query: &NotesQuery,
) -> Result<NotesView, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let (notes, facets) = store::fetch(&mut connection, query)?;

    let mut decorations = store::decorations(&mut connection)?;
    decorations.folders = folders::store::by_id(&mut connection, query.space_id.as_deref())?;
    drop(connection);

    let mut view = view::build(notes, facets, query);
    view.decorations = std::sync::Arc::new(decorations);

    Ok(view)
}

/// The whole note. A list sends previews ([`DisplayNote::truncated`]), so the editor and
/// every copy of a long body read it here.
pub fn get_note(engine: &Engine, id: String) -> Result<DisplayNote, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let note = store::get(&mut connection, &id)?;

    Ok(display(&mut connection, note)?)
}

/// A Dessin's small picture, for its card, which asks for it once it is drawn: a list sends
/// none, so that a search opens no picture. `None` when there is none to show.
pub fn drawing_thumbnail(engine: &Engine, id: String) -> Result<Option<String>, AppError> {
    picture_of(engine, &id, pictures::Which::Thumbnail)
}

/// A Dessin's large picture, for a Note that embeds it by its id.
pub fn drawing_picture(engine: &Engine, id: String) -> Result<Option<String>, AppError> {
    picture_of(engine, &id, pictures::Which::Picture)
}

/// The drawings a Note can embed, by title: their titles alone are opened.
pub fn list_drawings(engine: &Engine) -> Result<Vec<drawing::DrawingRef>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let (connection, vault) = connection.split();
    Ok(pictures::drawings(connection, vault)?)
}

fn picture_of(
    engine: &Engine,
    id: &str,
    which: pictures::Which,
) -> Result<Option<String>, AppError> {
    let mut connection = lock(&engine.db)?;
    let (connection, vault) = connection.split();
    Ok(pictures::read(connection, vault, id, which)?)
}

pub fn create_note(engine: &Engine, draft: NoteDraft) -> Result<DisplayNote, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;
        let note = store::create(&mut connection, draft, Utc::now())?;

        let answer = display(&mut connection, note)?;
        let changed = Changed::notes([answer.note.id.clone()]);
        Ok((answer, changed))
    })
}

/// What a paste into an empty Note is. Asked by the front end before it writes: typing keeps
/// a Note prose, and only a paste of code turns it into a snippet with a language.
#[allow(clippy::needless_pass_by_value)]
pub fn detect_language(content: String) -> Language {
    language::from_content(&content)
}

/// A Note cut into slides, with the mind map of its headings: what « Présenter » shows. The
/// draft is sent rather than the stored body, so what is presented is what is on screen.
pub async fn present_note(markdown: String) -> Result<model::Presentation, AppError> {
    crate::tools::off_thread(move || markdown::present(&markdown)).await
}

/// A Calcul note's sheet, every line computed, as it is typed: no lock, and nothing stored.
/// `today` is the local day of the window, as the notes' sections reckon it.
pub async fn evaluate_calc(
    sheet: String,
    today: chrono::NaiveDate,
) -> Result<calc::CalcSheet, AppError> {
    crate::tools::off_thread(move || calc::evaluate(&sheet, today)).await
}

/// Where the fields of a text lie, for the code field to mark them: what a field is stays here.
#[allow(clippy::needless_pass_by_value)]
pub fn field_spans(content: String) -> Vec<placeholder::FieldSpan> {
    placeholder::spans(&content)
}

/// A copy to start another note from, attachments included and history left behind. The
/// title is the front end's: its suffix is a translation.
pub fn duplicate_note(engine: &Engine, id: String, title: String) -> Result<DisplayNote, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;
        let note = duplicate::duplicate(&mut connection, &id, title, Utc::now())?;

        let answer = display(&mut connection, note)?;
        let changed = Changed {
            attachments: true,
            ..Changed::notes([answer.note.id.clone()])
        };
        Ok((answer, changed))
    })
}

/// The first launch: the space, its folders and the notes in one transaction, since a space
/// standing alone reads as "already seeded" for good. The strings stay on the front end, with
/// the translations. Answers the space it made, which the front end opens on.
pub fn seed_samples(
    engine: &Engine,
    space_name: String,
    folders: Vec<String>,
    notes: Vec<SampleNote>,
) -> Result<crate::spaces::model::Space, AppError> {
    engine.write(|db| {
        let name = SpaceDraft { name: space_name }.validated_name()?;
        let folder_names = folders
            .iter()
            .map(|folder| crate::folders::model::validated_name(folder))
            .collect::<Result<Vec<_>, _>>()?;

        let mut connection = lock(db)?;

        let answer = store::seed(&mut connection, &name, &folder_names, notes, Utc::now())?;
        let changed = Changed::everything();
        Ok((answer, changed))
    })
}

pub fn update_note(engine: &Engine, id: String, patch: NotePatch) -> Result<DisplayNote, AppError> {
    engine.write(|db| {
        // Read before anything is locked: two PNGs, or the patch is refused.
        let drawn = patch
            .pictures
            .as_ref()
            .map(drawing::DrawingPictures::decoded)
            .transpose()?;
        // `Some(None)`: the pictures are taken away, the drawing having been emptied.
        let mut connection = lock(db)?;
        let note = store::update_drawn(
            &mut connection,
            &id,
            &patch,
            drawn.as_ref().map(Option::as_ref),
            Utc::now(),
        )?;

        let answer = display(&mut connection, note)?;
        let changed = Changed::notes([id.clone()]);
        Ok((answer, changed))
    })
}

/// The note rewritten as `into`, with the body it had: its undo is that body written back.
pub fn convert_note(
    engine: &Engine,
    id: String,
    into: kind::NoteKind,
) -> Result<convert::NoteConversion, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;
        let (note, previous) = store::convert(&mut connection, &id, into, Utc::now())?;

        let answer = convert::NoteConversion {
            note: display(&mut connection, note)?,
            previous,
        };
        let changed = Changed::notes([id.clone()]);
        Ok((answer, changed))
    })
}

/// The bodies kept beside a note, newest first.
///
/// Instants and sizes, never the bodies.
pub fn list_revisions(engine: &Engine, id: String) -> Result<Vec<Revision>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    let (connection, vault) = connection.split();

    Ok(store::revisions::list(connection, vault, &id)?)
}

/// What going back to a kept body would change, line by line, against the current text.
pub fn revision_diff(
    engine: &Engine,
    id: String,
    revision_id: String,
) -> Result<Vec<DiffLine>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let (connection, vault) = connection.split();

    let Some((current, version)) = store::revisions::compare(connection, vault, &id, &revision_id)?
    else {
        return Err(StorageError::RevisionNotFound(revision_id).into());
    };

    Ok(revision::diff(&current, &version))
}

/// Goes back to a kept body, dropping it and every body kept after it from the history.
///
/// Leaves `updated_at` alone: putting something back is not editing it.
pub fn restore_revision(
    engine: &Engine,
    id: String,
    revision_id: String,
) -> Result<DisplayNote, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;
        let note = store::restore_revision(&mut connection, &id, &revision_id)?;

        let answer = display(&mut connection, note)?;
        let changed = Changed::notes([id.clone()]);
        Ok((answer, changed))
    })
}

/// Moves to the trash: the note comes back through [`restore_notes`] for
/// [`trash::RETENTION`].
pub fn delete_note(engine: &Engine, id: String) -> Result<(), AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        store::trash::trash(&mut connection, &id, Utc::now())?;
        let changed = Changed::notes([id.clone()]);
        Ok(((), changed))
    })
}

pub fn delete_notes(engine: &Engine, ids: Vec<String>) -> Result<u32, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = count(store::trash::trash_many(&mut connection, &ids, Utc::now())?);
        let changed = Changed::notes(ids.clone());
        Ok((answer, changed))
    })
}

pub fn restore_notes(engine: &Engine, ids: Vec<String>) -> Result<u32, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = count(store::trash::restore_many(&mut connection, &ids)?);
        let changed = Changed::notes(ids.clone());
        Ok((answer, changed))
    })
}

/// Purges first: the trash must never show a note a restart would erase. `search` narrows it
/// the way the canvas search does, folded; empty lists everything.
pub fn list_trash(engine: &Engine, search: String) -> Result<Vec<TrashedNote>, AppError> {
    let db = &engine.db;
    trash::purge_expired(db)?;

    let mut connection = lock(db)?;

    Ok(trash_matching(&mut connection, &search)?
        .into_iter()
        .map(|(note, deleted_at)| trash::trashed(note, deleted_at))
        .collect())
}

/// What the trash lists for `search` back where it was: the way out of a mistaken bulk deletion.
pub fn restore_trash(engine: &Engine, search: String) -> Result<u32, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;
        let ids: Vec<String> = trash_matching(&mut connection, &search)?
            .into_iter()
            .map(|(note, _)| note.id)
            .collect();

        let answer = count(store::trash::restore_many(&mut connection, &ids)?);
        let changed = Changed::notes(ids.clone());
        Ok((answer, changed))
    })
}

fn trash_matching(
    connection: &mut Library,
    search: &str,
) -> Result<Vec<(model::Note, chrono::DateTime<Utc>)>, AppError> {
    let needle = view::fold(search.trim());

    Ok(store::trash::list_trashed(connection)?
        .into_iter()
        .filter(|(note, _)| needle.is_empty() || view::matches_search(note, &needle))
        .collect())
}

pub fn purge_notes(engine: &Engine, ids: Vec<String>) -> Result<u32, AppError> {
    engine.write(|db| {
        let changed = Changed {
            attachments: true,
            ..Changed::notes(ids.clone())
        };
        Ok((count(trash::purge(db, ids)?), changed))
    })
}

pub fn empty_trash(engine: &Engine) -> Result<u32, AppError> {
    engine.write(|db| {
        let ids = {
            let mut connection = lock(db)?;
            store::trash::trashed_ids(&mut connection)?
        };

        let answer = count(trash::purge(db, ids)?);
        let changed = Changed {
            attachments: true,
            ..Changed::default()
        };
        Ok((answer, changed))
    })
}

pub fn move_notes(
    engine: &Engine,
    ids: Vec<String>,
    space_id: String,
) -> Result<Vec<model::NotePlacement>, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = store::move_many(&mut connection, &ids, &space_id, Utc::now())?;
        let changed = Changed::notes(ids.clone());
        Ok((answer, changed))
    })
}

/// The undo of [`move_notes`]: each note goes back to the space it left.
pub fn move_notes_back(
    engine: &Engine,
    placements: Vec<model::NotePlacement>,
) -> Result<u32, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = count(store::restore_placements(&mut connection, &placements)?);
        let changed = Changed::notes(placements.iter().map(|one| one.note_id.clone()));
        Ok((answer, changed))
    })
}

/// Normalized here as everywhere else, or a typed `#urgent` would not join `urgent`.
pub fn tag_notes(
    engine: &Engine,
    ids: Vec<String>,
    tags: Vec<String>,
) -> Result<Vec<model::NoteTag>, AppError> {
    engine.write(|db| {
        let normalized = model::normalize_tags(&tags);

        let mut connection = lock(db)?;

        let answer = store::tag_many(&mut connection, &ids, &normalized, Utc::now())?;
        let changed = Changed::notes(ids.clone());
        Ok((answer, changed))
    })
}

/// The undo of [`tag_notes`]: exactly the pairs it added, and nothing wider.
pub fn untag_notes(engine: &Engine, pairs: Vec<model::NoteTag>) -> Result<u32, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = count(store::untag_many(&mut connection, &pairs)?);
        let changed = Changed::notes(pairs.iter().map(|one| one.note_id.clone()));
        Ok((answer, changed))
    })
}

/// Answers the pairs it took off: normalized like every tag, so `#Urgent` finds `urgent`.
pub fn remove_tag(
    engine: &Engine,
    ids: Vec<String>,
    tag: String,
) -> Result<Vec<model::NoteTag>, AppError> {
    engine.write(|db| {
        let Some(tag) = model::normalize_tags(&[tag]).into_iter().next() else {
            return Ok((Vec::new(), Changed::default()));
        };

        let mut connection = lock(db)?;

        let answer = store::remove_tag(&mut connection, &ids, &tag, Utc::now())?;
        let changed = Changed::notes(ids.clone());
        Ok((answer, changed))
    })
}

/// The undo of [`remove_tag`]: exactly the pairs it took off.
pub fn restore_tags(engine: &Engine, pairs: Vec<model::NoteTag>) -> Result<u32, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = count(store::restore_tags(&mut connection, &pairs)?);
        let changed = Changed::notes(pairs.iter().map(|one| one.note_id.clone()));
        Ok((answer, changed))
    })
}

/// Answers the notes whose pin changed; undone by the same call, the other way, over those.
pub fn set_pinned(
    engine: &Engine,
    ids: Vec<String>,
    pinned: bool,
) -> Result<Vec<String>, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = store::set_pinned_many(&mut connection, &ids, pinned)?;
        let changed = Changed::notes(answer.clone());
        Ok((answer, changed))
    })
}

/// Answers each note whose priority moved, with the one it had; `updated_at` stays.
pub fn set_priority(
    engine: &Engine,
    ids: Vec<String>,
    priority: Priority,
) -> Result<Vec<model::NotePriority>, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = store::set_priority_many(&mut connection, &ids, priority)?;
        let changed = Changed::notes(ids.clone());
        Ok((answer, changed))
    })
}

/// The undo of [`set_priority`]: each note gets back the priority it had.
pub fn restore_priorities(
    engine: &Engine,
    previous: Vec<model::NotePriority>,
) -> Result<u32, AppError> {
    engine.write(|db| {
        let mut connection = lock(db)?;

        let answer = count(store::restore_priorities(&mut connection, &previous)?);
        let changed = Changed::notes(previous.iter().map(|one| one.note_id.clone()));
        Ok((answer, changed))
    })
}

/// What a corpus-wide tag action is about to touch, asked before it runs.
pub fn count_notes_tagged(engine: &Engine, tags: Vec<String>) -> Result<u32, AppError> {
    let db = &engine.db;
    let normalized = model::normalize_tags(&tags);

    let mut connection = lock(db)?;

    Ok(count(store::count_notes_tagged(
        &mut connection,
        &normalized,
    )?))
}

pub fn list_tags(engine: &Engine) -> Result<Vec<TagUsage>, AppError> {
    let mut connection = lock(&engine.db)?;
    let mut index = engine.index()?;
    index.ready(&mut connection)?;

    Ok(index
        .facets(None)
        .tags
        .into_iter()
        .map(|facet| TagUsage {
            tag: facet.value,
            note_count: facet.count,
        })
        .collect())
}

/// Renames one tag or merges several: renaming onto an existing tag is a merge anyway,
/// since a note cannot carry one twice.
pub fn rename_tags(engine: &Engine, tags: Vec<String>, into: String) -> Result<u32, AppError> {
    engine.write(|db| {
        let (sources, target) = model::retagging(&tags, &into)?;

        let mut connection = lock(db)?;

        let answer = count(store::retag(&mut connection, &sources, &target)?);
        let changed = Changed::everything();
        Ok((answer, changed))
    })
}

/// A list rather than one tag at a time: one round trip per tag is one lock per tag.
pub fn delete_tags(engine: &Engine, tags: Vec<String>) -> Result<u32, AppError> {
    engine.write(|db| {
        let tags = model::normalize_tags(&tags);

        let mut connection = lock(db)?;

        let answer = count(store::drop_tags(&mut connection, &tags)?);
        let changed = Changed::everything();
        Ok((answer, changed))
    })
}

/// A command of its own rather than a `NotePatch` field, so `updated_at` stays put: filling a
/// field is not editing the note.
pub fn set_placeholder_values(
    engine: &Engine,
    id: String,
    values: BTreeMap<String, String>,
) -> Result<DisplayNote, AppError> {
    engine.write(|db| {
        let retained = placeholder::normalize_values(values);

        let mut connection = lock(db)?;
        let note = store::set_placeholder_values(&mut connection, &id, &retained)?;

        let answer = display(&mut connection, note)?;
        let changed = Changed::notes([id.clone()]);
        Ok((answer, changed))
    })
}

/// No note identifier: the palette fills an unsaved draft as readily as a saved note.
/// The database is read only for the global variables.
pub fn fill_placeholders(
    engine: &Engine,
    content: String,
    values: BTreeMap<String, String>,
) -> Result<String, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let globals = store::global_placeholder_values(&mut connection)?;

    Ok(placeholder::fill(
        &content,
        &placeholder::resolve(&globals, &values),
    ))
}

pub fn list_global_placeholders(engine: &Engine) -> Result<BTreeMap<String, String>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::global_placeholder_values(&mut connection)?)
}

/// Stores the whole set: what is not sent is what the user removed.
pub fn set_global_placeholders(
    engine: &Engine,
    values: BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, AppError> {
    engine.write(|db| {
        let retained = placeholder::normalize_values(values);

        let mut connection = lock(db)?;
        store::replace_global_placeholder_values(&mut connection, &retained)?;

        let answer = retained;
        let changed = Changed {
            globals: true,
            ..Changed::default()
        };
        Ok((answer, changed))
    })
}

/// What only the database knows, each a query of its own: hence not in [`model::decorate`].
fn display(connection: &mut Library, note: model::Note) -> Result<DisplayNote, StorageError> {
    let mut decorated = model::decorate_now(note);
    store::decorations(connection)?.apply([&mut decorated]);

    Ok(decorated)
}

/// Reference note for the feature tests: a field added to [`model::Note`] is declared
/// here rather than in every module that builds one.
#[cfg(test)]
pub(crate) mod fixtures {
    use std::collections::BTreeMap;

    use chrono::{DateTime, Utc};

    use super::kind::NoteKind;
    use super::language::Language;
    use super::model::{Note, NoteLifecycle};
    use super::priority::Priority;
    use crate::db::iso8601;

    pub(crate) const NOW: &str = "2026-07-25T09:00:00.000Z";

    pub(crate) fn at(iso: &str) -> DateTime<Utc> {
        iso8601::parse(iso).expect("tests write valid instants")
    }

    pub(crate) fn note() -> Note {
        Note {
            id: "n-1".to_string(),
            space_id: "s-1".to_string(),
            folder_id: None,
            title: "Title".to_string(),
            language: Language::Txt,
            content: "Content".to_string(),
            source: String::new(),
            tags: vec!["auth".to_string()],
            pinned: false,
            created_at: at(NOW),
            updated_at: at(NOW),
            lifecycle: NoteLifecycle::Permanent,
            kind: NoteKind::Snippet,
            priority: Priority::None,
            items: Vec::new(),
            placeholder_values: BTreeMap::new(),
        }
    }
}
