use engine::error::ErrorCode;
use engine::folders::create_folder;
use engine::folders::model::FolderDraft;
use engine::notes::{create_note, get_note};
use engine::spaces::model::SpaceContents;
use engine::spaces::model::SpaceDraft;
use engine::spaces::{
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
        .call(|e| rename_space(e, id.clone(), named("Work")))
        .unwrap();
    let pinned = session.call(|e| pin_space(e, id.clone(), true)).unwrap();
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
        code(session.call(|e| create_space(e, named("personal")))),
        ErrorCode::DuplicateSpaceName
    ));
}

#[test]
fn a_space_without_a_readable_name_is_refused() {
    let session = Session::open();

    assert!(matches!(
        code(session.call(|e| create_space(e, named(" ")))),
        ErrorCode::InvalidInput
    ));
}

#[test]
fn a_deleted_space_hands_its_notes_to_its_refuge() {
    let session = Session::open();
    let doomed = session.space("Personal");
    let refuge = session.space("Work");
    let note = session
        .call(|e| create_note(e, snippet(&doomed)))
        .unwrap()
        .note
        .id;

    session
        .call(|e| delete_space(e, doomed.clone(), refuge.clone()))
        .unwrap();
    let moved = session.call(|e| get_note(e, note)).unwrap();

    assert_eq!(session.spaces(), ["Work"]);
    assert_eq!(moved.note.space_id, refuge);
}

/// Checked before the lock: the cascade would take the notes the moment they arrived.
#[test]
fn a_space_cannot_be_its_own_refuge() {
    let session = Session::open();
    let id = session.space("Personal");

    assert!(matches!(
        code(session.call(|e| delete_space(e, id.clone(), id))),
        ErrorCode::InvalidInput
    ));
    assert_eq!(session.spaces(), ["Personal"]);
}

/// What the confirmation names before a deletion, counted under the lock.
#[test]
fn a_space_counts_what_its_deletion_would_move_and_take() {
    let session = Session::open();
    let id = session.space("Personal");
    session.call(|e| create_note(e, snippet(&id))).unwrap();
    session
        .call(|e| {
            create_folder(
                e,
                FolderDraft {
                    space_id: id.clone(),
                    parent_id: None,
                    name: "Perf".to_string(),
                },
            )
        })
        .unwrap();

    assert_eq!(
        session
            .call(|e| count_space_contents(e, id.clone()))
            .unwrap(),
        SpaceContents {
            notes: 1,
            folders: 1
        }
    );
    assert!(matches!(
        code(session.call(|e| count_space_contents(e, "ghost".to_string()))),
        ErrorCode::SpaceNotFound
    ));
}
