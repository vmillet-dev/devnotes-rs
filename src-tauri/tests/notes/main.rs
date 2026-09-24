//! The notes feature below its commands: the store, the view, and the rules they apply,
//! one module per subject.

use devnotes_lib::db::Library;
use devnotes_lib::db::open_in_memory;
use devnotes_lib::db::schema::{
    note_items, note_placeholders, note_revisions, note_tags, notes as notes_table,
    spaces as spaces_table,
};
use devnotes_lib::error::StorageError;
use devnotes_lib::notes::checklist::{ChecklistItem, NoteKind};
use devnotes_lib::notes::language::Language;
use devnotes_lib::notes::model::{
    DisplayNote, Note, NoteDraft, NoteLifecycle, NotePatch, PREVIEW_LINES, decorate,
};
use devnotes_lib::notes::store::trash::{expired_ids, list_trashed, purge, restore_many, trash};
use devnotes_lib::notes::store::{
    all, by_ids, count_notes_tagged, create, drop_tags, fetch, get, global_placeholder_values,
    insert_imported, move_many, replace_global_placeholder_values, restore_placements, retag, seed,
    set_placeholder_values, tag_many, tag_usage, untag_many, update,
};
use devnotes_lib::notes::view::{self, NoteFilter, NotesQuery, NotesView};
use devnotes_lib::spaces::store as spaces;
use diesel::prelude::*;
use std::collections::BTreeMap;

#[path = "../common/mod.rs"]
mod common;
use common::{at, snippet, space, t0, t1};

mod batches;
mod checklists;
mod exchange;
mod fields;
mod queries;
mod revisions;
mod seeding;
mod storage;
mod tags;
mod trash;
mod writes;

fn sample(folder: Option<u32>, draft: NoteDraft) -> devnotes_lib::notes::model::SampleNote {
    devnotes_lib::notes::model::SampleNote { folder, draft }
}

/// No such shortcut exists in production code: it would invite re-filtering on the
/// front end.
fn list(connection: &mut Library) -> Result<Vec<Note>, StorageError> {
    fetch(
        connection,
        &NotesQuery {
            space_id: None,
            folder_id: None,
            search: String::new(),
            filter: NoteFilter::All,
            tags: Vec::new(),
            languages: Vec::new(),
            now: t0(),
            tz_offset_minutes: 0,
            pinned_first: true,
        },
    )
    .map(|(notes, _)| notes)
}

fn query(connection: &mut Library, request: &NotesQuery) -> Result<NotesView, StorageError> {
    let (notes, facets) = fetch(connection, request)?;
    Ok(view::build(notes, facets, request))
}

/// The snippet these scenarios start from: tagged and sourced, since so many are about that.
fn draft(space_id: &str) -> NoteDraft {
    NoteDraft {
        source: "API Gateway / Auth".to_string(),
        tags: vec!["auth".to_string(), "api".to_string()],
        ..snippet(space_id)
    }
}

fn item(text: &str, done: bool) -> ChecklistItem {
    ChecklistItem {
        text: text.to_string(),
        done,
    }
}

fn checklist(space_id: &str, items: Vec<ChecklistItem>) -> NoteDraft {
    NoteDraft {
        kind: NoteKind::Checklist,
        content: String::new(),
        items,
        ..draft(space_id)
    }
}

/// This section's reference snippet: two fields, one of them with a default.
fn templated(space_id: &str) -> NoteDraft {
    NoteDraft {
        content: "psql -h {{host}} -p {{port=5432}}".to_string(),
        ..draft(space_id)
    }
}

fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect()
}

/// Neutral query: tests override one field at a time, so each states exactly what
/// it exercises.
fn all_notes() -> NotesQuery {
    NotesQuery {
        space_id: None,
        folder_id: None,
        search: String::new(),
        filter: NoteFilter::All,
        tags: Vec::new(),
        languages: Vec::new(),
        now: t1(),
        tz_offset_minutes: 0,
        pinned_first: true,
    }
}

fn matched_ids(view: &NotesView) -> Vec<String> {
    view.sections
        .iter()
        .flat_map(|section| section.notes.iter().map(|note| note.id.clone()))
        .collect()
}

fn tagged(space_id: &str, tags: &[&str]) -> NoteDraft {
    NoteDraft {
        tags: tags.iter().copied().map(String::from).collect(),
        ..draft(space_id)
    }
}

fn written_in(space_id: &str, language: Language) -> NoteDraft {
    NoteDraft {
        language,
        ..draft(space_id)
    }
}
