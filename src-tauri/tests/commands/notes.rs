use std::collections::BTreeMap;

use chrono::Utc;
use devnotes_lib::error::ErrorCode;
use devnotes_lib::notes::model::{NoteDraft, NotePatch, SampleNote};
use devnotes_lib::notes::revision::DiffLine;
use devnotes_lib::notes::view::{NoteFilter, NotesQuery};
use devnotes_lib::notes::{
    count_notes_tagged, create_note, delete_note, delete_notes, delete_tags, duplicate_note,
    empty_trash, fill_placeholders, get_note, list_global_placeholders, list_revisions, list_tags,
    list_trash, move_notes, move_notes_back, purge_notes, query_notes, rename_tags, restore_notes,
    restore_revision, revision_diff, seed_samples, set_global_placeholders, set_placeholder_values,
    tag_notes, untag_notes, update_note,
};

use super::common::snippet;
use super::{Session, code};

fn everything(search: &str) -> NotesQuery {
    NotesQuery {
        space_id: None,
        folder_id: None,
        search: search.to_string(),
        filter: NoteFilter::All,
        tags: Vec::new(),
        languages: Vec::new(),
        now: Utc::now(),
        tz_offset_minutes: 0,
        pinned_first: true,
    }
}

fn note(session: &Session, draft: NoteDraft) -> String {
    session.call(|app| create_note(draft, app)).unwrap().note.id
}

fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect()
}

fn ids(of: &str) -> Vec<String> {
    vec![of.to_string()]
}

/// The front end never builds the outlet before the unlock; a command that raced it answers
/// rather than unwrapping an empty `Mutex`.
#[test]
fn a_command_run_before_the_unlock_answers_locked() {
    let session = Session::locked();

    assert!(matches!(
        code(session.call(|app| query_notes(everything(""), app))),
        ErrorCode::Locked
    ));
}

#[test]
fn a_note_written_through_the_commands_reads_back_and_is_found() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = note(&session, snippet(&space));

    let patch = NotePatch {
        title: Some("psql to prod".to_string()),
        ..Default::default()
    };
    let updated = session
        .call(|app| update_note(id.clone(), patch, app))
        .unwrap();
    let read = session.call(|app| get_note(id.clone(), app)).unwrap();
    let found = session
        .call(|app| query_notes(everything("prod"), app))
        .unwrap();

    assert_eq!(updated.note.title, "psql to prod");
    assert_eq!(read.note.title, "psql to prod");
    assert_eq!(found.matched, 1);
    assert_eq!(found.sections[0].notes[0].note.id, id);
}

#[test]
fn a_note_that_does_not_exist_is_named_as_such() {
    let session = Session::open();

    assert!(matches!(
        code(session.call(|app| get_note("missing".to_string(), app))),
        ErrorCode::NoteNotFound
    ));
}

#[test]
fn a_duplicate_takes_the_title_it_is_given() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = note(&session, snippet(&space));

    let copy = session
        .call(|app| duplicate_note(id.clone(), "Titre (copie)".to_string(), app))
        .unwrap();

    assert_ne!(copy.note.id, id);
    assert_eq!(copy.note.title, "Titre (copie)");
    assert_eq!(copy.note.content, "Contenu");
}

#[test]
fn the_samples_arrive_in_one_space_with_their_folders() {
    let session = Session::open();
    let sample = SampleNote {
        folder: Some(1),
        draft: snippet(""),
    };

    let space = session
        .call(|app| {
            seed_samples(
                "Samples".to_string(),
                vec!["Scripts".to_string(), "Notes".to_string()],
                vec![sample],
                app,
            )
        })
        .unwrap();
    let seeded = session
        .call(|app| query_notes(everything(""), app))
        .unwrap();

    assert_eq!(session.spaces(), ["Samples"]);
    assert_eq!(seeded.matched, 1);
    assert_eq!(seeded.sections[0].notes[0].note.space_id, space.id);
    assert_eq!(
        seeded.sections[0].notes[0].folder.as_ref().unwrap().name,
        "Notes"
    );
}

/// Checked before the lock: a blank name never reaches the transaction.
#[test]
fn samples_under_a_blank_name_are_refused() {
    let session = Session::open();

    assert!(matches!(
        code(session.call(|app| seed_samples(" ".to_string(), Vec::new(), Vec::new(), app))),
        ErrorCode::InvalidInput
    ));
    assert!(session.spaces().is_empty());
}

#[test]
fn a_kept_body_is_listed_compared_and_put_back() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = note(&session, snippet(&space));
    let edit = NotePatch {
        content: Some("psql -h prod".to_string()),
        ..Default::default()
    };
    session
        .call(|app| update_note(id.clone(), edit, app))
        .unwrap();

    let kept = session.call(|app| list_revisions(id.clone(), app)).unwrap();
    let revision = kept[0].id.clone();
    let diff = session
        .call(|app| revision_diff(id.clone(), revision.clone(), app))
        .unwrap();
    let restored = session
        .call(|app| restore_revision(id.clone(), revision, app))
        .unwrap();

    assert_eq!(kept.len(), 1);
    assert!(diff.contains(&DiffLine::Restored {
        text: "Contenu".to_string()
    }));
    assert_eq!(restored.note.content, "Contenu");
}

#[test]
fn comparing_with_a_revision_that_is_not_kept_is_named_as_such() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = note(&session, snippet(&space));

    assert!(matches!(
        code(session.call(|app| revision_diff(id, "missing".to_string(), app))),
        ErrorCode::RevisionNotFound
    ));
}

#[test]
fn the_trash_takes_notes_gives_them_back_and_forgets_them() {
    let session = Session::open();
    let space = session.space("Personal");
    let first = note(&session, snippet(&space));
    let second = note(&session, snippet(&space));

    session.call(|app| delete_note(first.clone(), app)).unwrap();
    let trashed = session.call(list_trash).unwrap();
    let restored = session.call(|app| restore_notes(ids(&first), app)).unwrap();

    let deleted = session
        .call(|app| delete_notes(vec![first.clone(), second.clone()], app))
        .unwrap();
    let purged = session.call(|app| purge_notes(ids(&first), app)).unwrap();
    let emptied = session.call(empty_trash).unwrap();

    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].note.id, first);
    assert_eq!((restored, deleted, purged, emptied), (1, 2, 1, 1));
    assert!(session.call(list_trash).unwrap().is_empty());
}

#[test]
fn a_move_is_undone_by_the_placements_it_answered() {
    let session = Session::open();
    let personal = session.space("Personal");
    let work = session.space("Work");
    let id = note(&session, snippet(&personal));

    let placements = session
        .call(|app| move_notes(ids(&id), work.clone(), app))
        .unwrap();
    let moved = session.call(|app| get_note(id.clone(), app)).unwrap();
    let undone = session
        .call(|app| move_notes_back(placements, app))
        .unwrap();
    let back = session.call(|app| get_note(id.clone(), app)).unwrap();

    assert_eq!(moved.note.space_id, work);
    assert_eq!(undone, 1);
    assert_eq!(back.note.space_id, personal);
}

/// A typed `#urgent` joins `urgent`: the command normalises before the store sees it.
#[test]
fn tags_are_normalised_counted_renamed_and_dropped() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = note(&session, snippet(&space));

    let added = session
        .call(|app| tag_notes(ids(&id), vec!["#urgent".to_string()], app))
        .unwrap();
    let counted = session
        .call(|app| count_notes_tagged(vec!["urgent".to_string()], app))
        .unwrap();
    let usage = session.call(list_tags).unwrap();
    let untagged = session.call(|app| untag_notes(added.clone(), app)).unwrap();

    session
        .call(|app| tag_notes(ids(&id), vec!["urgent".to_string()], app))
        .unwrap();
    let renamed = session
        .call(|app| rename_tags(vec!["urgent".to_string()], "later".to_string(), app))
        .unwrap();
    let dropped = session
        .call(|app| delete_tags(vec!["later".to_string()], app))
        .unwrap();

    assert_eq!(added[0].tag, "urgent");
    assert_eq!(counted, 1);
    assert_eq!((usage[0].tag.as_str(), usage[0].note_count), ("urgent", 1));
    assert_eq!((untagged, renamed, dropped), (1, 1, 1));
    assert!(session.call(list_tags).unwrap().is_empty());
}

#[test]
fn renaming_onto_a_blank_tag_is_refused() {
    let session = Session::open();

    assert!(matches!(
        code(session.call(|app| rename_tags(vec!["urgent".to_string()], "#".to_string(), app))),
        ErrorCode::InvalidInput
    ));
}

/// A note's own value first, then the global one: the palette fills with both.
#[test]
fn fields_fill_from_the_note_then_from_the_globals() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = note(
        &session,
        NoteDraft {
            content: "psql -h {{host}} -U {{user}}".to_string(),
            ..snippet(&space)
        },
    );

    let filled = session
        .call(|app| set_placeholder_values(id.clone(), values(&[("host", "db")]), app))
        .unwrap();
    let globals = session
        .call(|app| set_global_placeholders(values(&[("user", "admin")]), app))
        .unwrap();
    let listed = session.call(list_global_placeholders).unwrap();
    let text = session
        .call(|app| {
            fill_placeholders(
                filled.note.content.clone(),
                filled.note.placeholder_values.clone(),
                app,
            )
        })
        .unwrap();

    assert_eq!(filled.note.placeholder_values, values(&[("host", "db")]));
    assert_eq!(globals, listed);
    assert_eq!(text, "psql -h db -U admin");
}
