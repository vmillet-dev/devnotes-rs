use devnotes_lib::error::ErrorCode;
use devnotes_lib::folders::create_folder;
use devnotes_lib::folders::model::FolderDraft;
use devnotes_lib::notes::{create_note, get_note};
use devnotes_lib::spaces::model::SpaceContents;
use devnotes_lib::spaces::model::SpaceDraft;
use devnotes_lib::spaces::{
    count_space_contents, create_space, delete_space, list_spaces, pin_space, rename_space,
};

use super::common::snippet;
use super::{Session, code};

fn named(name: &str) -> SpaceDraft {
    SpaceDraft {
        name: name.to_string(),
    }
}

/// Pinned first, then by name: the one order a space has.
#[test]
fn a_space_is_renamed_and_pinned_to_the_head_of_the_list() {
    let session = Session::open();
    session.space("Archive");
    let id = session.space("Personal");

    let renamed = session
        .call(|app| rename_space(id.clone(), named("Work"), app))
        .unwrap();
    let pinned = session
        .call(|app| pin_space(id.clone(), true, app))
        .unwrap();
    let listed = session.call(list_spaces).unwrap();

    assert_eq!(renamed.name, "Work");
    assert!(pinned.pinned);
    assert_eq!(listed[0].id, id);
}

#[test]
fn a_second_space_of_the_same_name_is_refused() {
    let session = Session::open();
    session.space("Personal");

    assert!(matches!(
        code(session.call(|app| create_space(named("personal"), app))),
        ErrorCode::DuplicateSpaceName
    ));
}

#[test]
fn a_space_without_a_readable_name_is_refused() {
    let session = Session::open();

    assert!(matches!(
        code(session.call(|app| create_space(named(" "), app))),
        ErrorCode::InvalidInput
    ));
}

#[test]
fn a_deleted_space_hands_its_notes_to_its_refuge() {
    let session = Session::open();
    let doomed = session.space("Personal");
    let refuge = session.space("Work");
    let note = session
        .call(|app| create_note(snippet(&doomed), app))
        .unwrap()
        .note
        .id;

    session
        .call(|app| delete_space(doomed.clone(), refuge.clone(), app))
        .unwrap();
    let moved = session.call(|app| get_note(note, app)).unwrap();

    assert_eq!(session.spaces(), ["Work"]);
    assert_eq!(moved.note.space_id, refuge);
}

/// Checked before the lock: the cascade would take the notes the moment they arrived.
#[test]
fn a_space_cannot_be_its_own_refuge() {
    let session = Session::open();
    let id = session.space("Personal");

    assert!(matches!(
        code(session.call(|app| delete_space(id.clone(), id, app))),
        ErrorCode::InvalidInput
    ));
    assert_eq!(session.spaces(), ["Personal"]);
}

/// What the confirmation names before a deletion, counted under the lock.
#[test]
fn a_space_counts_what_its_deletion_would_move_and_take() {
    let session = Session::open();
    let id = session.space("Personal");
    session.call(|app| create_note(snippet(&id), app)).unwrap();
    session
        .call(|app| {
            create_folder(
                FolderDraft {
                    space_id: id.clone(),
                    parent_id: None,
                    name: "Perf".to_string(),
                },
                app,
            )
        })
        .unwrap();

    assert_eq!(
        session
            .call(|app| count_space_contents(id.clone(), app))
            .unwrap(),
        SpaceContents {
            notes: 1,
            folders: 1
        }
    );
    assert!(matches!(
        code(session.call(|app| count_space_contents("ghost".to_string(), app))),
        ErrorCode::SpaceNotFound
    ));
}
