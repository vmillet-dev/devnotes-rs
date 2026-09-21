//! The folder as data: a region of a space that notes are filed into, and that never
//! takes a note with it when it goes.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use devnotes_lib::db::{Library, iso8601, open_in_memory};
use devnotes_lib::error::StorageError;
use devnotes_lib::folders::model::{FolderColour, NoteFiling};
use devnotes_lib::folders::store::{
    create, delete, file_many, list, recolour, rename, restore_filings,
};
use devnotes_lib::notes::checklist::NoteKind;
use devnotes_lib::notes::language::Language;
use devnotes_lib::notes::model::{Note, NoteDraft, NoteLifecycle, NotePatch};
use devnotes_lib::notes::store::{create as create_note, update};
use devnotes_lib::spaces::store as spaces;

fn at(iso: &str) -> DateTime<Utc> {
    iso8601::parse(iso).expect("tests write valid instants")
}

fn t0() -> DateTime<Utc> {
    at("2026-07-25T09:00:00.000Z")
}

fn t1() -> DateTime<Utc> {
    at("2026-07-25T10:00:00.000Z")
}

fn space(connection: &mut Library, name: &str) -> String {
    spaces::create(connection, name).unwrap().id
}

fn draft(space_id: &str) -> NoteDraft {
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

fn note_in(connection: &mut Library, space_id: &str) -> Note {
    create_note(connection, draft(space_id), t0()).unwrap()
}

fn folder_of(connection: &mut Library, note_id: &str) -> Option<String> {
    use devnotes_lib::db::schema::notes;
    use diesel::prelude::*;

    notes::table
        .find(note_id)
        .select(notes::folder_id)
        .first::<Option<String>>(connection.db())
        .unwrap()
}

fn updated_at_of(connection: &mut Library, note_id: &str) -> String {
    use devnotes_lib::db::schema::notes;
    use diesel::prelude::*;

    notes::table
        .find(note_id)
        .select(notes::updated_at)
        .first::<String>(connection.db())
        .unwrap()
}

fn names(connection: &mut Library, space_id: &str) -> Vec<String> {
    list(connection, Some(space_id))
        .unwrap()
        .into_iter()
        .map(|folder| folder.name)
        .collect()
}

#[test]
fn a_created_folder_is_listed_back_inside_its_space() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let other = space(&mut connection, "Veille");

    let created = create(&mut connection, &sql, "Perf", t0()).unwrap();
    create(&mut connection, &other, "Liens", t0()).unwrap();

    let listed = list(&mut connection, Some(&sql)).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);
    assert_eq!(listed[0].name, "Perf");
    assert_eq!(listed[0].space_id, sql);
}

/// Reading order, which is the order the board lays its zones out in — and which a
/// rename must not reshuffle.
#[test]
fn folders_come_back_in_the_order_they_were_made() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");

    create(
        &mut connection,
        &sql,
        "Migrations",
        at("2026-07-25T09:00:00.000Z"),
    )
    .unwrap();
    create(
        &mut connection,
        &sql,
        "Perf",
        at("2026-07-25T09:30:00.000Z"),
    )
    .unwrap();
    let reporting = create(
        &mut connection,
        &sql,
        "Reporting",
        at("2026-07-25T10:00:00.000Z"),
    )
    .unwrap();

    rename(&mut connection, &reporting.id, "Analytics").unwrap();

    assert_eq!(
        names(&mut connection, &sql),
        ["Migrations", "Perf", "Analytics"]
    );
}

/// Assigned rather than chosen: drawing a folder stays one gesture, and two made back
/// to back must not come out the same colour.
#[test]
fn the_palette_rotates_so_neighbours_differ() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");

    let first = create(&mut connection, &sql, "Migrations", t0()).unwrap();
    let second = create(&mut connection, &sql, "Perf", t0()).unwrap();

    assert_ne!(first.colour, second.colour);
    assert_eq!(first.colour, FolderColour::nth(0));
    assert_eq!(second.colour, FolderColour::nth(1));
}

/// Each space starts the rotation over: its own board is what a colour tells apart.
#[test]
fn the_rotation_is_counted_inside_one_space() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let veille = space(&mut connection, "Veille");

    create(&mut connection, &sql, "Migrations", t0()).unwrap();
    create(&mut connection, &sql, "Perf", t0()).unwrap();
    let elsewhere = create(&mut connection, &veille, "Liens", t0()).unwrap();

    assert_eq!(elsewhere.colour, FolderColour::nth(0));
}

#[test]
fn a_folder_can_be_given_another_colour() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let folder = create(&mut connection, &sql, "Perf", t0()).unwrap();

    let recoloured = recolour(&mut connection, &folder.id, FolderColour::Red).unwrap();

    assert_eq!(recoloured.colour, FolderColour::Red);
    assert_eq!(
        list(&mut connection, Some(&sql)).unwrap()[0].colour,
        FolderColour::Red
    );
}

#[test]
fn two_folders_of_one_space_cannot_share_a_name_whatever_the_case() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    create(&mut connection, &sql, "Perf", t0()).unwrap();

    let refused = create(&mut connection, &sql, "perf", t0());

    assert!(matches!(refused, Err(StorageError::DuplicateFolderName(_))));
    assert_eq!(names(&mut connection, &sql).len(), 1);
}

/// Uniqueness is per space, not per library: two spaces both wanting a "Perf" is the
/// normal case, not a collision.
#[test]
fn the_same_name_is_free_again_in_another_space() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let veille = space(&mut connection, "Veille");
    create(&mut connection, &sql, "Perf", t0()).unwrap();

    assert!(create(&mut connection, &veille, "Perf", t0()).is_ok());
}

#[test]
fn correcting_the_case_of_a_name_is_not_a_duplicate_of_itself() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let folder = create(&mut connection, &sql, "perf", t0()).unwrap();

    let renamed = rename(&mut connection, &folder.id, "Perf").unwrap();

    assert_eq!(renamed.name, "Perf");
}

#[test]
fn a_folder_of_an_unknown_space_is_refused() {
    let mut connection = open_in_memory().unwrap();

    let refused = create(&mut connection, "nowhere", "Perf", t0());

    assert!(matches!(refused, Err(StorageError::SpaceNotFound(_))));
}

#[test]
fn filing_a_note_answers_where_it_came_from() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);

    let filed = file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    assert_eq!(
        filed,
        [NoteFiling {
            note_id: note.id.clone(),
            folder_id: None
        }]
    );
    assert_eq!(folder_of(&mut connection, &note.id), Some(perf.id));
}

#[test]
fn filing_with_no_folder_takes_a_note_back_out() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    let unfiled = file_many(&mut connection, std::slice::from_ref(&note.id), None, t1()).unwrap();

    assert_eq!(unfiled[0].folder_id, Some(perf.id));
    assert_eq!(folder_of(&mut connection, &note.id), None);
}

/// A batch reports what it actually changed, so the undo cannot unfile a note the batch
/// never touched.
#[test]
fn a_note_already_in_the_folder_is_not_reported_as_filed() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
    let already = note_in(&mut connection, &sql);
    let moving = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&already.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    let filed = file_many(
        &mut connection,
        &[already.id.clone(), moving.id.clone()],
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    assert_eq!(filed.len(), 1);
    assert_eq!(filed[0].note_id, moving.id);
}

/// A folder belongs to one space, so filing across spaces would show a chip the space
/// switcher can never reach.
#[test]
fn a_note_of_another_space_is_left_where_it_is() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let veille = space(&mut connection, "Veille");
    let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
    let elsewhere = note_in(&mut connection, &veille);

    let filed = file_many(
        &mut connection,
        std::slice::from_ref(&elsewhere.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    assert!(filed.is_empty());
    assert_eq!(folder_of(&mut connection, &elsewhere.id), None);
}

#[test]
fn undoing_a_filing_puts_every_note_back_where_it_was() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let migrations = create(&mut connection, &sql, "Migrations", t0()).unwrap();
    let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
    let loose = note_in(&mut connection, &sql);
    let filed = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&filed.id),
        Some(&migrations.id),
        t1(),
    )
    .unwrap();

    let batch = file_many(
        &mut connection,
        &[loose.id.clone(), filed.id.clone()],
        Some(&perf.id),
        t1(),
    )
    .unwrap();
    restore_filings(&mut connection, &batch).unwrap();

    assert_eq!(folder_of(&mut connection, &loose.id), None);
    assert_eq!(folder_of(&mut connection, &filed.id), Some(migrations.id));
}

/// ⚠️ Undoing is not editing, and the canvas sorts on that column.
#[test]
fn undoing_a_filing_leaves_the_instant_alone() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);

    let batch = file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();
    restore_filings(&mut connection, &batch).unwrap();

    assert_eq!(
        updated_at_of(&mut connection, &note.id),
        iso8601::format(t1())
    );
}

/// The one thing a folder must never do.
#[test]
fn deleting_a_folder_leaves_its_notes_loose_rather_than_deleting_them() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    delete(&mut connection, &perf.id).unwrap();

    assert!(list(&mut connection, Some(&sql)).unwrap().is_empty());
    assert_eq!(folder_of(&mut connection, &note.id), None);
}

#[test]
fn deleting_an_unknown_folder_says_so_rather_than_answering_ok() {
    let mut connection = open_in_memory().unwrap();

    assert!(matches!(
        delete(&mut connection, "nowhere"),
        Err(StorageError::FolderNotFound(_))
    ));
}

/// The folders go with their space through the cascade; their notes went to the refuge
/// first, and come out of it loose.
#[test]
fn deleting_a_space_takes_its_folders_and_unfiles_the_notes_it_hands_over() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let refuge = space(&mut connection, "Veille");
    let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    spaces::delete(&mut connection, &sql, &refuge).unwrap();

    assert!(list(&mut connection, Some(&sql)).unwrap().is_empty());
    assert_eq!(folder_of(&mut connection, &note.id), None);
}

/// ⚠️ The chip would otherwise name a folder the space switcher can never reach.
#[test]
fn moving_a_note_to_another_space_takes_it_out_of_its_folder() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let veille = space(&mut connection, "Veille");
    let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    let patch = NotePatch {
        space_id: Some(veille.clone()),
        ..NotePatch::default()
    };
    update(&mut connection, &note.id, &patch, t1()).unwrap();

    assert_eq!(folder_of(&mut connection, &note.id), None);
}

/// Editing a note is not refiling it.
#[test]
fn a_patch_that_says_nothing_about_the_space_leaves_the_folder_alone() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    let patch = NotePatch {
        title: Some("Autre".to_string()),
        ..NotePatch::default()
    };
    update(&mut connection, &note.id, &patch, t1()).unwrap();

    assert_eq!(folder_of(&mut connection, &note.id), Some(perf.id));
}

/// A note created inside a folder arrives already filed — one of the two places that do.
#[test]
fn a_draft_can_name_the_folder_it_is_born_in() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();

    let born = create_note(
        &mut connection,
        NoteDraft {
            folder_id: Some(perf.id.clone()),
            ..draft(&sql)
        },
        t0(),
    )
    .unwrap();

    assert_eq!(born.folder_id, Some(perf.id));
}

mod board {
    use super::*;

    use devnotes_lib::folders::board::{self, BoardFrame, BoardPoint, BoardQuery, BoardView};
    use devnotes_lib::folders::store::board as geometry;
    use devnotes_lib::notes::store::fetch;
    use devnotes_lib::notes::view::{NoteFilter, NotesQuery};

    fn request(space_id: &str) -> BoardQuery {
        BoardQuery {
            space_id: space_id.to_string(),
            search: String::new(),
            filter: NoteFilter::All,
            tags: Vec::new(),
            languages: Vec::new(),
            now: t1(),
        }
    }

    /// What `board_view` does, minus the decoration passes that need a connection.
    fn view(connection: &mut Library, query: &BoardQuery) -> BoardView {
        let folders = list(connection, Some(&query.space_id)).unwrap();
        let (notes, facets) = fetch(
            connection,
            &NotesQuery {
                space_id: Some(query.space_id.clone()),
                folder_id: None,
                search: String::new(),
                filter: NoteFilter::All,
                tags: Vec::new(),
                languages: Vec::new(),
                now: query.now,
                tz_offset_minutes: 0,
                pinned_first: true,
            },
        )
        .unwrap();

        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut loose_ids: Vec<String> = Vec::new();
        for note in &notes {
            match &note.folder_id {
                Some(id) => *counts.entry(id.clone()).or_default() += 1,
                None => loose_ids.push(note.id.clone()),
            }
        }

        let folder_ids: Vec<String> = folders.iter().map(|folder| folder.id.clone()).collect();
        let (frames, positions) = geometry::geometry(
            connection,
            &query.space_id,
            &folder_ids,
            &counts,
            &loose_ids,
        )
        .unwrap();

        board::build(notes, folders, &frames, &positions, facets, query)
    }

    fn titled(connection: &mut Library, space_id: &str, title: &str) -> Note {
        create_note(
            connection,
            NoteDraft {
                title: title.to_string(),
                ..draft(space_id)
            },
            t0(),
        )
        .unwrap()
    }

    #[test]
    fn a_board_draws_every_folder_as_a_zone_holding_its_notes() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        // ⚠️ Distinct instants: `list` orders by `(created_at, id)`, so two folders made
        // in the same millisecond fall back to their UUIDs — stable across launches, which
        // is what the board needs, but not something a test can name.
        let migrations = create(&mut connection, &sql, "Migrations", t0()).unwrap();
        create(&mut connection, &sql, "Perf", t1()).unwrap();
        let filed = note_in(&mut connection, &sql);
        note_in(&mut connection, &sql);
        file_many(
            &mut connection,
            std::slice::from_ref(&filed.id),
            Some(&migrations.id),
            t1(),
        )
        .unwrap();

        let board = view(&mut connection, &request(&sql));

        assert_eq!(board.zones.len(), 2);
        assert_eq!(board.zones[0].folder.name, "Migrations");
        assert_eq!(board.zones[0].notes.len(), 1);
        assert!(board.zones[1].notes.is_empty());
        assert_eq!(board.loose.len(), 1);
    }

    /// A filed card flows inside its zone; only a loose one carries a place of its own.
    #[test]
    fn a_filed_card_has_no_position_and_a_loose_one_does() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
        let filed = note_in(&mut connection, &sql);
        note_in(&mut connection, &sql);
        file_many(
            &mut connection,
            std::slice::from_ref(&filed.id),
            Some(&perf.id),
            t1(),
        )
        .unwrap();

        let board = view(&mut connection, &request(&sql));

        assert!(board.zones[0].notes[0].position.is_none());
        assert!(board.loose[0].position.is_some());
    }

    /// The board would look shuffled at every launch otherwise.
    #[test]
    fn the_first_layout_is_written_down_and_read_back_unchanged() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        create(&mut connection, &sql, "Migrations", t0()).unwrap();
        create(&mut connection, &sql, "Perf", t0()).unwrap();
        note_in(&mut connection, &sql);

        let first = view(&mut connection, &request(&sql));
        let second = view(&mut connection, &request(&sql));

        assert_eq!(
            first
                .zones
                .iter()
                .map(|zone| zone.frame)
                .collect::<Vec<_>>(),
            second
                .zones
                .iter()
                .map(|zone| zone.frame)
                .collect::<Vec<_>>(),
        );
        assert_eq!(first.loose[0].position, second.loose[0].position);
    }

    /// A frame the user moved is theirs; a later read must not lay it out again.
    #[test]
    fn a_stored_frame_survives_the_next_read() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
        view(&mut connection, &request(&sql));

        let moved = BoardFrame {
            x: 700,
            y: 500,
            width: 300,
            height: 200,
        };
        connection
            .transaction(|connection, _vault| geometry::set_frame(connection, &perf.id, moved))
            .unwrap();

        assert_eq!(view(&mut connection, &request(&sql)).zones[0].frame, moved);
    }

    #[test]
    fn a_folder_added_later_lands_beside_the_others_rather_than_on_top() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        create(&mut connection, &sql, "Migrations", t0()).unwrap();
        let first = view(&mut connection, &request(&sql)).zones[0].frame;

        create(&mut connection, &sql, "Perf", t1()).unwrap();
        let board = view(&mut connection, &request(&sql));

        assert_eq!(board.zones[0].frame, first);
        assert_ne!(board.zones[1].frame, first);
    }

    /// Dimmed, never dropped: reflowing the survivors throws away the only thing the board
    /// has that the date view does not.
    #[test]
    fn a_search_dims_what_it_does_not_match_and_removes_nothing() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
        let wanted = titled(&mut connection, &sql, "Index sur orders");
        titled(&mut connection, &sql, "Dump nocturne");
        file_many(
            &mut connection,
            std::slice::from_ref(&wanted.id),
            Some(&perf.id),
            t1(),
        )
        .unwrap();

        let board = view(
            &mut connection,
            &BoardQuery {
                search: "orders".to_string(),
                ..request(&sql)
            },
        );

        assert_eq!(board.zones[0].notes.len(), 1);
        assert_eq!(board.loose.len(), 1);
        assert!(board.zones[0].notes[0].matches);
        assert!(!board.loose[0].matches);
        assert_eq!(board.matched, 1);
        assert!(board.is_filtering);
    }

    /// Accents fold both ways here too: it is `notes::view`'s own matcher, not a second one.
    #[test]
    fn the_board_search_folds_accents_like_the_canvas_does() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        titled(&mut connection, &sql, "Étape suivante");

        let board = view(
            &mut connection,
            &BoardQuery {
                search: "etape".to_string(),
                ..request(&sql)
            },
        );

        assert!(board.loose[0].matches);
    }

    #[test]
    fn a_quick_filter_dims_rather_than_narrows_too() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        create_note(
            &mut connection,
            NoteDraft {
                pinned: true,
                ..draft(&sql)
            },
            t0(),
        )
        .unwrap();
        create_note(&mut connection, draft(&sql), t0()).unwrap();

        let board = view(
            &mut connection,
            &BoardQuery {
                filter: NoteFilter::Pinned,
                ..request(&sql)
            },
        );

        assert_eq!(board.loose.len(), 2);
        assert_eq!(board.matched, 1);
    }

    /// Filing a card into a zone drops the place it had on the background.
    #[test]
    fn filing_a_loose_note_forgets_where_it_sat() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
        let note = note_in(&mut connection, &sql);
        view(&mut connection, &request(&sql));

        let placed = connection
            .transaction(|connection, _vault| geometry::positions(connection, &sql))
            .unwrap();
        assert!(placed.contains_key(&note.id));

        file_many(
            &mut connection,
            std::slice::from_ref(&note.id),
            Some(&perf.id),
            t1(),
        )
        .unwrap();

        let after = connection
            .transaction(|connection, _vault| geometry::positions(connection, &sql))
            .unwrap();
        assert!(!after.contains_key(&note.id));
    }

    #[test]
    fn a_stored_position_is_what_a_later_read_hands_back() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let note = note_in(&mut connection, &sql);
        view(&mut connection, &request(&sql));

        let chosen = BoardPoint { x: 640, y: 480 };
        connection
            .transaction(|connection, _vault| geometry::set_position(connection, &note.id, chosen))
            .unwrap();

        assert_eq!(
            view(&mut connection, &request(&sql)).loose[0].position,
            Some(chosen)
        );
    }

    /// ⚠️ The report: a note captured from the clipboard was written under a card that was
    /// already on the board, and had to be dragged off to be found. A note created now is
    /// the most recently updated, so it arrives first in the list and used to be handed the
    /// seat its index gave it — seat zero, where the board's first read had put another.
    #[test]
    fn a_note_created_later_is_placed_beside_the_cards_rather_than_on_one() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        titled(&mut connection, &sql, "Dump nocturne");
        let already_there = view(&mut connection, &request(&sql)).loose[0].position;

        create_note(
            &mut connection,
            NoteDraft {
                title: "Collé du presse-papiers".to_string(),
                ..draft(&sql)
            },
            t1(),
        )
        .unwrap();

        let board = view(&mut connection, &request(&sql));
        let places: Vec<_> = board.loose.iter().map(|entry| entry.position).collect();

        assert_eq!(places.len(), 2);
        assert!(
            places.contains(&already_there),
            "the card already there did not move"
        );
        assert_ne!(places[0], places[1], "two cards were written to one place");
    }

    /// Every unplaced card in the same pass, not just the first one.
    #[test]
    fn a_first_read_of_several_loose_notes_gives_each_its_own_place() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        for title in ["Un", "Deux", "Trois", "Quatre", "Cinq"] {
            titled(&mut connection, &sql, title);
        }

        let board = view(&mut connection, &request(&sql));
        let mut places: Vec<_> = board
            .loose
            .iter()
            .map(|entry| entry.position.expect("a loose card is always placed"))
            .map(|at| (at.x, at.y))
            .collect();
        places.sort_unstable();
        places.dedup();

        assert_eq!(places.len(), 5);
    }

    #[test]
    fn the_surface_grows_to_hold_whatever_is_furthest_out() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let note = note_in(&mut connection, &sql);
        view(&mut connection, &request(&sql));

        connection
            .transaction(|connection, _vault| {
                geometry::set_position(connection, &note.id, BoardPoint { x: 3000, y: 2000 })
            })
            .unwrap();

        let board = view(&mut connection, &request(&sql));
        assert!(board.width > 3000);
        assert!(board.height > 2000);
    }

    /// A board of one space says nothing about the next one.
    #[test]
    fn a_board_never_reaches_into_another_space() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let veille = space(&mut connection, "Veille");
        create(&mut connection, &sql, "Perf", t0()).unwrap();
        create(&mut connection, &veille, "Liens", t0()).unwrap();
        note_in(&mut connection, &veille);

        let board = view(&mut connection, &request(&sql));

        assert_eq!(board.zones.len(), 1);
        assert!(board.loose.is_empty());
    }
}

mod gesture {
    use super::*;

    use devnotes_lib::folders::board::{
        BoardFrame, BoardPoint, CardPlacement, MIN_ZONE_HEIGHT, MIN_ZONE_WIDTH, ZonePlacement,
        clamp, clamp_point, zone_at,
    };
    use devnotes_lib::folders::store::board as geometry;

    fn frames(connection: &mut Library, space_id: &str) -> HashMap<String, BoardFrame> {
        connection
            .transaction(|connection, _vault| geometry::frames(connection, space_id))
            .unwrap()
    }

    fn positions(connection: &mut Library, space_id: &str) -> HashMap<String, BoardPoint> {
        connection
            .transaction(|connection, _vault| geometry::positions(connection, space_id))
            .unwrap()
    }

    #[test]
    fn a_moved_zone_and_a_moved_card_are_written_as_one_batch() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
        let note = note_in(&mut connection, &sql);

        let frame = BoardFrame {
            x: 600,
            y: 320,
            width: 520,
            height: 400,
        };
        let point = BoardPoint { x: 48, y: 720 };
        geometry::save_layout(
            &mut connection,
            &[ZonePlacement {
                folder_id: perf.id.clone(),
                frame,
            }],
            &[CardPlacement {
                note_id: note.id.clone(),
                position: point,
            }],
        )
        .unwrap();

        assert_eq!(frames(&mut connection, &sql).get(&perf.id), Some(&frame));
        assert_eq!(positions(&mut connection, &sql).get(&note.id), Some(&point));
    }

    /// ⚠️ A zone nothing can be dropped into is not a zone. Clamped rather than refused:
    /// answering an error mid-gesture leaves the interface holding a frame nothing stored.
    #[test]
    fn a_zone_squashed_to_nothing_keeps_room_for_one_card() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();

        geometry::save_layout(
            &mut connection,
            &[ZonePlacement {
                folder_id: perf.id.clone(),
                frame: BoardFrame {
                    x: -50,
                    y: -50,
                    width: 4,
                    height: 4,
                },
            }],
            &[],
        )
        .unwrap();

        let stored = frames(&mut connection, &sql);
        let frame = stored.get(&perf.id).unwrap();
        assert_eq!(frame.x, 0);
        assert_eq!(frame.y, 0);
        assert_eq!(frame.width, MIN_ZONE_WIDTH);
        assert_eq!(frame.height, MIN_ZONE_HEIGHT);
    }

    /// ⚠️ A filed card flows inside its zone. Writing a position for one would put back the
    /// row `file_many` had just dropped, and the table would stop meaning "this is loose".
    #[test]
    fn a_card_filed_since_the_drag_is_not_given_a_position_back() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
        let note = note_in(&mut connection, &sql);
        file_many(
            &mut connection,
            std::slice::from_ref(&note.id),
            Some(&perf.id),
            t1(),
        )
        .unwrap();

        geometry::save_layout(
            &mut connection,
            &[],
            &[CardPlacement {
                note_id: note.id.clone(),
                position: BoardPoint { x: 10, y: 10 },
            }],
        )
        .unwrap();

        assert!(!positions(&mut connection, &sql).contains_key(&note.id));
    }

    /// Every zone or none: half a board is a board nobody arranged.
    #[test]
    fn a_batch_naming_a_folder_that_is_gone_writes_nothing_at_all() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
        let note = note_in(&mut connection, &sql);

        let refused = geometry::save_layout(
            &mut connection,
            &[
                ZonePlacement {
                    folder_id: perf.id.clone(),
                    frame: BoardFrame {
                        x: 600,
                        y: 320,
                        width: 520,
                        height: 400,
                    },
                },
                ZonePlacement {
                    folder_id: "gone".to_string(),
                    frame: BoardFrame {
                        x: 0,
                        y: 0,
                        width: 520,
                        height: 400,
                    },
                },
            ],
            &[CardPlacement {
                note_id: note.id.clone(),
                position: BoardPoint { x: 48, y: 720 },
            }],
        );

        assert!(matches!(refused, Err(StorageError::FolderNotFound(_))));
        assert!(frames(&mut connection, &sql).is_empty());
        assert!(positions(&mut connection, &sql).is_empty());
    }

    #[test]
    fn an_empty_batch_is_not_an_error() {
        let mut connection = open_in_memory().unwrap();

        assert!(geometry::save_layout(&mut connection, &[], &[]).is_ok());
    }

    /// Membership comes from the drop, and from nothing else.
    #[test]
    fn a_point_names_the_zone_it_falls_in() {
        let zones = vec![
            (
                "a".to_string(),
                BoardFrame {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 100,
                },
            ),
            (
                "b".to_string(),
                BoardFrame {
                    x: 200,
                    y: 0,
                    width: 100,
                    height: 100,
                },
            ),
        ];

        assert_eq!(
            zone_at(&zones, BoardPoint { x: 50, y: 50 }),
            Some("a".into())
        );
        assert_eq!(
            zone_at(&zones, BoardPoint { x: 250, y: 50 }),
            Some("b".into())
        );
    }

    /// The free background is a legitimate answer: dropping there unfiles the note.
    #[test]
    fn a_point_on_the_background_names_no_zone() {
        let zones = vec![(
            "a".to_string(),
            BoardFrame {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
            },
        )];

        assert_eq!(zone_at(&zones, BoardPoint { x: 150, y: 50 }), None);
        // The far edges belong to the next zone along, not to this one.
        assert_eq!(zone_at(&zones, BoardPoint { x: 100, y: 50 }), None);
        assert_eq!(zone_at(&zones, BoardPoint { x: 0, y: 0 }), Some("a".into()));
    }

    /// Drawn later means drawn on top, so it is what a drop lands in.
    #[test]
    fn overlapping_zones_hand_the_drop_to_the_topmost() {
        let zones = vec![
            (
                "under".to_string(),
                BoardFrame {
                    x: 0,
                    y: 0,
                    width: 200,
                    height: 200,
                },
            ),
            (
                "over".to_string(),
                BoardFrame {
                    x: 50,
                    y: 50,
                    width: 100,
                    height: 100,
                },
            ),
        ];

        assert_eq!(
            zone_at(&zones, BoardPoint { x: 100, y: 100 }),
            Some("over".into())
        );
    }

    #[test]
    fn a_runaway_drag_cannot_put_anything_off_the_board() {
        assert_eq!(clamp_point(BoardPoint { x: -40, y: -40 }).x, 0);
        assert_eq!(
            clamp(BoardFrame {
                x: 10,
                y: 10,
                width: 1_000_000,
                height: 10
            })
            .width,
            devnotes_lib::folders::board::MAX_SIDE
        );
    }
}

/// ⚠️ A chip naming the folder every card is already in is noise, and the breadcrumb above
/// says it once. Same reason the board resolves none.
#[test]
fn a_card_inside_an_opened_folder_carries_no_chip() {
    use devnotes_lib::folders::store::by_id;
    use devnotes_lib::notes::view::{self, NoteFilter, NotesQuery};

    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    let scoped = NotesQuery {
        space_id: Some(sql.clone()),
        folder_id: Some(perf.id.clone()),
        search: String::new(),
        filter: NoteFilter::All,
        tags: Vec::new(),
        languages: Vec::new(),
        now: t1(),
        tz_offset_minutes: 0,
        pinned_first: true,
    };
    let wide = NotesQuery {
        folder_id: None,
        ..scoped.clone()
    };

    // Scoped to the folder: the decoration pass is skipped, exactly as the command does.
    let (notes, facets) = devnotes_lib::notes::store::fetch(&mut connection, &scoped).unwrap();
    let inside = view::build(notes, facets, &scoped);
    assert!(inside.sections[0].notes[0].folder.is_none());

    // Outside it, the chip is what says where the note lives.
    let folders = by_id(&mut connection, Some(&sql)).unwrap();
    let (notes, facets) = devnotes_lib::notes::store::fetch(&mut connection, &wide).unwrap();
    let mut outside = view::build(notes, facets, &wide);
    view::apply_folders(&mut outside, &folders);
    assert_eq!(
        outside.sections[0].notes[0]
            .folder
            .as_ref()
            .map(|folder| folder.name.as_str()),
        Some("Perf")
    );
}
