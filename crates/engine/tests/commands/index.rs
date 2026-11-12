//! Every write keeps the index true: right after each one, it holds what one built from the
//! database would — or nothing, for the next read to build.

use std::collections::BTreeMap;

use engine::Engine;
use engine::attachments::{attach_file, delete_attachment};
use engine::folders::model::FolderDraft;
use engine::folders::{create_folder, delete_folder, file_notes, move_folder, rename_folder};
use engine::notes::model::NotePatch;
use engine::notes::priority::Priority;
use engine::notes::{
    create_note, delete_notes, delete_tags, duplicate_note, empty_trash, move_notes,
    move_notes_back, remove_tag, rename_tags, restore_notes, restore_priorities, restore_tags,
    set_global_placeholders, set_pinned, set_placeholder_values, set_priority, tag_notes,
    untag_notes, update_note,
};
use engine::spaces::delete_space;

use super::Session;
use super::common::snippet;

/// A session whose index holds its library, and a note in a space.
fn indexed() -> (tempfile::TempDir, Session, String, String) {
    let scratch = tempfile::tempdir().unwrap();
    let session = Session::on_disk(scratch.path());
    let space = session.space("Personal");
    let note = session
        .call(|e| create_note(e, snippet(&space)))
        .unwrap()
        .note
        .id;
    session.wait(engine::index::fill);
    assert_eq!(session.engine.indexed(), 1);
    (scratch, session, space, note)
}

/// The write, then the check: a later write reading the same note again would hide a miss.
fn written<T, E: std::fmt::Debug>(
    session: &Session,
    write: impl FnOnce(&Engine) -> Result<T, E>,
) -> T {
    let answer = session.call(write).unwrap();
    session.call(Engine::index_matches_library).unwrap();
    answer
}

fn folder(name: &str, space: &str) -> FolderDraft {
    FolderDraft {
        space_id: space.to_string(),
        name: name.to_string(),
        parent_id: None,
    }
}

#[test]
fn a_note_written_edited_and_copied_is_read_again() {
    let (_scratch, session, space, note) = indexed();
    let patch = NotePatch {
        title: Some("psql to prod".to_string()),
        ..NotePatch::default()
    };
    let values = BTreeMap::from([("host".to_string(), "prod".to_string())]);

    written(&session, |e| create_note(e, snippet(&space)));
    written(&session, |e| update_note(e, note.clone(), patch));
    written(&session, |e| {
        duplicate_note(e, note.clone(), "Copie".to_string())
    });
    written(&session, |e| {
        set_placeholder_values(e, note.clone(), values)
    });

    assert_eq!(session.engine.indexed(), 3);
}

#[test]
fn a_batch_and_its_undo_are_read_again() {
    let (_scratch, session, _space, note) = indexed();
    let ids = vec![note];
    let other = session.space("Work");

    let tagged = written(&session, |e| {
        tag_notes(e, ids.clone(), vec!["auth".to_string()])
    });
    written(&session, |e| untag_notes(e, tagged));
    let removed = written(&session, |e| remove_tag(e, ids.clone(), "auth".to_string()));
    written(&session, |e| restore_tags(e, removed));
    written(&session, |e| set_pinned(e, ids.clone(), true));
    let previous = written(&session, |e| set_priority(e, ids.clone(), Priority::High));
    written(&session, |e| restore_priorities(e, previous));
    let placements = written(&session, |e| move_notes(e, ids.clone(), other.clone()));
    written(&session, |e| move_notes_back(e, placements));

    written(&session, |e| delete_notes(e, ids.clone()));
    assert_eq!(session.engine.indexed(), 0);
    written(&session, |e| restore_notes(e, ids.clone()));
    assert_eq!(session.engine.indexed(), 1);
    written(&session, |e| delete_notes(e, ids.clone()));
    written(&session, empty_trash);
}

#[test]
fn folders_attachments_and_global_values_are_read_again() {
    let (scratch, session, space, note) = indexed();
    let file = scratch.path().join("capture.png");
    std::fs::write(&file, b"\x89PNG").unwrap();
    let globals = BTreeMap::from([("env".to_string(), "staging".to_string())]);

    let sql = written(&session, |e| create_folder(e, folder("SQL", &space)));
    let prod = written(&session, |e| create_folder(e, folder("Prod", &space)));
    written(&session, |e| {
        file_notes(
            e,
            vec![note.clone()],
            Some(sql.id.clone()),
            engine::folders::board::PreviewLines::default(),
        )
    });
    written(&session, |e| {
        rename_folder(e, sql.id.clone(), "Postgres".to_string())
    });
    written(&session, |e| {
        move_folder(e, sql.id.clone(), Some(prod.id.clone()))
    });
    let attachment = written(&session, |e| {
        attach_file(e, note.clone(), file.to_string_lossy().to_string())
    });
    written(&session, |e| delete_attachment(e, attachment.id));
    written(&session, |e| set_global_placeholders(e, globals));
    written(&session, |e| {
        delete_folder(e, prod.id, engine::folders::board::PreviewLines::default())
    });
}

#[test]
fn a_corpus_wide_write_leaves_an_index_the_next_read_builds_whole() {
    let (_scratch, session, space, note) = indexed();
    written(&session, |e| {
        tag_notes(e, vec![note], vec!["auth".to_string()])
    });

    written(&session, |e| {
        rename_tags(e, vec!["auth".to_string()], "login".to_string())
    });
    session.wait(engine::index::fill);
    written(&session, |e| delete_tags(e, vec!["login".to_string()]));

    session.wait(engine::index::fill);
    let refuge = session.space("Refuge");
    written(&session, |e| delete_space(e, space, refuge));
}
