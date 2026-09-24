use super::*;
use devnotes_lib::notes::revision::{KEEP, Revision};
use devnotes_lib::notes::store::restore_revision;
use devnotes_lib::notes::store::revisions::{compare, content_of, list};

fn body(content: &str) -> NotePatch {
    NotePatch {
        content: Some(content.to_string()),
        ..Default::default()
    }
}

fn snippet(connection: &mut Library) -> String {
    let space_id = space(connection, "Personal");
    create(connection, draft(&space_id), t0()).unwrap().id
}

fn kept(connection: &mut Library, id: &str) -> Vec<Revision> {
    let (db, vault) = connection.split();
    list(db, vault, id).unwrap()
}

fn opened(connection: &mut Library, note_id: &str, revision_id: &str) -> String {
    let (db, vault) = connection.split();
    content_of(db, vault, note_id, revision_id)
        .unwrap()
        .unwrap()
}

/// It is the body before the edit that is worth keeping: the version that worked.
#[test]
fn an_edit_keeps_the_body_it_replaced() {
    let mut connection = open_in_memory().unwrap();
    let id = snippet(&mut connection);

    update(&mut connection, &id, &body("psql -h prod"), t1()).unwrap();

    let history = kept(&mut connection, &id);
    assert_eq!(history.len(), 1);
    assert_eq!(opened(&mut connection, &id, &history[0].id), "Contenu");
}

/// Creating a note writes nothing until the first change worth keeping, and that
/// write is the **title**: the body arrives as a second update, replacing the empty
/// string the row was born with. Without the guard, every note came out of its first
/// editing session already carrying a revision of nothing.
#[test]
fn the_first_body_typed_replaces_nothing_worth_keeping() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let id = create(
        &mut connection,
        NoteDraft {
            content: String::new(),
            ..draft(&space_id)
        },
        t0(),
    )
    .unwrap()
    .id;

    update(&mut connection, &id, &body("select 1"), t1()).unwrap();

    assert!(kept(&mut connection, &id).is_empty());

    // And the one after it is kept, which is the whole point.
    update(&mut connection, &id, &body("select 2"), t1()).unwrap();
    assert_eq!(kept(&mut connection, &id).len(), 1);
}

/// The front end already computes what moved, and the back end agrees: a
/// title-only edit stores no body.
#[test]
fn a_title_only_edit_keeps_nothing() {
    let mut connection = open_in_memory().unwrap();
    let id = snippet(&mut connection);

    update(
        &mut connection,
        &id,
        &NotePatch {
            title: Some("Un autre titre".to_string()),
            ..Default::default()
        },
        t1(),
    )
    .unwrap();

    assert!(kept(&mut connection, &id).is_empty());
}

/// The editor commits on blur *and* on every closing path, so one session writes
/// the same body several times. Without the skip, each write would push a duplicate
/// and rotate a genuinely different version out of the cap.
#[test]
fn writing_the_same_body_again_keeps_one_copy_of_it() {
    let mut connection = open_in_memory().unwrap();
    let id = snippet(&mut connection);

    update(&mut connection, &id, &body("une"), t1()).unwrap();
    update(&mut connection, &id, &body("deux"), t1()).unwrap();
    update(&mut connection, &id, &body("deux"), t1()).unwrap();

    // "Contenu" then "une": the third write moved nothing, and the second was
    // already the newest kept body.
    let history = kept(&mut connection, &id);
    assert_eq!(history.len(), 2);
}

/// A count and not a time window: a body runs to tens of kilobytes.
#[test]
fn only_the_last_few_are_kept() {
    let mut connection = open_in_memory().unwrap();
    let id = snippet(&mut connection);

    for step in 0..KEEP + 5 {
        update(
            &mut connection,
            &id,
            &body(&format!("version {step}")),
            t1(),
        )
        .unwrap();
    }

    assert_eq!(kept(&mut connection, &id).len(), KEEP);
}

#[test]
fn a_kept_body_goes_back_onto_the_note() {
    let mut connection = open_in_memory().unwrap();
    let id = snippet(&mut connection);
    update(&mut connection, &id, &body("casse"), t1()).unwrap();
    let history = kept(&mut connection, &id);

    let note = restore_revision(&mut connection, &id, &history[0].id).unwrap();

    assert_eq!(note.content, "Contenu");
    assert_eq!(
        by_ids(&mut connection, &[id]).unwrap()[0].content,
        "Contenu"
    );
}

/// The canvas sorts on `updated_at`, and putting something back is not editing it
/// — the same line `restore_many`, `restore_placements` and `untag_many` hold.
#[test]
fn putting_a_body_back_does_not_float_the_note_to_the_top() {
    let mut connection = open_in_memory().unwrap();
    let id = snippet(&mut connection);
    update(&mut connection, &id, &body("casse"), t1()).unwrap();
    let before = by_ids(&mut connection, std::slice::from_ref(&id)).unwrap()[0].updated_at;
    let history = kept(&mut connection, &id);

    restore_revision(&mut connection, &id, &history[0].id).unwrap();

    assert_eq!(
        by_ids(&mut connection, &[id]).unwrap()[0].updated_at,
        before
    );
}

/// Going back, not adding a row: nothing of the replaced body stays in the history.
#[test]
fn a_restore_keeps_nothing_of_what_it_replaced() {
    let mut connection = open_in_memory().unwrap();
    let id = snippet(&mut connection);
    update(&mut connection, &id, &body("casse"), t1()).unwrap();
    let history = kept(&mut connection, &id);

    restore_revision(&mut connection, &id, &history[0].id).unwrap();

    assert!(kept(&mut connection, &id).is_empty());
}

/// A → B → C, back to B: C is gone, and so is B from the list, since it is the text now.
#[test]
fn going_back_drops_the_version_and_everything_newer() {
    let mut connection = open_in_memory().unwrap();
    let id = snippet(&mut connection);
    update(&mut connection, &id, &body("B"), t1()).unwrap();
    update(&mut connection, &id, &body("C"), t1()).unwrap();
    let history = kept(&mut connection, &id);
    assert_eq!(opened(&mut connection, &id, &history[0].id), "B");

    let note = restore_revision(&mut connection, &id, &history[0].id).unwrap();

    assert_eq!(note.content, "B");
    let after = kept(&mut connection, &id);
    assert_eq!(after.len(), 1);
    assert_eq!(opened(&mut connection, &id, &after[0].id), "Contenu");
}

/// Going back further takes the newer ones with it, and leaves the older ones.
#[test]
fn going_back_to_the_oldest_leaves_nothing_newer() {
    let mut connection = open_in_memory().unwrap();
    let id = snippet(&mut connection);
    update(&mut connection, &id, &body("B"), t1()).unwrap();
    update(&mut connection, &id, &body("C"), t1()).unwrap();
    let history = kept(&mut connection, &id);

    restore_revision(&mut connection, &id, &history[1].id).unwrap();

    assert!(kept(&mut connection, &id).is_empty());
    assert_eq!(
        by_ids(&mut connection, &[id]).unwrap()[0].content,
        "Contenu"
    );
}

/// The preview compares the kept body with the text it would replace.
#[test]
fn the_preview_compares_the_version_with_the_current_text() {
    let mut connection = open_in_memory().unwrap();
    let id = snippet(&mut connection);
    update(&mut connection, &id, &body("casse"), t1()).unwrap();
    let history = kept(&mut connection, &id);

    let (db, vault) = connection.split();
    let pair = compare(db, vault, &id, &history[0].id).unwrap();

    assert_eq!(pair, Some(("casse".to_string(), "Contenu".to_string())));
}

#[test]
fn a_revision_of_another_note_cannot_be_previewed() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let mine = create(&mut connection, draft(&space_id), t0()).unwrap().id;
    let theirs = create(&mut connection, draft(&space_id), t0()).unwrap().id;
    update(&mut connection, &theirs, &body("le leur"), t1()).unwrap();
    let history = kept(&mut connection, &theirs);

    let (db, vault) = connection.split();
    assert_eq!(compare(db, vault, &mine, &history[0].id).unwrap(), None);
}

/// An id comes from the front end: one note's history must not be reachable
/// through another note.
#[test]
fn a_revision_of_another_note_is_not_reachable() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let mine = create(&mut connection, draft(&space_id), t0()).unwrap().id;
    let theirs = create(&mut connection, draft(&space_id), t0()).unwrap().id;
    update(&mut connection, &theirs, &body("le leur"), t1()).unwrap();
    let history = kept(&mut connection, &theirs);

    let refused = restore_revision(&mut connection, &mine, &history[0].id);

    assert!(matches!(refused, Err(StorageError::RevisionNotFound(_))));
}

/// Said out loud rather than shipped silently: a checklist's items live in
/// `note_items`, a second table to snapshot and a two-step restore. Half the note
/// kinds get nothing from this first version.
#[test]
fn a_checklist_keeps_nothing_yet() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let id = create(
        &mut connection,
        NoteDraft {
            kind: NoteKind::Checklist,
            content: String::new(),
            items: vec![ChecklistItem {
                text: "un item".to_string(),
                done: false,
            }],
            ..draft(&space_id)
        },
        t0(),
    )
    .unwrap()
    .id;

    update(&mut connection, &id, &body("un corps"), t1()).unwrap();

    assert!(kept(&mut connection, &id).is_empty());
}

/// A history of every body in plaintext beside a sealed library would undo the
/// encryption entirely.
#[test]
fn a_kept_body_is_not_readable_in_the_column_it_sits_in() {
    let mut connection = open_in_memory().unwrap();
    let id = snippet(&mut connection);
    update(&mut connection, &id, &body("psql -h prod"), t1()).unwrap();

    let stored: Vec<String> = note_revisions::table
        .select(note_revisions::content)
        .load(connection.db())
        .unwrap();

    assert_eq!(stored.len(), 1);
    assert!(!stored[0].contains("Contenu"));
}

/// Deleting a note takes its history with it: `ON DELETE CASCADE`, which is inert
/// without `PRAGMA foreign_keys` — set per connection in `db::configure`.
#[test]
fn purging_a_note_takes_its_history_with_it() {
    let mut connection = open_in_memory().unwrap();
    let id = snippet(&mut connection);
    update(&mut connection, &id, &body("psql -h prod"), t1()).unwrap();

    trash(&mut connection, &id, t1()).unwrap();
    purge(&mut connection, std::slice::from_ref(&id)).unwrap();

    let left: i64 = note_revisions::table
        .count()
        .get_result(connection.db())
        .unwrap();
    assert_eq!(left, 0);
}
