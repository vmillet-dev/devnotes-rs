use chrono::Utc;
use devnotes_lib::error::ErrorCode;
use devnotes_lib::folders::board::{
    BoardFrame, BoardQuery, BoardScope, MIN_ZONE_HEIGHT, MIN_ZONE_WIDTH, ZonePlacement,
};
use devnotes_lib::folders::model::{FolderColour, FolderDraft};
use devnotes_lib::folders::{
    arrange_board, board_view, create_folder, delete_folder, file_notes, file_notes_back,
    list_folders, recolour_folder, rename_folder, save_board_layout,
};
use devnotes_lib::notes::create_note;
use devnotes_lib::notes::view::NoteFilter;

use super::common::snippet;
use super::{Session, code};

fn folder(session: &Session, space_id: &str, name: &str) -> String {
    let draft = FolderDraft {
        space_id: space_id.to_string(),
        name: name.to_string(),
    };

    session.call(|app| create_folder(draft, app)).unwrap().id
}

fn board(space_id: &str) -> BoardQuery {
    BoardQuery {
        space_id: space_id.to_string(),
        search: String::new(),
        filter: NoteFilter::All,
        tags: Vec::new(),
        languages: Vec::new(),
        now: Utc::now(),
    }
}

#[test]
fn a_folder_is_created_renamed_recoloured_and_deleted() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = folder(&session, &space, "Scripts");

    let renamed = session
        .call(|app| rename_folder(id.clone(), "Shell".to_string(), app))
        .unwrap();
    let recoloured = session
        .call(|app| recolour_folder(id.clone(), FolderColour::Amber, app))
        .unwrap();
    let listed = session
        .call(|app| list_folders(Some(space.clone()), app))
        .unwrap();
    session.call(|app| delete_folder(id.clone(), app)).unwrap();

    assert_eq!(renamed.name, "Shell");
    assert_eq!(recoloured.colour, FolderColour::Amber);
    assert_eq!(listed.len(), 1);
    assert!(
        session
            .call(|app| list_folders(None, app))
            .unwrap()
            .is_empty()
    );
}

/// Checked before the lock, like every name.
#[test]
fn a_folder_without_a_readable_name_is_refused() {
    let session = Session::open();
    let space = session.space("Personal");
    let draft = FolderDraft {
        space_id: space,
        name: "   ".to_string(),
    };

    assert!(matches!(
        code(session.call(|app| create_folder(draft, app))),
        ErrorCode::InvalidInput
    ));
}

#[test]
fn a_folder_that_does_not_exist_is_named_as_such() {
    let session = Session::open();

    assert!(matches!(
        code(session.call(|app| rename_folder("missing".to_string(), "Shell".to_string(), app))),
        ErrorCode::FolderNotFound
    ));
}

#[test]
fn a_filing_is_undone_by_the_filings_it_answered() {
    let session = Session::open();
    let space = session.space("Personal");
    let zone = folder(&session, &space, "Scripts");
    let note = session
        .call(|app| create_note(snippet(&space), app))
        .unwrap()
        .note
        .id;

    let filings = session
        .call(|app| file_notes(vec![note.clone()], Some(zone.clone()), app))
        .unwrap();
    let filed = session.call(|app| board_view(board(&space), app)).unwrap();
    let undone = session.call(|app| file_notes_back(filings, app)).unwrap();
    let unfiled = session.call(|app| board_view(board(&space), app)).unwrap();

    assert_eq!(filed.zones[0].notes.len(), 1);
    assert!(filed.loose.is_empty());
    assert_eq!(undone, 1);
    assert!(unfiled.zones[0].notes.is_empty());
    assert_eq!(unfiled.loose.len(), 1);
}

/// A layout saved by hand is the one a tidy-up answers as `previous`, for its undo.
#[test]
fn a_saved_layout_is_what_a_tidy_up_hands_back() {
    let session = Session::open();
    let space = session.space("Personal");
    let zone = folder(&session, &space, "Scripts");
    let frame = BoardFrame {
        x: 960,
        y: 640,
        width: MIN_ZONE_WIDTH * 2,
        height: MIN_ZONE_HEIGHT * 2,
    };
    let placement = ZonePlacement {
        folder_id: zone.clone(),
        frame,
    };

    session
        .call(|app| save_board_layout(vec![placement], Vec::new(), app))
        .unwrap();
    let saved = session.call(|app| board_view(board(&space), app)).unwrap();
    let arranged = session
        .call(|app| arrange_board(space.clone(), BoardScope::Everything, app))
        .unwrap();

    assert_eq!(saved.zones[0].frame, frame);
    assert_eq!(arranged.previous.zones[0].folder_id, zone);
    assert_eq!(arranged.previous.zones[0].frame, frame);
}
