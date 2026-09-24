//! The instants, the space and the note every integration binary builds its scenarios on.

// Each binary compiles its own copy and uses part of it.
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use devnotes_lib::db::{Library, iso8601};
use devnotes_lib::notes::checklist::NoteKind;
use devnotes_lib::notes::language::Language;
use devnotes_lib::notes::model::{NoteDraft, NoteLifecycle};
use devnotes_lib::spaces::store as spaces;

pub(crate) fn at(iso: &str) -> DateTime<Utc> {
    iso8601::parse(iso).expect("tests write valid instants")
}

pub(crate) fn t0() -> DateTime<Utc> {
    at("2026-07-25T09:00:00.000Z")
}

pub(crate) fn t1() -> DateTime<Utc> {
    at("2026-07-25T10:00:00.000Z")
}

pub(crate) fn space(connection: &mut Library, name: &str) -> String {
    spaces::create(connection, name).unwrap().id
}

/// A plain snippet in `space_id`: a title, a body, and nothing a scenario did not ask for.
pub(crate) fn snippet(space_id: &str) -> NoteDraft {
    NoteDraft {
        space_id: space_id.to_string(),
        folder_id: None,
        title: "Titre".to_string(),
        language: Language::Txt,
        content: "Contenu".to_string(),
        source: String::new(),
        tags: Vec::new(),
        pinned: false,
        lifecycle: NoteLifecycle::Permanent,
        kind: NoteKind::Snippet,
        items: Vec::new(),
    }
}
