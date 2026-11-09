use chrono::Utc;
use engine::error::ErrorCode;
use engine::folders::board::{
    BoardFrame, BoardQuery, BoardScope, MIN_ZONE_HEIGHT, MIN_ZONE_WIDTH, PreviewLines,
    ZonePlacement,
};
use engine::folders::model::{FolderColour, FolderDraft};
use engine::folders::{
    arrange_board, board_view, create_folder, delete_folder, file_notes, file_notes_back,
    list_folders, move_folder, move_folder_back, recolour_folder, rename_folder, save_board_layout,
};
use engine::notes::view::{Grouping, NoteFilter, NoteOrder, NotesQuery};
use engine::notes::{create_note, query_notes};

use super::common::snippet;
use super::{Session, code};

fn folder(session: &Session, space_id: &str, name: &str) -> String {
    let draft = FolderDraft {
        space_id: space_id.to_string(),
        parent_id: None,
        name: name.to_string(),
    };

    session.call(|e| create_folder(e, draft)).unwrap().id
}

fn board(space_id: &str) -> BoardQuery {
    BoardQuery {
        space_id: space_id.to_string(),
        root_id: None,
        search: String::new(),
        filter: NoteFilter::All,
        tags: Vec::new(),
        languages: Vec::new(),
        kinds: Vec::new(),
        priorities: Vec::new(),
        now: Utc::now(),
        preview_lines: PreviewLines::default(),
    }
}

#[test]
fn a_folder_is_created_renamed_recoloured_and_deleted() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = folder(&session, &space, "Scripts");

    let renamed = session
        .call(|e| rename_folder(e, id.clone(), "Shell".to_string()))
        .unwrap();
    let recoloured = session
        .call(|e| recolour_folder(e, id.clone(), FolderColour::Amber))
        .unwrap();
    let listed = session
        .call(|e| list_folders(e, Some(space.clone())))
        .unwrap();
    session
        .call(|e| delete_folder(e, id.clone(), PreviewLines::default()))
        .unwrap();

    assert_eq!(renamed.name, "Shell");
    assert_eq!(recoloured.colour, FolderColour::Amber);
    assert_eq!(listed.len(), 1);
    assert!(session.call(|e| list_folders(e, None)).unwrap().is_empty());
}

/// Checked before the lock, like every name.
#[test]
fn a_folder_without_a_readable_name_is_refused() {
    let session = Session::open();
    let space = session.space("Personal");
    let draft = FolderDraft {
        space_id: space,
        parent_id: None,
        name: "   ".to_string(),
    };

    assert!(matches!(
        code(session.call(|e| create_folder(e, draft))),
        ErrorCode::InvalidInput
    ));
}

#[test]
fn a_folder_that_does_not_exist_is_named_as_such() {
    let session = Session::open();

    assert!(matches!(
        code(session.call(|e| rename_folder(e, "missing".to_string(), "Shell".to_string()))),
        ErrorCode::FolderNotFound
    ));
}

#[test]
fn a_filing_is_undone_by_the_filings_it_answered() {
    let session = Session::open();
    let space = session.space("Personal");
    let zone = folder(&session, &space, "Scripts");
    let note = session
        .call(|e| create_note(e, snippet(&space)))
        .unwrap()
        .note
        .id;

    let filings = session
        .call(|e| {
            file_notes(
                e,
                vec![note.clone()],
                Some(zone.clone()),
                PreviewLines::default(),
            )
        })
        .unwrap();
    let filed = session.call(|e| board_view(e, board(&space))).unwrap();
    let undone = session
        .call(|e| file_notes_back(e, filings, PreviewLines::default()))
        .unwrap();
    let unfiled = session.call(|e| board_view(e, board(&space))).unwrap();

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
        .call(|e| save_board_layout(e, None, vec![placement], Vec::new()))
        .unwrap();
    let saved = session.call(|e| board_view(e, board(&space))).unwrap();
    let arranged = session
        .call(|e| {
            arrange_board(
                e,
                space.clone(),
                None,
                BoardScope::Everything,
                PreviewLines::default(),
            )
        })
        .unwrap();

    assert_eq!(saved.zones[0].frame, frame);
    assert_eq!(arranged.previous.zones[0].folder_id, zone);
    assert_eq!(arranged.previous.zones[0].frame, frame);
}

fn notes_query(space_id: &str) -> NotesQuery {
    NotesQuery {
        space_id: Some(space_id.to_string()),
        folder_id: None,
        search: String::new(),
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

fn sub_folder(session: &Session, space_id: &str, parent: &str, name: &str) -> String {
    let draft = FolderDraft {
        space_id: space_id.to_string(),
        parent_id: Some(parent.to_string()),
        name: name.to_string(),
    };

    session.call(|e| create_folder(e, draft)).unwrap().id
}

fn filed_note(session: &Session, space_id: &str, folder_id: &str) -> String {
    let id = session
        .call(|e| create_note(e, snippet(space_id)))
        .unwrap()
        .note
        .id;
    session
        .call(|e| {
            file_notes(
                e,
                vec![id.clone()],
                Some(folder_id.to_string()),
                PreviewLines::default(),
            )
        })
        .unwrap();
    id
}

/// On the board, a card names its folder only when it comes from below its zone, by the deepest
/// folder and its path; inside an opened folder, the same.
#[test]
fn a_card_names_the_sub_folder_it_comes_from_and_no_other() {
    let session = Session::open();
    let space = session.space("Personal");
    let sql = folder(&session, &space, "SQL");
    let perf = sub_folder(&session, &space, &sql, "Perf");
    let own = filed_note(&session, &space, &sql);
    let deeper = filed_note(&session, &space, &perf);

    let board = session.call(|e| board_view(e, board(&space))).unwrap();
    let chip = |id: &str| {
        board.zones[0]
            .notes
            .iter()
            .find(|entry| entry.note.id == id)
            .and_then(|entry| entry.note.folder.clone())
    };
    assert!(chip(&own).is_none());
    let named = chip(&deeper).unwrap();
    assert_eq!(named.name, "Perf");
    assert_eq!(named.path, ["SQL", "Perf"]);
    assert_eq!(board.zones[0].children[0].name, "Perf");

    let mut inside = notes_query(&space);
    inside.folder_id = Some(sql.clone());
    let view = session.call(|e| query_notes(e, inside)).unwrap();
    let chips: Vec<Option<String>> = view.sections[0]
        .cards
        .iter()
        .map(|card| view.display(card).folder.map(|folder| folder.name))
        .collect();
    assert_eq!(chips.len(), 2);
    assert!(chips.contains(&None) && chips.contains(&Some("Perf".to_string())));
}

#[test]
fn a_folder_moved_into_another_comes_back_with_its_undo() {
    let session = Session::open();
    let space = session.space("Personal");
    let sql = folder(&session, &space, "SQL");
    let perf = folder(&session, &space, "Perf");

    let was = session
        .call(|e| move_folder(e, perf.clone(), Some(sql.clone())))
        .unwrap();
    let nested = session.call(|e| board_view(e, board(&space))).unwrap();
    let refused = session.call(|e| move_folder(e, sql.clone(), Some(perf.clone())));
    let back = session.call(|e| move_folder_back(e, was)).unwrap();
    let flat = session.call(|e| board_view(e, board(&space))).unwrap();

    assert_eq!(nested.zones.len(), 1);
    assert_eq!(nested.zones[0].children[0].id, perf);
    assert!(matches!(code(refused), ErrorCode::InvalidInput));
    assert!(back);
    assert_eq!(flat.zones.len(), 2);
}
