use chrono::Utc;
use engine::error::ErrorCode;
use engine::folders::board::{
    BoardFrame, BoardQuery, BoardScope, MIN_ZONE_HEIGHT, MIN_ZONE_WIDTH, PreviewLines,
    ZonePlacement,
};
use engine::folders::count_folder_contents;
use engine::folders::model::{FolderColour, FolderDraft};
use engine::folders::{
    arrange_board, board_view, create_folder, delete_folder, file_notes, file_notes_back,
    list_folders, move_folder, move_folder_back, move_notes_into, recolour_folder, rename_folder,
    save_board_layout,
};
use engine::notes::model::HeldNotes;
use engine::notes::view::{Grouping, NoteFilter, NoteOrder, NotesQuery};
use engine::notes::{
    create_note, delete_notes, get_note, list_trash, move_notes, move_notes_back, query_notes,
    restore_notes,
};

use super::common::snippet;
use super::{Session, code};

fn folder(session: &Session, space_id: &str, name: &str) -> String {
    let draft = FolderDraft {
        space_id: space_id.to_string(),
        parent_id: None,
        name: name.to_string(),
    };

    session.call(|e| create_folder(e, &draft)).unwrap().id
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
        tz_offset_minutes: 0,
        preview_lines: PreviewLines::default(),
    }
}

#[test]
fn a_folder_is_created_renamed_recoloured_and_deleted() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = folder(&session, &space, "Scripts");

    let renamed = session.call(|e| rename_folder(e, &id, "Shell")).unwrap();
    let recoloured = session
        .call(|e| recolour_folder(e, &id, FolderColour::Amber))
        .unwrap();
    let listed = session.call(|e| list_folders(e, Some(&space))).unwrap();
    session
        .call(|e| delete_folder(e, &id, PreviewLines::default(), HeldNotes::Keep))
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
        code(session.call(|e| create_folder(e, &draft))),
        ErrorCode::InvalidInput
    ));
}

#[test]
fn a_folder_that_does_not_exist_is_named_as_such() {
    let session = Session::open();

    assert!(matches!(
        code(session.call(|e| rename_folder(e, "missing", "Shell"))),
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
                std::slice::from_ref(&note),
                Some(&zone),
                PreviewLines::default(),
            )
        })
        .unwrap();
    let filed = session.call(|e| board_view(e, &board(&space))).unwrap();
    let undone = session
        .call(|e| file_notes_back(e, &filings, PreviewLines::default()))
        .unwrap();
    let unfiled = session.call(|e| board_view(e, &board(&space))).unwrap();

    assert_eq!(filed.zones[0].notes.len(), 1);
    assert!(filed.loose.is_empty());
    assert_eq!(undone, 1);
    assert!(unfiled.zones[0].notes.is_empty());
    assert_eq!(unfiled.loose.len(), 1);
}

#[test]
fn notes_of_two_spaces_go_into_one_folder_and_back_where_each_was() {
    let session = Session::open();
    let (home, work) = (session.space("Personal"), session.space("Work"));
    let (scripts, runbooks) = (
        folder(&session, &home, "Scripts"),
        folder(&session, &work, "Runbooks"),
    );
    let note = |space: &str| {
        session
            .call(|e| create_note(e, snippet(space)))
            .unwrap()
            .note
            .id
    };
    let (filed, there) = (note(&home), note(&work));
    let ids = [filed.clone(), there.clone()];
    let lines = PreviewLines::default();
    session
        .call(|e| file_notes(e, std::slice::from_ref(&filed), Some(&scripts), lines))
        .unwrap();
    let place = |id: &String| {
        let note = session.call(|e| get_note(e, id)).unwrap().note;
        (note.space_id, note.folder_id)
    };

    let moved = session
        .call(|e| move_notes_into(e, &ids, &runbooks, lines))
        .unwrap();
    assert_eq!(moved.len(), 2);
    for id in &ids {
        assert_eq!(place(id), (work.clone(), Some(runbooks.clone())));
    }
    let again = session.call(|e| move_notes_into(e, &ids, &runbooks, lines));
    assert!(again.unwrap().is_empty(), "already there");

    session.call(|e| move_notes_back(e, &moved)).unwrap();
    assert_eq!(place(&filed), (home.clone(), Some(scripts)));
    assert_eq!(place(&there), (work.clone(), None));
    assert!(matches!(
        code(session.call(|e| move_notes_into(e, &ids, "missing", lines))),
        ErrorCode::FolderNotFound
    ));

    // To the space it is in: out of its folder, and back in with the undo.
    let out = session.call(|e| move_notes(e, &ids, &home)).unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(place(&filed), (home.clone(), None));
    session.call(|e| move_notes_back(e, &out)).unwrap();
    assert!(place(&filed).1.is_some() && place(&there).1.is_none());
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
        .call(|e| save_board_layout(e, None, &[placement], &Vec::new()))
        .unwrap();
    let saved = session.call(|e| board_view(e, &board(&space))).unwrap();
    let arranged = session
        .call(|e| {
            arrange_board(
                e,
                &space,
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

    session.call(|e| create_folder(e, &draft)).unwrap().id
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
                std::slice::from_ref(&id),
                Some(&folder_id.to_string()),
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

    let board = session.call(|e| board_view(e, &board(&space))).unwrap();
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
    let view = session.call(|e| query_notes(e, &inside)).unwrap();
    let chips: Vec<Option<String>> = view.sections[0]
        .cards
        .iter()
        .map(|card| {
            view.display(card)
                .folder
                .as_ref()
                .map(|folder| folder.name.clone())
        })
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
        .call(|e| move_folder(e, &perf, (&space, Some(&sql))))
        .unwrap();
    let nested = session.call(|e| board_view(e, &board(&space))).unwrap();
    let refused = session.call(|e| move_folder(e, &sql, (&space, Some(&perf))));
    let back = session.call(|e| move_folder_back(e, &was)).unwrap();
    let flat = session.call(|e| board_view(e, &board(&space))).unwrap();

    assert_eq!(nested.zones.len(), 1);
    assert_eq!(nested.zones[0].children[0].id, perf);
    assert!(matches!(code(refused), ErrorCode::InvalidInput));
    assert!(back);
    assert_eq!(flat.zones.len(), 2);
}

/// Into another space, a folder takes its sub-folder and every note filed in it, the trashed one
/// included; its undo brings exactly that back.
#[test]
fn a_folder_moved_into_another_space_takes_its_tree_and_notes_and_comes_back() {
    let session = Session::open();
    let work = session.space("Work");
    let home = session.space("Home");
    let sql = folder(&session, &work, "SQL");
    let perf = sub_folder(&session, &work, &sql, "Perf");
    let filed = filed_note(&session, &work, &sql);
    let deeper = filed_note(&session, &work, &perf);
    let trashed = filed_note(&session, &work, &perf);
    let loose = session
        .call(|e| create_note(e, snippet(&work)))
        .unwrap()
        .note
        .id;
    session
        .call(|e| delete_notes(e, std::slice::from_ref(&trashed)))
        .unwrap();
    let placed = |id: &str| {
        let note = session.call(|e| get_note(e, id)).unwrap();
        (note.space_id.clone(), note.folder_id.clone())
    };

    let was = session
        .call(|e| move_folder(e, &sql, (&home, None)))
        .unwrap();

    assert_eq!(was.space_id, work);
    assert_eq!(was.notes.len(), 3);
    let folders = session.call(|e| list_folders(e, Some(&home))).unwrap();
    assert_eq!(folders.len(), 2);
    assert!(
        session
            .call(|e| list_folders(e, Some(&work)))
            .unwrap()
            .is_empty()
    );
    assert_eq!(placed(&filed), (home.clone(), Some(sql.clone())));
    assert_eq!(placed(&deeper), (home.clone(), Some(perf.clone())));
    assert_eq!(placed(&loose).0, work);
    let listed = |space: &str| {
        let view = session
            .call(|e| query_notes(e, &notes_query(space)))
            .unwrap();
        view.sections
            .iter()
            .map(|section| section.cards.len())
            .sum::<usize>()
    };
    assert_eq!(listed(&home), 2);
    session
        .call(|e| restore_notes(e, std::slice::from_ref(&trashed)))
        .unwrap();
    assert_eq!(placed(&trashed), (home.clone(), Some(perf.clone())));

    assert!(session.call(|e| move_folder_back(e, &was)).unwrap());
    assert_eq!(placed(&filed), (work.clone(), Some(sql.clone())));
    assert_eq!(placed(&trashed), (work.clone(), Some(perf.clone())));
    assert_eq!(
        session
            .call(|e| list_folders(e, Some(&work)))
            .unwrap()
            .len(),
        2
    );
    assert!(
        session
            .call(|e| list_folders(e, Some(&home)))
            .unwrap()
            .is_empty()
    );
}

/// Into another space, a name taken among the new siblings or a tree too deep there is refused,
/// and nothing moves.
#[test]
fn a_folder_that_cannot_go_into_another_space_moves_nothing() {
    let session = Session::open();
    let work = session.space("Work");
    let home = session.space("Home");
    let sql = folder(&session, &work, "SQL");
    let filed = filed_note(&session, &work, &sql);
    folder(&session, &home, "SQL");

    let taken = session.call(|e| move_folder(e, &sql, (&home, None)));

    assert!(matches!(code(taken), ErrorCode::DuplicateFolderName));
    let note = session.call(|e| get_note(e, &filed)).unwrap();
    assert_eq!(note.space_id, work);

    let mut parent = folder(&session, &home, "Deep");
    for level in 1..engine::folders::model::MAX_DEPTH {
        parent = sub_folder(&session, &home, &parent, &format!("L{level}"));
    }
    let deep = session.call(|e| move_folder(e, &sql, (&home, Some(&parent))));
    assert!(matches!(code(deep), ErrorCode::InvalidInput));
    assert_eq!(
        session
            .call(|e| list_folders(e, Some(&work)))
            .unwrap()
            .len(),
        1
    );
}

/// A sheet's `aujourd'hui` is the local day on the board as in the list: at 00:30 at UTC+2,
/// both read the same date (#1478, N16).
#[test]
fn the_board_reads_today_as_the_list_does() {
    let session = Session::open();
    let space = session.space("Personal");
    let draft = engine::notes::model::NoteDraft {
        kind: engine::notes::kind::NoteKind::Calc,
        content: "jour = aujourd'hui".to_string(),
        ..snippet(&space)
    };
    session.call(|e| create_note(e, draft)).unwrap();
    let now = chrono::DateTime::parse_from_rfc3339("2026-07-25T22:30:00Z")
        .unwrap()
        .with_timezone(&Utc);

    let listed = session
        .call(|e| {
            query_notes(
                e,
                &NotesQuery {
                    now,
                    tz_offset_minutes: -120,
                    ..notes_query(&space)
                },
            )
        })
        .unwrap();
    let on_board = session
        .call(|e| {
            board_view(
                e,
                &BoardQuery {
                    now,
                    tz_offset_minutes: -120,
                    ..board(&space)
                },
            )
        })
        .unwrap();

    let list_card = listed.display(listed.cards().next().unwrap()).calc.clone();
    let board_card = on_board.loose[0].note.calc.clone();
    assert_eq!(board_card, list_card);
}

fn nested(session: &Session, space_id: &str, parent: &str, name: &str) -> String {
    let draft = FolderDraft {
        space_id: space_id.to_string(),
        parent_id: Some(parent.to_string()),
        name: name.to_string(),
    };

    session.call(|e| create_folder(e, &draft)).unwrap().id
}

fn filed(session: &Session, space_id: &str, folder: &str) -> String {
    let id = session
        .call(|e| create_note(e, snippet(space_id)))
        .unwrap()
        .note
        .id;
    let ids = [id.clone()];
    let into = folder.to_string();
    session
        .call(|e| file_notes(e, &ids, Some(&into), PreviewLines::default()))
        .unwrap();
    id
}

/// What the panel names before the second click: every live note under it, and its sub-folders.
#[test]
fn a_folder_counts_what_is_under_it() {
    let session = Session::open();
    let space = session.space("Personal");
    let ops = folder(&session, &space, "Ops");
    let deep = nested(&session, &space, &ops, "Deep");
    filed(&session, &space, &ops);
    filed(&session, &space, &deep);

    let contents = session.call(|e| count_folder_contents(e, &ops)).unwrap();

    assert_eq!((contents.notes, contents.folders), (2, 1));
}

/// Its whole subtree goes, every note under it waits in the trash, and comes back loose.
#[test]
fn a_folder_deleted_with_its_notes_takes_its_sub_folders_and_trashes_them() {
    let session = Session::open();
    let space = session.space("Personal");
    let ops = folder(&session, &space, "Ops");
    let deep = nested(&session, &space, &ops, "Deep");
    let kept = folder(&session, &space, "Kept");
    let mut held = vec![
        filed(&session, &space, &ops),
        filed(&session, &space, &deep),
    ];
    let other = filed(&session, &space, &kept);
    let loose = session
        .call(|e| create_note(e, snippet(&space)))
        .unwrap()
        .note
        .id;

    let trashed = session
        .call(|e| delete_folder(e, &ops, PreviewLines::default(), HeldNotes::Trash))
        .unwrap();

    assert_eq!(trashed, 2);
    let left: Vec<String> = session
        .call(|e| list_folders(e, Some(&space)))
        .unwrap()
        .into_iter()
        .map(|folder| folder.id)
        .collect();
    assert_eq!(left, std::slice::from_ref(&kept));
    let mut in_trash: Vec<String> = session
        .call(|e| list_trash(e, ""))
        .unwrap()
        .into_iter()
        .map(|row| row.note.id)
        .collect();
    in_trash.sort();
    held.sort();
    assert_eq!(in_trash, held);
    assert!(session.call(|e| get_note(e, &other)).is_ok());
    assert!(session.call(|e| get_note(e, &loose)).is_ok());

    session.call(|e| restore_notes(e, &held)).unwrap();
    for id in &held {
        let back = session.call(|e| get_note(e, id)).unwrap();
        assert_eq!(back.note.space_id, space);
        assert_eq!(back.note.folder_id, None, "loose in its space");
    }
}
