use std::collections::BTreeMap;

use chrono::Utc;
use engine::error::ErrorCode;
use engine::notes::checklist::ChecklistItem;
use engine::notes::kind::NoteKind;
use engine::notes::model::{NoteDraft, NotePatch, SampleNote};
use engine::notes::priority::Priority;
use engine::notes::revision::DiffLine;
use engine::notes::trash::TrashedNote;
use engine::notes::view::{Grouping, NoteFilter, NoteOrder, NotesQuery};
use engine::notes::{
    convert_note, count_notes_tagged, create_note, delete_note, delete_notes, delete_tags,
    duplicate_note, empty_trash, field_spans, fill_placeholders, get_note,
    list_global_placeholders, list_revisions, list_tags, list_trash, move_notes, move_notes_back,
    purge_notes, query_notes, remove_tag, rename_tags, restore_notes, restore_priorities,
    restore_revision, restore_tags, restore_trash, revision_diff, seed_samples,
    set_global_placeholders, set_pinned, set_placeholder_values, set_priority, tag_notes,
    untag_notes, update_note,
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
        kinds: Vec::new(),
        priorities: Vec::new(),
        order: NoteOrder::default(),
        grouping: Grouping::default(),
        ranked: false,
        now: Utc::now(),
        tz_offset_minutes: 0,
        pinned_first: true,
    }
}

fn note(session: &Session, draft: NoteDraft) -> String {
    session.call(|e| create_note(e, draft)).unwrap().note.id
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
        code(session.call(|e| query_notes(e, everything("")))),
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
    let updated = session.call(|e| update_note(e, id.clone(), patch)).unwrap();
    let read = session.call(|e| get_note(e, id.clone())).unwrap();
    let found = session
        .call(|e| query_notes(e, everything("prod")))
        .unwrap();

    assert_eq!(updated.note.title, "psql to prod");
    assert_eq!(read.note.title, "psql to prod");
    assert_eq!(found.matched, 1);
    assert_eq!(found.sections[0].cards[0].id, id);
}

#[test]
fn a_note_that_does_not_exist_is_named_as_such() {
    let session = Session::open();

    assert!(matches!(
        code(session.call(|e| get_note(e, "missing".to_string()))),
        ErrorCode::NoteNotFound
    ));
}

#[test]
fn a_duplicate_takes_the_title_it_is_given() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = note(&session, snippet(&space));

    let copy = session
        .call(|e| duplicate_note(e, id.clone(), "Titre (copie)".to_string()))
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
        .call(|e| {
            seed_samples(
                e,
                "Samples".to_string(),
                vec!["Scripts".to_string(), "Notes".to_string()],
                vec![sample],
            )
        })
        .unwrap();
    let seeded = session.call(|e| query_notes(e, everything(""))).unwrap();

    assert_eq!(session.spaces(), ["Samples"]);
    assert_eq!(seeded.matched, 1);
    assert_eq!(seeded.sections[0].cards[0].space_id, space.id);
    assert_eq!(
        seeded
            .display(&seeded.sections[0].cards[0])
            .folder
            .unwrap()
            .name,
        "Notes"
    );
}

/// Checked before the lock: a blank name never reaches the transaction.
#[test]
fn samples_under_a_blank_name_are_refused() {
    let session = Session::open();

    assert!(matches!(
        code(session.call(|e| seed_samples(e, " ".to_string(), Vec::new(), Vec::new()))),
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
    session.call(|e| update_note(e, id.clone(), edit)).unwrap();

    let kept = session.call(|e| list_revisions(e, id.clone())).unwrap();
    let revision = kept[0].id.clone();
    let diff = session
        .call(|e| revision_diff(e, id.clone(), revision.clone()))
        .unwrap();
    let restored = session
        .call(|e| restore_revision(e, id.clone(), revision))
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
        code(session.call(|e| revision_diff(e, id, "missing".to_string()))),
        ErrorCode::RevisionNotFound
    ));
}

#[test]
fn the_trash_takes_notes_gives_them_back_and_forgets_them() {
    let session = Session::open();
    let space = session.space("Personal");
    let first = note(&session, snippet(&space));
    let second = note(&session, snippet(&space));

    session.call(|e| delete_note(e, first.clone())).unwrap();
    let trashed = session.call(|e| list_trash(e, String::new())).unwrap();
    let restored = session.call(|e| restore_notes(e, ids(&first))).unwrap();

    let deleted = session
        .call(|e| delete_notes(e, vec![first.clone(), second.clone()]))
        .unwrap();
    let purged = session.call(|e| purge_notes(e, ids(&first))).unwrap();
    let emptied = session.call(empty_trash).unwrap();

    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].note.id, first);
    assert_eq!((restored, deleted, purged, emptied), (1, 2, 1, 1));
    assert!(
        session
            .call(|e| list_trash(e, String::new()))
            .unwrap()
            .is_empty()
    );
}

/// Found by a word the canvas search would find, folded; what was found is what comes back.
#[test]
fn the_trash_is_searched_and_what_it_found_restored() {
    let session = Session::open();
    let space = session.space("Personal");
    let mut titled = snippet(&space);
    titled.title = "Rotation du certificat".to_string();
    let found = note(&session, titled);
    let other = note(&session, snippet(&space));
    session
        .call(|e| delete_notes(e, vec![found.clone(), other.clone()]))
        .unwrap();

    let narrowed = session
        .call(|e| list_trash(e, " CERTIFICAT ".to_string()))
        .unwrap();
    let restored = session
        .call(|e| restore_trash(e, "certificat".to_string()))
        .unwrap();
    let left = session.call(|e| list_trash(e, String::new())).unwrap();
    let rest = session.call(|e| restore_trash(e, String::new())).unwrap();

    let ids = |notes: &[TrashedNote]| {
        notes
            .iter()
            .map(|note| note.note.id.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&narrowed), [found]);
    assert_eq!(restored, 1);
    assert_eq!(ids(&left), [other]);
    assert_eq!(rest, 1);
}

#[test]
fn a_move_is_undone_by_the_placements_it_answered() {
    let session = Session::open();
    let personal = session.space("Personal");
    let work = session.space("Work");
    let id = note(&session, snippet(&personal));

    let placements = session
        .call(|e| move_notes(e, ids(&id), work.clone()))
        .unwrap();
    let moved = session.call(|e| get_note(e, id.clone())).unwrap();
    let undone = session.call(|e| move_notes_back(e, placements)).unwrap();
    let back = session.call(|e| get_note(e, id.clone())).unwrap();

    assert_eq!(moved.note.space_id, work);
    assert_eq!(undone, 1);
    assert_eq!(back.note.space_id, personal);
}

#[test]
fn a_priority_is_set_and_put_back_without_touching_the_modification_date() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = note(&session, snippet(&space));
    let before = session.call(|e| get_note(e, id.clone())).unwrap();

    let previous = session
        .call(|e| set_priority(e, ids(&id), Priority::Urgent))
        .unwrap();
    let set = session.call(|e| get_note(e, id.clone())).unwrap();
    let restored = session
        .call(|e| restore_priorities(e, previous.clone()))
        .unwrap();
    let back = session.call(|e| get_note(e, id.clone())).unwrap();

    assert_eq!(previous.len(), 1);
    assert_eq!(set.note.priority, Priority::Urgent);
    assert_eq!(set.note.updated_at, before.note.updated_at);
    assert_eq!(restored, 1);
    assert_eq!(back.note.priority, Priority::None);
}

/// The body before a conversion is in the history; writing that body back is the undo.
#[test]
fn a_note_changes_kind_keeps_its_body_in_the_history_and_comes_back() {
    let session = Session::open();
    let space = session.space("Personal");
    let mut draft = snippet(&space);
    draft.content = "- [x] Freeze the merges\nDeploy".to_string();
    let original = draft.content.clone();
    let id = note(&session, draft);

    let converted = session
        .call(|e| convert_note(e, id.clone(), NoteKind::Checklist))
        .unwrap();
    let history = session.call(|e| list_revisions(e, id.clone())).unwrap();
    let previous = converted.previous.clone();
    let back = session
        .call(|e| {
            update_note(
                e,
                id.clone(),
                NotePatch {
                    kind: Some(previous.kind),
                    language: Some(previous.language),
                    content: Some(previous.content.clone()),
                    items: Some(previous.items.clone()),
                    ..NotePatch::default()
                },
            )
        })
        .unwrap();
    let unchanged = session
        .call(|e| convert_note(e, id.clone(), NoteKind::Snippet))
        .unwrap();

    assert_eq!(converted.note.note.kind, NoteKind::Checklist);
    assert_eq!(converted.note.note.content, "");
    assert_eq!(
        converted.note.note.items,
        [
            ChecklistItem {
                text: "Freeze the merges".to_string(),
                done: true
            },
            ChecklistItem {
                text: "Deploy".to_string(),
                done: false
            },
        ]
    );
    assert_eq!(history.len(), 1);
    assert_eq!(previous.kind, NoteKind::Snippet);
    assert_eq!(back.note.kind, NoteKind::Snippet);
    assert_eq!(back.note.content, original);
    assert!(back.note.items.is_empty());
    assert_eq!(unchanged.previous.content, original);
}

/// Taken off the live notes that carry it, found as typed; put back pair by pair.
#[test]
fn a_tag_is_taken_off_a_selection_and_put_back_exactly() {
    let session = Session::open();
    let space = session.space("Personal");
    let tagged = |tags: &[&str]| {
        let mut draft = snippet(&space);
        draft.tags = tags.iter().map(ToString::to_string).collect();
        note(&session, draft)
    };
    let both = tagged(&["ops", "urgent"]);
    let ops = tagged(&["ops"]);
    let bare = tagged(&[]);
    let trashed = tagged(&["ops"]);
    session.call(|e| delete_note(e, trashed.clone())).unwrap();
    let tags_of = |id: &String| session.call(|e| get_note(e, id.clone())).unwrap().note.tags;

    let mut removed = session
        .call(|e| {
            remove_tag(
                e,
                vec![both.clone(), ops.clone(), bare.clone(), trashed.clone()],
                " #OPS ".to_string(),
            )
        })
        .unwrap();
    removed.sort_by(|a, b| a.note_id.cmp(&b.note_id));
    let after = (tags_of(&both), tags_of(&ops), tags_of(&bare));
    let restored = session.call(|e| restore_tags(e, removed.clone())).unwrap();

    let mut expected = vec![both.clone(), ops.clone()];
    expected.sort();
    assert_eq!(
        removed
            .iter()
            .map(|pair| pair.note_id.clone())
            .collect::<Vec<_>>(),
        expected
    );
    assert!(removed.iter().all(|pair| pair.tag == "ops"));
    assert_eq!(after, (vec!["urgent".to_string()], vec![], vec![]));
    assert_eq!(restored, 2);
    assert_eq!(tags_of(&both), ["ops", "urgent"]);
    assert_eq!(tags_of(&ops), ["ops"]);
    assert!(
        session
            .call(|e| remove_tag(e, ids(&both), "#".to_string()))
            .unwrap()
            .is_empty()
    );
}

/// Triage, like the priority: the modification date stays, and the undo is the other way round.
#[test]
fn notes_are_pinned_together_and_unpinned_back_without_touching_the_modification_date() {
    let session = Session::open();
    let space = session.space("Personal");
    let loose = note(&session, snippet(&space));
    let mut held = snippet(&space);
    held.pinned = true;
    let held = note(&session, held);
    let before = session.call(|e| get_note(e, loose.clone())).unwrap();

    let changed = session
        .call(|e| set_pinned(e, vec![loose.clone(), held.clone()], true))
        .unwrap();
    let pinned = session.call(|e| get_note(e, loose.clone())).unwrap();
    let undone = session
        .call(|e| set_pinned(e, changed.clone(), false))
        .unwrap();
    let pinned_after = |id: &String| {
        session
            .call(|e| get_note(e, id.clone()))
            .unwrap()
            .note
            .pinned
    };

    assert_eq!(changed, std::slice::from_ref(&loose));
    assert!(pinned.note.pinned);
    assert_eq!(pinned.note.updated_at, before.note.updated_at);
    assert_eq!(undone, std::slice::from_ref(&loose));
    assert!(!pinned_after(&loose));
    assert!(pinned_after(&held));
}

/// The editor marks what Rust calls a field, and nothing it does not.
#[test]
fn the_fields_of_a_text_are_found_for_the_editor() {
    let spans = field_spans("{{host}} {{ user.name }}".to_string());

    assert_eq!(spans.len(), 1);
    assert_eq!((spans[0].from, spans[0].to), (0, 8));
}

/// A typed `#urgent` joins `urgent`: the command normalises before the store sees it.
#[test]
fn tags_are_normalised_counted_renamed_and_dropped() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = note(&session, snippet(&space));

    let added = session
        .call(|e| tag_notes(e, ids(&id), vec!["#urgent".to_string()]))
        .unwrap();
    let counted = session
        .call(|e| count_notes_tagged(e, vec!["urgent".to_string()]))
        .unwrap();
    let usage = session.call(list_tags).unwrap();
    let untagged = session.call(|e| untag_notes(e, added.clone())).unwrap();

    session
        .call(|e| tag_notes(e, ids(&id), vec!["urgent".to_string()]))
        .unwrap();
    let renamed = session
        .call(|e| rename_tags(e, vec!["urgent".to_string()], "later".to_string()))
        .unwrap();
    let dropped = session
        .call(|e| delete_tags(e, vec!["later".to_string()]))
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
        code(session.call(|e| rename_tags(e, vec!["urgent".to_string()], "#".to_string()))),
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
        .call(|e| set_placeholder_values(e, id.clone(), values(&[("host", "db")])))
        .unwrap();
    let globals = session
        .call(|e| set_global_placeholders(e, values(&[("user", "admin")])))
        .unwrap();
    let listed = session.call(list_global_placeholders).unwrap();
    let text = session
        .call(|e| {
            fill_placeholders(
                e,
                filled.note.content.clone(),
                filled.note.placeholder_values.clone(),
            )
        })
        .unwrap();

    assert_eq!(filled.note.placeholder_values, values(&[("host", "db")]));
    assert_eq!(globals, listed);
    assert_eq!(text, "psql -h db -U admin");
}
