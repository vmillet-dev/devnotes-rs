use engine::attachments::{attach_file, list_attachments};
use engine::error::ErrorCode;
use engine::folders::create_folder;
use engine::folders::model::FolderDraft;
use engine::notes::model::HeldNotes;
use engine::notes::{create_note, get_note, list_trash, restore_notes};
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
        .call(|e| rename_space(e, &id, &named("Work")))
        .unwrap();
    let pinned = session.call(|e| pin_space(e, &id, true)).unwrap();
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
        code(session.call(|e| create_space(e, &named("personal")))),
        ErrorCode::DuplicateSpaceName
    ));
}

#[test]
fn a_space_without_a_readable_name_is_refused() {
    let session = Session::open();

    assert!(matches!(
        code(session.call(|e| create_space(e, &named(" ")))),
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
        .call(|e| delete_space(e, &doomed, &refuge, HeldNotes::Keep))
        .unwrap();
    let moved = session.call(|e| get_note(e, &note)).unwrap();

    assert_eq!(session.spaces(), ["Work"]);
    assert_eq!(moved.note.space_id, refuge);
}

/// Checked before the lock: the cascade would take the notes the moment they arrived.
#[test]
fn a_space_cannot_be_its_own_refuge() {
    let session = Session::open();
    let id = session.space("Personal");

    assert!(matches!(
        code(session.call(|e| delete_space(e, &id, &id, HeldNotes::Keep))),
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
                &FolderDraft {
                    space_id: id.clone(),
                    parent_id: None,
                    name: "Perf".to_string(),
                },
            )
        })
        .unwrap();

    assert_eq!(
        session.call(|e| count_space_contents(e, &id)).unwrap(),
        SpaceContents {
            notes: 1,
            folders: 1
        }
    );
    assert!(matches!(
        code(session.call(|e| count_space_contents(e, "ghost"))),
        ErrorCode::SpaceNotFound
    ));
}

/// Nothing is erased: the notes wait in the trash, and come back into the refuge chosen.
#[test]
fn a_space_deleted_with_its_notes_trashes_them_for_the_refuge_to_take_back() {
    let scratch = tempfile::tempdir().unwrap();
    let session = Session::on_disk(scratch.path());
    let doomed = session.space("Personal");
    let refuge = session.space("Work");
    let shelf = session
        .call(|e| {
            let draft = FolderDraft {
                space_id: doomed.clone(),
                parent_id: None,
                name: "Shelf".to_string(),
            };
            create_folder(e, &draft)
        })
        .unwrap()
        .id;
    let doomed_note = session
        .call(|e| create_note(e, snippet(&doomed)))
        .unwrap()
        .note
        .id;
    session
        .call(|e| {
            let ids = [doomed_note.clone()];
            let lines = engine::folders::board::PreviewLines::default();
            engine::folders::file_notes(e, &ids, Some(&shelf), lines)
        })
        .unwrap();
    let kept = session
        .call(|e| create_note(e, snippet(&refuge)))
        .unwrap()
        .note
        .id;
    let source = scratch.path().join("capture.png");
    std::fs::write(&source, b"\x89PNG").unwrap();
    session
        .wait(|e| attach_file(e, doomed_note.clone(), source.to_string_lossy().to_string()))
        .unwrap();

    let moved = session
        .call(|e| delete_space(e, &doomed, &refuge, HeldNotes::Trash))
        .unwrap();

    assert_eq!(moved, 1);
    assert_eq!(session.spaces(), ["Work"]);
    let trash = session.call(|e| list_trash(e, "")).unwrap();
    let trashed: Vec<&str> = trash.iter().map(|row| row.note.id.as_str()).collect();
    assert_eq!(
        trashed,
        [doomed_note.as_str()],
        "the refuge's own notes stay"
    );
    assert!(session.call(|e| get_note(e, &kept)).is_ok());

    session
        .call(|e| restore_notes(e, std::slice::from_ref(&doomed_note)))
        .unwrap();
    let back = session.call(|e| get_note(e, &doomed_note)).unwrap();
    assert_eq!(back.note.space_id, refuge);
    assert_eq!(back.note.folder_id, None);
    assert_eq!(
        session
            .call(|e| list_attachments(e, &doomed_note))
            .unwrap()
            .len(),
        1
    );
}
